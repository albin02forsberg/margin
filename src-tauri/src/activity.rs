// Suggested sessions from ActivityWatch (https://activitywatch.net): its local REST API
// has window and AFK events, which we turn into blocks of work worth logging.

use chrono::{DateTime, Duration, DurationRound, FixedOffset, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};

/// Activity less than this apart joins one block.
const GAP_SECS: i64 = 5 * 60;
/// Shorter blocks are noise.
const MIN_BLOCK_SECS: i64 = 15 * 60;

/// An event as local time. AFK events carry their status ("afk"/"not-afk") as `app`.
#[derive(Clone, Debug)]
pub struct Span {
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub app: String,
    pub title: String,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Suggestion {
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    /// Top 3 apps and titles by time.
    pub apps: Vec<String>,
    pub titles: Vec<String>,
    pub project: Option<String>,
}

#[derive(Deserialize)]
struct AwEvent {
    timestamp: DateTime<FixedOffset>,
    duration: f64,
    data: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
struct Bucket {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    last_updated: String,
}

/// `/api/0/buckets/` → the most recently updated (window, afk) bucket ids.
pub fn pick_buckets(json: &str) -> Result<(Option<String>, Option<String>), String> {
    let b: HashMap<String, Bucket> = serde_json::from_str(json).map_err(|e| format!("ActivityWatch buckets: {e}"))?;
    // ponytail: ISO strings from one server compare chronologically; parse them if a server mixes offsets.
    let newest = |kind: &str| b.iter().filter(|(_, v)| v.kind == kind).max_by(|x, y| x.1.last_updated.cmp(&y.1.last_updated)).map(|(id, _)| id.clone());
    Ok((newest("currentwindow"), newest("afkstatus")))
}

/// `/api/0/buckets/<id>/events` → spans in TZ.
pub fn parse_events<Tz: TimeZone>(json: &str, tz: &Tz) -> Result<Vec<Span>, String> {
    let evs: Vec<AwEvent> = serde_json::from_str(json).map_err(|e| format!("ActivityWatch events: {e}"))?;
    Ok(evs
        .into_iter()
        .map(|e| {
            let s = |k: &str| e.data.get(k).and_then(|v| v.as_str()).map(String::from);
            let end = e.timestamp + Duration::milliseconds((e.duration * 1000.0) as i64);
            let local = |t: DateTime<FixedOffset>| t.with_timezone(tz).naive_local();
            Span { start: local(e.timestamp), end: local(end), app: s("app").or_else(|| s("status")).unwrap_or_default(), title: s("title").unwrap_or_default() }
        })
        .collect())
}

/// `activity_exclude` patterns, case-insensitive.
pub fn exclude_rules(pats: &[String]) -> Result<Vec<Regex>, String> {
    pats.iter().map(|p| Regex::new(&format!("(?i){p}")).map_err(|e| format!("activity_exclude: {e}"))).collect()
}

/// Blocks of work on DAY: WINDOW events minus EXCLUDEd ones, AFK time and TRACKED spans,
/// joined across gaps under 5 min, at least 15 min long; PROJECTS named in a block's titles
/// or apps are guessed (the one seen longest).
pub fn suggest(window: &[Span], afk: &[Span], tracked: &[(NaiveDateTime, NaiveDateTime)], exclude: &[Regex], projects: &[String], day: NaiveDate) -> Vec<Suggestion> {
    let (d0, d1) = (day.and_time(NaiveTime::MIN), (day + Duration::days(1)).and_time(NaiveTime::MIN));
    let cuts: Vec<_> = afk.iter().filter(|a| a.app == "afk").map(|a| (a.start, a.end)).chain(tracked.iter().copied()).collect();
    let mut pieces: Vec<Span> = vec![];
    // ponytail: O(events × cuts), fine for a day; sweep sorted lists if it ever shows up.
    for w in window.iter().filter(|w| !exclude.iter().any(|r| r.is_match(&w.app) || r.is_match(&w.title))) {
        let mut parts = vec![(w.start.max(d0), w.end.min(d1))];
        for &(cs, ce) in &cuts {
            parts = parts.into_iter().flat_map(|(s, e)| [(s, e.min(cs)), (s.max(ce), e)]).filter(|(s, e)| s < e).collect();
        }
        pieces.extend(parts.into_iter().filter(|(s, e)| s < e).map(|(start, end)| Span { start, end, ..w.clone() }));
    }
    pieces.sort_by_key(|p| p.start);
    let mut blocks: Vec<Vec<Span>> = vec![];
    let mut end = d0;
    for mut p in pieces {
        p.start = p.start.max(end); // overlapping events (AW heartbeats) count once
        if p.start >= p.end {
            continue;
        }
        let join = !blocks.is_empty() && (p.start - end).num_seconds() < GAP_SECS && !tracked.iter().any(|&(s, e)| s < p.start && end < e);
        end = p.end;
        match blocks.last_mut() {
            Some(b) if join => b.push(p),
            _ => blocks.push(vec![p]),
        }
    }
    blocks
        .into_iter()
        .filter(|b| (b[b.len() - 1].end - b[0].start).num_seconds() >= MIN_BLOCK_SECS)
        .map(|b| {
            let secs = |p: &Span| (p.end - p.start).num_seconds();
            let top = |key: fn(&Span) -> &str| {
                let mut t: HashMap<&str, i64> = HashMap::new();
                b.iter().filter(|p| !key(p).is_empty()).for_each(|p| *t.entry(key(p)).or_default() += secs(p));
                let mut v: Vec<_> = t.into_iter().collect();
                v.sort_by(|x, y| y.1.cmp(&x.1).then(x.0.cmp(y.0)));
                v.into_iter().take(3).map(|(k, _)| k.to_string()).collect::<Vec<_>>()
            };
            let project = projects
                .iter()
                .filter(|n| !n.trim().is_empty())
                .map(|n| {
                    let n_lc = n.to_lowercase();
                    (b.iter().filter(|p| p.title.to_lowercase().contains(&n_lc) || p.app.to_lowercase().contains(&n_lc)).map(secs).sum::<i64>(), n)
                })
                .filter(|(s, _)| *s > 0)
                .max_by_key(|(s, n)| (*s, n.len()))
                .map(|(_, n)| n.clone());
            // Whole minutes, inside the block so they can't touch logged time.
            let m = Duration::minutes(1);
            let (start, end) = (b[0].start.duration_round_up(m).unwrap_or(b[0].start), b[b.len() - 1].end.duration_trunc(m).unwrap_or(b[b.len() - 1].end));
            Suggestion { start, end, apps: top(|p| &p.app), titles: top(|p| &p.title), project }
        })
        .collect()
}

/// GET PATH from the server at BASE (`http://host:port`). Only loopback addresses, so
/// nothing leaves the machine. HTTP/1.0 keeps the response unchunked.
fn get(base: &str, path: &str) -> Result<String, String> {
    let host = base.trim().trim_end_matches('/').strip_prefix("http://").ok_or("activitywatch_url must look like http://localhost:5600")?;
    let addrs: Vec<_> = host.to_socket_addrs().map_err(|e| format!("activitywatch_url {base}: {e}"))?.collect();
    if addrs.is_empty() || !addrs.iter().all(|a| a.ip().is_loopback()) {
        return Err(format!("activitywatch_url {base} isn't on this machine; only localhost is allowed"));
    }
    let unreachable = |e: std::io::Error| format!("ActivityWatch isn't reachable at {base} ({e})");
    let mut s = TcpStream::connect_timeout(&addrs[0], std::time::Duration::from_secs(2)).map_err(unreachable)?;
    s.set_read_timeout(Some(std::time::Duration::from_secs(10))).map_err(unreachable)?;
    s.write_all(format!("GET {path} HTTP/1.0\r\nHost: {host}\r\nAccept: application/json\r\n\r\n").as_bytes()).map_err(unreachable)?;
    let mut buf = vec![];
    s.read_to_end(&mut buf).map_err(unreachable)?;
    let text = String::from_utf8_lossy(&buf);
    let (head, body) = text.split_once("\r\n\r\n").ok_or("ActivityWatch sent a malformed response")?;
    match head.split_whitespace().nth(1) {
        Some("200") => Ok(body.to_string()),
        status => Err(format!("ActivityWatch {path}: HTTP {}", status.unwrap_or("?"))),
    }
}

/// Window and AFK spans around DAY from the server at BASE.
pub fn fetch(base: &str, day: NaiveDate) -> Result<(Vec<Span>, Vec<Span>), String> {
    let (window, afk) = pick_buckets(&get(base, "/api/0/buckets/")?)?;
    let window = window.ok_or("ActivityWatch has no window watcher (aw-watcher-window) data")?;
    let utc = |d: NaiveDate| {
        let t = d.and_time(NaiveTime::MIN);
        Local.from_local_datetime(&t).earliest().map_or(t, |l| l.naive_utc()).format("%Y-%m-%dT%H:%M:%SZ")
    };
    let q = format!("start={}&end={}", utc(day), utc(day + Duration::days(1)));
    let events = |id: &str| parse_events(&get(base, &format!("/api/0/buckets/{id}/events?{q}"))?, &Local);
    Ok((events(&window)?, afk.as_deref().map(events).transpose()?.unwrap_or_default()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dt(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap()
    }
    /// Events as the API returns them: newest first, UTC.
    fn ev(ts: &str, mins: f64, data: &str) -> String {
        format!(r#"{{"id":1,"timestamp":"{ts}","duration":{},"data":{data}}}"#, mins * 60.0)
    }
    fn win(app: &str, title: &str) -> String {
        format!(r#"{{"app":"{app}","title":"{title}"}}"#)
    }
    const CEST: i32 = 2 * 3600;

    #[test]
    fn buckets() {
        let json = r#"{
          "aw-watcher-window_old": {"id":"aw-watcher-window_old","type":"currentwindow","client":"aw-watcher-window","hostname":"old","created":"2025-01-01T00:00:00+00:00","last_updated":"2025-06-01T00:00:00+00:00"},
          "aw-watcher-window_pc": {"id":"aw-watcher-window_pc","type":"currentwindow","client":"aw-watcher-window","hostname":"pc","created":"2025-01-01T00:00:00+00:00","last_updated":"2026-10-02T08:00:00+00:00"},
          "aw-watcher-afk_pc": {"id":"aw-watcher-afk_pc","type":"afkstatus","client":"aw-watcher-afk","hostname":"pc","created":"2025-01-01T00:00:00+00:00","last_updated":"2026-10-02T08:00:00+00:00"},
          "aw-watcher-web-firefox": {"id":"aw-watcher-web-firefox","type":"web.tab.current","client":"aw-client-web","hostname":"pc","created":"2025-01-01T00:00:00+00:00"}
        }"#;
        assert_eq!(pick_buckets(json).unwrap(), (Some("aw-watcher-window_pc".into()), Some("aw-watcher-afk_pc".into())));
        assert_eq!(pick_buckets("{}").unwrap(), (None, None));
    }

    #[test]
    fn suggestions_from_a_day() {
        let tz = FixedOffset::east_opt(CEST).unwrap();
        let window = format!(
            "[{}]",
            [
                ev("2026-10-01T10:59:59.250000+00:00", 30.5, &win("firefox", "Lunch menu")),
                ev("2026-10-01T08:53:00+00:00", 27.0, &win("Slack", "general")),
                ev("2026-10-01T08:50:00+00:00", 2.0, &win("KeePassXC", "bank.kdbx")),
                ev("2026-10-01T08:00:00+00:00", 50.0, &win("Firefox", "albin02forsberg/margin - GitHub")),
                ev("2026-10-01T07:10:00+00:00", 48.0, &win("Code", "lib.rs - margin - Visual Studio Code")),
                ev("2026-10-01T07:10:00+00:00", 5.0, &win("Code", "lib.rs - margin - Visual Studio Code")),
                ev("2026-10-01T06:00:00+00:00", 5.0, &win("Code", "early")),
                ev("2026-09-30T21:50:00+00:00", 20.0, &win("Code", "yesterday")),
            ]
            .join(",")
        );
        let afk = format!("[{}]", [ev("2026-10-01T09:20:00+00:00", 90.0, r#"{"status":"afk"}"#), ev("2026-10-01T07:00:00+00:00", 140.0, r#"{"status":"not-afk"}"#)].join(","));
        let (w, a) = (parse_events(&window, &tz).unwrap(), parse_events(&afk, &tz).unwrap());
        assert_eq!((w[0].start, w[0].end), (dt("2026-10-01 12:59") + Duration::milliseconds(59_250), dt("2026-10-01 13:30") + Duration::milliseconds(29_250))); // UTC → local
        let projects = vec!["Margin".to_string(), "Acme".to_string(), "".to_string()];
        let rules = exclude_rules(&["keepass".into()]).unwrap();
        let day = |d: &str| d.parse::<NaiveDate>().unwrap();
        let s = suggest(&w, &a, &[], &rules, &projects, day("2026-10-01"));
        // 08:00 alone is under 15 min; 09:10–11:20 joins across the excluded 2 min; 11:20–12:50 is AFK;
        // lunch is rounded inward to whole minutes.
        assert_eq!(s.iter().map(|s| (s.start, s.end)).collect::<Vec<_>>(), vec![(dt("2026-10-01 09:10"), dt("2026-10-01 11:20")), (dt("2026-10-01 13:00"), dt("2026-10-01 13:30"))]);
        assert_eq!(s[0].apps, vec!["Firefox", "Code", "Slack"]);
        assert!(!s[0].titles.iter().any(|t| t.contains("kdbx")), "excluded titles never reach the UI");
        assert_eq!(s[0].project.as_deref(), Some("Margin"));
        assert_eq!(s[1].project, None);
        // The 23:50–00:10 event is clipped to the day it falls in.
        assert!(suggest(&w, &a, &[], &rules, &projects, day("2026-09-30")).is_empty());
        // Already logged time is subtracted, and a block doesn't bridge a logged gap.
        let tracked = [(dt("2026-10-01 09:50"), dt("2026-10-01 09:52")), (dt("2026-10-01 13:00"), dt("2026-10-01 14:00"))];
        let s = suggest(&w, &a, &tracked, &rules, &projects, day("2026-10-01"));
        assert_eq!(s.iter().map(|s| (s.start, s.end)).collect::<Vec<_>>(), vec![(dt("2026-10-01 09:10"), dt("2026-10-01 09:50")), (dt("2026-10-01 09:52"), dt("2026-10-01 11:20"))]);
        assert_eq!(s[0].project.as_deref(), Some("Margin"));
    }

    #[test]
    fn only_loopback() {
        assert!(get("http://example.com:5600", "/").unwrap_err().contains("only localhost"));
        assert!(get("https://localhost:5600", "/").is_err());
        assert!(exclude_rules(&["(".into()]).is_err());
        // A stand-in server: the request is plain HTTP/1.0, the body comes back as is.
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}/", l.local_addr().unwrap());
        let srv = std::thread::spawn(move || {
            let (mut c, _) = l.accept().unwrap();
            let (mut req, mut buf) = (vec![], [0; 512]);
            while !req.ends_with(b"\r\n\r\n") {
                let n = c.read(&mut buf).unwrap();
                req.extend_from_slice(&buf[..n]);
            }
            c.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"a\": 1}").unwrap();
            String::from_utf8_lossy(&req).into_owned()
        });
        assert_eq!(get(&base, "/api/0/buckets/").unwrap(), r#"{"a": 1}"#);
        assert!(srv.join().unwrap().starts_with("GET /api/0/buckets/ HTTP/1.0\r\nHost: 127.0.0.1:"));
    }
}
