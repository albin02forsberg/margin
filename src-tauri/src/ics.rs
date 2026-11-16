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
            if h.archived() || h.keyword.as_deref().is_some_and(|k| kw.is_done(k)) {
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
    let hm = |t: &Option<String>| t.as_deref().and_then(|t| chrono::NaiveTime::parse_from_str(t, "%H:%M").ok());
    let start = hm(&ts.time).map(|t| ts.date.and_time(t));
    let last = ts.end.unwrap_or(ts.date);
    let mut lines = vec![
        "BEGIN:VEVENT".to_string(),
        // Hashing keeps the UID short and free of characters calendars dislike.
        format!("UID:{:016x}@margin", fnv(uid)),
        format!("DTSTAMP:{day}T000000Z"),
    ];
    match start {
        Some(t) => {
            lines.push(format!("DTSTART:{}", t.format("%Y%m%dT%H%M%S")));
            // A time or date range ends at its end; otherwise an hour later.
            let fin = hm(&ts.end_time).map(|e| last.and_time(e)).filter(|e| *e > t).unwrap_or(last.and_time(t.time()) + chrono::Duration::hours(1));
            lines.push(format!("DTEND:{}", fin.format("%Y%m%dT%H%M%S")));
        }
        None => {
            lines.push(format!("DTSTART;VALUE=DATE:{day}"));
            lines.push(format!("DTEND;VALUE=DATE:{}", (last + chrono::Duration::days(1)).format("%Y%m%d")));
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

/// An RFC 5545 DURATION (`[+]P[nW][nD][T[nH][nM][nS]]`); negative or malformed gives None.
fn duration(v: &str) -> Option<Duration> {
    let v = v.trim().strip_prefix('+').unwrap_or(v.trim()).strip_prefix('P')?;
    let (mut total, mut n, mut time, mut any) = (0i64, None::<i64>, false, false);
    for c in v.chars() {
        if let Some(d) = c.to_digit(10) {
            n = Some(n.unwrap_or(0).checked_mul(10)?.checked_add(d as i64)?);
            continue;
        }
        let secs = match (c, time) {
            ('T', false) if n.is_none() => { time = true; continue }
            ('W', false) => 604800,
            ('D', false) => 86400,
            ('H', true) => 3600,
            ('M', true) => 60,
            ('S', true) => 1,
            _ => return None,
        };
        total = total.checked_add(n.take()?.checked_mul(secs)?)?;
        any = true;
    }
    (any && n.is_none()).then(|| Duration::try_seconds(total)).flatten()
}

fn date(v: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(v.get(..8)?, "%Y%m%d").ok()
}

/// The local day of an EXDATE / RECURRENCE-ID value: a `...Z` time is converted like DTSTART's, a date is taken as is.
fn local_date(v: &str) -> Option<NaiveDate> {
    when(v).map(|d| d.date()).or_else(|| date(v))
}

fn unescape(s: &str) -> String {
    s.replace("\\n", " ").replace("\\N", " ").replace("\\,", ",").replace("\\;", ";").replace("\\\\", "\\")
}

/// Whether a series starting on FIRST under RRULE has an occurrence on DAY. DAILY, WEEKLY,
/// MONTHLY and YEARLY (INTERVAL, BYDAY with `2MO`/`-1FR` ordinals, BYMONTHDAY, BYMONTH, UNTIL,
/// COUNT) are understood; other rules only occur on FIRST.
// ponytail: no BYSETPOS/BYWEEKNO/BYYEARDAY, and a YEARLY BYDAY ordinal counts within the month, not the year.
fn occurs(rule: &str, first: NaiveDate, day: NaiveDate) -> bool {
    let r: HashMap<&str, &str> = rule.split(';').filter_map(|p| p.split_once('=')).collect();
    let n = r.get("INTERVAL").and_then(|v| v.parse::<i64>().ok()).unwrap_or(1).max(1);
    let nums = |k: &str| -> Vec<i64> { r.get(k).map_or(vec![], |v| v.split(',').filter_map(|x| x.parse().ok()).collect()) };
    const DAYS: [&str; 7] = ["MO", "TU", "WE", "TH", "FR", "SA", "SU"];
    // (ordinal, weekday): 0 is every such weekday.
    let by: Vec<(i64, Weekday)> = r.get("BYDAY").map_or(vec![], |v| {
        v.split(',').filter_map(|d| Some((d.get(..d.len().saturating_sub(2)).and_then(|o| o.parse().ok()).unwrap_or(0),Weekday::try_from(DAYS.iter().position(|x| d.ends_with(x))? as u8).ok()?))).collect()
    });
    let (mdays, months) = (nums("BYMONTHDAY"), nums("BYMONTH"));
    let weekday = |d: NaiveDate| by.iter().any(|b| b.1 == d.weekday());
    // On a BYMONTHDAY (negative: from the end) and a BYDAY (with an ordinal: the nth in the month), else on FIRST's day.
    let in_month = |d: NaiveDate| {
        let (dd, last) = (d.day() as i64, d.num_days_in_month() as i64);
        (mdays.is_empty() || mdays.iter().any(|&m| m == dd || m == dd - last - 1))
            && (by.is_empty() || by.iter().any(|&(k, w)| w == d.weekday() && (k == 0 || k == (dd - 1) / 7 + 1 || k == -((last - dd) / 7 + 1))))
            && (!mdays.is_empty() || !by.is_empty() || d.day() == first.day())
    };
    let monday = |d: NaiveDate| d - Duration::days(d.weekday().num_days_from_monday() as i64);
    let month = |d: NaiveDate| d.year() as i64 * 12 + d.month0() as i64;
    let hit = |d: NaiveDate| match r.get("FREQ").copied() {
        Some("DAILY") => (d - first).num_days() % n == 0 && (by.is_empty() || weekday(d)),
        Some("WEEKLY") => (monday(d) - monday(first)).num_days() / 7 % n == 0 && if by.is_empty() { d.weekday() == first.weekday() } else { weekday(d) },
        Some("MONTHLY") => (month(d) - month(first)) % n == 0 && (months.is_empty() || months.contains(&(d.month() as i64))) && in_month(d),
        Some("YEARLY") => (d.year() - first.year()) as i64 % n == 0 && if months.is_empty() { d.month() == first.month() } else { months.contains(&(d.month() as i64)) } && in_month(d),
        _ => d == first,
    };
    if day < first || r.get("UNTIL").and_then(|v| date(v)).is_some_and(|u| day > u) || !hit(day) {
        return false;
    }
    // ponytail: walks the series day by day for COUNT; fine for a few years of a series.
    r.get("COUNT").and_then(|v| v.parse::<usize>().ok()).is_none_or(|c| first.iter_days().take_while(|d| *d <= day).filter(|d| hit(*d)).count() <= c)
}

/// Timed events of an .ics TEXT that overlap DAY, recurring ones (see `occurs`) included,
/// minus cancelled ones, EXDATEs and occurrences moved by a RECURRENCE-ID. The end is DTEND, else
/// DTSTART + DURATION. All-day events and events with neither are left out.
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
    let moved: Vec<(String, NaiveDate)> = evs.iter().filter_map(|e| Some((get(e, "UID")?, local_date(&get(e, "RECURRENCE-ID")?)?))).collect();
    let (d0, d1) = (day.and_time(NaiveTime::MIN), (day + Duration::days(1)).and_time(NaiveTime::MIN));
    let mut out = vec![];
    for e in &evs {
        let Some(start) = get(e, "DTSTART").and_then(|v| when(&v)) else { continue };
        let Some(end) = get(e, "DTEND").and_then(|v| when(&v)).or_else(|| get(e, "DURATION").and_then(|v| duration(&v)).map(|d| start + d)) else { continue };
        if get(e, "STATUS").is_some_and(|s| s.eq_ignore_ascii_case("CANCELLED")) {
            continue;
        }
        // A recurring event's occurrences starting on DAY or up to its span of midnights earlier.
        let spans = match get(e, "RRULE") {
            Some(rule) if get(e, "RECURRENCE-ID").is_none() => {
                let uid = get(e, "UID").unwrap_or_default();
                let ex: Vec<NaiveDate> = e.iter().filter(|p| p.0 == "EXDATE").flat_map(|p| p.2.split(',').filter_map(local_date).collect::<Vec<_>>()).collect();
                (0..=(end.date() - start.date()).num_days().clamp(0, 366))
                    .map(|k| day - Duration::days(k))
                    .filter(|s| !ex.contains(s) && !moved.contains(&(uid.clone(), *s)) && occurs(&rule, start.date(), *s))
                    .map(|s| (s.and_time(start.time()), s.and_time(start.time()) + (end - start)))
                    .collect()
            }
            _ => vec![(start, end)],
        };
        for (start, end) in spans {
            if start < d1 && d0 < end {
                let attendees = e.iter().filter(|p| p.0 == "ATTENDEE").map(|p| std::iter::successors(Some(("", p.1.as_str())), |r| split_unquoted(r.1, ';')).find_map(|r| r.1.strip_prefix("CN=").map(|c| split_unquoted(c, ';').map_or(c, |x| x.0))).map_or(p.2.trim_start_matches("mailto:").to_string(), |c| c.trim_matches('"').to_string())).collect();
                out.push(Event { start, end, summary: get(e, "SUMMARY").map(|s| unescape(&s)).filter(|s| !s.trim().is_empty()).unwrap_or("Busy".into()), attendees });
            }
        }
    }
    out
}

/// How long a fetched calendar is reused before it's fetched again.
const CALENDAR_TTL: std::time::Duration = std::time::Duration::from_secs(300);
/// The last fetched calendar: its URL, when, and the text or error.
pub type CalendarCache = std::sync::Mutex<Option<(String, std::time::Instant, Result<String, String>)>>;
pub static CALENDAR: CalendarCache = std::sync::Mutex::new(None);

/// The .ics text of `activity_calendar` SRC: an `https://` (or `webcal://`) URL goes through
/// FETCH, its answer (or error) reused for 5 minutes from CACHE; anything else is a file path.
/// Errors never show the URL: private calendar URLs hold a secret.
pub fn calendar(src: &str, cache: &CalendarCache, now: std::time::Instant, fetch: impl FnOnce(&str) -> Result<String, String>) -> Result<String, String> {
    if src.starts_with("http://") {
        return Err("activity_calendar: only https:// URLs are read".into());
    }
    let Some(url) = src.strip_prefix("webcal://").map(|r| format!("https://{r}")).or_else(|| src.starts_with("https://").then(|| src.to_string())) else {
        return std::fs::read_to_string(crate::config::expand(src)).map_err(|e| format!("activity_calendar {src}: {e}"));
    };
    let mut c = cache.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((_, _, r)) = c.as_ref().filter(|(u, at, _)| *u == url && now.duration_since(*at) < CALENDAR_TTL) {
        return r.clone();
    }
    let r = fetch(&url).map_err(|e| format!("activity_calendar (URL): {e}"));
    *c = Some((url, now, r.clone()));
    r
}

/// GET URL over HTTPS (10 s timeout) on a thread of its own, so it works from any caller. Errors leave the URL out.
pub fn get(url: &str) -> Result<String, String> {
    let url = url.to_string();
    let run = async move {
        if rustls::crypto::CryptoProvider::get_default().is_none() {
            let _ = rustls::crypto::ring::default_provider().install_default();
        }
        let e = |e: reqwest::Error| e.without_url().to_string();
        let resp = reqwest::Client::builder().user_agent("Margin").timeout(std::time::Duration::from_secs(10)).build().map_err(e)?.get(url).send().await.map_err(e)?;
        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status().as_u16()));
        }
        resp.text().await.map_err(e)
    };
    std::thread::spawn(move || tauri::async_runtime::block_on(run)).join().map_err(|_| "fetch failed".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn calendar_from_a_url() {
        let (cache, t0) = (CalendarCache::default(), std::time::Instant::now());
        let calls = std::cell::RefCell::new(vec![]);
        let fetch = |u: &str| {
            calls.borrow_mut().push(u.to_string());
            if u.contains("bad") { Err("HTTP 404".to_string()) } else { Ok(format!("ics of {u}")) }
        };
        let at = |s: u64| t0 + std::time::Duration::from_secs(s);
        assert_eq!(calendar("https://c.example/s3cret.ics", &cache, t0, fetch), Ok("ics of https://c.example/s3cret.ics".into()));
        assert_eq!(calendar("https://c.example/s3cret.ics", &cache, at(299), fetch), Ok("ics of https://c.example/s3cret.ics".into()));
        assert_eq!(calls.borrow().len(), 1, "reused within 5 minutes");
        calendar("https://c.example/s3cret.ics", &cache, at(300), fetch).unwrap();
        assert_eq!(calls.borrow().len(), 2, "fetched again after");
        calendar("webcal://c.example/other.ics", &cache, at(301), fetch).unwrap();
        assert_eq!(calls.borrow()[2], "https://c.example/other.ics", "another URL isn't the cached one; webcal is https");
        let err = calendar("https://c.example/bad/s3cret", &cache, at(302), fetch).unwrap_err();
        assert_eq!(err, "activity_calendar (URL): HTTP 404");
        assert_eq!(calendar("https://c.example/bad/s3cret", &cache, at(303), fetch), Err(err), "errors are cached too");
        assert_eq!(calls.borrow().len(), 4);
        assert!(!calendar("http://c.example/s3cret", &cache, t0, fetch).unwrap_err().contains("s3cret"));
        assert!(calendar("/no/such/file.ics", &cache, t0, fetch).unwrap_err().starts_with("activity_calendar /no/such/file.ics: "), "a path is read as a file");
        assert_eq!(calls.borrow().len(), 4);
    }

    #[test]
    fn duration_instead_of_dtend() {
        let d = |v: &str| duration(v).map(|d| d.num_seconds());
        assert_eq!((d("PT1H30M"), d("P1W"), d("P1DT2H"), d("+PT45S"), d("P0D")), (Some(5400), Some(604800), Some(93600), Some(45), Some(0)));
        for bad in ["", "P", "PT", "-PT1H", "PT1H30", "P1H", "PT1D", "1H", "PTH"] {
            assert_eq!(d(bad), None, "{bad}");
        }
        let ev = |extra: &str| format!("BEGIN:VEVENT\nDTSTART:20260105T100000\n{extra}\nSUMMARY:X\nEND:VEVENT");
        let on = |ics: &str, day: &str| events_on(ics, day.parse().unwrap());
        let e = &on(&ev("DURATION:PT1H30M"), "2026-01-05")[0];
        assert_eq!(e.end, "2026-01-05T11:30:00".parse().unwrap());
        assert_eq!(on(&ev("RRULE:FREQ=WEEKLY\nDURATION:PT1H"), "2026-01-12")[0].end, "2026-01-12T11:00:00".parse().unwrap(), "recurring");
        assert_eq!(on(&ev("DURATION:P1W"), "2026-01-11").len(), 1, "spans days");
        assert!(on(&ev(""), "2026-01-05").is_empty(), "no end, no duration");
        assert_eq!(on(&ev("DTEND:20260105T103000\nDURATION:PT5H"), "2026-01-05")[0].end, "2026-01-05T10:30:00".parse().unwrap(), "DTEND wins");
        assert!(on("BEGIN:VEVENT\nDTSTART;VALUE=DATE:20260105\nDURATION:P1D\nEND:VEVENT", "2026-01-05").is_empty(), "all-day stays out");
    }

    #[test]
    fn utc_exdate_and_recurrence_id_use_the_local_day() {
        let on = |ics: &str, day: NaiveDate| events_on(ics, day);
        let local = |v: &str| when(v).unwrap().date();
        let base = "BEGIN:VEVENT\nUID:a\nDTSTART:20260105T233000Z\nDTEND:20260106T000000Z\nRRULE:FREQ=WEEKLY\n";
        let (d1, d2, d3) = (local("20260112T233000Z"), local("20260119T233000Z"), local("20260126T233000Z"));
        let ics = format!("{base}EXDATE:20260112T233000Z\nSUMMARY:X\nEND:VEVENT\nBEGIN:VEVENT\nUID:a\nRECURRENCE-ID:20260119T233000Z\nDTSTART:20300101T100000\nDTEND:20300101T110000\nSUMMARY:M\nEND:VEVENT");
        assert!(on(&ics, d1).is_empty(), "EXDATE");
        assert!(on(&ics, d2).is_empty(), "moved");
        assert_eq!(on(&ics, d3).len(), 1, "other weeks stay");
    }

    #[test]
    fn recurring_event_crossing_midnight() {
        let on = |ics: &str, day: &str| events_on(ics, day.parse().unwrap());
        let ev = |extra: &str| format!("BEGIN:VEVENT\nUID:a\nDTSTART:20260104T220000\n{extra}\nSUMMARY:X\nEND:VEVENT");
        let night = ev("DTEND:20260105T010000\nRRULE:FREQ=WEEKLY");
        assert_eq!(on(&night, "2026-01-12")[0].start, "2026-01-11T22:00:00".parse().unwrap(), "Monday part");
        assert_eq!(on(&night, "2026-01-11").len(), 1, "Sunday part");
        assert!(on(&night, "2026-01-13").is_empty());
        assert_eq!(on(&ev("DURATION:P2D\nRRULE:FREQ=WEEKLY"), "2026-01-13").len(), 1, "second day");
        assert!(on(&ev("DTEND:20260105T010000\nRRULE:FREQ=WEEKLY\nEXDATE:20260111T220000"), "2026-01-12").is_empty(), "EXDATE hides the Monday part");
    }

    #[test]
    fn monthly_and_yearly() {
        let on = |rule: &str, first: &str, days: &[&str]| days.iter().map(|d| occurs(rule, first.parse().unwrap(), d.parse().unwrap())).collect::<Vec<_>>();
        assert_eq!(on("FREQ=MONTHLY", "2026-01-15", &["2026-01-15", "2026-03-15", "2026-03-16", "2025-12-15"]), [true, true, false, false], "by date");
        assert_eq!(on("FREQ=MONTHLY", "2026-01-31", &["2026-02-28", "2026-03-31", "2026-04-30"]), [false, true, false], "a month without the 31st is skipped");
        assert_eq!(on("FREQ=MONTHLY;BYDAY=2MO", "2026-01-12", &["2026-10-12", "2026-10-05", "2026-10-19", "2026-10-13"]), [true, false, false, false], "second Monday");
        assert_eq!(on("FREQ=MONTHLY;BYDAY=-1FR", "2026-01-30", &["2026-10-30", "2026-10-23", "2026-02-27"]), [true, false, true], "last Friday");
        assert_eq!(on("FREQ=MONTHLY;BYMONTHDAY=1,-1", "2026-01-01", &["2026-02-01", "2026-02-28", "2026-02-27"]), [true, true, false], "first and last day");
        assert_eq!(on("FREQ=MONTHLY;INTERVAL=2", "2026-01-15", &["2026-02-15", "2026-03-15", "2027-01-15"]), [false, true, true], "every other month, across a year");
        assert_eq!(on("FREQ=MONTHLY;COUNT=3", "2026-01-15", &["2026-03-15", "2026-04-15"]), [true, false]);
        assert_eq!(on("FREQ=MONTHLY;UNTIL=20260301T000000Z", "2026-01-15", &["2026-02-15", "2026-03-15"]), [true, false]);
        assert_eq!(on("FREQ=YEARLY", "2024-03-10", &["2026-03-10", "2026-03-11", "2026-04-10"]), [true, false, false], "same month and day");
        assert_eq!(on("FREQ=YEARLY", "2024-02-29", &["2025-02-28", "2028-02-29"]), [false, true], "29 February only in leap years");
        assert_eq!(on("FREQ=YEARLY;INTERVAL=2;COUNT=2", "2024-03-10", &["2025-03-10", "2026-03-10", "2028-03-10"]), [false, true, false]);
        assert_eq!(on("FREQ=YEARLY;BYMONTH=11;BYDAY=4TH", "2025-11-27", &["2026-11-26", "2026-11-19"]), [true, false], "fourth Thursday of November");

        // EXDATE and a moved occurrence work as for weekly series.
        let ics = ["BEGIN:VEVENT", "UID:m", "DTSTART:20260112T100000", "DTEND:20260112T110000", "RRULE:FREQ=MONTHLY;BYDAY=2MO", "EXDATE:20260209T100000", "SUMMARY:Review", "END:VEVENT",
            "BEGIN:VEVENT", "UID:m", "RECURRENCE-ID:20260309T100000", "DTSTART:20260310T130000", "DTEND:20260310T140000", "SUMMARY:Review (moved)", "END:VEVENT"].join("\n");
        let names = |d: &str| events_on(&ics, d.parse().unwrap()).into_iter().map(|e| e.summary).collect::<Vec<_>>();
        assert_eq!((names("2026-01-12"), names("2026-02-09"), names("2026-03-09"), names("2026-03-10"), names("2026-04-13")), (vec!["Review".into()], vec![], vec![], vec!["Review (moved)".into()], vec!["Review".into()]));
    }

    #[test]
    #[ignore = "needs the network"]
    fn fetches_over_https() {
        assert!(get("https://example.com/").unwrap().contains("<html"));
        let err = get("https://margin.invalid/s3cret.ics").unwrap_err();
        assert!(!err.contains("s3cret") && !err.contains("margin.invalid"), "{err}");
    }

    #[test]
    fn builds_feed() {
        let kw = Kw { todo: vec!["TODO".into()], done: vec!["DONE".into()] };
        let doc = "#+category: Work\n* TODO Call Ann, re: plans\nSCHEDULED: <2026-10-02 Fri 14:00>\n\
                   * TODO Water plants\nSCHEDULED: <2026-10-03 Sat +1w>\n\
                   * TODO Report\nDEADLINE: <2026-10-05 Mon>\n\
                   * DONE Old\nSCHEDULED: <2026-10-01 Thu>\n\
                   * Shelved :ARCHIVE:\n** TODO Stale\nSCHEDULED: <2026-10-01 Thu>\n\
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
        assert!(!ics.contains("Old") && !ics.contains("Stale"));
        assert_eq!(ics.matches("BEGIN:VEVENT").count(), 4);
        assert!(ics.ends_with("END:VCALENDAR\r\n"));
        assert_eq!(feed(&[Arc::new(crate::org::parse(Path::new("/n/a.org"), doc, &kw))], &kw), ics, "stable output");

        // Time and date ranges end where they say.
        let doc = "* Talk\n<2026-10-16 Fri 10:00-11:30>\n* Trip\n<2026-10-19 Mon>--<2026-10-21 Wed>\n* Conf\n<2026-10-19 Mon 09:00>--<2026-10-20 Tue 17:00>\n";
        let ics = feed(&[Arc::new(crate::org::parse(Path::new("/n/a.org"), doc, &kw))], &kw);
        for s in ["DTSTART:20261016T100000\r\nDTEND:20261016T113000\r\n", "DTSTART;VALUE=DATE:20261019\r\nDTEND;VALUE=DATE:20261022\r\n",
            "DTSTART:20261019T090000\r\nDTEND:20261020T170000\r\n"] {
            assert!(ics.contains(s), "missing {s:?} in\n{ics}");
        }
        assert_eq!(ics.matches("BEGIN:VEVENT").count(), 3);

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
