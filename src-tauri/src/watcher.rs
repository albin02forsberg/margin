//! The built-in activity watcher (opt-in: `activity_watcher`): every 5 s the active app and
//! window title go to `<data>/activity/YYYY-MM-DD.jsonl`, so suggestions work without
//! ActivityWatch. The log stays on this machine.

use crate::activity::Span;
use chrono::{Duration, NaiveDate, NaiveDateTime};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;

const POLL_SECS: i64 = 5;
/// No input for this long is away: nothing is recorded (ActivityWatch's default too).
const AFK_MINS: i64 = 3;
/// An unchanged window is still written this often, so a crash loses at most this much.
const MAX_SECS: i64 = 5 * 60;
const MAX_TITLE: usize = 300;

/// The record being grown by the sampler, shared so quitting can write it (`flush`).
static OPEN: Mutex<Option<Rec>> = Mutex::new(None);

/// One line of the log: a window active from START to END.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Rec {
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub app: String,
    pub title: String,
}

/// APP and TITLE as recorded: None when an EXCLUDE pattern matches either, the title cut to 300 chars.
pub fn clean(app: &str, title: &str, exclude: &[Regex]) -> Option<(String, String)> {
    if exclude.iter().any(|r| r.is_match(app) || r.is_match(title)) {
        return None;
    }
    Some((app.to_string(), title.chars().take(MAX_TITLE).collect()))
}

/// One poll at NOW seeing SAMPLE, the open record CUR's window last seen at SEEN (NOW, or
/// earlier after sleep or once away). CUR grows while the window stays the same; otherwise,
/// at midnight or after 5 min it's closed at SEEN and returned for writing.
pub fn step(cur: Option<Rec>, sample: Option<(String, String)>, seen: NaiveDateTime, now: NaiveDateTime) -> (Option<Rec>, Option<Rec>) {
    let same = |r: &&Rec| seen == now && sample.as_ref().is_some_and(|(a, t)| *a == r.app && *t == r.title) && now.date() == r.start.date() && now - r.start < Duration::seconds(MAX_SECS);
    if let Some(r) = cur.as_ref().filter(same) {
        return (Some(Rec { end: now, ..r.clone() }), None);
    }
    let done = cur.map(|r| Rec { end: seen.max(r.start), ..r }).filter(|r| r.end > r.start);
    (sample.map(|(app, title)| Rec { start: now, end: now, app, title }), done)
}

/// The log's lines as spans; malformed ones (a half-written last line) are skipped.
pub fn parse(text: &str) -> Vec<Span> {
    text.lines()
        .filter_map(|l| serde_json::from_str::<Rec>(l).ok())
        .map(|r| Span { start: r.start, end: r.end, app: r.app, title: r.title, url: String::new() })
        .collect()
}

/// DAY's spans from the log in DIR (none if there's no file).
pub fn spans(dir: &Path, day: NaiveDate) -> Vec<Span> {
    parse(&std::fs::read_to_string(dir.join(format!("{day}.jsonl"))).unwrap_or_default())
}

/// Delete DIR's day files more than DAYS before TODAY (DAYS <= 0 keeps everything).
pub fn prune(dir: &Path, today: NaiveDate, days: i64) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    if days <= 0 {
        return;
    }
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        let day = name.strip_suffix(".jsonl").and_then(|d| d.parse::<NaiveDate>().ok());
        if day.is_some_and(|d| (today - d).num_days() > days) {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

fn append(dir: &Path, r: &Rec) -> std::io::Result<()> {
    use std::io::Write;
    std::fs::create_dir_all(dir)?;
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(dir.join(format!("{}.jsonl", r.start.date())))?;
    writeln!(f, "{}", serde_json::to_string(r)?)
}

fn write(dir: &Path, r: &Rec) {
    append(dir, r).unwrap_or_else(|e| eprintln!("activity log {}: {e}", dir.display()))
}

/// Write the open record to DIR, ended when its window was last seen (on quit or when the
/// watcher is turned off); one that never grew is dropped.
pub fn flush(dir: &Path) {
    if let Some(r) = OPEN.lock().unwrap().take().filter(|r| r.end > r.start) {
        write(dir, &r);
    }
}

/// The active window's app and title: X11, Windows, macOS, and KDE or Hyprland under Wayland
/// (elsewhere on Wayland only XWayland windows show).
#[cfg(desktop)]
fn active() -> Option<(String, String)> {
    active_win_pos_rs::get_active_window().ok().map(|w| (w.app_name, w.title))
}

/// Poll every 5 s while `activity_watcher` is on (checked each poll, so the setting applies at
/// once); the idle source is only set up once it's first on.
#[cfg(desktop)]
pub fn start(app: &tauri::AppHandle) {
    use crate::{activity, timeclock::now, App};
    use tauri::Manager;
    let app = app.clone();
    std::thread::spawn(move || {
        let (mut last, mut day, mut idle) = (now(), None, None);
        loop {
            std::thread::sleep(std::time::Duration::from_secs(POLL_SECS as u64));
            let (c, t) = (app.state::<App>().cfg(), now());
            let dir = c.data().join("activity");
            // A bad pattern records nothing rather than what it should have hidden.
            let exclude = activity::exclude_rules(&c.activity_exclude).ok();
            let Some(exclude) = exclude.filter(|_| c.activity_watcher) else {
                flush(&dir);
                last = t;
                continue;
            };
            if day != Some(t.date()) {
                prune(&dir, t.date(), c.activity_retention_days);
                day = Some(t.date());
            }
            let away = idle.get_or_insert_with(|| crate::idle::source(Duration::minutes(AFK_MINS))).as_ref().map_or(Duration::zero(), |f| f());
            let afk = away >= Duration::minutes(AFK_MINS);
            let seen = if t - last > Duration::seconds(3 * POLL_SECS) { last } else if afk { t - away } else { t };
            let sample = if afk { None } else { active().and_then(|(a, title)| clean(&a, &title, &exclude)) };
            let mut open = OPEN.lock().unwrap();
            let (next, done) = step(open.take(), sample, seen, t);
            if let Some(r) = done {
                write(&dir, &r);
            }
            (*open, last) = (next, t);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::activity::{exclude_rules, suggest, Known};

    fn t(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(&format!("2026-10-01 {s}"), "%Y-%m-%d %H:%M:%S").unwrap()
    }
    fn w(app: &str, title: &str) -> Option<(String, String)> {
        Some((app.into(), title.into()))
    }

    /// Needs a desktop session: `cargo test -- --ignored active_window --nocapture`.
    #[test]
    #[ignore]
    fn active_window() {
        let w = active().expect("no active window");
        println!("{w:?}");
        assert!(!w.0.is_empty() || !w.1.is_empty());
    }

    #[test]
    fn samples_merge_and_close() {
        let r = |s: &str, e: &str, app: &str| Rec { start: t(s), end: t(e), app: app.into(), title: "x".into() };
        let (cur, done) = step(None, w("Code", "x"), t("09:00:00"), t("09:00:00"));
        assert_eq!((cur.clone(), done), (Some(r("09:00:00", "09:00:00", "Code")), None));
        let (cur, done) = step(cur, w("Code", "x"), t("09:00:05"), t("09:00:05")); // same window: grows
        assert_eq!((cur.clone(), done), (Some(r("09:00:00", "09:00:05", "Code")), None));
        let (cur, done) = step(cur, w("Firefox", "x"), t("09:00:10"), t("09:00:10")); // changed: written
        assert_eq!((cur.clone(), done), (Some(r("09:00:10", "09:00:10", "Firefox")), Some(r("09:00:00", "09:00:10", "Code"))));
        // Woke from sleep: closed when last seen, a new one starts now.
        let (cur, done) = step(cur, w("Firefox", "x"), t("09:00:15"), t("10:00:00"));
        assert_eq!((cur.clone(), done), (Some(r("10:00:00", "10:00:00", "Firefox")), Some(r("09:00:10", "09:00:15", "Firefox"))));
        // Away (or an excluded window): closed, nothing open; a record that never grew isn't written.
        assert_eq!(step(cur, None, t("10:00:00"), t("10:03:00")), (None, None));
        // Written every 5 min even when nothing changes.
        let long = Some(r("09:00:00", "09:04:58", "Code"));
        assert_eq!(step(long, w("Code", "x"), t("09:05:03"), t("09:05:03")), (Some(r("09:05:03", "09:05:03", "Code")), Some(r("09:00:00", "09:05:03", "Code"))));
        // Midnight starts a new day's record.
        let late = Rec { start: t("23:59:50"), end: t("23:59:58"), app: "Code".into(), title: "x".into() };
        let next = t("00:00:00") + Duration::days(1);
        assert_eq!(step(Some(late), w("Code", "x"), next, next).0.unwrap().start, next);
    }

    #[test]
    fn flush_writes_the_open_record_once() {
        let dir = std::env::temp_dir().join(format!("margin-flush-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let r = Rec { start: t("09:00:00"), end: t("09:03:20"), app: "Code".into(), title: "x".into() };
        *OPEN.lock().unwrap() = Some(r.clone());
        flush(&dir);
        flush(&dir);
        let text = std::fs::read_to_string(dir.join("2026-10-01.jsonl")).unwrap();
        assert_eq!(text.lines().map(|l| serde_json::from_str::<Rec>(l).unwrap()).collect::<Vec<_>>(), vec![r]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn exclusion_and_truncation_before_write() {
        let ex = exclude_rules(&["keepass".into(), "Private Browsing".into()]).unwrap();
        assert_eq!(clean("KeePassXC", "bank.kdbx", &ex), None);
        assert_eq!(clean("firefox", "Bank — Private Browsing", &ex), None);
        assert_eq!(clean("Code", &"é".repeat(400), &ex).unwrap().1.chars().count(), 300);
    }

    #[test]
    fn log_to_suggestions_and_retention() {
        let dir = std::env::temp_dir().join(format!("margin-watcher-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let rec = |s: &str, e: &str, app: &str, title: &str| Rec { start: t(s), end: t(e), app: app.into(), title: title.into() };
        append(&dir, &rec("09:00:00", "09:20:00", "Code", "lib.rs - margin")).unwrap();
        append(&dir, &rec("09:21:00", "09:40:00", "firefox", "Margin issues")).unwrap();
        std::fs::OpenOptions::new().append(true).open(dir.join("2026-10-01.jsonl")).and_then(|mut f| std::io::Write::write_all(&mut f, b"{\"start\":\"2026")).unwrap();
        let day = t("00:00:00").date();
        let spans = spans(&dir, day);
        assert_eq!(spans.len(), 2, "the half-written line is skipped");
        let projects = vec!["Margin".to_string()];
        let s = suggest(&spans, &[], &[], &[], Known { projects: &projects, ..Default::default() }, day);
        assert_eq!(s.iter().map(|s| (s.start, s.end, s.project.as_deref())).collect::<Vec<_>>(), vec![(t("09:00:00"), t("09:40:00"), Some("Margin"))]);
        std::fs::write(dir.join("2026-08-31.jsonl"), "").unwrap();
        std::fs::write(dir.join("2026-09-01.jsonl"), "").unwrap();
        std::fs::write(dir.join("notes.txt"), "").unwrap();
        prune(&dir, day, 0);
        assert!(dir.join("2026-08-31.jsonl").exists(), "0 keeps everything");
        prune(&dir, day, 30);
        let mut left: Vec<_> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name().into_string().unwrap()).collect();
        left.sort();
        assert_eq!(left, ["2026-09-01.jsonl", "2026-10-01.jsonl", "notes.txt"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
