// Line-based org parser covering what the agenda and zettel index need:
// headlines, TODO keywords, priority, tags, planning lines, property
// drawers, active timestamps and id: links. Plus the text transforms
// (TODO cycling with repeaters, SCHEDULED/DEADLINE) shared by editor and agenda.

use chrono::{Datelike, Duration, Months, NaiveDate, Weekday};
use regex::Regex;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::SystemTime;

static TS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"<(\d{4}-\d{2}-\d{2})(?:\s+[^\s>\d]+)?(?:\s+(\d{1,2}:\d{2})(?:-\d{1,2}:\d{2})?)?(?:\s+(\.\+|\+\+|\+)(\d+)([hdwmy]))?[^>]*>").unwrap()
});
static TS_HEAD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^<\d{4}-\d{2}-\d{2}(?:\s+[^\s>\d]+)?").unwrap());
static LINK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[\[id:([^\]]+)\](?:\[([^\]]*)\])?\]").unwrap());
static TAGS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+(:[^\s]+:)\s*$").unwrap());
static PROP: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*:([^:\s]+):\s*(.*?)\s*$").unwrap());

#[derive(Clone, Debug)]
pub struct Kw {
    pub todo: Vec<String>,
    pub done: Vec<String>,
}

impl Kw {
    fn is_kw(&self, w: &str) -> bool {
        self.todo.iter().chain(&self.done).any(|k| k == w)
    }
    pub fn is_done(&self, w: &str) -> bool {
        self.done.iter().any(|k| k == w)
    }
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Ts {
    pub date: NaiveDate,
    pub time: Option<String>,
    /// (kind "+", "++" or ".+", n, unit)
    pub repeater: Option<(String, u32, char)>,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct Headline {
    pub line: usize,
    pub level: usize,
    pub keyword: Option<String>,
    pub priority: Option<String>,
    pub title: String,
    pub tags: Vec<String>,
    pub scheduled: Option<Ts>,
    pub deadline: Option<Ts>,
    pub timestamps: Vec<Ts>,
    pub id: Option<String>,
    pub category: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct Link {
    pub target: String,
    pub line: usize,
    pub text: String,
}

#[derive(Clone, Debug, Default)]
pub struct OrgFile {
    pub path: PathBuf,
    pub title: String,
    pub id: Option<String>,
    pub headlines: Vec<Headline>,
    pub links: Vec<Link>,
}

fn parse_ts(c: &regex::Captures) -> Option<Ts> {
    Some(Ts {
        date: c[1].parse().ok()?,
        time: c.get(2).map(|m| m.as_str().to_string()),
        repeater: c.get(3).map(|k| (k.as_str().to_string(), c[4].parse().unwrap_or(1), c[5].chars().next().unwrap())),
    })
}

fn heading_level(line: &str) -> Option<usize> {
    let n = line.bytes().take_while(|&b| b == b'*').count();
    (n > 0 && line[n..].starts_with(' ')).then_some(n)
}

fn is_planning(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("SCHEDULED:") || t.starts_with("DEADLINE:") || t.starts_with("CLOSED:")
}

/// Split a headline into (level, keyword, priority, title, tags).
fn split_headline(line: &str, kw: &Kw) -> Option<(usize, Option<String>, Option<String>, String, Vec<String>)> {
    let level = heading_level(line)?;
    let mut rest = line[level..].trim();
    let mut tags = vec![];
    if let Some(c) = TAGS.captures(rest) {
        tags = c[1].split(':').filter(|t| !t.is_empty()).map(String::from).collect();
        rest = rest[..c.get(0).unwrap().start()].trim_end();
    } else if rest.starts_with(':') && rest.ends_with(':') && !rest.contains(' ') && rest.len() > 1 {
        tags = rest.split(':').filter(|t| !t.is_empty()).map(String::from).collect();
        rest = "";
    }
    let (first, after) = rest.split_once(' ').unwrap_or((rest, ""));
    let keyword = kw.is_kw(first).then(|| first.to_string());
    if keyword.is_some() {
        rest = after.trim_start();
    }
    let mut priority = None;
    if rest.len() >= 4 && rest.starts_with("[#") && rest.as_bytes()[3] == b']' {
        priority = Some(rest[2..3].to_string());
        rest = rest[4..].trim_start();
    }
    Some((level, keyword, priority, rest.to_string(), tags))
}

pub fn parse(path: &Path, text: &str, kw: &Kw) -> OrgFile {
    let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    let mut f = OrgFile { path: path.to_path_buf(), title: stem.clone(), ..Default::default() };
    let mut file_cat = stem;
    let mut cats: Vec<(usize, String)> = vec![]; // (level, category) stack
    let mut cur: Option<Headline> = None;
    let mut in_drawer = false;
    for (i, &line) in lines.iter().enumerate() {
        for c in LINK.captures_iter(line) {
            f.links.push(Link { target: c[1].to_string(), line: i, text: line.trim().to_string() });
        }
        if let Some((level, keyword, priority, title, tags)) = split_headline(line, kw) {
            if let Some(h) = cur.take() {
                cats.push((h.level, h.category.clone()));
                f.headlines.push(h);
            }
            in_drawer = false;
            while cats.last().is_some_and(|(l, _)| *l >= level) {
                cats.pop();
            }
            let category = cats.last().map_or(file_cat.clone(), |(_, c)| c.clone());
            cur = Some(Headline { line: i, level, keyword, priority, title, tags, category, ..Default::default() });
            continue;
        }
        let lower = line.trim_start().to_lowercase();
        let Some(h) = cur.as_mut() else {
            // File-level preamble.
            if lower.starts_with("#+title:") {
                f.title = line.trim_start()[8..].trim().to_string();
            } else if lower.starts_with("#+category:") {
                file_cat = line.trim_start()[11..].trim().to_string();
            } else if let Some(c) = PROP.captures(line) {
                if c[1].eq_ignore_ascii_case("ID") {
                    f.id = Some(c[2].to_string());
                } else if c[1].eq_ignore_ascii_case("CATEGORY") {
                    file_cat = c[2].to_string();
                }
            }
            continue;
        };
        if is_planning(line) && i == h.line + 1 {
            for c in TS.captures_iter(line) {
                let kw_before = &line[..c.get(0).unwrap().start()];
                if kw_before.trim_end().ends_with("SCHEDULED:") {
                    h.scheduled = parse_ts(&c);
                } else if kw_before.trim_end().ends_with("DEADLINE:") {
                    h.deadline = parse_ts(&c);
                }
            }
            continue;
        }
        if lower.trim() == ":properties:" {
            in_drawer = true;
            continue;
        }
        if in_drawer {
            if lower.trim() == ":end:" {
                in_drawer = false;
            } else if let Some(c) = PROP.captures(line) {
                if c[1].eq_ignore_ascii_case("ID") {
                    h.id = Some(c[2].to_string());
                } else if c[1].eq_ignore_ascii_case("CATEGORY") {
                    h.category = c[2].to_string();
                }
            }
            continue;
        }
        h.timestamps.extend(TS.captures_iter(line).filter_map(|c| parse_ts(&c)));
    }
    if let Some(h) = cur {
        f.headlines.push(h);
    }
    f
}

// ---------------------------------------------------------------- scanning

#[derive(Default)]
pub struct Cache(Mutex<HashMap<PathBuf, (SystemTime, Arc<OrgFile>)>>);

impl Cache {
    pub fn clear(&self) {
        self.0.lock().unwrap().clear();
    }
}

pub fn org_paths(dir: &Path) -> Vec<PathBuf> {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_entry(|e| e.depth() == 0 || !e.file_name().to_string_lossy().starts_with('.'))
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file() && e.path().extension().is_some_and(|x| x == "org"))
        .map(|e| e.into_path())
        .collect()
}

/// Parse every .org file under DIR, reusing cached parses whose mtime is unchanged.
pub fn scan(dir: &Path, kw: &Kw, cache: &Cache) -> Vec<Arc<OrgFile>> {
    let mut map = cache.0.lock().unwrap();
    let mut fresh = HashMap::new();
    for p in org_paths(dir) {
        let Ok(mtime) = std::fs::metadata(&p).and_then(|m| m.modified()) else { continue };
        let f = match map.remove(&p) {
            Some((t, f)) if t == mtime => f,
            _ => Arc::new(parse(&p, &std::fs::read_to_string(&p).unwrap_or_default(), kw)),
        };
        fresh.insert(p, (mtime, f));
    }
    *map = fresh;
    let mut v: Vec<_> = map.values().map(|(_, f)| f.clone()).collect();
    v.sort_by(|a, b| a.path.cmp(&b.path));
    v
}

// ---------------------------------------------------------------- agenda

#[derive(Serialize, Clone, Debug)]
pub struct Item {
    pub date: NaiveDate,
    pub kind: &'static str,
    pub label: String,
    pub time: Option<String>,
    pub path: PathBuf,
    pub line: usize,
    pub keyword: Option<String>,
    pub priority: Option<String>,
    pub title: String,
    pub tags: Vec<String>,
    pub category: String,
}

fn item(f: &OrgFile, h: &Headline, date: NaiveDate, kind: &'static str, label: String, time: Option<String>) -> Item {
    Item {
        date,
        kind,
        label,
        time,
        path: f.path.clone(),
        line: h.line,
        keyword: h.keyword.clone(),
        priority: h.priority.clone(),
        title: h.title.clone(),
        tags: h.tags.clone(),
        category: h.category.clone(),
    }
}

pub fn add_interval(d: NaiveDate, n: u32, unit: char) -> NaiveDate {
    match unit {
        'd' => d + Duration::days(n as i64),
        'w' => d + Duration::weeks(n as i64),
        'm' => d.checked_add_months(Months::new(n)).unwrap_or(d),
        'y' => d.checked_add_months(Months::new(n * 12)).unwrap_or(d),
        _ => d,
    }
}

/// Dates within [start, end] on which TS occurs, projecting repeaters forward.
fn occurrences(ts: &Ts, start: NaiveDate, end: NaiveDate) -> Vec<NaiveDate> {
    let mut out = vec![];
    let mut d = ts.date;
    for _ in 0..2000 {
        if d > end {
            break;
        }
        if d >= start {
            out.push(d);
        }
        match &ts.repeater {
            Some((_, n, u)) if *n > 0 && "dwmy".contains(*u) => d = add_interval(d, *n, *u),
            _ => break,
        }
    }
    out
}

pub fn agenda(files: &[Arc<OrgFile>], kw: &Kw, start: NaiveDate, days: i64, today: NaiveDate) -> Vec<Item> {
    let end = start + Duration::days(days - 1);
    let today_in = start <= today && today <= end;
    let mut out = vec![];
    for f in files {
        for h in &f.headlines {
            let done = h.keyword.as_deref().is_some_and(|k| kw.is_done(k));
            if let Some(s) = &h.scheduled {
                let overdue = !done && s.date < today && today_in;
                for d in occurrences(s, start, end) {
                    if !(overdue && d == today) {
                        out.push(item(f, h, d, "scheduled", "Scheduled:".into(), s.time.clone()));
                    }
                }
                if overdue {
                    out.push(item(f, h, today, "scheduled", format!("Sched.{:>3}x:", (today - s.date).num_days()), None));
                }
            }
            if let Some(dl) = &h.deadline {
                for d in occurrences(dl, start, end) {
                    out.push(item(f, h, d, "deadline", "Deadline:".into(), dl.time.clone()));
                }
                let diff = (dl.date - today).num_days();
                if !done && today_in && diff < 0 {
                    out.push(item(f, h, today, "overdue", format!("{} d. ago:", -diff), None));
                } else if !done && today_in && (1..=14).contains(&diff) {
                    out.push(item(f, h, today, "warning", format!("In {diff} d.:"), None));
                }
            }
            for ts in &h.timestamps {
                for d in occurrences(ts, start, end) {
                    out.push(item(f, h, d, "timestamp", String::new(), ts.time.clone()));
                }
            }
        }
    }
    let rank = |k: &str| ["overdue", "deadline", "scheduled", "warning", "timestamp"].iter().position(|x| *x == k);
    out.sort_by(|a, b| {
        (a.date, a.time.is_none(), &a.time, rank(a.kind), &a.priority).cmp(&(b.date, b.time.is_none(), &b.time, rank(b.kind), &b.priority))
    });
    out
}

pub fn todos(files: &[Arc<OrgFile>], kw: &Kw) -> Vec<Item> {
    let mut out = vec![];
    for f in files {
        for h in &f.headlines {
            if let Some(k) = h.keyword.as_deref().filter(|k| !kw.is_done(k)) {
                let date = h.scheduled.as_ref().or(h.deadline.as_ref()).map_or(NaiveDate::MAX, |t| t.date);
                out.push(item(f, h, date, "todo", k.to_string(), None));
            }
        }
    }
    out.sort_by(|a, b| (&a.priority.is_none(), &a.priority, a.date).cmp(&(&b.priority.is_none(), &b.priority, b.date)));
    out
}

// ---------------------------------------------------------------- text transforms

/// Line of the headline at or above LINE.
pub fn headline_at(lines: &[String], line: usize) -> Option<usize> {
    (0..=line.min(lines.len().saturating_sub(1))).rev().find(|&i| heading_level(&lines[i]).is_some())
}

fn ts_string(d: NaiveDate, time: Option<&str>) -> String {
    match time {
        Some(t) => format!("<{} {}>", d.format("%Y-%m-%d %a"), t),
        None => format!("<{}>", d.format("%Y-%m-%d %a")),
    }
}

/// Replace the date (and day name) of the timestamp following KIND on a planning line.
fn shift_planning(line: &str, kind: &str, f: impl Fn(NaiveDate, &(String, u32, char)) -> NaiveDate) -> String {
    let Some(pos) = line.find(&format!("{kind}:")) else { return line.to_string() };
    let rest = &line[pos..];
    let Some(c) = TS.captures(rest) else { return line.to_string() };
    let Some(ts) = parse_ts(&c) else { return line.to_string() };
    let Some(rep) = &ts.repeater else { return line.to_string() };
    let m = c.get(0).unwrap();
    let shifted = TS_HEAD.replace(m.as_str(), format!("<{}", f(ts.date, rep).format("%Y-%m-%d %a")));
    format!("{}{}{}{}", &line[..pos], &rest[..m.start()], shifted, &rest[m.end()..])
}

/// Cycle the TODO keyword of the headline at LINE by DIR (+1/-1). Marking a
/// repeating task done shifts its dates and resets it, like org.
pub fn cycle_todo(text: &str, line: usize, kw: &Kw, dir: i32, today: NaiveDate) -> String {
    let mut lines: Vec<String> = text.lines().map(String::from).collect();
    let Some(hl) = headline_at(&lines, line) else { return text.to_string() };
    let Some((level, cur, prio, title, tags)) = split_headline(&lines[hl], kw) else { return text.to_string() };
    let seq: Vec<Option<&str>> = std::iter::once(None).chain(kw.todo.iter().chain(&kw.done).map(|s| Some(s.as_str()))).collect();
    let i = seq.iter().position(|k| *k == cur.as_deref()).unwrap_or(0) as i32;
    let mut next = seq[(i + dir).rem_euclid(seq.len() as i32) as usize].map(String::from);

    let planning = hl + 1 < lines.len() && is_planning(&lines[hl + 1]);
    let repeats = planning && TS.captures_iter(&lines[hl + 1]).any(|c| c.get(3).is_some());
    if next.as_deref().is_some_and(|k| kw.is_done(k)) && repeats {
        for kind in ["SCHEDULED", "DEADLINE"] {
            lines[hl + 1] = shift_planning(&lines[hl + 1], kind, |d, (k, n, u)| match k.as_str() {
                ".+" => add_interval(today, *n, *u),
                "++" => {
                    let mut d = add_interval(d, *n, *u);
                    while d <= today {
                        d = add_interval(d, *n, *u);
                    }
                    d
                }
                _ => add_interval(d, *n, *u),
            });
        }
        next = cur.filter(|k| !kw.is_done(k)).or(kw.todo.first().cloned());
    }
    lines[hl] = build_headline(level, next.as_deref(), prio.as_deref(), &title, &tags);
    join(lines, text)
}

fn build_headline(level: usize, kw: Option<&str>, prio: Option<&str>, title: &str, tags: &[String]) -> String {
    let mut s = "*".repeat(level);
    if let Some(k) = kw {
        s += &format!(" {k}");
    }
    if let Some(p) = prio {
        s += &format!(" [#{p}]");
    }
    if !title.is_empty() {
        s += &format!(" {title}");
    }
    if !tags.is_empty() {
        s += &format!(" :{}:", tags.join(":"));
    }
    s
}

fn join(lines: Vec<String>, original: &str) -> String {
    let mut s = lines.join("\n");
    if original.ends_with('\n') {
        s.push('\n');
    }
    s
}

/// Set (or with DATE None, remove) SCHEDULED/DEADLINE on the headline at LINE.
pub fn set_planning(text: &str, line: usize, kind: &str, date: Option<(NaiveDate, Option<String>)>) -> String {
    let mut lines: Vec<String> = text.lines().map(String::from).collect();
    let Some(hl) = headline_at(&lines, line) else { return text.to_string() };
    let re = Regex::new(&format!(r"\s*{kind}:\s*<[^>]*>")).unwrap();
    let new = date.map(|(d, t)| format!("{kind}: {}", ts_string(d, t.as_deref())));
    if hl + 1 < lines.len() && is_planning(&lines[hl + 1]) {
        let p = &lines[hl + 1];
        let mut updated = match (&new, re.is_match(p)) {
            (Some(n), true) => re.replace(p, format!(" {n}")).trim_start().to_string(),
            (Some(n), false) => format!("{} {n}", p.trim_end()),
            (None, _) => re.replace(p, "").to_string(),
        };
        updated = updated.trim().to_string();
        if updated.is_empty() {
            lines.remove(hl + 1);
        } else {
            lines[hl + 1] = updated;
        }
    } else if let Some(n) = new {
        lines.insert(hl + 1, n);
    }
    join(lines, text)
}

/// Parse org-read-date style input: "", "today", "tomorrow", "+3d", "-1w",
/// "mon".."sun", "2026-10-02", "10-02", "15" (day of month), each optionally
/// followed by a time "14:00".
pub fn read_date(input: &str, today: NaiveDate) -> Option<(NaiveDate, Option<String>)> {
    let mut parts = input.split_whitespace();
    let first = parts.next().unwrap_or("").to_lowercase();
    let mut time = parts.next().map(String::from);
    let mut first = first.as_str();
    if first.contains(':') && time.is_none() {
        time = Some(first.to_string());
        first = "";
    }
    let rel = Regex::new(r"^([+-])(\d+)([dwmy]?)$").unwrap();
    let date = if first.is_empty() || first == "today" || first == "." {
        today
    } else if first == "tomorrow" {
        today + Duration::days(1)
    } else if let Some(c) = rel.captures(first) {
        let n: u32 = c[2].parse().ok()?;
        let unit = c[3].chars().next().unwrap_or('d');
        if &c[1] == "+" {
            add_interval(today, n, unit)
        } else {
            match unit {
                'd' => today - Duration::days(n as i64),
                'w' => today - Duration::weeks(n as i64),
                'm' => today.checked_sub_months(Months::new(n))?,
                _ => today.checked_sub_months(Months::new(n * 12))?,
            }
        }
    } else if let Some(wd) = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"].iter().position(|w| first.starts_with(w)) {
        let target = Weekday::try_from(wd as u8).ok()?;
        let mut d = today + Duration::days(1);
        while d.weekday() != target {
            d += Duration::days(1);
        }
        d
    } else if let Ok(d) = first.parse::<NaiveDate>() {
        d
    } else if let Ok(d) = NaiveDate::parse_from_str(&format!("{}-{first}", today.year()), "%Y-%m-%d") {
        d
    } else if let Ok(day) = first.parse::<u32>() {
        today.with_day(day)?
    } else {
        return None;
    };
    Some((date, time))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kw() -> Kw {
        Kw { todo: vec!["TODO".into(), "NEXT".into()], done: vec!["DONE".into()] }
    }
    fn d(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    const DOC: &str = ":PROPERTIES:\n:ID: file-1\n:END:\n#+title: My Note\n#+category: Work\n\
* TODO [#A] Write report :work:urgent:\nSCHEDULED: <2026-10-01 Thu> DEADLINE: <2026-10-05 Mon 10:00>\n:PROPERTIES:\n:ID: h-1\n:END:\nSee [[id:other][Other]].\n\
** NEXT Sub\n:PROPERTIES:\n:CATEGORY: Sub\n:END:\n*** DONE Deep\n* Meeting\n<2026-10-03 Sat 14:00>\n* TODO Water plants\nSCHEDULED: <2026-09-28 Mon +1w>\n";

    #[test]
    fn parses_headlines() {
        let f = parse(Path::new("/n/x.org"), DOC, &kw());
        assert_eq!((f.title.as_str(), f.id.as_deref()), ("My Note", Some("file-1")));
        let h = &f.headlines[0];
        assert_eq!((h.keyword.as_deref(), h.priority.as_deref(), h.title.as_str()), (Some("TODO"), Some("A"), "Write report"));
        assert_eq!(h.tags, vec!["work", "urgent"]);
        assert_eq!(h.scheduled.as_ref().unwrap().date, d("2026-10-01"));
        assert_eq!(h.deadline.as_ref().unwrap().time.as_deref(), Some("10:00"));
        assert_eq!((h.id.as_deref(), h.category.as_str()), (Some("h-1"), "Work"));
        assert_eq!(f.headlines[1].category, "Sub");
        assert_eq!(f.headlines[2].category, "Sub"); // inherited
        assert_eq!(f.headlines[3].timestamps[0].time.as_deref(), Some("14:00"));
        assert_eq!(f.links[0].target, "other");
    }

    #[test]
    fn agenda_week() {
        let f = Arc::new(parse(Path::new("/n/x.org"), DOC, &kw()));
        let items = agenda(&[f], &kw(), d("2026-09-28"), 7, d("2026-10-02"));
        let got: Vec<(String, &str, &str)> = items.iter().map(|i| (i.date.to_string(), i.kind, i.title.as_str())).collect();
        assert!(got.contains(&("2026-10-02".into(), "scheduled", "Write report")), "{got:?}"); // overdue, on today
        assert!(got.contains(&("2026-10-02".into(), "warning", "Write report")));
        assert!(got.contains(&("2026-10-03".into(), "timestamp", "Meeting")));
        assert!(got.contains(&("2026-09-28".into(), "scheduled", "Water plants")));
        assert!(!got.iter().any(|g| g.0 == "2026-10-04")); // nothing on Sunday
    }

    #[test]
    fn cycle_and_repeat() {
        let t = "* TODO Water plants\nSCHEDULED: <2026-09-28 Mon +1w>\n";
        let next = cycle_todo(t, 0, &kw(), 1, d("2026-10-02"));
        assert_eq!(next, "* NEXT Water plants\nSCHEDULED: <2026-09-28 Mon +1w>\n");
        let done = cycle_todo(&next, 1, &kw(), 1, d("2026-10-02"));
        assert_eq!(done, "* NEXT Water plants\nSCHEDULED: <2026-10-05 Mon +1w>\n");
        let catchup = cycle_todo("* NEXT X\nDEADLINE: <2026-09-01 Tue ++1w>", 0, &kw(), 1, d("2026-10-02"));
        assert_eq!(catchup, "* NEXT X\nDEADLINE: <2026-10-06 Tue ++1w>");
        assert_eq!(cycle_todo("* X :t:", 0, &kw(), 1, d("2026-10-02")), "* TODO X :t:");
        assert_eq!(cycle_todo("* DONE X", 0, &kw(), 1, d("2026-10-02")), "* X");
    }

    #[test]
    fn planning() {
        let t = "* TODO A\nbody\n";
        let s = set_planning(t, 1, "SCHEDULED", Some((d("2026-10-02"), None)));
        assert_eq!(s, "* TODO A\nSCHEDULED: <2026-10-02 Fri>\nbody\n");
        let s = set_planning(&s, 0, "DEADLINE", Some((d("2026-10-09"), Some("09:00".into()))));
        assert_eq!(s, "* TODO A\nSCHEDULED: <2026-10-02 Fri> DEADLINE: <2026-10-09 Fri 09:00>\nbody\n");
        let s = set_planning(&s, 0, "SCHEDULED", None);
        assert_eq!(s, "* TODO A\nDEADLINE: <2026-10-09 Fri 09:00>\nbody\n");
        assert_eq!(set_planning(&s, 0, "DEADLINE", None), t);
    }

    #[test]
    fn dates() {
        let t = d("2026-10-02"); // Friday
        assert_eq!(read_date("", t), Some((t, None)));
        assert_eq!(read_date("+3d", t).unwrap().0, d("2026-10-05"));
        assert_eq!(read_date("+1w 14:00", t), Some((d("2026-10-09"), Some("14:00".into()))));
        assert_eq!(read_date("mon", t).unwrap().0, d("2026-10-05"));
        assert_eq!(read_date("fri", t).unwrap().0, d("2026-10-09"));
        assert_eq!(read_date("12-24", t).unwrap().0, d("2026-12-24"));
        assert_eq!(read_date("2027-01-01", t).unwrap().0, d("2027-01-01"));
        assert_eq!(read_date("15", t).unwrap().0, d("2026-10-15"));
        assert_eq!(read_date("nonsense", t), None);
    }
}
