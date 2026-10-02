//! Desktop notifications before timed agenda entries and for upcoming deadlines.

use crate::org::{self, Item, Kw};
use crate::App;
use chrono::{Duration, NaiveDateTime, NaiveTime};
use std::collections::BTreeSet;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

#[derive(Debug, PartialEq)]
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
/// WARN_DAYS ahead. Done tasks are skipped. Callers drop keys already sent.
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
                if (0..=warn_days).contains(&days) {
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
    if !c.reminders {
        return;
    }
    let now = crate::timeclock::now();
    let today = now.date();
    let items = org::agenda(&s.files(), &s.kw(), today, c.deadline_warning_days.max(0) + 2, today);
    let due = due(&items, &s.kw(), now, Duration::minutes(c.remind_before_minutes), c.deadline_warning_days);
    let file = c.data().join("reminders-sent.txt");
    let mut sent: BTreeSet<String> = std::fs::read_to_string(&file).unwrap_or_default().lines().map(str::to_string).collect();
    let before = sent.len();
    sent.retain(|k| k.get(..10).and_then(|d| d.parse::<chrono::NaiveDate>().ok()).is_some_and(|d| d >= today));
    let mut changed = sent.len() != before;
    for d in due {
        if sent.insert(d.key.clone()) {
            changed = true;
            notify(app, d);
        }
    }
    if changed {
        let _ = std::fs::write(&file, sent.into_iter().map(|k| k + "\n").collect::<String>());
    }
}

/// Show D; clicking it opens the entry where the desktop reports clicks (Linux, via D-Bus).
fn notify(app: &AppHandle, d: Due) {
    #[cfg(target_os = "linux")]
    if let Ok(h) = notify_rust::Notification::new().appname("Margin").summary(&d.title).body(&d.body).action("default", "Open").show() {
        let app = app.clone();
        std::thread::spawn(move || {
            h.wait_for_action(|a| {
                if a == "default" {
                    crate::tray::show(&app);
                    let _ = app.emit("open-entry", serde_json::json!({ "path": d.path, "line": d.line }));
                }
            })
        });
        return;
    }
    let _ = app.notification().builder().title(&d.title).body(&d.body).show();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::sync::Arc;

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
}
