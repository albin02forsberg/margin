//! Calendar feed: scheduled tasks, deadlines and appointments as an .ics file
//! that calendar apps can subscribe to.

use crate::org::{Kw, OrgFile, Ts};
use std::sync::Arc;

/// iCalendar text for every open entry with a date. Timed entries last an hour,
/// the rest are all-day; repeaters become RRULEs. Output only depends on the
/// files, so rewriting an unchanged feed is a no-op.
pub fn feed(files: &[Arc<OrgFile>], kw: &Kw) -> String {
    let mut out = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Margin//Margin//EN\r\nX-WR-CALNAME:Margin\r\n");
    for f in files {
        for h in &f.headlines {
            if h.keyword.as_deref().is_some_and(|k| kw.is_done(k)) {
                continue;
            }
            let base = h.id.clone().unwrap_or_else(|| format!("{}#{}", f.path.display(), h.title));
            let mut add = |kind: &str, ts: &Ts, summary: String| event(&mut out, &format!("{kind}-{base}"), ts, &summary, &f.category);
            if let Some(s) = &h.scheduled {
                add("scheduled", s, h.title.clone());
            }
            if let Some(d) = &h.deadline {
                add("deadline", d, format!("Due: {}", h.title));
            }
            for (i, t) in h.timestamps.iter().enumerate() {
                add(&format!("at{i}"), t, h.title.clone());
            }
        }
    }
    out + "END:VCALENDAR\r\n"
}

fn event(out: &mut String, uid: &str, ts: &Ts, summary: &str, category: &str) {
    let day = ts.date.format("%Y%m%d");
    let start = ts.time.as_deref().and_then(|t| chrono::NaiveTime::parse_from_str(t, "%H:%M").ok()).map(|t| ts.date.and_time(t));
    let mut lines = vec![
        "BEGIN:VEVENT".to_string(),
        // Hashing keeps the UID short and free of characters calendars dislike.
        format!("UID:{:016x}@margin", fnv(uid)),
        format!("DTSTAMP:{day}T000000Z"),
    ];
    match start {
        Some(t) => {
            lines.push(format!("DTSTART:{}", t.format("%Y%m%dT%H%M%S")));
            lines.push(format!("DTEND:{}", (t + chrono::Duration::hours(1)).format("%Y%m%dT%H%M%S")));
        }
        None => {
            lines.push(format!("DTSTART;VALUE=DATE:{day}"));
            lines.push(format!("DTEND;VALUE=DATE:{}", (ts.date + chrono::Duration::days(1)).format("%Y%m%d")));
        }
    }
    if let Some((_, n, unit)) = &ts.repeater {
        let freq = match unit {
            'h' => "HOURLY",
            'd' => "DAILY",
            'w' => "WEEKLY",
            'm' => "MONTHLY",
            _ => "YEARLY",
        };
        lines.push(format!("RRULE:FREQ={freq};INTERVAL={n}"));
    }
    lines.push(format!("SUMMARY:{}", escape(summary)));
    if !category.is_empty() {
        lines.push(format!("CATEGORIES:{}", escape(category)));
    }
    lines.push("END:VEVENT".into());
    for l in lines {
        fold(out, &l);
    }
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace(';', "\\;").replace(',', "\\,").replace('\n', "\\n")
}

/// Lines longer than 75 bytes continue on the next line after a space (RFC 5545 §3.1).
fn fold(out: &mut String, line: &str) {
    let mut n = 0;
    for c in line.chars() {
        if n + c.len_utf8() > 75 {
            out.push_str("\r\n ");
            n = 1;
        }
        out.push(c);
        n += c.len_utf8();
    }
    out.push_str("\r\n");
}

fn fnv(s: &str) -> u64 {
    s.bytes().fold(0xcbf29ce484222325, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn builds_feed() {
        let kw = Kw { todo: vec!["TODO".into()], done: vec!["DONE".into()] };
        let doc = "#+category: Work\n* TODO Call Ann, re: plans\nSCHEDULED: <2026-10-02 Fri 14:00>\n\
                   * TODO Water plants\nSCHEDULED: <2026-10-03 Sat +1w>\n\
                   * TODO Report\nDEADLINE: <2026-10-05 Mon>\n\
                   * DONE Old\nSCHEDULED: <2026-10-01 Thu>\n\
                   * Dentist\n<2026-10-07 Wed 09:30>\n";
        let f = Arc::new(crate::org::parse(Path::new("/n/a.org"), doc, &kw));
        let ics = feed(&[f], &kw);
        let has = |s: &str| assert!(ics.contains(s), "missing {s:?} in\n{ics}");
        has("BEGIN:VCALENDAR\r\n");
        has("DTSTART:20261002T140000\r\nDTEND:20261002T150000\r\n");
        has("SUMMARY:Call Ann\\, re: plans\r\n");
        has("DTSTART;VALUE=DATE:20261003\r\nDTEND;VALUE=DATE:20261004\r\nRRULE:FREQ=WEEKLY;INTERVAL=1\r\n");
        has("SUMMARY:Due: Report\r\n");
        has("DTSTART:20261007T093000\r\n");
        has("CATEGORIES:Work\r\n");
        assert!(!ics.contains("Old"));
        assert_eq!(ics.matches("BEGIN:VEVENT").count(), 4);
        assert!(ics.ends_with("END:VCALENDAR\r\n"));
        assert_eq!(feed(&[Arc::new(crate::org::parse(Path::new("/n/a.org"), doc, &kw))], &kw), ics, "stable output");

        let mut long = String::new();
        fold(&mut long, &format!("SUMMARY:{}", "é".repeat(60)));
        assert!(long.split("\r\n").all(|l| l.len() <= 75));
    }
}
