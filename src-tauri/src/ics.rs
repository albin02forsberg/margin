//! Calendar feed: scheduled tasks, deadlines and appointments as an .ics file
//! that calendar apps can subscribe to; and reading a day's events from one.

use crate::org::{Kw, OrgFile, Ts};
use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc, Weekday};
use std::collections::HashMap;
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

/// A timed calendar event, in local time.
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub summary: String,
    /// Names (CN) or addresses.
    pub attendees: Vec<String>,
}

/// Splits S at its first SEP outside double quotes (ics params may quote `:` and `;`).
fn split_unquoted(s: &str, sep: char) -> Option<(&str, &str)> {
    let mut q = false;
    let i = s.char_indices().find(|&(_, c)| {
        q ^= c == '"';
        c == sep && !q
    })?.0;
    Some((&s[..i], &s[i + 1..]))
}

/// A DTSTART/DTEND value → local time: UTC (`…Z`) is converted, TZID and floating times
/// are taken as local; dates (all-day) give None.
// ponytail: TZID is assumed to be this machine's zone (no tz database); add chrono-tz if other zones matter.
fn when(v: &str) -> Option<NaiveDateTime> {
    let (v, utc) = v.strip_suffix('Z').map_or((v, false), |v| (v, true));
    let t = NaiveDateTime::parse_from_str(v, "%Y%m%dT%H%M%S").ok()?;
    Some(if utc { Utc.from_utc_datetime(&t).with_timezone(&Local).naive_local() } else { t })
}

fn date(v: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(v.get(..8)?, "%Y%m%d").ok()
}

fn unescape(s: &str) -> String {
    s.replace("\\n", " ").replace("\\N", " ").replace("\\,", ",").replace("\\;", ";").replace("\\\\", "\\")
}

/// Whether a series starting on FIRST under RRULE has an occurrence on DAY. DAILY and WEEKLY
/// (INTERVAL, BYDAY, UNTIL, COUNT) are understood; other rules only occur on FIRST.
fn occurs(rule: &str, first: NaiveDate, day: NaiveDate) -> bool {
    let r: HashMap<&str, &str> = rule.split(';').filter_map(|p| p.split_once('=')).collect();
    let n = r.get("INTERVAL").and_then(|v| v.parse::<i64>().ok()).unwrap_or(1).max(1);
    const DAYS: [&str; 7] = ["MO", "TU", "WE", "TH", "FR", "SA", "SU"];
    let by: Vec<Weekday> = r.get("BYDAY").map_or(vec![], |v| v.split(',').filter_map(|d| DAYS.iter().position(|x| d.ends_with(x))).filter_map(|i| Weekday::try_from(i as u8).ok()).collect());
    let monday = |d: NaiveDate| d - Duration::days(d.weekday().num_days_from_monday() as i64);
    let hit = |d: NaiveDate| match r.get("FREQ").copied() {
        Some("DAILY") => (d - first).num_days() % n == 0 && (by.is_empty() || by.contains(&d.weekday())),
        Some("WEEKLY") => (monday(d) - monday(first)).num_days() / 7 % n == 0 && if by.is_empty() { d.weekday() == first.weekday() } else { by.contains(&d.weekday()) },
        _ => d == first,
    };
    if day < first || r.get("UNTIL").and_then(|v| date(v)).is_some_and(|u| day > u) || !hit(day) {
        return false;
    }
    // ponytail: walks the series day by day for COUNT; fine for a few years of a series.
    r.get("COUNT").and_then(|v| v.parse::<usize>().ok()).is_none_or(|c| first.iter_days().take_while(|d| *d <= day).filter(|d| hit(*d)).count() <= c)
}

/// Timed events of an .ics TEXT that overlap DAY, recurring ones (see `occurs`) included,
/// minus cancelled ones, EXDATEs and occurrences moved by a RECURRENCE-ID. All-day events and
/// events without DTEND are left out.
pub fn events_on(text: &str, day: NaiveDate) -> Vec<Event> {
    let text = text.replace("\r\n", "\n").replace("\n ", "").replace("\n\t", "");
    // Each VEVENT's (NAME, params, value)s, without nested components (alarms).
    type Props = Vec<(String, String, String)>;
    let (mut evs, mut cur, mut depth): (Vec<Props>, Option<Props>, i32) = (vec![], None, 0);
    for l in text.lines() {
        match l.trim_end() {
            "BEGIN:VEVENT" => (cur, depth) = (Some(vec![]), 0),
            "END:VEVENT" => evs.extend(cur.take()),
            l if l.starts_with("BEGIN:") => depth += 1,
            l if l.starts_with("END:") => depth -= 1,
            l => {
                if let (Some(c), 0, Some((k, v))) = (cur.as_mut(), depth, split_unquoted(l, ':')) {
                    let (name, params) = k.split_once(';').unwrap_or((k, ""));
                    c.push((name.to_uppercase(), params.to_string(), v.to_string()));
                }
            }
        }
    }
    let get = |e: &Props, n: &str| e.iter().find(|p| p.0 == n).map(|p| p.2.clone());
    let moved: Vec<(String, NaiveDate)> = evs.iter().filter_map(|e| Some((get(e, "UID")?, date(&get(e, "RECURRENCE-ID")?)?))).collect();
    let (d0, d1) = (day.and_time(NaiveTime::MIN), (day + Duration::days(1)).and_time(NaiveTime::MIN));
    let mut out = vec![];
    for e in &evs {
        let (Some(start), Some(end)) = (get(e, "DTSTART").and_then(|v| when(&v)), get(e, "DTEND").and_then(|v| when(&v))) else { continue };
        if get(e, "STATUS").is_some_and(|s| s.eq_ignore_ascii_case("CANCELLED")) {
            continue;
        }
        let (start, end) = match get(e, "RRULE") {
            Some(rule) if get(e, "RECURRENCE-ID").is_none() => {
                let uid = get(e, "UID").unwrap_or_default();
                let ex = e.iter().filter(|p| p.0 == "EXDATE").flat_map(|p| p.2.split(',').filter_map(date).collect::<Vec<_>>()).any(|x| x == day);
                if ex || moved.contains(&(uid, day)) || !occurs(&rule, start.date(), day) {
                    continue;
                }
                (day.and_time(start.time()), day.and_time(start.time()) + (end - start))
            }
            _ => (start, end),
        };
        if start < d1 && d0 < end {
            let attendees = e.iter().filter(|p| p.0 == "ATTENDEE").map(|p| std::iter::successors(Some(("", p.1.as_str())), |r| split_unquoted(r.1, ';')).find_map(|r| r.1.strip_prefix("CN=").map(|c| split_unquoted(c, ';').map_or(c, |x| x.0))).map_or(p.2.trim_start_matches("mailto:").to_string(), |c| c.trim_matches('"').to_string())).collect();
            out.push(Event { start, end, summary: get(e, "SUMMARY").map(|s| unescape(&s)).filter(|s| !s.trim().is_empty()).unwrap_or("Busy".into()), attendees });
        }
    }
    out
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

    #[test]
    fn reads_events() {
        let ics = [
            "BEGIN:VCALENDAR",
            "BEGIN:VEVENT", "UID:a", "DTSTART:20261005T070000Z", "DTEND:20261005T073000Z", "SUMMARY:Sync with Acme\\, Inc",
            "ATTENDEE;CN=\"Ann Lee\";ROLE=REQ-PARTICIPANT:mailto:ann@acme.com", "ATTENDEE:mailto:bob@x.org",
            "BEGIN:VALARM", "SUMMARY:Alarm", "TRIGGER:-PT10M", "END:VALARM", "END:VEVENT",
            // Mondays and Wednesdays, but not 30 Sep; 7 Oct moved to 11:00.
            "BEGIN:VEVENT", "UID:standup", "DTSTART;TZID=Europe/Stockholm:20260921T091500", "DTEND;TZID=Europe/Stockholm:20260921T093000",
            "RRULE:FREQ=WEEKLY;BYDAY=MO,WE", "EXDATE;TZID=Europe/Stockholm:20260930T091500", "SUMMARY:Stand", " up", "END:VEVENT",
            "BEGIN:VEVENT", "UID:standup", "RECURRENCE-ID;TZID=Europe/Stockholm:20261007T091500",
            "DTSTART;TZID=Europe/Stockholm:20261007T110000", "DTEND;TZID=Europe/Stockholm:20261007T111500", "SUMMARY:Standup (moved)", "END:VEVENT",
            "BEGIN:VEVENT", "UID:c", "DTSTART;VALUE=DATE:20261005", "DTEND;VALUE=DATE:20261006", "SUMMARY:Holiday", "END:VEVENT",
            "BEGIN:VEVENT", "UID:d", "DTSTART:20261005T120000", "DTEND:20261005T130000", "STATUS:CANCELLED", "SUMMARY:Lunch", "END:VEVENT",
            "BEGIN:VEVENT", "UID:e", "DTSTART:20260928T140000", "DTEND:20260928T150000", "RRULE:FREQ=DAILY;COUNT=3", "END:VEVENT",
            "BEGIN:VEVENT", "UID:q", "DTSTART;TZID=\"(UTC+01:00) Amsterdam\":20261008T100000", "DTEND;TZID=\"(UTC+01:00) Amsterdam\":20261008T110000",
            "ATTENDEE;CN=\"Lee: Sales\":mailto:l@x.org", "ATTENDEE;CN=\"A; B\";ROLE=OPT:mailto:a@x.org", "END:VEVENT",
            "END:VCALENDAR",
        ]
        .join("\r\n");
        let on = |d: &str| events_on(&ics, d.parse().unwrap());
        let at = |d: &str| NaiveDateTime::parse_from_str(d, "%Y-%m-%d %H:%M").unwrap();
        let short = |d: &str| on(d).into_iter().map(|e| (e.start, e.end, e.summary)).collect::<Vec<_>>();
        let e = on("2026-10-05");
        let utc7 = Utc.with_ymd_and_hms(2026, 10, 5, 7, 0, 0).unwrap().with_timezone(&Local).naive_local();
        assert_eq!(e[0], Event { start: utc7, end: utc7 + Duration::minutes(30), summary: "Sync with Acme, Inc".into(), attendees: vec!["Ann Lee".into(), "bob@x.org".into()] });
        assert_eq!(short("2026-10-05")[1..], [(at("2026-10-05 09:15"), at("2026-10-05 09:30"), "Standup".into())], "all-day and cancelled are left out");
        assert_eq!(short("2026-09-30"), [(at("2026-09-30 14:00"), at("2026-09-30 15:00"), "Busy".into())], "EXDATE; COUNT's third day");
        assert_eq!(short("2026-10-07"), [(at("2026-10-07 11:00"), at("2026-10-07 11:15"), "Standup (moved)".into())]);
        assert_eq!(on("2026-10-08")[0].attendees, ["Lee: Sales", "A; B"], "quoted TZID keeps the event; quoted CN stays whole");
        assert!(short("2026-10-01").is_empty() && short("2026-10-06").is_empty() && short("2026-09-14").is_empty());
    }
}
