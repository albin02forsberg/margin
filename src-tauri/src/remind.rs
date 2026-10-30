//! Desktop notifications before timed agenda entries and for upcoming deadlines.

use crate::org::{self, Item, Kw};
use crate::App;
use chrono::{Duration, NaiveDateTime, NaiveTime};
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

#[derive(Debug, Clone, PartialEq)]
pub struct Due {
    /// Starts with the date it belongs to, so old keys can be dropped.
    pub key: String,
    pub title: String,
    pub body: String,
    /// The entry, for opening it from the notification.
    pub path: std::path::PathBuf,
    pub line: usize,
}

/// Reminders that are due at NOW: timed entries (scheduled, deadline, appointment)
/// from BEFORE ahead until they start, and date-only deadlines once a day from
/// their own `-3d` warning period ahead (else WARN_DAYS). Done tasks are skipped. Callers drop keys already sent.
pub fn due(items: &[Item], kw: &Kw, now: NaiveDateTime, before: Duration, warn_days: i64) -> Vec<Due> {
    let mut out = vec![];
    for it in items {
        if it.keyword.as_deref().is_some_and(|k| kw.is_done(k)) {
            continue;
        }
        let at = it.time.as_deref().and_then(|t| NaiveTime::parse_from_str(t.split('-').next()?.trim(), "%H:%M").ok());
        let place = format!("{}:{}", it.path.display(), it.line);
        match (it.kind, at) {
            ("scheduled" | "deadline" | "timestamp", Some(t)) => {
                let at = it.date.and_time(t);
                if at - before <= now && now <= at {
                    let what = if it.kind == "deadline" { "Due" } else { "At" };
                    let (key, body) = (format!("{} {} {place} {t}", it.date, it.kind), format!("{what} {}", t.format("%H:%M")));
                    out.push(Due { key, title: it.title.clone(), body, path: it.path.clone(), line: it.line });
                }
            }
            ("deadline", None) => {
                let days = (it.date - now.date()).num_days();
                if (0..=it.warn.unwrap_or(warn_days)).contains(&days) {
                    let body = match days {
                        0 => "Due today".into(),
                        1 => "Due tomorrow".into(),
                        n => format!("Due in {n} days"),
                    };
                    out.push(Due { key: format!("{} deadline {place}", now.date()), title: it.title.clone(), body, path: it.path.clone(), line: it.line });
                }
            }
            _ => {}
        }
    }
    out
}

/// Check every 30 s and notify; sent keys live in the data folder so a restart doesn't repeat them.
pub fn start(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || loop {
        tick(&app);
        std::thread::sleep(std::time::Duration::from_secs(30));
    });
}

fn tick(app: &AppHandle) {
    let s = app.state::<App>();
    let c = s.cfg();
    let now = crate::timeclock::now();
    let today = now.date();
    focus(app, &s, c.focus_minutes, c.focus_break_minutes, now);
    long_session(app, &s, c.max_session_hours, now);
    if !c.reminders {
        return;
    }
    let files = s.files();
    // Wide enough for the longest per-deadline warning cookie.
    let span = files.iter().flat_map(|f| &f.headlines).filter_map(|h| h.deadline.as_ref().and_then(org::warn_days)).fold(c.deadline_warning_days, i64::max);
    let items = org::agenda(&files, &s.kw(), today, span.max(0) + 2, today);
    let due = due(&items, &s.kw(), now, Duration::minutes(c.remind_before_minutes), c.deadline_warning_days);
    let file = c.data().join("reminders-sent.txt");
    let (mut sent, mut changed) = load_sent(&file, today);
    if once_a_day(&mut sent, today, "day-done", || crate::day_done_now(&s)) {
        changed = true;
        let _ = app.notification().builder().title("Margin").body("Today's hours are done.").show();
    }
    for d in due {
        if sent.insert(d.key.clone()) {
            changed = true;
            notify(app, d);
        }
    }
    // After the normal path, so a date-only deadline it re-sent today is seen and not repeated.
    let kw = s.kw();
    let open = |p: &Path, line: usize| files.iter().filter(|f| f.path == p).flat_map(|f| &f.headlines).any(|h| h.line == line && !h.keyword.as_deref().is_some_and(|k| kw.is_done(k)));
    for d in take_snoozed(&mut lock(&SNOOZED), now, &sent, open) {
        notify(app, d);
    }
    if changed {
        let _ = std::fs::write(&file, sent.into_iter().map(|k| k + "\n").collect::<String>());
    }
}

/// Nudge a break once per block of FOCUS minutes on the running clock (timeclock::focus_due). The
/// nudged block start is kept in memory (a block past midnight) and in focus-sent.txt (a restart).
fn focus(app: &AppHandle, s: &App, focus: i64, rest: i64, now: NaiveDateTime) {
    static LAST: std::sync::Mutex<Option<NaiveDateTime>> = std::sync::Mutex::new(None);
    let Some((since, project, _)) = s.tc().current().filter(|_| focus > 0) else { return };
    let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(body) = focus_nudge(&mut last, &s.cfg().data().join("focus-sent.txt"), since, &project, now, focus, rest) {
        let _ = app.notification().builder().title("Time for a break").body(body).show();
    }
}

/// The break nudge's body when the block starting SINCE is due and not yet nudged (LAST, else FILE);
/// records it in both.
fn focus_nudge(last: &mut Option<NaiveDateTime>, file: &Path, since: NaiveDateTime, project: &str, now: NaiveDateTime, focus: i64, rest: i64) -> Option<String> {
    if last.is_none() {
        *last = std::fs::read_to_string(file).ok().and_then(|t| t.trim().parse().ok());
    }
    if !crate::timeclock::focus_due(since, now, focus, *last) {
        return None;
    }
    *last = Some(since);
    let _ = std::fs::write(file, format!("{since:?}")); // ISO 8601, what parse() reads
    let what = if project.is_empty() { String::new() } else { format!(" on {project}") };
    Some(format!("{}{what}. Take {rest} min? Pause is in the top-bar clock.", crate::timeclock::format_hm((now - since).num_seconds() as f64 / 3600.0)))
}

/// Ask once per session when the running clock passes MAX hours (timeclock::long_session_due), like
/// focus(): the notified session start is kept in memory and in long-sent.txt (a restart).
fn long_session(app: &AppHandle, s: &App, max: f64, now: NaiveDateTime) {
    static LAST: std::sync::Mutex<Option<NaiveDateTime>> = std::sync::Mutex::new(None);
    let Some((since, project, _)) = s.tc().current().filter(|_| max > 0.0) else { return };
    let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(body) = long_nudge(&mut last, &s.cfg().data().join("long-sent.txt"), since, &project, now, max) {
        let _ = app.notification().builder().title("Margin").body(body).show();
    }
}

/// The long-session notification's body when the session starting SINCE is due and not yet notified
/// (LAST, else FILE); records it in both.
fn long_nudge(last: &mut Option<NaiveDateTime>, file: &Path, since: NaiveDateTime, project: &str, now: NaiveDateTime, max: f64) -> Option<String> {
    if last.is_none() {
        *last = std::fs::read_to_string(file).ok().and_then(|t| t.trim().parse().ok());
    }
    if !crate::timeclock::long_session_due(since, now, max, *last) {
        return None;
    }
    *last = Some(since);
    let _ = std::fs::write(file, format!("{since:?}"));
    let what = if project.is_empty() { String::new() } else { format!(" on {project}") };
    Some(format!("Still working{what}? Clock has run {}", crate::timeclock::format_hm((now - since).num_seconds() as f64 / 3600.0)))
}

/// Sent reminder keys from FILE, without those dated before TODAY; and whether any were dropped.
fn load_sent(file: &Path, today: chrono::NaiveDate) -> (BTreeSet<String>, bool) {
    let mut sent: BTreeSet<String> = std::fs::read_to_string(file).unwrap_or_default().lines().map(str::to_string).collect();
    let before = sent.len();
    sent.retain(|k| k.get(..10).and_then(|d| d.parse::<chrono::NaiveDate>().ok()).is_some_and(|d| d >= today));
    let changed = sent.len() != before;
    (sent, changed)
}

/// Whether the once-a-day event NAME should fire now (COND holds and it was not sent today); records it in SENT.
fn once_a_day(sent: &mut BTreeSet<String>, today: chrono::NaiveDate, name: &str, cond: impl FnOnce() -> bool) -> bool {
    let key = format!("{today} {name}");
    !sent.contains(&key) && cond() && sent.insert(key)
}

/// The last reminder shown, for "snooze"; and snoozed ones with the time they fire again (lost on restart).
static SHOWN: Mutex<Option<Due>> = Mutex::new(None);
static SNOOZED: Mutex<Vec<(NaiveDateTime, Due)>> = Mutex::new(vec![]);

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// When a snooze of MINUTES from NOW ends; None means tomorrow 9:00.
fn snooze_until(now: NaiveDateTime, minutes: Option<i64>) -> NaiveDateTime {
    minutes.map_or_else(|| (now.date() + Duration::days(1)).and_hms_opt(9, 0, 0).unwrap(), |m| now + Duration::minutes(m))
}

/// Take the snoozed reminders whose time has come. Dropped instead: tasks no longer OPEN (done or gone),
/// and ones from an earlier day that the normal path already SENT again today (date-only deadlines).
fn take_snoozed(q: &mut Vec<(NaiveDateTime, Due)>, now: NaiveDateTime, sent: &BTreeSet<String>, open: impl Fn(&Path, usize) -> bool) -> Vec<Due> {
    let today = now.date().to_string();
    let (due, rest) = std::mem::take(q).into_iter().partition::<Vec<_>, _>(|(at, _)| *at <= now);
    *q = rest;
    due.into_iter().map(|(_, d)| d).filter(|d| open(&d.path, d.line) && !(d.key.get(..10).is_some_and(|k| k < today.as_str()) && sent.contains(&format!("{today}{}", &d.key[10..])))).collect()
}

/// Queue D to fire again after MINUTES (None: tomorrow 9:00); the confirmation to show.
fn snooze(d: Due, minutes: Option<i64>) -> String {
    let at = snooze_until(crate::timeclock::now(), minutes);
    let msg = format!("Snoozed \"{}\" until {}", d.title, at.format(if minutes.is_some() { "%H:%M" } else { "tomorrow %H:%M" }));
    lock(&SNOOZED).push((at, d));
    msg
}

/// Snooze the last reminder shown; None when there is none (or it was snoozed already).
#[tauri::command]
pub fn remind_snooze(minutes: Option<i64>) -> Option<String> {
    lock(&SHOWN).take().map(|d| snooze(d, minutes))
}

/// Show D; clicking it opens the entry where the desktop reports clicks (Linux, via D-Bus).
fn notify(app: &AppHandle, d: Due) {
    *lock(&SHOWN) = Some(d.clone());
    #[cfg(target_os = "linux")]
    if let Ok(h) = notify_rust::Notification::new().appname("Margin").summary(&d.title).body(&d.body).action("default", "Open").action("snooze10", "Snooze 10 min").action("snooze60", "Snooze 1 h").show() {
        let app = app.clone();
        std::thread::spawn(move || {
            h.wait_for_action(|a| match a {
                "default" => {
                    crate::tray::show(&app);
                    let _ = tauri::Emitter::emit(&app, "open-entry", serde_json::json!({ "path": d.path, "line": d.line }));
                }
                "snooze10" | "snooze60" => {
                    let _ = lock(&SHOWN).take_if(|s| *s == d); // so the palette can't queue it twice
                    snooze(d, Some(if a == "snooze10" { 10 } else { 60 }));
                }
                _ => {}
            })
        });
        return;
    }
    let _ = app.notification().builder().title(&d.title).body(&d.body).show();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn once_a_day_fires_once() {
        let (mut sent, day) = (BTreeSet::new(), "2026-10-02".parse().unwrap());
        assert!(!once_a_day(&mut sent, day, "day-done", || false));
        assert!(once_a_day(&mut sent, day, "day-done", || true));
        assert!(!once_a_day(&mut sent, day, "day-done", || panic!("not evaluated once sent")));
        assert!(sent.contains("2026-10-02 day-done"));
    }

    #[test]
    fn load_sent_drops_old_keys() {
        let dir = std::env::temp_dir().join(format!("margin-remind-{}", crate::notes::gen_id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("reminders-sent.txt");
        let day = "2026-10-02".parse().unwrap();
        assert_eq!(load_sent(&file, day), (BTreeSet::new(), false));
        std::fs::write(&file, "2026-10-01 day-done\n2026-10-02 day-done\nbad\n").unwrap();
        assert_eq!(load_sent(&file, day), (BTreeSet::from(["2026-10-02 day-done".to_string()]), true));
        std::fs::write(&file, "2026-10-03 deadline /n/a.org:0\n").unwrap();
        assert!(!load_sent(&file, day).1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn focus_nudges_once_per_block_across_restarts() {
        let dir = std::env::temp_dir().join(format!("margin-focus-{}", crate::notes::gen_id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("focus-sent.txt");
        let day: chrono::NaiveDate = "2026-10-02".parse().unwrap();
        let (since, now) = (day.and_hms_opt(9, 0, 0).unwrap(), day.and_hms_opt(9, 50, 0).unwrap());
        let mut last = None;
        assert_eq!(focus_nudge(&mut last, &file, since, "", day.and_hms_opt(9, 20, 0).unwrap(), 45, 5), None);
        assert_eq!(focus_nudge(&mut last, &file, since, "Acme", now, 45, 5).as_deref(), Some("50m on Acme. Take 5 min? Pause is in the top-bar clock."));
        assert_eq!(last, Some(since));
        assert_eq!(focus_nudge(&mut last, &file, since, "Acme", now, 45, 5), None);
        // A restart reads the nudged block back from the file.
        let mut fresh = None;
        assert_eq!(focus_nudge(&mut fresh, &file, since, "", now, 45, 5), None);
        assert_eq!(fresh, Some(since));
        let later = day.and_hms_opt(11, 0, 0).unwrap();
        assert_eq!(focus_nudge(&mut fresh, &file, later, "", day.and_hms_opt(12, 30, 0).unwrap(), 45, 5).as_deref(), Some("1h 30m. Take 5 min? Pause is in the top-bar clock."));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn long_session_notifies_once_across_restarts() {
        let dir = std::env::temp_dir().join(format!("margin-long-{}", crate::notes::gen_id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("long-sent.txt");
        let day: chrono::NaiveDate = "2026-10-02".parse().unwrap();
        let (since, now) = (day.and_hms_opt(8, 0, 0).unwrap(), day.and_hms_opt(12, 12, 0).unwrap());
        let mut last = None;
        assert_eq!(long_nudge(&mut last, &file, since, "Acme", day.and_hms_opt(11, 0, 0).unwrap(), 4.0), None);
        assert_eq!(long_nudge(&mut last, &file, since, "Acme", now, 4.0).as_deref(), Some("Still working on Acme? Clock has run 4h 12m"));
        assert_eq!(long_nudge(&mut last, &file, since, "Acme", now, 4.0), None);
        let mut fresh = None; // a restart reads it back
        assert_eq!(long_nudge(&mut fresh, &file, since, "Acme", now, 4.0), None);
        let later = day.and_hms_opt(13, 0, 0).unwrap(); // resumed or switched: a new session
        assert_eq!(long_nudge(&mut fresh, &file, later, "", day.and_hms_opt(17, 0, 0).unwrap(), 4.0).as_deref(), Some("Still working? Clock has run 4h 00m"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn picks_due_reminders() {
        let kw = Kw { todo: vec!["TODO".into()], done: vec!["DONE".into()] };
        let doc = "* TODO Call Ann\nSCHEDULED: <2026-10-02 Fri 14:00>\n\
                   * Standup\n<2026-10-02 Fri 14:05-14:20>\n\
                   * DONE Old call\nSCHEDULED: <2026-10-02 Fri 14:00>\n\
                   * TODO Report\nDEADLINE: <2026-10-03 Sat>\n\
                   * TODO Later\nDEADLINE: <2026-10-09 Fri>\n";
        let f = Arc::new(org::parse(Path::new("/n/a.org"), doc, &kw));
        let day = "2026-10-02".parse().unwrap();
        let items = org::agenda(&[f], &kw, day, 3, day);
        let at = |t: &str| day.and_time(NaiveTime::parse_from_str(t, "%H:%M").unwrap());
        let titles = |now| due(&items, &kw, now, Duration::minutes(10), 1).into_iter().map(|d| format!("{}: {}", d.title, d.body)).collect::<Vec<_>>();

        assert_eq!(titles(at("13:49")), ["Report: Due tomorrow"]);
        assert_eq!(titles(at("13:55")), ["Call Ann: At 14:00", "Standup: At 14:05", "Report: Due tomorrow"]);
        assert_eq!(titles(at("14:03")), ["Standup: At 14:05", "Report: Due tomorrow"]);
        assert_eq!(titles(at("14:06")), ["Report: Due tomorrow"]);

        let keys: Vec<String> = due(&items, &kw, at("13:55"), Duration::minutes(10), 1).into_iter().map(|d| d.key).collect();
        assert_eq!(keys[0], "2026-10-02 scheduled /n/a.org:0 14:00:00");
        assert!(keys.iter().all(|k| k.starts_with("2026-10-02")));
    }

    #[test]
    fn snoozed_reminders_fire_once_when_due() {
        let day: chrono::NaiveDate = "2026-10-02".parse().unwrap();
        let at = |d: chrono::NaiveDate, h, m| d.and_hms_opt(h, m, 0).unwrap();
        let d = |key: &str, line| Due { key: key.into(), title: "T".into(), body: String::new(), path: "/n/a.org".into(), line };
        assert_eq!(snooze_until(at(day, 23, 55), Some(10)), at(day.succ_opt().unwrap(), 0, 5));
        assert_eq!(snooze_until(at(day, 23, 55), None), at(day.succ_opt().unwrap(), 9, 0));
        assert_eq!(snooze_until(at(day, 0, 5), None), at(day.succ_opt().unwrap(), 9, 0));

        let timed = d("2026-10-02 scheduled /n/a.org:0 14:00:00", 0);
        let mut q = vec![(at(day, 14, 10), timed.clone()), (at(day, 14, 10), d("2026-10-02 scheduled /n/a.org:5 14:00:00", 5))];
        let sent = BTreeSet::new();
        let open = |_: &Path, line| line != 5; // line 5 was marked done meanwhile
        assert!(take_snoozed(&mut q, at(day, 14, 9), &sent, open).is_empty());
        assert_eq!(q.len(), 2);
        assert_eq!(take_snoozed(&mut q, at(day, 14, 10), &sent, open), [timed]);
        assert!(q.is_empty());

        // A date-only deadline snoozed until tomorrow: the normal path re-sent it at midnight, so it is dropped.
        let dl = d("2026-10-02 deadline /n/a.org:0", 0);
        let next = day.succ_opt().unwrap();
        let mut q = vec![(at(next, 9, 0), dl.clone())];
        let sent = BTreeSet::from(["2026-10-03 deadline /n/a.org:0".to_string()]);
        assert!(take_snoozed(&mut q, at(next, 9, 0), &sent, |_, _| true).is_empty());
        // Not re-sent today (no longer in its warning period): it fires.
        let mut q = vec![(at(next, 9, 0), dl.clone())];
        assert_eq!(take_snoozed(&mut q, at(next, 9, 0), &BTreeSet::new(), |_, _| true), [dl]);
    }

    #[test]
    fn deadline_cookie_overrides_warn_days() {
        let kw = Kw { todo: vec!["TODO".into()], done: vec![] };
        let doc = "* TODO Far\nDEADLINE: <2026-10-12 Mon -10d>\n* TODO Near\nDEADLINE: <2026-10-04 -0d>\n* TODO Plain\nDEADLINE: <2026-10-03>\n";
        let f = Arc::new(org::parse(Path::new("/n/a.org"), doc, &kw));
        let day: chrono::NaiveDate = "2026-10-02".parse().unwrap();
        let items = org::agenda(&[f], &kw, day, 12, day);
        let got: Vec<String> = due(&items, &kw, day.and_hms_opt(9, 0, 0).unwrap(), Duration::minutes(10), 1).into_iter().map(|d| format!("{}: {}", d.title, d.body)).collect();
        assert_eq!(got, ["Plain: Due tomorrow", "Far: Due in 10 days"]);
    }
}
