// Line-based org parser covering what the agenda and zettel index need:
// headlines, TODO keywords, priority, tags, planning lines, property
// drawers, active timestamps and id:/file: links. Plus the text transforms
// (TODO cycling with repeaters, SCHEDULED/DEADLINE) shared by editor and agenda.

use chrono::{Datelike, Duration, Months, NaiveDate, NaiveDateTime, Weekday};
use regex::Regex;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::SystemTime;

static TS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"<(\d{4}-\d{2}-\d{2})(?:\s+[^\s>\d]+)?(?:\s+(\d{1,2}:\d{2})(?:-\d{1,2}:\d{2})?)?(?:\s+(\.\+|\+\+|\+)(\d+)([hdwmy])(?:/(\d+)([hdwmy]))?)?[^>]*>").unwrap()
});
static ALIAS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#""([^"]+)"|(\S+)"#).unwrap());
static TS_HEAD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^<\d{4}-\d{2}-\d{2}(?:\s+[^\s>\d]+)?").unwrap());
static LINK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[\[(id|file):([^\]]+)\](?:\[([^\]]*)\])?\]").unwrap());
static TAGS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+(:[^\s]+:)\s*$").unwrap());
static STATE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"^\s*- State "([^"]+)"\s+from .*?\[(\d{4}-\d{2}-\d{2})"#).unwrap());
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
    /// Habit max of `.+2d/3d`: (3, 'd').
    pub max: Option<(u32, char)>,
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
    /// `:STYLE: habit`
    pub habit: bool,
    /// Dates of `- State "DONE" from ...` log entries (any done keyword).
    pub done_log: Vec<NaiveDate>,
}

#[derive(Serialize, Clone, Debug)]
pub struct Link {
    /// An ID, or for `file:` links (FILE set) the raw path.
    pub target: String,
    pub file: bool,
    pub line: usize,
    pub text: String,
}

#[derive(Clone, Debug, Default)]
pub struct OrgFile {
    pub path: PathBuf,
    pub title: String,
    pub category: String,
    pub id: Option<String>,
    pub headlines: Vec<Headline>,
    pub links: Vec<Link>,
    /// `:ROAM_ALIASES:` of the file ("quoted names" may contain spaces).
    pub aliases: Vec<String>,
    /// The text it was parsed from, so searches needn't re-read the file.
    pub text: String,
}

fn parse_ts(c: &regex::Captures) -> Option<Ts> {
    Some(Ts {
        date: c[1].parse().ok()?,
        time: c.get(2).map(|m| m.as_str().to_string()),
        repeater: c.get(3).map(|k| (k.as_str().to_string(), c[4].parse().unwrap_or(1), c[5].chars().next().unwrap())),
        max: c.get(6).map(|m| (m.as_str().parse().unwrap_or(1), c[7].chars().next().unwrap())),
    })
}

fn heading_level(line: &str) -> Option<usize> {
    let n = line.bytes().take_while(|&b| b == b'*').count();
    (n > 0 && line[n..].starts_with(' ')).then_some(n)
}

pub(crate) fn is_planning(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("SCHEDULED:") || t.starts_with("DEADLINE:") || t.starts_with("CLOSED:")
}

/// (level, keyword, priority, title, tags)
pub(crate) type HeadParts = (usize, Option<String>, Option<String>, String, Vec<String>);

/// Split a headline into its parts.
pub(crate) fn split_headline(line: &str, kw: &Kw) -> Option<HeadParts> {
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
    let mut f = OrgFile { path: path.to_path_buf(), title: stem.clone(), text: text.to_string(), ..Default::default() };
    let mut file_cat = stem;
    let mut cats: Vec<(usize, String)> = vec![]; // (level, category) stack
    let mut cur: Option<Headline> = None;
    let mut in_drawer = false;
    for (i, &line) in lines.iter().enumerate() {
        for c in LINK.captures_iter(line) {
            f.links.push(Link { target: c[2].to_string(), file: &c[1] == "file", line: i, text: line.trim().to_string() });
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
                } else if c[1].eq_ignore_ascii_case("ROAM_ALIASES") {
                    f.aliases = ALIAS.captures_iter(&c[2]).map(|a| a.get(1).or(a.get(2)).unwrap().as_str().to_string()).collect();
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
                } else if c[1].eq_ignore_ascii_case("STYLE") {
                    h.habit = c[2].eq_ignore_ascii_case("habit");
                }
            }
            continue;
        }
        if let Some(c) = STATE.captures(line).filter(|c| kw.is_done(&c[1])) {
            h.done_log.extend(c[2].parse::<NaiveDate>().ok());
            continue;
        }
        h.timestamps.extend(TS.captures_iter(line).filter_map(|c| parse_ts(&c)));
    }
    if let Some(h) = cur {
        f.headlines.push(h);
    }
    f.category = file_cat;
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
    /// Set on today's agenda entry of a habit, and on its Tasks entry.
    pub habit: Option<Habit>,
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
        habit: None,
    }
}

pub const HABIT_DAYS: i64 = 21;

/// A habit's last HABIT_DAYS days ending today, one char per day: 'x' done,
/// '-' due but not yet overdue (between min and max of `.+2d/3d`), '!' due
/// and overdue, '.' not due; STREAK counts completions in a row with none
/// late (0 when overdue now).
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Habit {
    pub bar: String,
    pub streak: u32,
}

/// History of H if it is a habit. The due date in effect before each logged
/// completion is rebuilt from the repeater as org would have shifted it:
/// `.+` previous completion + interval; `+` SCHEDULED minus one interval per
/// later completion; `++` the first SCHEDULED - k·interval after the previous
/// completion (early completions aren't recorded, so they're assumed on time).
/// Before the first completion it is unknown, except for `+`.
pub fn habit(h: &Headline, today: NaiveDate) -> Option<Habit> {
    let s = h.scheduled.as_ref().filter(|_| h.habit)?;
    let (k, n, u) = s.repeater.clone()?;
    let mut done = h.done_log.clone();
    done.sort();
    done.dedup();
    let m = done.len();
    let back = |j: usize| sub_interval(s.date, n * j as u32, u);
    // Due date before done[i]; i == m is now.
    let due = |i: usize| match k.as_str() {
        _ if i == m => Some(s.date),
        "+" => Some(back(m - i)),
        _ if i == 0 => None,
        ".+" => Some(add_interval(done[i - 1], n, u)),
        _ => (0..2000).map(back).take_while(|&g| g > done[i - 1]).last(),
    };
    // Overdue after this; the due date itself without a max.
    let late = |d: NaiveDate| s.max.map_or(d, |(mn, mu)| d + (add_interval(d, mn, mu) - add_interval(d, n, u)));
    let bar = (0..HABIT_DAYS)
        .map(|i| today - Duration::days(HABIT_DAYS - 1 - i))
        .map(|d| {
            let i = done.partition_point(|&c| c < d);
            match due(i) {
                _ if done.get(i) == Some(&d) => 'x',
                Some(due) if d >= due && d <= late(due) && late(due) > due => '-',
                Some(due) if d >= due => '!',
                _ => '.',
            }
        })
        .collect();
    let on_time = |i: usize| due(i).is_none_or(|d| done[i] <= late(d));
    let streak = if m == 0 || today > late(s.date) { 0 } else { 1 + (1..m).rev().take_while(|&i| on_time(i)).count() as u32 };
    Some(Habit { bar, streak })
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

fn sub_interval(d: NaiveDate, n: u32, unit: char) -> NaiveDate {
    match unit {
        'm' => d.checked_sub_months(Months::new(n)).unwrap_or(d),
        'y' => d.checked_sub_months(Months::new(n * 12)).unwrap_or(d),
        _ => d - (add_interval(d, n, unit) - d),
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
                let hb = |d| habit(h, today).filter(|_| d == today);
                for d in occurrences(s, start, end) {
                    if !(overdue && d == today) {
                        out.push(Item { habit: hb(d), ..item(f, h, d, "scheduled", String::new(), s.time.clone()) });
                    }
                }
                if overdue {
                    let label = format!("Scheduled {}d ago", (today - s.date).num_days());
                    out.push(Item { habit: hb(today), ..item(f, h, today, "scheduled", label, None) });
                }
            }
            if let Some(dl) = &h.deadline {
                for d in occurrences(dl, start, end) {
                    out.push(item(f, h, d, "deadline", "Due".into(), dl.time.clone()));
                }
                let diff = (dl.date - today).num_days();
                if !done && today_in && diff < 0 {
                    out.push(item(f, h, today, "overdue", format!("Overdue {}d", -diff), None));
                } else if !done && today_in && (1..=14).contains(&diff) {
                    out.push(item(f, h, today, "warning", format!("Due in {diff}d"), None));
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

/// Tasks for which KEEP holds (open ones only, unless DONE), by priority then date.
pub fn todos(files: &[Arc<OrgFile>], kw: &Kw, done: bool, keep: impl Fn(&OrgFile, &Headline) -> bool, today: NaiveDate) -> Vec<Item> {
    let mut out = vec![];
    for f in files {
        for h in &f.headlines {
            if let Some(k) = h.keyword.as_deref().filter(|k| (done || !kw.is_done(k)) && keep(f, h)) {
                let date = h.scheduled.as_ref().or(h.deadline.as_ref()).map_or(NaiveDate::MAX, |t| t.date);
                out.push(Item { habit: habit(h, today), ..item(f, h, date, "todo", k.to_string(), None) });
            }
        }
    }
    out.sort_by(|a, b| (&a.priority.is_none(), &a.priority, a.date).cmp(&(&b.priority.is_none(), &b.priority, b.date)));
    out
}

// ---------------------------------------------------------------- task queries

type Filter = Box<dyn Fn(&OrgFile, &Headline) -> bool>;

/// Parse a task query: space-separated terms, all of which must hold. `todo:NEXT`,
/// `tag:work`, `pri:A`, `file:inbox` (file name contains), `scheduled:` / `due:` with
/// an optional `<`, `<=`, `>`, `>=` before a read_date date (`due:<+7d`, `due:today`)
/// or `none` / `any`; anything else (or a `"quoted phrase"`) must appear in the title.
/// A leading `-` negates a term. The bool says whether done tasks should be searched
/// too, i.e. the query asks for a done keyword (`todo:DONE`).
pub fn query(q: &str, today: NaiveDate, kw: &Kw) -> Result<(Filter, bool), String> {
    let mut words = vec![];
    let (mut cur, mut quoted) = (String::new(), false);
    for c in q.chars() {
        if c == '"' {
            quoted = !quoted;
        } else if c.is_whitespace() && !quoted {
            if !cur.is_empty() {
                words.push(std::mem::take(&mut cur));
            }
            continue;
        }
        cur.push(c);
    }
    words.extend(Some(cur).filter(|c| !c.is_empty()));
    let mut terms: Vec<Filter> = vec![];
    let mut done = false;
    for word in &words {
        let (neg, w) = match word.strip_prefix('-') {
            Some(w) if !w.is_empty() => (true, w),
            _ => (false, word.as_str()),
        };
        let (key, val) = if w.starts_with('"') { ("", w.trim_matches('"')) } else { w.split_once(':').unwrap_or(("", w)) };
        let v = val.to_lowercase();
        let t: Filter = match key.to_lowercase().as_str() {
            "todo" => {
                done |= !neg && kw.done.iter().any(|k| k.to_lowercase() == v);
                Box::new(move |_, h| h.keyword.as_ref().is_some_and(|k| k.to_lowercase() == v))
            }
            "tag" => Box::new(move |_, h| h.tags.iter().any(|t| t.to_lowercase() == v)),
            "pri" => Box::new(move |_, h| h.priority.as_ref().is_some_and(|p| p.to_lowercase() == v)),
            "file" => Box::new(move |f, _| f.path.file_stem().is_some_and(|s| s.to_string_lossy().to_lowercase().contains(&v))),
            k @ ("scheduled" | "due") => {
                let get: fn(&Headline) -> Option<NaiveDate> = if k == "due" { |h: &Headline| h.deadline.as_ref().map(|t| t.date) } else { |h: &Headline| h.scheduled.as_ref().map(|t| t.date) };
                match v.as_str() {
                    "none" => Box::new(move |_, h| get(h).is_none()),
                    "any" => Box::new(move |_, h| get(h).is_some()),
                    _ => {
                        let (op, rest) = ["<=", ">=", "<", ">"].iter().find_map(|o| v.strip_prefix(o).map(|r| (*o, r))).unwrap_or(("=", &v));
                        let d = read_date(rest, today).ok_or_else(|| format!("can't read date in \"{word}\""))?.0;
                        Box::new(move |_, h| {
                            let Some(x) = get(h) else { return false };
                            match op { "<=" => x <= d, ">=" => x >= d, "<" => x < d, ">" => x > d, _ => x == d }
                        })
                    }
                }
            }
            _ => Box::new(move |_, h| h.title.to_lowercase().contains(&v)),
        };
        terms.push(if neg { Box::new(move |f, h| !t(f, h)) } else { t });
    }
    Ok((Box::new(move |f, h| terms.iter().all(|t| t(f, h))), done))
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

/// Replace the date (and day name) of the timestamp following KIND on a
/// planning line; F returns None to leave it alone.
fn shift_planning(line: &str, kind: &str, f: impl Fn(&Ts) -> Option<NaiveDate>) -> String {
    let Some(pos) = line.find(&format!("{kind}:")) else { return line.to_string() };
    let rest = &line[pos..];
    let Some(c) = TS.captures(rest) else { return line.to_string() };
    let Some(new) = parse_ts(&c).and_then(|ts| f(&ts)) else { return line.to_string() };
    let m = c.get(0).unwrap();
    let shifted = TS_HEAD.replace(m.as_str(), format!("<{}", new.format("%Y-%m-%d %a")));
    format!("{}{}{}{}", &line[..pos], &rest[..m.start()], shifted, &rest[m.end()..])
}

/// The editable parts of a headline.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Parts {
    pub level: usize,
    pub keyword: Option<String>,
    pub priority: Option<String>,
    pub title: String,
    pub tags: Vec<String>,
}

impl Parts {
    fn parse(line: &str, kw: &Kw) -> Option<Parts> {
        split_headline(line, kw).map(|(level, keyword, priority, title, tags)| Parts { level, keyword, priority, title, tags })
    }
    fn line(&self) -> String {
        let mut s = "*".repeat(self.level);
        if let Some(k) = &self.keyword {
            s += &format!(" {k}");
        }
        if let Some(p) = &self.priority {
            s += &format!(" [#{p}]");
        }
        if !self.title.is_empty() {
            s += &format!(" {}", self.title);
        }
        if !self.tags.is_empty() {
            s += &format!(" :{}:", self.tags.join(":"));
        }
        s
    }
}

fn to_lines(text: &str) -> Vec<String> {
    text.lines().map(String::from).collect()
}

/// The headline at or above LINE.
pub fn heading(text: &str, line: usize, kw: &Kw) -> Option<Parts> {
    let lines = to_lines(text);
    Parts::parse(&lines[headline_at(&lines, line)?], kw)
}

fn edit_heading(text: &str, line: usize, kw: &Kw, f: impl FnOnce(&mut Parts)) -> String {
    let mut lines = to_lines(text);
    let Some(hl) = headline_at(&lines, line) else { return text.to_string() };
    let Some(mut p) = Parts::parse(&lines[hl], kw) else { return text.to_string() };
    f(&mut p);
    lines[hl] = p.line();
    join(lines, text)
}

/// Set the TODO keyword (None clears it). Marking a repeating task done
/// shifts its dates, sets `:LAST_REPEAT:`, logs `- State "DONE" from "TODO"
/// [date time]` below the planning line and drawers (inside a LOGBOOK if
/// there is one) and resets it, like org.
pub fn set_keyword(text: &str, line: usize, kw: &Kw, new: Option<&str>, now: NaiveDateTime) -> String {
    let today = now.date();
    let mut lines = to_lines(text);
    let Some(hl) = headline_at(&lines, line) else { return text.to_string() };
    let Some(mut p) = Parts::parse(&lines[hl], kw) else { return text.to_string() };
    let mut next = new.map(String::from);
    let planning = hl + 1 < lines.len() && is_planning(&lines[hl + 1]);
    let repeats = planning && TS.captures_iter(&lines[hl + 1]).any(|c| c.get(3).is_some());
    if next.as_deref().is_some_and(|k| kw.is_done(k)) && repeats {
        for kind in ["SCHEDULED", "DEADLINE"] {
            lines[hl + 1] = shift_planning(&lines[hl + 1], kind, |ts| {
                let (k, n, u) = ts.repeater.as_ref()?;
                Some(match k.as_str() {
                    ".+" => add_interval(today, *n, *u),
                    "++" => {
                        let mut d = add_interval(ts.date, *n, *u);
                        while d <= today {
                            d = add_interval(d, *n, *u);
                        }
                        d
                    }
                    _ => add_interval(ts.date, *n, *u),
                })
            });
        }
        let stamp = format!("[{}]", now.format("%Y-%m-%d %a %H:%M"));
        let last = format!(":LAST_REPEAT: {stamp}");
        if lines.get(hl + 2).is_some_and(|l| l.trim().eq_ignore_ascii_case(":PROPERTIES:")) {
            let end = (hl + 3..lines.len()).find(|&j| lines[j].trim().eq_ignore_ascii_case(":END:")).unwrap_or(lines.len());
            match (hl + 3..end).find(|&j| lines[j].trim_start().to_ascii_uppercase().starts_with(":LAST_REPEAT:")) {
                Some(j) => lines[j] = last,
                None => lines.insert(end, last),
            }
        } else {
            lines.splice(hl + 2..hl + 2, [":PROPERTIES:".into(), last, ":END:".into()]);
        }
        let q = |k: Option<&str>| format!("\"{}\"", k.unwrap_or(""));
        let log = format!("- State {:<12} from {:<12} {stamp}", q(next.as_deref()), q(p.keyword.as_deref()));
        let mut i = hl + 2;
        if lines.get(i).is_some_and(|l| l.trim().eq_ignore_ascii_case(":PROPERTIES:")) {
            i = (i..lines.len()).find(|&j| lines[j].trim().eq_ignore_ascii_case(":END:")).map_or(i, |j| j + 1);
        }
        if lines.get(i).is_some_and(|l| l.trim().eq_ignore_ascii_case(":LOGBOOK:")) {
            i += 1;
        }
        lines.insert(i, log);
        next = p.keyword.clone().filter(|k| !kw.is_done(k)).or(kw.todo.first().cloned());
    }
    p.keyword = next;
    lines[hl] = p.line();
    join(lines, text)
}

/// Cycle the TODO keyword of the headline at LINE by DIR (+1/-1).
pub fn cycle_todo(text: &str, line: usize, kw: &Kw, dir: i32, now: NaiveDateTime) -> String {
    let Some(p) = heading(text, line, kw) else { return text.to_string() };
    let seq: Vec<Option<&str>> = std::iter::once(None).chain(kw.todo.iter().chain(&kw.done).map(|s| Some(s.as_str()))).collect();
    let i = seq.iter().position(|k| *k == p.keyword.as_deref()).unwrap_or(0) as i32;
    set_keyword(text, line, kw, seq[(i + dir).rem_euclid(seq.len() as i32) as usize], now)
}

pub fn set_priority(text: &str, line: usize, kw: &Kw, prio: Option<&str>) -> String {
    edit_heading(text, line, kw, |p| p.priority = prio.map(|s| s.to_uppercase()))
}

/// S-up (DIR 1) raises priority C → B → A → none → C, like org.
pub fn cycle_priority(text: &str, line: usize, kw: &Kw, dir: i32) -> String {
    edit_heading(text, line, kw, |p| {
        let seq = [None, Some("A"), Some("B"), Some("C")];
        let i = seq.iter().position(|x| p.priority.as_deref() == *x).unwrap_or(0) as i32;
        p.priority = seq[(i - dir).rem_euclid(4) as usize].map(String::from);
    })
}

pub fn set_tags(text: &str, line: usize, kw: &Kw, tags: Vec<String>) -> String {
    edit_heading(text, line, kw, |p| p.tags = tags.into_iter().filter(|t| !t.is_empty()).collect())
}

/// Move the KIND (SCHEDULED/DEADLINE) date of the headline at LINE by DAYS.
pub fn shift_date(text: &str, line: usize, kind: &str, days: i64) -> String {
    let mut lines = to_lines(text);
    let Some(hl) = headline_at(&lines, line) else { return text.to_string() };
    if hl + 1 < lines.len() && is_planning(&lines[hl + 1]) {
        lines[hl + 1] = shift_planning(&lines[hl + 1], kind, |ts| Some(ts.date + Duration::days(days)));
    }
    join(lines, text)
}

// ---------------------------------------------------------------- subtrees: refile, archive, capture

/// [start, end) line range of the subtree whose headline is at HL.
fn subtree_range(lines: &[String], hl: usize) -> (usize, usize) {
    let lvl = heading_level(&lines[hl]).unwrap_or(0);
    let end = (hl + 1..lines.len()).find(|&i| heading_level(&lines[i]).is_some_and(|l| l <= lvl)).unwrap_or(lines.len());
    (hl, end)
}

/// Shift heading levels so the first heading in SUB lands at LEVEL.
fn relevel(sub: &mut [String], level: usize) {
    let Some(base) = sub.iter().find_map(|l| heading_level(l)) else { return };
    for l in sub.iter_mut() {
        if let Some(n) = heading_level(l) {
            let new = (n as isize + level as isize - base as isize).max(1) as usize;
            *l = format!("{}{}", "*".repeat(new), &l[n..]);
        }
    }
}

/// Insert SUB as the last child of the heading at DST, or at top level at the end.
fn insert_subtree(lines: &mut Vec<String>, dst: Option<usize>, mut sub: Vec<String>) {
    let at = match dst {
        Some(d) => {
            relevel(&mut sub, heading_level(&lines[d]).unwrap_or(0) + 1);
            subtree_range(lines, d).1
        }
        None => {
            relevel(&mut sub, 1);
            lines.len()
        }
    };
    lines.splice(at..at, sub);
}

fn take_subtree(lines: &mut Vec<String>, line: usize) -> Result<Vec<String>, String> {
    let hl = headline_at(lines, line).ok_or("not on a heading")?;
    let (s, e) = subtree_range(lines, hl);
    Ok(lines.drain(s..e).collect())
}

fn check_target(lines: &[String], dst: Option<usize>) -> Result<(), String> {
    match dst {
        Some(d) if lines.get(d).and_then(|l| heading_level(l)).is_none() => Err("refile target is not a heading".into()),
        _ => Ok(()),
    }
}

/// Move the subtree at LINE of SRC under heading DST_LINE of DST (None: top level).
pub fn refile(src: &str, line: usize, dst: &str, dst_line: Option<usize>) -> Result<(String, String), String> {
    let mut s = to_lines(src);
    let sub = take_subtree(&mut s, line)?;
    let mut d = to_lines(dst);
    check_target(&d, dst_line)?;
    insert_subtree(&mut d, dst_line, sub);
    Ok((join(s, src), join_nl(d)))
}

/// Refile within one file.
pub fn refile_same(text: &str, line: usize, dst_line: Option<usize>) -> Result<String, String> {
    let mut l = to_lines(text);
    check_target(&l, dst_line)?;
    let hl = headline_at(&l, line).ok_or("not on a heading")?;
    let (s, e) = subtree_range(&l, hl);
    let dst = match dst_line {
        Some(d) if (s..e).contains(&d) => return Err("can't refile a subtree under itself".into()),
        Some(d) if d >= e => Some(d - (e - s)),
        d => d,
    };
    let sub: Vec<String> = l.drain(s..e).collect();
    insert_subtree(&mut l, dst, sub);
    Ok(join(l, text))
}

/// Add KEY to the property drawer of SUB's heading, creating the drawer.
fn set_property(sub: &mut Vec<String>, key: &str, val: &str) {
    let i = if sub.len() > 1 && is_planning(&sub[1]) { 2 } else { 1 };
    let entry = format!(":{key}: {val}");
    if sub.get(i).is_some_and(|l| l.trim().eq_ignore_ascii_case(":PROPERTIES:")) {
        let end = (i + 1..sub.len()).find(|&j| sub[j].trim().eq_ignore_ascii_case(":END:")).unwrap_or(sub.len());
        sub.insert(end, entry);
    } else {
        sub.splice(i..i, [":PROPERTIES:".to_string(), entry, ":END:".to_string()]);
    }
}

/// Move the subtree at LINE of SRC to the end of the archive file text DST, org style.
pub fn archive(src: &str, line: usize, dst: &str, src_path: &str, now: chrono::NaiveDateTime) -> Result<(String, String), String> {
    let mut s = to_lines(src);
    let mut sub = take_subtree(&mut s, line)?;
    set_property(&mut sub, "ARCHIVE_TIME", &now.format("%Y-%m-%d %a %H:%M").to_string());
    set_property(&mut sub, "ARCHIVE_FILE", src_path);
    let mut d = to_lines(dst);
    if d.is_empty() {
        d = vec!["#    -*- mode: org -*-".into(), String::new(), String::new(), format!("Archived entries from file {src_path}"), String::new()];
    }
    insert_subtree(&mut d, None, sub);
    Ok((join(s, src), join_nl(d)))
}

/// Insert a captured ENTRY into DST: as last child of the first heading titled
/// HEADING (created at the end if missing), or at the end of the file.
pub fn capture_insert(dst: &str, heading: Option<&str>, entry: &str, kw: &Kw) -> String {
    let mut d = to_lines(dst);
    let target = heading.filter(|h| !h.is_empty()).map(|h| match d.iter().position(|l| split_headline(l, kw).is_some_and(|p| p.3 == h)) {
        Some(i) => i,
        None => {
            d.push(format!("* {h}"));
            d.len() - 1
        }
    });
    insert_subtree(&mut d, target, to_lines(entry.trim_end()));
    join_nl(d)
}

/// Stands in for %? between expanding a template and filing it.
const CURSOR: char = '\u{1}';

/// Expand capture-template BODY: %t %T active date/timestamp, %u %U inactive ones, %^{Prompt}
/// via ASK, %i the selection SEL (continuation lines indented like the line it's on), %? the
/// cursor, %% a literal %.
pub fn expand_template(body: &str, now: chrono::NaiveDateTime, sel: &str, mut ask: impl FnMut(&str) -> String) -> String {
    let (d, t) = (now.date(), now.format("%H:%M").to_string());
    let inactive = |s: String| format!("[{}]", &s[1..s.len() - 1]);
    let (mut out, mut rest) = (String::new(), body);
    while let Some(i) = rest.find('%') {
        out.push_str(&rest[..i]);
        rest = &rest[i + 1..];
        let prompt = rest.strip_prefix("^{").and_then(|r| r.find('}').map(|e| &r[..e]));
        let (s, n) = match (rest.chars().next(), prompt) {
            (Some('t'), _) => (ts_string(d, None), 1),
            (Some('T'), _) => (ts_string(d, Some(&t)), 1),
            (Some('u'), _) => (inactive(ts_string(d, None)), 1),
            (Some('U'), _) => (inactive(ts_string(d, Some(&t))), 1),
            (Some('i'), _) => {
                let ind: String = out[out.rfind('\n').map_or(0, |j| j + 1)..].chars().take_while(|c| c.is_whitespace()).collect();
                (sel.split('\n').enumerate().map(|(k, l)| if k == 0 || l.is_empty() { l.into() } else { format!("{ind}{l}") }).collect::<Vec<_>>().join("\n"), 1)
            }
            (Some('?'), _) => (CURSOR.to_string(), 1),
            (Some('%'), _) => ("%".into(), 1),
            (Some('^'), Some(p)) => (ask(p), p.len() + 3),
            _ => ("%".into(), 0),
        };
        out.push_str(&s);
        rest = &rest[n..];
    }
    out + rest
}

/// The distinct %^{Prompt} names in BODY, in order, for the UI to ask before expanding.
pub fn template_prompts(body: &str) -> Vec<String> {
    let mut v: Vec<String> = vec![];
    expand_template(body, chrono::NaiveDateTime::default(), "", |p| {
        if !v.iter().any(|x| x == p) {
            v.push(p.into());
        }
        String::new()
    });
    v
}

/// capture_insert an expanded template ENTRY; returns the new text and the (line, UTF-16 column)
/// of its %? cursor, or of the entry's end if it has none. A %? alone on the last line joins the
/// line before, so it leaves no blank line.
pub fn capture_template(dst: &str, heading: Option<&str>, entry: &str, kw: &Kw) -> (String, usize, usize) {
    let e = entry.trim_end();
    let entry = match e.strip_suffix(CURSOR) {
        Some(p) => format!("{}{CURSOR}", p.trim_end_matches('\n')),
        None if e.contains(CURSOR) => entry.to_string(),
        None => format!("{e}{CURSOR}"),
    };
    let text = capture_insert(dst, heading, &entry, kw);
    let i = text.find(CURSOR).unwrap_or(text.len());
    let bol = text[..i].rfind('\n').map_or(0, |j| j + 1);
    (text.replacen(CURSOR, "", 1), text[..i].matches('\n').count(), text[bol..i].encode_utf16().count())
}

/// A new task heading with optional planning line.
pub fn task_entry(title: &str, keyword: Option<&str>, priority: Option<&str>, tags: Vec<String>, scheduled: Option<(NaiveDate, Option<String>)>, deadline: Option<(NaiveDate, Option<String>)>) -> String {
    let p = Parts { level: 1, keyword: keyword.map(String::from), priority: priority.map(String::from), title: title.trim().into(), tags };
    let planning: Vec<String> = [("SCHEDULED", scheduled), ("DEADLINE", deadline)]
        .into_iter()
        .filter_map(|(k, d)| d.map(|(d, t)| format!("{k}: {}", ts_string(d, t.as_deref()))))
        .collect();
    if planning.is_empty() { p.line() + "\n" } else { format!("{}\n{}\n", p.line(), planning.join(" ")) }
}

/// strftime that reports bad format strings instead of panicking.
pub fn fmt_time(t: chrono::NaiveDateTime, f: &str) -> Option<String> {
    use std::fmt::Write;
    let mut s = String::new();
    write!(s, "{}", t.format(f)).ok()?;
    Some(s)
}

#[derive(Serialize, Clone, Debug)]
pub struct Target {
    pub path: PathBuf,
    pub line: Option<usize>,
    pub label: String,
}

/// Refile/capture targets: every file (top level) and every heading, labelled by outline path.
pub fn targets(files: &[Arc<OrgFile>], root: &Path) -> Vec<Target> {
    let mut out = vec![];
    for f in files {
        let rel = f.path.strip_prefix(root).unwrap_or(&f.path).display().to_string();
        out.push(Target { path: f.path.clone(), line: None, label: rel.clone() });
        let mut stack: Vec<(usize, &str)> = vec![];
        for h in &f.headlines {
            while stack.last().is_some_and(|(l, _)| *l >= h.level) {
                stack.pop();
            }
            stack.push((h.level, &h.title));
            let olp: Vec<&str> = stack.iter().map(|(_, t)| *t).collect();
            out.push(Target { path: f.path.clone(), line: Some(h.line), label: format!("{rel} / {}", olp.join(" / ")) });
        }
    }
    out
}

pub fn all_tags(files: &[Arc<OrgFile>]) -> Vec<String> {
    let mut t: Vec<String> = files.iter().flat_map(|f| f.headlines.iter().flat_map(|h| h.tags.clone())).collect();
    t.sort();
    t.dedup();
    t
}

fn join_nl(lines: Vec<String>) -> String {
    let mut s = lines.join("\n");
    s.push('\n');
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
    fn at() -> NaiveDateTime {
        d("2026-10-02").and_hms_opt(14, 30, 0).unwrap()
    }
    fn d(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    const DOC: &str = ":PROPERTIES:\n:ID: file-1\n:END:\n#+title: My Note\n#+category: Work\n\
* TODO [#A] Write report :work:urgent:\nSCHEDULED: <2026-10-01 Thu> DEADLINE: <2026-10-05 Mon 10:00>\n:PROPERTIES:\n:ID: h-1\n:END:\nSee [[id:other][Other]] and [[file:b.org]].\n\
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
        assert_eq!((f.links[0].target.as_str(), f.links[0].file), ("other", false));
        assert_eq!((f.links[1].target.as_str(), f.links[1].file), ("b.org", true));
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
        let log = |from: &str| format!(":PROPERTIES:\n:LAST_REPEAT: [2026-10-02 Fri 14:30]\n:END:\n- State \"DONE\"       from \"{from}\"       [2026-10-02 Fri 14:30]");
        let t = "* TODO Water plants\nSCHEDULED: <2026-09-28 Mon +1w>\n";
        let next = cycle_todo(t, 0, &kw(), 1, at());
        assert_eq!(next, "* NEXT Water plants\nSCHEDULED: <2026-09-28 Mon +1w>\n");
        let done = cycle_todo(&next, 1, &kw(), 1, at());
        assert_eq!(done, format!("* NEXT Water plants\nSCHEDULED: <2026-10-05 Mon +1w>\n{}\n", log("NEXT")));
        let catchup = cycle_todo("* NEXT X\nDEADLINE: <2026-09-01 Tue ++1w>", 0, &kw(), 1, at());
        assert_eq!(catchup, format!("* NEXT X\nDEADLINE: <2026-10-06 Tue ++1w>\n{}", log("NEXT")));
        // LAST_REPEAT goes into an existing drawer (replacing an old one); log after it, inside a LOGBOOK.
        let t = "* TODO H\nSCHEDULED: <2026-10-01 Thu .+1d/3d>\n:PROPERTIES:\n:STYLE: habit\n:LAST_REPEAT: [2026-09-30 Wed 08:00]\n:END:\n:LOGBOOK:\n- State \"DONE\"       from \"TODO\"       [2026-09-30 Wed]\n:END:\n";
        let done = set_keyword(t, 0, &kw(), Some("DONE"), at());
        assert_eq!(done, "* TODO H\nSCHEDULED: <2026-10-03 Sat .+1d/3d>\n:PROPERTIES:\n:STYLE: habit\n:LAST_REPEAT: [2026-10-02 Fri 14:30]\n:END:\n:LOGBOOK:\n- State \"DONE\"       from \"TODO\"       [2026-10-02 Fri 14:30]\n- State \"DONE\"       from \"TODO\"       [2026-09-30 Wed]\n:END:\n");
        // Both timed and date-only entries are read.
        let h = &parse(Path::new("h.org"), &done, &kw()).headlines[0];
        assert_eq!(h.done_log, vec![d("2026-10-02"), d("2026-09-30")]);
        assert_eq!(h.scheduled.as_ref().unwrap().max, Some((3, 'd')));
        let t = "* TODO H\nSCHEDULED: <2026-10-01 Thu .+1d>\n:PROPERTIES:\n:STYLE: habit\n:END:\n";
        assert!(set_keyword(t, 0, &kw(), Some("DONE"), at()).contains(":STYLE: habit\n:LAST_REPEAT: [2026-10-02 Fri 14:30]\n:END:\n- State"));
    }

    #[test]
    fn habits() {
        let log = |ds: &[&str]| ds.iter().map(|s| format!("- State \"DONE\"       from \"TODO\"       [{s} Mon 08:00]\n")).collect::<String>();
        let doc = |sched: &str, ds: &[&str]| format!("* TODO Run\nSCHEDULED: <{sched}>\n:PROPERTIES:\n:STYLE: habit\n:END:\n{}", log(ds));
        let hab = |sched: &str, ds: &[&str], today: &str| habit(&parse(Path::new("h.org"), &doc(sched, ds), &kw()).headlines[0], d(today)).unwrap();
        // Daily, done the last 3 days; the 18 days before are unknown history.
        let h = hab("2026-10-03 Sat .+1d", &["2026-09-30", "2026-10-01", "2026-10-02"], "2026-10-02");
        assert_eq!((h.bar.as_str(), h.streak), (format!("{}xxx", ".".repeat(18)).as_str(), 3));
        // A missed day breaks the streak; due-but-open today keeps it.
        let h = hab("2026-10-02 Fri .+1d", &["2026-09-28", "2026-09-30", "2026-10-01"], "2026-10-02");
        assert_eq!((&h.bar[16..], h.streak), ("x!xx!", 2));
        // Overdue now: streak 0, open days marked due.
        let h = hab("2026-09-30 Wed .+2d", &["2026-09-28"], "2026-10-02");
        assert_eq!((&h.bar[16..], h.streak), ("x.!!!", 0));
        // Weekly: only days past the due date count as due.
        let h = hab("2026-10-05 Mon ++1w", &["2026-09-21", "2026-09-28"], "2026-10-02");
        assert_eq!((&h.bar[9..], h.streak), ("x......x....", 2));
        // `+`: due dates step back one interval per completion from SCHEDULED.
        let h = hab("2026-10-05 Mon +1w", &["2026-09-23", "2026-09-30"], "2026-10-02");
        assert_eq!((&h.bar[9..], h.streak), ("!!x....!!x..", 1));
        // `++`: due on the first grid day after the previous completion.
        let h = hab("2026-10-05 Mon ++1w", &["2026-09-16", "2026-09-30"], "2026-10-02");
        assert_eq!((&h.bar[4..], h.streak), ("x....!!!!!!!!!x..", 1));
        // `.+2d/4d`: '-' from due until the max, then '!'.
        let h = hab("2026-10-02 Fri .+2d/4d", &["2026-09-26", "2026-09-30"], "2026-10-03");
        assert_eq!((&h.bar[13..], h.streak), ("x.--x.--", 2));
        let h = hab("2026-10-02 Fri .+2d/4d", &["2026-09-30"], "2026-10-05");
        assert_eq!((&h.bar[15..], h.streak), ("x.---!", 0));
        // Not a habit without STYLE.
        assert!(habit(&parse(Path::new("h.org"), "* TODO X\nSCHEDULED: <2026-10-01 Thu .+1d>\n", &kw()).headlines[0], d("2026-10-02")).is_none());
        // Agenda carries it on today's entry only.
        let fs = [Arc::new(parse(Path::new("h.org"), &doc("2026-10-02 Fri .+1d", &[]), &kw()))];
        let items = agenda(&fs, &kw(), d("2026-10-02"), 2, d("2026-10-02"));
        assert_eq!(items.iter().map(|i| i.habit.as_ref().map(|h| &h.bar[19..])).collect::<Vec<_>>(), vec![Some(".!"), None]);
        assert!(todos(&fs, &kw(), false, |_, _| true, d("2026-10-02"))[0].habit.is_some());
        assert_eq!(cycle_todo("* X :t:", 0, &kw(), 1, at()), "* TODO X :t:");
        assert_eq!(cycle_todo("* DONE X", 0, &kw(), 1, at()), "* X");
    }

    #[test]
    fn headline_edits() {
        let t = "* TODO [#B] Task :a:\nbody\n";
        assert_eq!(cycle_priority(t, 1, &kw(), 1), "* TODO [#A] Task :a:\nbody\n");
        assert_eq!(cycle_priority("* X", 0, &kw(), 1), "* [#C] X");
        assert_eq!(cycle_priority("* [#A] X", 0, &kw(), 1), "* X");
        assert_eq!(set_priority(t, 0, &kw(), None), "* TODO Task :a:\nbody\n");
        assert_eq!(set_tags(t, 0, &kw(), vec!["x".into(), "y".into()]), "* TODO [#B] Task :x:y:\nbody\n");
        assert_eq!(set_keyword(t, 0, &kw(), None, at()), "* [#B] Task :a:\nbody\n");
        assert_eq!(heading(t, 1, &kw()).unwrap().title, "Task");
        let s = "* TODO A\nSCHEDULED: <2026-10-02 Fri> DEADLINE: <2026-10-09 Fri +1w>\n";
        assert_eq!(shift_date(s, 0, "SCHEDULED", 1), "* TODO A\nSCHEDULED: <2026-10-03 Sat> DEADLINE: <2026-10-09 Fri +1w>\n");
        assert_eq!(shift_date(s, 0, "DEADLINE", -7), "* TODO A\nSCHEDULED: <2026-10-02 Fri> DEADLINE: <2026-10-02 Fri +1w>\n");
    }

    #[test]
    fn refile_archive_capture() {
        let src = "* Inbox\n** TODO Buy milk\nnote\n*** sub\n* Other\n";
        let (s, dst) = refile(src, 2, "#+title: P\n* Projects\n** Home\n* Later\n", Some(2)).unwrap();
        assert_eq!(s, "* Inbox\n* Other\n");
        assert_eq!(dst, "#+title: P\n* Projects\n** Home\n*** TODO Buy milk\nnote\n**** sub\n* Later\n");
        assert_eq!(refile_same(src, 1, Some(4)).unwrap(), "* Inbox\n* Other\n** TODO Buy milk\nnote\n*** sub\n");
        assert_eq!(refile_same(src, 1, None).unwrap(), "* Inbox\n* Other\n* TODO Buy milk\nnote\n** sub\n");
        assert!(refile_same(src, 1, Some(3)).is_err());
        let now = chrono::NaiveDateTime::parse_from_str("2026-10-02 14:30", "%Y-%m-%d %H:%M").unwrap();
        let (s, a) = archive("* DONE A\nSCHEDULED: <2026-10-01 Thu>\n* B\n", 0, "", "/n/x.org", now).unwrap();
        assert_eq!(s, "* B\n");
        assert!(a.ends_with("* DONE A\nSCHEDULED: <2026-10-01 Thu>\n:PROPERTIES:\n:ARCHIVE_TIME: 2026-10-02 Fri 14:30\n:ARCHIVE_FILE: /n/x.org\n:END:\n"), "{a}");
        assert_eq!(capture_insert("#+title: Inbox\n", Some("Tasks"), "* TODO new\n", &kw()), "#+title: Inbox\n* Tasks\n** TODO new\n");
        assert_eq!(capture_insert("* Tasks\n** old\n* Z\n", Some("Tasks"), "* TODO new", &kw()), "* Tasks\n** old\n** TODO new\n* Z\n");
        assert_eq!(capture_insert("", None, "* TODO x\n[2026-10-02 Fri]\n\n", &kw()), "* TODO x\n[2026-10-02 Fri]\n");
        assert_eq!(fmt_time(now, "daily/%Y-%m-%d.org").unwrap(), "daily/2026-10-02.org");
        assert_eq!(fmt_time(now, "%Q"), None);
        assert_eq!(task_entry("Call", Some("TODO"), Some("A"), vec!["home".into()], Some((d("2026-10-02"), None)), Some((d("2026-10-05"), Some("09:00".into())))),
            "* TODO [#A] Call :home:\nSCHEDULED: <2026-10-02 Fri> DEADLINE: <2026-10-05 Mon 09:00>\n");
        assert_eq!(task_entry("x", Some("TODO"), None, vec![], None, None), "* TODO x\n");
    }

    #[test]
    fn templates() {
        let now = chrono::NaiveDateTime::parse_from_str("2026-10-02 14:30", "%Y-%m-%d %H:%M").unwrap();
        let body = "* %^{Title} %t\n%U %T %u 100%% %x %^{Who} %^{Title}\n%i%?";
        assert_eq!(template_prompts(body), ["Title", "Who"]);
        let ans = |p: &str| if p == "Title" { "Sync".into() } else { "Ann".into() };
        let e = expand_template(body, now, "sel", ans);
        assert_eq!(e, "* Sync <2026-10-02 Fri>\n[2026-10-02 Fri 14:30] <2026-10-02 Fri 14:30> [2026-10-02 Fri] 100% %x Ann Sync\nsel\u{1}");
        assert_eq!(expand_template("%^{open %", now, "", ans), "%^{open %");
        let (t, l, c) = capture_template("* Meetings\n* Z\n", Some("Meetings"), &e, &kw());
        assert_eq!(t, "* Meetings\n** Sync <2026-10-02 Fri>\n[2026-10-02 Fri 14:30] <2026-10-02 Fri 14:30> [2026-10-02 Fri] 100% %x Ann Sync\nsel\n* Z\n");
        assert_eq!((l, c), (3, 3));
        assert_eq!(capture_template("", None, "* Ä\n", &kw()), ("* Ä\n".into(), 0, 3));
        // %? ending the body leaves no blank line; one mid-body stays put.
        let e = expand_template("* N\n%U\n%i%?\n", now, "", ans);
        assert_eq!(capture_template("", None, &e, &kw()), ("* N\n[2026-10-02 Fri 14:30]\n".into(), 1, 22));
        assert_eq!(capture_template("", None, "* N\n\u{1}\nx\n", &kw()), ("* N\n\nx\n".into(), 1, 0));
        // Multi-line %i lines up with the line it's inserted on.
        assert_eq!(expand_template("* N\n  - %i\n%i", now, "a\n\nb", ans), "* N\n  - a\n\n  b\na\n\nb");
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
    fn queries() {
        let t = d("2026-10-02");
        let doc = "* NEXT [#A] Write report :work:\nDEADLINE: <2026-10-05 Mon>\n* TODO Call mum :home:\nSCHEDULED: <2026-10-02 Fri>\n\
* TODO [#B] Plan trip :home:work:\nSCHEDULED: <2026-10-20 Tue> DEADLINE: <2026-11-01 Sun>\n* DONE Old report :work:\n";
        let files = [Arc::new(parse(Path::new("/n/inbox.org"), doc, &kw())), Arc::new(parse(Path::new("/n/projects.org"), "* TODO Report draft\n", &kw()))];
        let q = |s: &str| {
            let (f, done) = query(s, t, &kw()).unwrap();
            todos(&files, &kw(), done, f, t).into_iter().map(|i| i.title).collect::<Vec<_>>().join(", ")
        };
        assert_eq!(q(""), "Write report, Plan trip, Call mum, Report draft");
        assert_eq!(q("todo:next"), "Write report");
        assert_eq!(q("tag:work"), "Write report, Plan trip");
        assert_eq!(q("-tag:home"), "Write report, Report draft");
        assert_eq!(q("pri:A"), "Write report");
        assert_eq!(q("file:proj"), "Report draft");
        assert_eq!(q("-file:inbox"), "Report draft");
        assert_eq!(q("due:<+7d"), "Write report");
        assert_eq!(q("due:2026-10-05"), "Write report");
        assert_eq!(q("due:>=+1w"), "Plan trip");
        assert_eq!(q("scheduled:today"), "Call mum");
        assert_eq!(q("scheduled:<=today"), "Call mum");
        assert_eq!(q("scheduled:>today"), "Plan trip");
        assert_eq!(q("REPORT"), "Write report, Report draft");
        assert_eq!(q("todo:DONE"), "Old report");
        assert_eq!(q("todo:done report"), "Old report");
        assert_eq!(q("-todo:DONE report"), "Write report, Report draft"); // done ones still left out
        assert_eq!(q("due:none"), "Call mum, Report draft");
        assert_eq!(q("due:any"), "Write report, Plan trip");
        assert_eq!(q("scheduled:none"), "Write report, Report draft");
        assert_eq!(q("-scheduled:any"), "Write report, Report draft");
        assert_eq!(q("scheduled:any due:none"), "Call mum");
        assert_eq!(q("\"write report\""), "Write report");
        assert_eq!(q("\"report\" draft"), "Report draft");
        assert_eq!(q("-\"call mum\" -\"plan trip\""), "Write report, Report draft");
        assert_eq!(q("\"plan  trip\""), ""); // phrase spacing is literal
        assert_eq!(q("\"due:none\""), ""); // quoted means title text, not a term
        assert_eq!(q("\"unclosed phrase"), "");
        assert_eq!(q("tag:work -tag:home report"), "Write report");
        assert_eq!(q("todo:TODO tag:home scheduled:<+1m"), "Plan trip, Call mum");
        assert_eq!(q("report -pri:a"), "Report draft");
        assert!(query("due:<someday", t, &kw()).is_err());
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
