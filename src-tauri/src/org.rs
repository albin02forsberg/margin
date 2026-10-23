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
    Regex::new(r"<(\d{4}-\d{2}-\d{2})(?:\s+[^\s>\d+\-.][^\s>\d]*)?(?:\s+(\d{1,2}:\d{2})(?:-(\d{1,2}:\d{2}))?)?(?:\s+(\.\+|\+\+|\+)(\d+)([hdwmy])(?:/(\d+)([hdwmy]))?)?(?:\s+--?(\d+)([dwmy]))?[^>]*>").unwrap()
});
/// A repeater cookie as typed after a date: `+1w`, `++1m`, `.+2d/3d`.
pub static REPEATER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\.\+|\+\+|\+)(\d+)([hdwmy])(?:/\d+[hdwmy])?$").unwrap());
/// A warning cookie as typed after a date: `-3d`, `-1w`.
static WARN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^-\d+[dwmy]$").unwrap());
static ALIAS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#""([^"]+)"|(\S+)"#).unwrap());
static TS_HEAD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^<\d{4}-\d{2}-\d{2}(?:\s+[^\s>\d+\-.][^\s>\d]*)?").unwrap());
pub(crate) static LINK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[\[(id|file):([^\]]+)\](?:\[([^\]]*)\])?\]").unwrap());
static TAGS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+(:[^\s]+:)\s*$").unwrap());
static STATE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"^\s*- State "([^"]+)"\s+from .*?\[(\d{4}-\d{2}-\d{2})"#).unwrap());
static CLOSED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"CLOSED:\s*\[(\d{4}-\d{2}-\d{2})(?:\s+[^\s\]\d]+)?(?:\s+(\d{1,2}:\d{2}))?[^\]]*\]\s*").unwrap());
static PROP: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*:([^:\s]+):\s*(.*?)\s*$").unwrap());
static EFFORT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?:(\d{1,5}):(\d{2})|(?:(\d{1,5})\s*h)?\s*(?:(\d+)\s*m(?:in)?)?)$").unwrap());

/// An `:EFFORT:` value in minutes: `1:30`, `2h`, `45min`, `1h 30m`, or plain minutes `45`.
pub fn parse_effort(s: &str) -> Option<u32> {
    let s = s.trim().to_lowercase();
    if let Ok(m) = s.parse() {
        return Some(m);
    }
    let c = EFFORT.captures(&s).filter(|_| !s.is_empty())?;
    let n = |i: usize| c.get(i).map_or(Some(0), |m| m.as_str().parse::<u32>().ok());
    Some(n(1)? * 60 + n(2)? + n(3)? * 60 + n(4)?)
}

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
    /// Deadline warning cookie of `<... -3d>` (or `--3d`): (3, 'd').
    pub warn: Option<(u32, char)>,
    /// End of a time range `<... 10:00-11:00>`, or the time of a range's second timestamp.
    pub end_time: Option<String>,
    /// Last day of a date range `<a>--<b>`.
    pub end: Option<NaiveDate>,
}

impl Ts {
    /// The time as the agenda shows it: "10:00", or "10:00-11:00" for a time range.
    pub fn span(&self) -> Option<String> {
        let t = self.time.clone()?;
        Some(match self.end_time.as_ref().filter(|_| self.end.is_none()) {
            Some(e) => format!("{t}-{e}"),
            None => t,
        })
    }
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct Headline {
    pub line: usize,
    pub level: usize,
    pub keyword: Option<String>,
    pub priority: Option<String>,
    pub title: String,
    pub tags: Vec<String>,
    /// Tags from `#+FILETAGS:` and ancestor headings, minus its own.
    pub inherited: Vec<String>,
    pub scheduled: Option<Ts>,
    pub deadline: Option<Ts>,
    pub timestamps: Vec<Ts>,
    pub id: Option<String>,
    pub category: String,
    /// `:STYLE: habit`
    pub habit: bool,
    /// Dates of `- State "DONE" from ...` log entries (any done keyword).
    pub done_log: Vec<NaiveDate>,
    /// `CLOSED: [date time]` on the planning line (midnight without a time).
    pub closed: Option<NaiveDateTime>,
    /// `:EFFORT:` in minutes.
    pub effort: Option<u32>,
}

impl Headline {
    /// In a subtree tagged `:ARCHIVE:` (archived in place), which org's agenda skips.
    pub fn archived(&self) -> bool {
        self.tags.iter().chain(&self.inherited).any(|t| t == "ARCHIVE")
    }
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
    /// `#+FILETAGS:` of the file.
    pub tags: Vec<String>,
    /// The text it was parsed from, so searches needn't re-read the file.
    pub text: String,
}

fn parse_ts(c: &regex::Captures) -> Option<Ts> {
    Some(Ts {
        date: c[1].parse().ok()?,
        time: c.get(2).map(|m| m.as_str().to_string()),
        end_time: c.get(3).map(|m| m.as_str().to_string()),
        repeater: c.get(4).map(|k| (k.as_str().to_string(), c[5].parse().unwrap_or(1), c[6].chars().next().unwrap())),
        max: c.get(7).map(|m| (m.as_str().parse().unwrap_or(1), c[8].chars().next().unwrap())),
        warn: c.get(9).map(|m| (m.as_str().parse().unwrap_or(0), c[10].chars().next().unwrap())),
        end: None,
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
    let mut file_tags: Vec<String> = vec![];
    let mut tag_stack: Vec<(usize, Vec<String>)> = vec![]; // (level, own + inherited tags)
    let mut cur: Option<Headline> = None;
    let mut in_drawer = false;
    for (i, &line) in lines.iter().enumerate() {
        for c in LINK.captures_iter(line) {
            f.links.push(Link { target: c[2].to_string(), file: &c[1] == "file", line: i, text: line.trim().to_string() });
        }
        if let Some((level, keyword, priority, title, tags)) = split_headline(line, kw) {
            if let Some(h) = cur.take() {
                cats.push((h.level, h.category.clone()));
                tag_stack.push((h.level, h.inherited.iter().chain(&h.tags).cloned().collect()));
                f.headlines.push(h);
            }
            in_drawer = false;
            while cats.last().is_some_and(|(l, _)| *l >= level) {
                cats.pop();
            }
            while tag_stack.last().is_some_and(|(l, _)| *l >= level) {
                tag_stack.pop();
            }
            let category = cats.last().map_or(file_cat.clone(), |(_, c)| c.clone());
            let mut inherited: Vec<String> = vec![];
            for t in tag_stack.last().map_or(&file_tags, |(_, t)| t) {
                if !tags.contains(t) && !inherited.contains(t) {
                    inherited.push(t.clone());
                }
            }
            cur = Some(Headline { line: i, level, keyword, priority, title, tags, inherited, category, ..Default::default() });
            continue;
        }
        let lower = line.trim_start().to_lowercase();
        let Some(h) = cur.as_mut() else {
            // File-level preamble.
            if lower.starts_with("#+title:") {
                f.title = line.trim_start()[8..].trim().to_string();
            } else if lower.starts_with("#+category:") {
                file_cat = line.trim_start()[11..].trim().to_string();
            } else if lower.starts_with("#+filetags:") {
                file_tags.extend(line.trim_start()[11..].split(':').map(str::trim).filter(|t| !t.is_empty()).map(String::from));
                f.tags = file_tags.clone();
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
            h.closed = CLOSED.captures(line).and_then(|c| {
                let t = c.get(2).map_or(Some(Default::default()), |t| chrono::NaiveTime::parse_from_str(t.as_str(), "%H:%M").ok());
                Some(c[1].parse::<NaiveDate>().ok()?.and_time(t?))
            });
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
                } else if c[1].eq_ignore_ascii_case("EFFORT") {
                    h.effort = parse_effort(&c[2]);
                }
            }
            continue;
        }
        if let Some(c) = STATE.captures(line).filter(|c| kw.is_done(&c[1])) {
            h.done_log.extend(c[2].parse::<NaiveDate>().ok());
            continue;
        }
        // `<a>--<b>` folds into one ranged timestamp.
        let mut prev = usize::MAX;
        for c in TS.captures_iter(line) {
            let m = c.get(0).unwrap();
            let Some(ts) = parse_ts(&c) else { continue };
            match h.timestamps.last_mut() {
                Some(a) if line.get(prev..m.start()) == Some("--") && a.end.is_none() && ts.date >= a.date => (a.end, a.end_time) = ((ts.date > a.date).then_some(ts.date), ts.time),
                _ => h.timestamps.push(ts),
            }
            prev = m.end();
        }
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
    pub inherited: Vec<String>,
    pub category: String,
    /// Set on today's agenda entry of a habit, and on its Tasks entry.
    pub habit: Option<Habit>,
    pub closed: Option<NaiveDateTime>,
    /// `:EFFORT:` in minutes.
    pub effort: Option<u32>,
    /// Minutes clocked on the task (timeclock::spent_on); filled in by the commands.
    pub spent: Option<u32>,
    /// A deadline's own warning period in days (`-3d` cookie), for reminders.
    #[serde(skip)]
    pub warn: Option<i64>,
}

/// Days of a deadline's `-3d`/`-1w` warning cookie.
pub fn warn_days(ts: &Ts) -> Option<i64> {
    ts.warn.map(|(n, u)| i64::from(n) * match u { 'w' => 7, 'm' => 30, 'y' => 365, _ => 1 })
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
        inherited: h.inherited.clone(),
        category: h.category.clone(),
        habit: None,
        closed: h.closed,
        effort: h.effort,
        spent: None,
        warn: None,
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
        for h in f.headlines.iter().filter(|h| !h.archived()) {
            let done = h.keyword.as_deref().is_some_and(|k| kw.is_done(k));
            if let Some(s) = &h.scheduled {
                // A `-Nd` cookie on SCHEDULED is org's delay: hide the first occurrence until N days after its date.
                let delay = warn_days(s).unwrap_or(0).max(0);
                let overdue = !done && s.date < today && today_in && today >= s.date + Duration::days(delay);
                let hb = |d| habit(h, today).filter(|_| d == today);
                let mut ds = occurrences(s, start, end);
                // The delayed first occurrence lands on date+N (unless a repeater already hits it).
                let moved = s.date + Duration::days(delay);
                if delay > 0 && (start..=end).contains(&moved) && !ds.contains(&moved) {
                    ds.push(moved);
                }
                for d in ds {
                    if !(overdue && d == today) && !(delay > 0 && d == s.date) {
                        out.push(Item { habit: hb(d), ..item(f, h, d, "scheduled", String::new(), s.span()) });
                    }
                }
                if overdue {
                    let label = format!("Scheduled {}d ago", (today - s.date).num_days());
                    out.push(Item { habit: hb(today), ..item(f, h, today, "scheduled", label, None) });
                }
            }
            if let Some(dl) = &h.deadline {
                for d in occurrences(dl, start, end) {
                    out.push(Item { warn: warn_days(dl), ..item(f, h, d, "deadline", "Due".into(), dl.span()) });
                }
                let diff = (dl.date - today).num_days();
                // Warning period: the deadline's own `-3d` cookie, else org's default 14 days.
                let warn = warn_days(dl).unwrap_or(14);
                if !done && today_in && diff < 0 {
                    out.push(item(f, h, today, "overdue", format!("Overdue {}d", -diff), None));
                } else if !done && today_in && (1..=warn).contains(&diff) {
                    out.push(item(f, h, today, "warning", format!("Due in {diff}d"), None));
                }
            }
            for ts in &h.timestamps {
                if let Some(last) = ts.end {
                    // A date range shows on each of its days as "(i/n)", with the start time on the first.
                    let n = (last - ts.date).num_days() + 1;
                    for d in ts.date.max(start).iter_days().take_while(|&d| d <= last.min(end)) {
                        let i = (d - ts.date).num_days() + 1;
                        out.push(item(f, h, d, "timestamp", format!("({i}/{n})"), ts.time.clone().filter(|_| i == 1)));
                    }
                    continue;
                }
                for d in occurrences(ts, start, end) {
                    out.push(item(f, h, d, "timestamp", String::new(), ts.span()));
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

/// (path, line) of open tasks whose SCHEDULED date (after any `-Nd` delay) is before TODAY: what the
/// agenda labels "Scheduled Nd ago". Habits and archived subtrees are left out.
pub fn overdue_scheduled(files: &[Arc<OrgFile>], kw: &Kw, today: NaiveDate) -> Vec<(PathBuf, usize)> {
    let mut out = vec![];
    for f in files {
        for h in f.headlines.iter().filter(|h| !h.archived() && !h.habit && h.keyword.as_deref().is_some_and(|k| !kw.is_done(k))) {
            if h.scheduled.as_ref().is_some_and(|s| s.date < today && today >= s.date + Duration::days(warn_days(s).unwrap_or(0).max(0))) {
                out.push((f.path.clone(), h.line));
            }
        }
    }
    out
}

/// Tasks for which KEEP holds (open ones only, unless DONE), by priority then date.
pub fn todos(files: &[Arc<OrgFile>], kw: &Kw, done: bool, keep: impl Fn(&OrgFile, &Headline) -> bool, today: NaiveDate) -> Vec<Item> {
    let mut out = vec![];
    for f in files {
        for h in f.headlines.iter().filter(|h| !h.archived()) {
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
/// Minutes of a headline: its estimate, or the time clocked on it.
type Minutes = std::rc::Rc<dyn Fn(&Headline) -> Option<u32>>;

/// Parse a task query: space-separated terms, all of which must hold. `todo:NEXT`,
/// `tag:work`, `pri:A`, `file:inbox` (file name contains), `scheduled:` / `due:` /
/// `closed:` with an optional `<`, `<=`, `>`, `>=` before a read_date date (`due:<+7d`,
/// `closed:>=-7d`) or `none` / `any`; anything else (or a `"quoted phrase"`) must appear
/// in the title. `effort:` compares the `:EFFORT:` estimate the same way (`effort:<=30`,
/// `effort:>1h`, `effort:0:45`) or takes `none` / `any`; unestimated tasks never match a
/// comparison. `spent:` compares the minutes clocked on it (SPENT) the same way, and `over:yes`
/// / `over:no` whether an estimated task ran over its estimate. Values may be quoted too: `tag:"my tag"`, `file:"my notes"`.
/// A leading `-` negates a term. The bool says whether done tasks should be searched
/// too, i.e. the query asks for a done keyword (`todo:DONE`) or a `closed:` date.
pub fn query(q: &str, today: NaiveDate, kw: &Kw, spent: impl Fn(&Headline) -> Option<u32> + 'static) -> Result<(Filter, bool), String> {
    use std::rc::Rc;
    let spent: Minutes = Rc::new(spent);
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
        let (key, val) = if w.starts_with('"') { ("", w) } else { w.split_once(':').unwrap_or(("", w)) };
        let val = val.trim_matches('"');
        let v = val.to_lowercase();
        let t: Filter = match key.to_lowercase().as_str() {
            "todo" => {
                done |= !neg && kw.done.iter().any(|k| k.to_lowercase() == v);
                Box::new(move |_, h| h.keyword.as_ref().is_some_and(|k| k.to_lowercase() == v))
            }
            "tag" => Box::new(move |_, h| h.tags.iter().chain(&h.inherited).any(|t| t.to_lowercase() == v)),
            "pri" => Box::new(move |_, h| h.priority.as_ref().is_some_and(|p| p.to_lowercase() == v)),
            "file" => Box::new(move |f, _| f.path.file_stem().is_some_and(|s| s.to_string_lossy().to_lowercase().contains(&v))),
            k @ ("scheduled" | "due" | "closed") => {
                let get: fn(&Headline) -> Option<NaiveDate> = match k {
                    "due" => |h: &Headline| h.deadline.as_ref().map(|t| t.date),
                    "closed" => |h: &Headline| h.closed.map(|c| c.date()),
                    _ => |h: &Headline| h.scheduled.as_ref().map(|t| t.date),
                };
                done |= k == "closed" && !neg && v != "none";
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
            k @ ("effort" | "spent") => {
                let get: Minutes = if k == "effort" { Rc::new(|h: &Headline| h.effort) } else { spent.clone() };
                match v.as_str() {
                    "none" => Box::new(move |_, h| get(h).is_none()),
                    "any" => Box::new(move |_, h| get(h).is_some()),
                    _ => {
                        let (op, rest) = ["<=", ">=", "<", ">"].iter().find_map(|o| v.strip_prefix(o).map(|r| (*o, r))).unwrap_or(("=", &v));
                        let m = parse_effort(rest).ok_or_else(|| format!("can't read duration in \"{word}\""))?;
                        Box::new(move |_, h| {
                            let Some(x) = get(h) else { return false };
                            match op { "<=" => x <= m, ">=" => x >= m, "<" => x < m, ">" => x > m, _ => x == m }
                        })
                    }
                }
            }
            "over" => {
                let want = match v.as_str() { "yes" => true, "no" => false, _ => return Err(format!("over: takes yes or no, not \"{val}\"")) };
                let s = spent.clone();
                Box::new(move |_, h| h.effort.is_some_and(|e| s(h).unwrap_or(0) > e) == want)
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

pub fn ts_string(d: NaiveDate, time: Option<&str>, repeater: Option<&str>, warn: Option<&str>) -> String {
    let mut s = format!("<{}", d.format("%Y-%m-%d %a"));
    for p in [time, repeater, warn].into_iter().flatten() {
        s += &format!(" {p}");
    }
    s + ">"
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

/// Line of the next headline after HL (or the end): the limit of its own text.
fn heading_end(lines: &[String], hl: usize) -> usize {
    (hl + 1..lines.len()).find(|&i| heading_level(&lines[i]).is_some()).unwrap_or(lines.len())
}

/// Line of the `:END:` closing the drawer opened at START, if it is closed
/// before the next headline.
fn drawer_end(lines: &[String], start: usize) -> Option<usize> {
    let lim = (start + 1..lines.len()).find(|&i| heading_level(&lines[i]).is_some()).unwrap_or(lines.len());
    (start + 1..lim).find(|&j| lines[j].trim().eq_ignore_ascii_case(":END:"))
}

/// Set the TODO keyword (None clears it). Marking a repeating task done
/// shifts its dates, sets `:LAST_REPEAT:`, logs `- State "DONE" from "TODO"
/// [date time]` below the planning line and drawers (inside a LOGBOOK if
/// there is one) and resets it, like org. Otherwise, with LOG, finishing a
/// task stamps `CLOSED: [date time]` first on its planning line; reopening it
/// (to an open keyword or none) always removes that stamp.
pub fn set_keyword(text: &str, line: usize, kw: &Kw, new: Option<&str>, now: NaiveDateTime, log: bool) -> String {
    let today = now.date();
    let mut lines = to_lines(text);
    let Some(hl) = headline_at(&lines, line) else { return text.to_string() };
    let Some(mut p) = Parts::parse(&lines[hl], kw) else { return text.to_string() };
    let mut next = new.map(String::from);
    let planning = hl + 1 < lines.len() && is_planning(&lines[hl + 1]);
    let repeats = planning && TS.captures_iter(&lines[hl + 1]).any(|c| c.get(4).is_some());
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
            let end = drawer_end(&lines, hl + 2).unwrap_or_else(|| heading_end(&lines, hl));
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
            i = drawer_end(&lines, i).map_or(i, |j| j + 1);
        }
        if lines.get(i).is_some_and(|l| l.trim().eq_ignore_ascii_case(":LOGBOOK:")) {
            i += 1;
        }
        lines.insert(i, log);
        next = p.keyword.clone().filter(|k| !kw.is_done(k)).or(kw.todo.first().cloned());
    } else {
        let done = |k: Option<&str>| k.is_some_and(|k| kw.is_done(k));
        let is = done(next.as_deref());
        if (log && is && !done(p.keyword.as_deref())) || (!is && planning && CLOSED.is_match(&lines[hl + 1])) {
            let old = if planning { lines[hl + 1].clone() } else { String::new() };
            let indent = &old[..old.len() - old.trim_start().len()];
            let rest = CLOSED.replace(old.trim(), "").trim().to_string();
            let new = if !is {
                format!("{indent}{rest}")
            } else {
                format!("{indent}CLOSED: [{}]{}{rest}", now.format("%Y-%m-%d %a %H:%M"), if rest.is_empty() { "" } else { " " })
            };
            if !planning {
                lines.insert(hl + 1, new);
            } else if rest.is_empty() && !is {
                lines.remove(hl + 1);
            } else {
                lines[hl + 1] = new;
            }
        }
    }
    p.keyword = next;
    lines[hl] = p.line();
    join(lines, text)
}

/// Cycle the TODO keyword of the headline at LINE by DIR (+1/-1).
pub fn cycle_todo(text: &str, line: usize, kw: &Kw, dir: i32, now: NaiveDateTime, log: bool) -> String {
    let Some(p) = heading(text, line, kw) else { return text.to_string() };
    let seq: Vec<Option<&str>> = std::iter::once(None).chain(kw.todo.iter().chain(&kw.done).map(|s| Some(s.as_str()))).collect();
    let i = seq.iter().position(|k| *k == p.keyword.as_deref()).unwrap_or(0) as i32;
    set_keyword(text, line, kw, seq[(i + dir).rem_euclid(seq.len() as i32) as usize], now, log)
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

/// Replace the title (keyword, priority, tags kept); a blank title is ignored.
pub fn set_title(text: &str, line: usize, kw: &Kw, title: &str) -> String {
    edit_heading(text, line, kw, |p| if !title.trim().is_empty() { p.title = title.trim().to_string() })
}

pub fn set_tags(text: &str, line: usize, kw: &Kw, tags: Vec<String>) -> String {
    edit_heading(text, line, kw, |p| p.tags = tags.into_iter().filter(|t| !t.is_empty()).collect())
}

/// Set (Some minutes, written `H:MM` like org) or remove the `:EFFORT:` of the
/// heading at LINE. Removing it drops the drawer only when nothing else is left in it.
pub fn set_effort(text: &str, line: usize, minutes: Option<u32>) -> String {
    let mut lines = to_lines(text);
    let Some(hl) = headline_at(&lines, line) else { return text.to_string() };
    let i = if lines.get(hl + 1).is_some_and(|l| is_planning(l)) { hl + 2 } else { hl + 1 };
    let entry = minutes.map(|m| format!(":EFFORT: {}:{:02}", m / 60, m % 60));
    if !lines.get(i).is_some_and(|l| l.trim().eq_ignore_ascii_case(":PROPERTIES:")) {
        if let Some(e) = entry {
            lines.splice(i..i, [":PROPERTIES:".to_string(), e, ":END:".to_string()]);
        }
        return join(lines, text);
    }
    // An unclosed drawer is left alone.
    let Some(end) = drawer_end(&lines, i) else { return text.to_string() };
    let old = (i + 1..end).find(|&j| PROP.captures(&lines[j]).is_some_and(|c| c[1].eq_ignore_ascii_case("EFFORT")));
    match (old, entry) {
        (Some(j), Some(e)) => lines[j] = e,
        (None, Some(e)) => lines.insert(end, e),
        (Some(_), None) if end == i + 2 => { lines.drain(i..=end); },
        (Some(j), None) => { lines.remove(j); },
        (None, None) => {}
    }
    join(lines, text)
}

/// Add `- Note taken on [date time] \\` plus NOTE (indented) under the
/// headline at LINE, below its planning line and drawers, newest first like
/// org with `org-log-into-drawer` nil.
pub fn add_note(text: &str, line: usize, note: &str, now: NaiveDateTime) -> String {
    let mut lines = to_lines(text);
    let Some(hl) = headline_at(&lines, line) else { return text.to_string() };
    let mut i = hl + 1;
    if lines.get(i).is_some_and(|l| is_planning(l)) {
        i += 1;
    }
    while lines.get(i).is_some_and(|l| [":PROPERTIES:", ":LOGBOOK:"].iter().any(|d| l.trim().eq_ignore_ascii_case(d))) {
        // An unclosed drawer: insert before it rather than running on into later headings.
        let Some(j) = drawer_end(&lines, i) else { break };
        i = j + 1;
    }
    let head = format!("- Note taken on [{}] \\\\", now.format("%Y-%m-%d %a %H:%M"));
    let body = note.trim().lines().map(|l| if l.trim().is_empty() { String::new() } else { format!("  {}", l.trim_end()) });
    lines.splice(i..i, std::iter::once(head).chain(body));
    join(lines, text)
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

/// Cut the subtrees at LINES (none nested) out of S, returned in source order.
fn take_many(s: &mut Vec<String>, lines: &[usize]) -> Result<Vec<Vec<String>>, String> {
    let mut ls = lines.to_vec();
    ls.sort_unstable_by(|a, b| b.cmp(a));
    ls.dedup();
    let mut subs = ls.iter().map(|&l| take_subtree(s, l)).collect::<Result<Vec<_>, _>>()?;
    subs.reverse();
    Ok(subs)
}

/// `refile` for several subtrees of SRC (not nested), kept in source order under DST_LINE.
pub fn refile_many(src: &str, lines: &[usize], dst: &str, dst_line: Option<usize>) -> Result<(String, String), String> {
    let mut s = to_lines(src);
    let subs = take_many(&mut s, lines)?;
    let mut d = to_lines(dst);
    check_target(&d, dst_line)?;
    for sub in subs {
        insert_subtree(&mut d, dst_line, sub);
    }
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

/// Drop every `:ID:` property in SUB (so the copy doesn't duplicate it), and a
/// `:PROPERTIES:` drawer that this leaves empty.
fn strip_ids(sub: Vec<String>) -> Vec<String> {
    let is_id = |l: &str| l.trim_start().get(..4).is_some_and(|p| p.eq_ignore_ascii_case(":ID:"));
    let mut out = Vec::with_capacity(sub.len());
    let mut i = 0;
    while i < sub.len() {
        if sub[i].trim().eq_ignore_ascii_case(":PROPERTIES:") {
            if let Some(end) = drawer_end(&sub, i) {
                let kept: Vec<&String> = sub[i + 1..end].iter().filter(|l| !is_id(l)).collect();
                if !kept.is_empty() || end == i + 1 {
                    out.push(sub[i].clone());
                    out.extend(kept.into_iter().cloned());
                    out.push(sub[end].clone());
                }
                i = end + 1;
                continue;
            }
        }
        out.push(sub[i].clone());
        i += 1;
    }
    out
}

/// TEXT as the body of a duplicate note: without the top-of-file properties
/// block and `#+title:` (the new note writes its own), and without heading `:ID:`s.
pub fn duplicate_body(text: &str) -> String {
    let mut l = to_lines(text);
    let first = l.iter().position(|x| !x.trim().is_empty()).unwrap_or(l.len());
    if l.get(first).is_some_and(|x| x.trim().eq_ignore_ascii_case(":PROPERTIES:")) {
        if let Some(end) = drawer_end(&l, first) {
            l.drain(..=end);
        }
    }
    let head = l.iter().position(|x| heading_level(x).is_some()).unwrap_or(l.len());
    let is_title = |x: &String| x.trim_start().get(..8).is_some_and(|p| p.eq_ignore_ascii_case("#+title:"));
    let mut n = 0;
    l.retain(|x| {
        n += 1;
        n > head || !is_title(x)
    });
    join(strip_ids(l), text)
}

/// Copy of the subtree at LINE, without its `:ID:`s.
fn copy_subtree(lines: &[String], line: usize) -> Result<Vec<String>, String> {
    let hl = headline_at(lines, line).ok_or("not on a heading")?;
    let (s, e) = subtree_range(lines, hl);
    Ok(strip_ids(lines[s..e].to_vec()))
}

/// Copy the subtree at LINE of SRC under heading DST_LINE of DST (None: top
/// level), like org-refile-copy. Returns the new DST; SRC is unchanged.
pub fn refile_copy(src: &str, line: usize, dst: &str, dst_line: Option<usize>) -> Result<String, String> {
    let sub = copy_subtree(&to_lines(src), line)?;
    let mut d = to_lines(dst);
    check_target(&d, dst_line)?;
    insert_subtree(&mut d, dst_line, sub);
    Ok(join_nl(d))
}

/// Copy within one file; copying under the subtree itself is fine.
pub fn refile_copy_same(text: &str, line: usize, dst_line: Option<usize>) -> Result<String, String> {
    let mut l = to_lines(text);
    check_target(&l, dst_line)?;
    let sub = copy_subtree(&l, line)?;
    insert_subtree(&mut l, dst_line, sub);
    Ok(join(l, text))
}

static STEP: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\+?(\d{1,4})([dwmy])$").unwrap());

/// Insert N copies of the subtree at LINE right after it, copy k with every
/// active timestamp moved k×STEP (`+1w`, `2d`, `1m`, `1y`), like
/// org-clone-subtree-with-time-shift. Copies lose `:ID:`s, `CLOSED:` stamps,
/// `:LOGBOOK:` drawers and `CLOCK:` lines (so clocked time isn't counted
/// again), and done keywords go back to the first todo keyword.
pub fn clone_shift(text: &str, line: usize, n: u32, step: &str, kw: &Kw) -> Result<String, String> {
    let c = STEP.captures(step.trim()).ok_or_else(|| format!("can't read shift \"{step}\" (try +1w, +2d, +1m)"))?;
    let (by, unit): (u32, char) = (c[1].parse().unwrap(), c[2].chars().next().unwrap());
    if by == 0 || !(1..=52).contains(&n) {
        return Err("need 1–52 copies and a shift of at least 1".into());
    }
    let mut l = to_lines(text);
    let sub = copy_subtree(&l, line)?;
    let end = subtree_range(&l, headline_at(&l, line).unwrap()).1;
    let (mut base, mut i) = (vec![], 0);
    while i < sub.len() {
        let s = &sub[i];
        if s.trim().eq_ignore_ascii_case(":LOGBOOK:") {
            if let Some(e) = drawer_end(&sub, i) {
                i = e + 1;
                continue;
            }
        }
        i += 1;
        if s.trim_start().starts_with("CLOCK:") {
            continue;
        }
        if let Some(lvl) = heading_level(s) {
            let t = s[lvl..].trim_start();
            let w = t.split(' ').next().unwrap_or("");
            match kw.todo.first().filter(|_| kw.is_done(w)) {
                Some(todo) => base.push(format!("{} {todo}{}", &s[..lvl], &t[w.len()..])),
                None => base.push(s.clone()),
            }
        } else if is_planning(s) && CLOSED.is_match(s) {
            let s = CLOSED.replace(s, "");
            if !s.trim().is_empty() {
                base.push(s.trim_end().to_string());
            }
        } else {
            base.push(s.clone());
        }
    }
    let shifted = |s: &str, k: u32| {
        TS.replace_all(s, |c: &regex::Captures| {
            let m = &c[0];
            match NaiveDate::parse_from_str(&c[1], "%Y-%m-%d") {
                Ok(d) => TS_HEAD.replace(m, format!("<{}", add_interval(d, k * by, unit).format("%Y-%m-%d %a"))).into_owned(),
                Err(_) => m.to_string(),
            }
        })
        .into_owned()
    };
    let mut copies = vec![];
    for k in 1..=n {
        copies.extend(base.iter().map(|s| shifted(s.as_str(), k)));
    }
    l.splice(end..end, copies);
    Ok(join(l, text))
}

static COOKIE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s*\[\d*(?:/\d*|%)\]\s*").unwrap());

/// Extract the subtree at LINE into a new note (org-roam-extract-subtree).
/// MAKE gets the title (no keyword, priority, tags or `[n/m]` cookie) and the
/// body (the heading's own text and its children, relevelled so they start at
/// level 1) and returns the new note's ID. The source keeps the heading, its
/// planning line and drawers (so an `:ID:` still resolves), then a link.
pub fn extract_subtree(text: &str, line: usize, kw: &Kw, make: impl FnOnce(&str, &str) -> Result<String, String>) -> Result<String, String> {
    let mut l = to_lines(text);
    let hl = headline_at(&l, line).ok_or("not on a heading")?;
    let (_, _, _, title, _) = split_headline(&l[hl], kw).ok_or("not on a heading")?;
    let title = COOKIE.replace_all(&title, " ").trim().to_string();
    if title.is_empty() {
        return Err("the heading has no title".into());
    }
    let (_, mut e) = subtree_range(&l, hl);
    while e > hl + 1 && l[e - 1].trim().is_empty() {
        e -= 1; // trailing blank lines stay in the source
    }
    let mut s = hl + 1;
    if l.get(s).is_some_and(|x| is_planning(x)) {
        s += 1;
    }
    while s < e && l[s].trim().starts_with(':') && l[s].trim().ends_with(':') && l[s].trim().len() > 1 {
        let Some(j) = drawer_end(&l, s) else { break };
        s = j + 1;
    }
    let s = s.min(e);
    let mut body: Vec<String> = l[s..e].to_vec();
    relevel(&mut body, 1);
    while body.first().is_some_and(|x| x.trim().is_empty()) {
        body.remove(0);
    }
    let id = make(&title, &body.join("\n"))?;
    l.splice(s..e, [format!("[[id:{id}][{}]]", title.replace(']', ")"))]);
    Ok(join(l, text))
}

/// Sort the children of the heading at or above LINE (top-level headings for
/// None or a line above the first heading) by KEY: title | priority | status
/// | scheduled | deadline. Stable; headings without the key go last either
/// way. Blank lines between siblings stay where they were.
pub fn sort_children(text: &str, line: Option<usize>, kw: &Kw, key: &str, desc: bool) -> Result<String, String> {
    let mut l = to_lines(text);
    let (s, e) = match line.and_then(|n| headline_at(&l, n)) {
        Some(hl) => (hl + 1, subtree_range(&l, hl).1),
        None => (0, l.len()),
    };
    let heads: Vec<usize> = (s..e).filter(|&i| heading_level(&l[i]).is_some()).collect();
    let Some(lvl) = heads.iter().filter_map(|&i| heading_level(&l[i])).min() else { return Err("No headings to sort here.".into()) };
    let starts: Vec<usize> = heads.into_iter().filter(|&i| heading_level(&l[i]).unwrap() <= lvl).collect();
    if starts.len() < 2 {
        return Err("Only one heading here; nothing to sort.".into());
    }
    let kws: Vec<&String> = kw.todo.iter().chain(&kw.done).collect();
    let key_of = |sub: &[String]| -> Option<String> {
        let (_, k, p, title, _) = split_headline(&sub[0], kw)?;
        match key {
            "title" => Some(title.to_lowercase()),
            "priority" => p,
            "status" => k.and_then(|k| kws.iter().position(|w| **w == k)).map(|i| format!("{i:04}")),
            "scheduled" | "deadline" => {
                let p = sub.get(1).filter(|p| is_planning(p))?;
                let ts = TS.captures(&p[p.find(&format!("{}:", key.to_uppercase()))?..]).and_then(|c| parse_ts(&c))?;
                Some(format!("{} {}", ts.date, ts.time.unwrap_or_default()))
            }
            _ => None,
        }
    };
    if !["title", "priority", "status", "scheduled", "deadline"].contains(&key) {
        return Err(format!("unknown sort key {key}"));
    }
    let tail: Vec<String> = l.drain(starts[0]..e).collect();
    let mut subs: Vec<Vec<String>> = vec![];
    let mut blanks = vec![];
    for (j, &st) in starts.iter().enumerate() {
        let mut sub = tail[st - starts[0]..starts.get(j + 1).unwrap_or(&e) - starts[0]].to_vec();
        let n = sub.iter().rev().take_while(|x| x.trim().is_empty()).count();
        blanks.push(sub.split_off(sub.len() - n));
        subs.push(sub);
    }
    let mut keyed: Vec<(Option<String>, Vec<String>)> = subs.into_iter().map(|s| (key_of(&s), s)).collect();
    keyed.sort_by(|(a, _), (b, _)| match (a, b) {
        (Some(a), Some(b)) if desc => b.cmp(a),
        (Some(a), Some(b)) => a.cmp(b),
        (a, b) => b.is_some().cmp(&a.is_some()),
    });
    let sorted = keyed.into_iter().zip(blanks).flat_map(|((_, s), b)| s.into_iter().chain(b));
    l.splice(starts[0]..starts[0], sorted);
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

/// TEXT with an `:ID:` on the heading at or above LINE (reusing one in its drawer),
/// that ID, and the heading's plain title. Before the first heading the file-level ID.
pub fn ensure_heading_id(text: &str, line: usize, kw: &Kw) -> (String, String, String) {
    let mut lines = to_lines(text);
    let Some(hl) = headline_at(&lines, line) else {
        let (t, id) = crate::notes::ensure_id(text);
        let title = text.lines().find_map(|l| l.strip_prefix("#+title:")).unwrap_or("").trim().to_string();
        return (t, id, title);
    };
    let title = Parts::parse(&lines[hl], kw).map(|p| p.title).unwrap_or_default();
    let end = (hl + 1..lines.len()).find(|&i| heading_level(&lines[i]).is_some()).unwrap_or(lines.len());
    let i = if end > hl + 1 && is_planning(&lines[hl + 1]) { hl + 2 } else { hl + 1 };
    if lines.get(i).is_some_and(|l| l.trim().eq_ignore_ascii_case(":PROPERTIES:")) {
        for l in lines[i + 1..end].iter().take_while(|l| !l.trim().eq_ignore_ascii_case(":END:")) {
            if let Some(id) = l.trim().strip_prefix(":ID:").filter(|v| !v.trim().is_empty()) {
                return (text.to_string(), id.trim().to_string(), title);
            }
        }
    }
    let id = crate::notes::gen_id();
    let mut sub: Vec<String> = lines[hl..end].to_vec();
    set_property(&mut sub, "ID", &id);
    lines.splice(hl..end, sub);
    (join(lines, text), id, title)
}

/// Move the subtree at LINE of SRC to the end of the archive file text DST, org style.
pub fn archive(src: &str, line: usize, dst: &str, src_path: &str, now: chrono::NaiveDateTime) -> Result<(String, String), String> {
    let mut s = to_lines(src);
    let sub = take_subtree(&mut s, line)?;
    Ok((join(s, src), append_archived(dst, src_path, now, vec![sub])))
}

/// `archive` for several subtrees of SRC (not nested), appended in source order.
pub fn archive_many(src: &str, lines: &[usize], dst: &str, src_path: &str, now: chrono::NaiveDateTime) -> Result<(String, String), String> {
    let mut s = to_lines(src);
    let subs = take_many(&mut s, lines)?;
    Ok((join(s, src), append_archived(dst, src_path, now, subs)))
}

/// Archive every top-most done subtree of SRC (not repeaters or `:ARCHIVE:` subtrees),
/// in source order; also returns how many. Nothing done: both texts unchanged.
pub fn archive_done(src: &str, dst: &str, src_path: &str, now: chrono::NaiveDateTime, kw: &Kw) -> (String, String, usize) {
    let f = parse(Path::new(src_path), src, kw);
    let (mut picked, mut under): (Vec<usize>, Option<usize>) = (vec![], None);
    for h in &f.headlines {
        if under.is_some_and(|l| h.level > l) {
            continue;
        }
        under = None;
        let rep = [&h.scheduled, &h.deadline].into_iter().flatten().any(|t| t.repeater.is_some());
        if h.keyword.as_deref().is_some_and(|k| kw.is_done(k)) && !h.archived() && !rep {
            picked.push(h.line);
            under = Some(h.level);
        }
    }
    if picked.is_empty() {
        return (src.into(), dst.into(), 0);
    }
    let mut s = to_lines(src);
    // Cut bottom-up so earlier line numbers stay valid, then append in source order.
    let mut subs: Vec<Vec<String>> = picked.iter().rev().map(|&l| {
        let (a, b) = subtree_range(&s, l);
        s.drain(a..b).collect()
    }).collect();
    subs.reverse();
    (join(s, src), append_archived(dst, src_path, now, subs), picked.len())
}

/// DST (an archive file's text; org's header when empty) with SUBS appended at top level.
fn append_archived(dst: &str, src_path: &str, now: chrono::NaiveDateTime, subs: Vec<Vec<String>>) -> String {
    let mut d = to_lines(dst);
    if d.is_empty() {
        d = vec!["#    -*- mode: org -*-".into(), String::new(), String::new(), format!("Archived entries from file {src_path}"), String::new()];
    }
    for mut sub in subs {
        set_property(&mut sub, "ARCHIVE_TIME", &now.format("%Y-%m-%d %a %H:%M").to_string());
        set_property(&mut sub, "ARCHIVE_FILE", src_path);
        insert_subtree(&mut d, None, sub);
    }
    join_nl(d)
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
            (Some('t'), _) => (ts_string(d, None, None, None), 1),
            (Some('T'), _) => (ts_string(d, Some(&t), None, None), 1),
            (Some('u'), _) => (inactive(ts_string(d, None, None, None)), 1),
            (Some('U'), _) => (inactive(ts_string(d, Some(&t), None, None)), 1),
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
pub fn task_entry(title: &str, keyword: Option<&str>, priority: Option<&str>, tags: Vec<String>, scheduled: Option<When>, deadline: Option<When>) -> String {
    let p = Parts { level: 1, keyword: keyword.map(String::from), priority: priority.map(String::from), title: title.trim().into(), tags };
    let planning: Vec<String> = [("SCHEDULED", scheduled), ("DEADLINE", deadline)]
        .into_iter()
        .filter_map(|(k, d)| d.map(|(d, t, r, w)| format!("{k}: {}", ts_string(d, t.as_deref(), r.as_deref(), w.as_deref()))))
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
    /// Bare headline title (the note's title for a file entry).
    pub title: String,
}

/// Refile/capture targets: every file (top level) and every heading, labelled by outline path.
pub fn targets(files: &[Arc<OrgFile>], root: &Path) -> Vec<Target> {
    let mut out = vec![];
    for f in files {
        let rel = f.path.strip_prefix(root).unwrap_or(&f.path).display().to_string();
        out.push(Target { path: f.path.clone(), line: None, label: rel.clone(), title: f.title.clone() });
        let mut stack: Vec<(usize, &str)> = vec![];
        for h in &f.headlines {
            while stack.last().is_some_and(|(l, _)| *l >= h.level) {
                stack.pop();
            }
            stack.push((h.level, &h.title));
            let olp: Vec<&str> = stack.iter().map(|(_, t)| *t).collect();
            out.push(Target { path: f.path.clone(), line: Some(h.line), label: format!("{rel} / {}", olp.join(" / ")), title: h.title.clone() });
        }
    }
    out
}

pub fn all_tags(files: &[Arc<OrgFile>]) -> Vec<String> {
    let mut t: Vec<String> = files.iter().flat_map(|f| f.headlines.iter().flat_map(|h| h.tags.iter().chain(&h.inherited).cloned())).collect();
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

/// Sentinel repeater for `set_planning`: write no repeater, drop the old one.
pub const NO_REPEAT: &str = "norepeat";

/// The KIND (SCHEDULED/DEADLINE) timestamp of the headline at LINE, if any.
pub fn planning_ts(text: &str, line: usize, kind: &str) -> Option<Ts> {
    let lines: Vec<String> = text.lines().map(String::from).collect();
    let hl = headline_at(&lines, line)?;
    let p = lines.get(hl + 1).filter(|p| is_planning(p))?;
    TS.captures(&p[p.find(&format!("{kind}:"))?..]).and_then(|c| parse_ts(&c))
}

/// Set (or with DATE None, remove) SCHEDULED/DEADLINE on the headline at LINE.
pub fn set_planning(text: &str, line: usize, kind: &str, date: Option<When>) -> String {
    let mut lines: Vec<String> = text.lines().map(String::from).collect();
    let Some(hl) = headline_at(&lines, line) else { return text.to_string() };
    let re = Regex::new(&format!(r"\s*{kind}:\s*<[^>]*>")).unwrap();
    // A new date keeps the old timestamp's repeater and warning cookie unless it brings its own.
    let old = planning_ts(text, line, kind);
    let new = date.map(|(d, t, r, w)| {
        let stop = r.as_deref() == Some(NO_REPEAT);
        let r = r.filter(|_| !stop);
        let mut s = ts_string(d, t.as_deref(), r.as_deref(), None);
        s.pop();
        if let Some(Ts { repeater, max, .. }) = old.as_ref().filter(|_| !stop) {
            if let (None, Some((k, n, u))) = (&r, repeater) {
                s += &format!(" {k}{n}{u}");
                if let Some((n, u)) = max {
                    s += &format!("/{n}{u}");
                }
            }
        }
        match (w, old.as_ref().and_then(|o| o.warn)) {
            (Some(w), _) => s += &format!(" {w}"),
            (None, Some((n, u))) => s += &format!(" -{n}{u}"),
            _ => {}
        }
        format!("{kind}: {s}>")
    });
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

/// Move KIND on the headline at LINE to DATE, keeping its time, repeater and warning; no KIND: unchanged.
pub fn move_planning(text: &str, line: usize, kind: &str, date: NaiveDate) -> String {
    match planning_ts(text, line, kind) {
        Some(o) => set_planning(text, line, kind, Some((date, o.span(), None, None))),
        None => text.to_string(),
    }
}

/// Parse org-read-date style input: "", "today", "tomorrow", "+3d", "-1w",
/// "mon".."sun", "2026-10-02", "10-02", "15" (day of month), each optionally
/// followed by a time "14:00" or range "9:00-10:30", a repeater "+1w",
/// ".+2d/3d" and/or a warning cookie "-3d"; anything else is rejected.
pub fn read_date(input: &str, today: NaiveDate) -> Option<When> {
    let mut parts = input.split_whitespace();
    let first = parts.next().unwrap_or("").to_lowercase();
    let mut first = first.as_str();
    let (mut time, mut rep, mut warn) = (None, None, None);
    if first.contains(':') {
        time = Some(norm_time(first)?);
        first = "";
    }
    for p in parts {
        match norm_time(p) {
            Some(t) if time.is_none() => time = Some(t),
            _ if rep.is_none() && REPEATER.is_match(p) => rep = Some(p.to_string()),
            _ if warn.is_none() && WARN.is_match(p) => warn = Some(p.to_string()),
            _ => return None,
        }
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
    Some((date, time, rep, warn))
}

/// `read_date` for past days: a bare weekday name is the latest such day before today
/// (a name equal to today's weekday is 7 days back), `yesterday` works, and a future date is rejected.
pub fn read_past_date(input: &str, today: NaiveDate) -> Option<When> {
    let first = input.split_whitespace().next().unwrap_or("").to_lowercase();
    let input = if first == "yesterday" { input.to_lowercase().replacen("yesterday", "-1d", 1) } else { input.to_string() };
    let mut r = read_date(&input, today)?;
    if first.starts_with(|c: char| c.is_ascii_alphabetic()) && first != "today" && first != "tomorrow" && first != "yesterday" {
        r.0 -= Duration::days(7); // read_date walked forward to today+1..today+7
        if r.0 >= today {
            r.0 -= Duration::days(7);
        }
    }
    (r.0 <= today).then_some(r)
}

/// A date input as `read_date` reads it: date, time, repeater, warning cookie.
pub type When = (NaiveDate, Option<String>, Option<String>, Option<String>);

/// "9:30" -> "09:30", "9:00-10:30" -> "09:00-10:30"; None unless every part is a valid clock time.
fn norm_time(t: &str) -> Option<String> {
    let one = |s: &str| {
        let (h, m) = s.split_once(':')?;
        let ok = |p: &str, max: u32| (1..=2).contains(&p.len()) && p.bytes().all(|b| b.is_ascii_digit()) && p.parse::<u32>().is_ok_and(|n| n <= max);
        (ok(h, 23) && m.len() == 2 && ok(m, 59)).then(|| format!("{:02}:{m}", h.parse::<u32>().unwrap()))
    };
    match t.split_once('-') {
        Some((a, b)) => Some(format!("{}-{}", one(a)?, one(b)?)),
        None => one(t),
    }
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
    fn duplicate_body_drops_ids_and_title() {
        let b = duplicate_body(DOC);
        assert!(!b.contains(":ID:") && !b.to_lowercase().contains("#+title") && !b.contains("file-1"));
        assert!(b.starts_with("#+category: Work\n* TODO [#A] Write report"), "{b}");
        assert!(b.contains("[[id:other][Other]]") && b.contains(":CATEGORY: Sub"));
        let b = duplicate_body(":PROPERTIES:\n:ID: x\n:ROAM_ALIASES: a\n:END:\n#+TITLE: T\n#+filetags: :a:\ntext\n");
        assert_eq!(b, "#+filetags: :a:\ntext\n");
    }

    #[test]
    fn effort_parses_org_durations() {
        for (s, m) in [("1:30", Some(90)), ("0:30", Some(30)), ("2h", Some(120)), ("45min", Some(45)), ("45m", Some(45)), ("1h 30min", Some(90)), ("20", Some(20)), (" 2H ", Some(120))] {
            assert_eq!(parse_effort(s), m, "{s}");
        }
        for s in ["", "soon", "1:3", "h", "1.5h"] {
            assert_eq!(parse_effort(s), None, "{s}");
        }
        let f = parse(Path::new("/n/x.org"), "* TODO A\n:PROPERTIES:\n:Effort: 0:30\n:END:\n* TODO B\n:PROPERTIES:\n:EFFORT: later\n:END:\n", &kw());
        assert_eq!(f.headlines.iter().map(|h| h.effort).collect::<Vec<_>>(), [Some(30), None]);
    }

    #[test]
    fn set_effort_edits_the_drawer() {
        let sched = "* TODO A\nSCHEDULED: <2026-10-02 Fri>\nbody\n";
        let set = set_effort(sched, 0, Some(90));
        assert_eq!(set, "* TODO A\nSCHEDULED: <2026-10-02 Fri>\n:PROPERTIES:\n:EFFORT: 1:30\n:END:\nbody\n");
        assert_eq!(parse(Path::new("/x.org"), &set, &kw()).headlines[0].effort, Some(90)); // round-trip
        assert_eq!(set_effort(&set, 2, Some(30)), set.replace("1:30", "0:30"));
        // Removing the last property drops the drawer.
        assert_eq!(set_effort(&set, 0, None), sched);
        // An existing drawer: added at its end; removal keeps :ID:.
        let id = "* A\n:PROPERTIES:\n:ID: x\n:END:\n* B\n";
        let with = set_effort(id, 0, Some(240));
        assert_eq!(with, "* A\n:PROPERTIES:\n:ID: x\n:EFFORT: 4:00\n:END:\n* B\n");
        assert_eq!(set_effort(&with, 0, None), id);
        assert_eq!(set_effort("* A\n", 0, None), "* A\n");
        // An empty drawer stays; an unclosed one is left alone.
        assert_eq!(set_effort("* A\n:PROPERTIES:\n:END:\n", 0, None), "* A\n:PROPERTIES:\n:END:\n");
        assert_eq!(set_effort("* A\n:PROPERTIES:\n* B\n", 0, Some(15)), "* A\n:PROPERTIES:\n* B\n");
    }

    #[test]
    fn targets_carry_bare_titles() {
        let f = Arc::new(parse(Path::new("/n/x.org"), DOC, &kw()));
        let t: Vec<_> = targets(&[f], Path::new("/n")).into_iter().map(|t| (t.line, t.label, t.title)).collect();
        assert_eq!(t[0], (None, "x.org".into(), "My Note".into()));
        assert_eq!(t[2], (Some(11), "x.org / Write report / Sub".into(), "Sub".into()));
    }

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
    fn scheduled_delay_cookie() {
        let sched = |ts: &str, today: &str| {
            let f = Arc::new(parse(Path::new("/n/x.org"), &format!("* TODO A\nSCHEDULED: <{ts}>\n"), &kw()));
            let t = d(today);
            agenda(&[f], &kw(), t, 1, t).iter().filter(|i| i.kind == "scheduled").map(|i| i.label.clone()).collect::<Vec<_>>()
        };
        assert_eq!(sched("2026-10-02 Fri -3d", "2026-10-02"), Vec::<String>::new()); // on its date: hidden
        assert_eq!(sched("2026-10-02 Fri -3d", "2026-10-04"), Vec::<String>::new());
        assert_eq!(sched("2026-10-02 Fri -3d", "2026-10-05"), vec!["Scheduled 3d ago"]);
        assert_eq!(sched("2026-10-02 Fri", "2026-10-02"), vec![""]); // no cookie
        assert_eq!(sched("2026-10-02 Fri -0d", "2026-10-02"), vec![""]);
        // Only the first occurrence of a repeater is delayed.
        assert_eq!(sched("2026-10-02 Fri +1w -3d", "2026-10-09"), vec!["Scheduled 7d ago"]);
    }

    #[test]
    fn scheduled_delay_future_range() {
        let days = |ts: &str, start: &str, n: i64, today: &str| {
            let f = Arc::new(parse(Path::new("/n/x.org"), &format!("* TODO A\nSCHEDULED: <{ts}>\n"), &kw()));
            agenda(&[f], &kw(), d(start), n, d(today)).iter().filter(|i| i.kind == "scheduled").map(|i| (i.date.to_string(), i.label.clone())).collect::<Vec<_>>()
        };
        let p = |a: &str, l: &str| (a.to_string(), l.to_string());
        // Week not containing today: shown on date+N, not on date.
        assert_eq!(days("2026-10-12 Mon -3d", "2026-10-12", 7, "2026-10-01"), vec![p("2026-10-15", "")]);
        // Date+N still ahead of today inside a range with today.
        assert_eq!(days("2026-10-02 Fri -3d", "2026-10-02", 7, "2026-10-02"), vec![p("2026-10-05", "")]);
        // Today == date+N: exactly one (overdue label) item.
        assert_eq!(days("2026-10-02 Fri -3d", "2026-10-02", 7, "2026-10-05"), vec![p("2026-10-05", "Scheduled 3d ago")]);
        // Repeater: first on date+3, then date+7, date+14.
        let r = days("2026-10-02 Fri +1w -3d", "2026-10-01", 21, "2026-09-01");
        assert_eq!(r, vec![p("2026-10-05", ""), p("2026-10-09", ""), p("2026-10-16", "")]);
        // Moved date coinciding with a repeat occurrence is not doubled.
        assert_eq!(days("2026-10-02 Fri +1d -1d", "2026-10-02", 3, "2026-09-01"), vec![p("2026-10-03", ""), p("2026-10-04", "")]);
    }

    #[test]
    fn deadline_warning_cookie() {
        let t = d("2026-10-02");
        let warns = |dl: &str| {
            let f = Arc::new(parse(Path::new("/n/x.org"), &format!("* TODO A\nDEADLINE: <{dl}>\n"), &kw()));
            agenda(&[f], &kw(), t, 1, t).iter().find(|i| i.kind == "warning").map(|i| i.label.clone())
        };
        assert_eq!(warns("2026-10-05 Mon -3d"), Some("Due in 3d".into()));
        assert_eq!(warns("2026-10-06 Tue -3d"), None);
        assert_eq!(warns("2026-10-16 Fri"), Some("Due in 14d".into()));
        assert_eq!(warns("2026-10-17 Sat"), None);
        assert_eq!(warns("2026-10-03 Sat -0d"), None);
        assert_eq!(warns("2026-10-16 Fri --2w"), Some("Due in 14d".into()));
        assert_eq!(warns("2026-10-06 Tue 09:00 +1w -2d"), None);
        assert_eq!(warns("2026-10-04 Sun 09:00 +1w -2d"), Some("Due in 2d".into()));
        let h = &parse(Path::new("x.org"), "* TODO A\nDEADLINE: <2026-10-05 Mon +1w -2d>\n", &kw()).headlines[0];
        let dl = h.deadline.as_ref().unwrap();
        assert_eq!((dl.repeater.clone(), dl.warn), (Some(("+".into(), 1, 'w')), Some((2, 'd'))));
        // Shifting or setting a new date keeps the cookie.
        let s = "* TODO A\nDEADLINE: <2026-10-05 Mon -3d>\n";
        assert_eq!(shift_date(s, 0, "DEADLINE", 1), "* TODO A\nDEADLINE: <2026-10-06 Tue -3d>\n");
        assert_eq!(set_planning(s, 0, "DEADLINE", Some((d("2026-10-09"), Some("10:00".into()), None, None))), "* TODO A\nDEADLINE: <2026-10-09 Fri 10:00 -3d>\n");
        assert_eq!(set_planning(s, 0, "SCHEDULED", Some((d("2026-10-01"), None, None, None))), "* TODO A\nDEADLINE: <2026-10-05 Mon -3d> SCHEDULED: <2026-10-01 Thu>\n");
        // An explicit repeater replaces the old one (and its max), keeping the warning cookie (#449).
        let s = "* TODO A\nDEADLINE: <2026-10-05 Mon .+1w/2w -3d>\n";
        assert_eq!(set_planning(s, 0, "DEADLINE", Some((d("2026-10-09"), None, Some("+1m".into()), None))), "* TODO A\nDEADLINE: <2026-10-09 Fri +1m -3d>\n");
        assert_eq!(set_planning(s, 0, "DEADLINE", Some((d("2026-10-09"), None, None, None))), "* TODO A\nDEADLINE: <2026-10-09 Fri .+1w/2w -3d>\n");
    }

    #[test]
    fn timestamp_without_day_name() {
        let h = &parse(Path::new("x.org"), "* TODO A\nDEADLINE: <2026-10-30 -3d> SCHEDULED: <2026-10-28 .+1w/2w>\n", &kw()).headlines[0];
        assert_eq!(h.deadline.as_ref().unwrap().warn, Some((3, 'd')));
        let s = h.scheduled.as_ref().unwrap();
        assert_eq!((s.repeater.clone(), s.max), (Some((".+".into(), 1, 'w')), Some((2, 'w'))));
        let h = &parse(Path::new("x.org"), "* A\n<2026-10-30 lun. 10:00 +1d>\n", &kw()).headlines[0];
        assert_eq!((h.timestamps[0].time.as_deref(), h.timestamps[0].repeater.is_some()), (Some("10:00"), true));
        let t = "* TODO A\nDEADLINE: <2026-10-30 -3d>\n";
        assert_eq!(shift_date(t, 0, "DEADLINE", 1), "* TODO A\nDEADLINE: <2026-10-31 Sat -3d>\n");
    }

    #[test]
    fn time_and_date_ranges() {
        let doc = "* Talk\n<2026-10-16 Fri 10:00-11:00>\n* TODO Call\nSCHEDULED: <2026-10-16 Fri 09:00-09:30 +1w>\n\
                   * Trip\n<2026-10-18 Sun 08:00>--<2026-10-20 Tue 18:00>\n* Lunch\n<2026-10-17 Sat 12:00>--<2026-10-17 Sat 13:00>\n\
                   * Two\n<2026-10-10 Sat> and <2026-10-20 Tue>\n";
        let f = Arc::new(parse(Path::new("/n/x.org"), doc, &kw()));
        let trip = &f.headlines[2].timestamps;
        assert_eq!((trip.len(), trip[0].end, trip[0].end_time.as_deref()), (1, Some(d("2026-10-20")), Some("18:00")));
        assert_eq!(f.headlines[3].timestamps[0].span().as_deref(), Some("12:00-13:00"));
        assert_eq!(f.headlines[4].timestamps.len(), 2, "not a range without --");
        // The window ends mid-range: days 1 and 2 of 3 show, the time on the first only.
        let got: Vec<_> = agenda(std::slice::from_ref(&f), &kw(), d("2026-10-16"), 4, d("2026-10-16"))
            .into_iter().map(|i| format!("{} {} {} {}", i.date, i.title, i.time.unwrap_or_default(), i.label)).collect();
        assert_eq!(got, ["2026-10-16 Call 09:00-09:30 ", "2026-10-16 Talk 10:00-11:00 ", "2026-10-17 Lunch 12:00-13:00 ",
            "2026-10-18 Trip 08:00 (1/3)", "2026-10-19 Trip  (2/3)"]);
        let mid: Vec<_> = agenda(&[f], &kw(), d("2026-10-20"), 1, d("2026-10-16")).into_iter().map(|i| (i.title, i.label)).collect();
        assert_eq!(mid, [("Trip".to_string(), "(3/3)".to_string()), ("Two".into(), String::new())]);
    }

    #[test]
    fn overdue_and_move() {
        let t = "* TODO Old\nSCHEDULED: <2026-10-15 Thu 09:00-10:00 +1w -2d>\n* DONE Finished\nSCHEDULED: <2026-10-01 Thu>\n\
* TODO Habit\nSCHEDULED: <2026-10-01 Thu .+1d>\n:PROPERTIES:\n:STYLE: habit\n:END:\n* TODO Due only\nDEADLINE: <2026-10-01 Thu>\n\
* TODO Today\nSCHEDULED: <2026-10-20 Tue>\n* TODO Later\nSCHEDULED: <2026-10-25 Sun>\n* Plain\nSCHEDULED: <2026-10-01 Thu>\n\
* TODO Stale :ARCHIVE:\nSCHEDULED: <2026-10-01 Thu>\n* TODO Delayed\nSCHEDULED: <2026-10-18 Sun -5d>\n* NEXT Two\nDEADLINE: <2026-10-30 Fri> SCHEDULED: <2026-10-19 Mon>\n";
        let f = Arc::new(parse(Path::new("/n/a.org"), t, &kw()));
        let got: Vec<_> = overdue_scheduled(&[f], &kw(), d("2026-10-20")).into_iter().map(|(_, l)| t.lines().nth(l).unwrap().to_string()).collect();
        assert_eq!(got, ["* TODO Old", "* NEXT Two"]);
        // The absolute date replaces the old one; time range, repeater and warning stay, DEADLINE is untouched.
        assert_eq!(move_planning("* TODO A\nSCHEDULED: <2026-10-15 Thu 09:00-10:00 +1w -2d>\n", 0, "SCHEDULED", d("2026-10-20")),
            "* TODO A\nSCHEDULED: <2026-10-20 Tue 09:00-10:00 +1w -2d>\n");
        assert_eq!(move_planning("* NEXT Two\nDEADLINE: <2026-10-30 Fri> SCHEDULED: <2026-10-19 Mon .+2d/3d>\n", 0, "SCHEDULED", d("2026-10-21")),
            "* NEXT Two\nDEADLINE: <2026-10-30 Fri> SCHEDULED: <2026-10-21 Wed .+2d/3d>\n");
        assert_eq!(move_planning("* TODO B\nDEADLINE: <2026-10-30 Fri>\n", 0, "SCHEDULED", d("2026-10-21")), "* TODO B\nDEADLINE: <2026-10-30 Fri>\n");
    }

    #[test]
    fn set_planning_keeps_repeater() {
        let s = "* TODO A\nSCHEDULED: <2026-10-05 Mon .+2d/3d> DEADLINE: <2026-10-09 Fri +1w -2d>\n";
        let s = set_planning(s, 0, "SCHEDULED", Some((d("2026-10-06"), Some("09:00".into()), None, None)));
        let s = set_planning(&s, 0, "DEADLINE", Some((d("2026-10-16"), None, None, None)));
        assert_eq!(s, "* TODO A\nSCHEDULED: <2026-10-06 Tue 09:00 .+2d/3d> DEADLINE: <2026-10-16 Fri +1w -2d>\n");
        // An explicit warning cookie replaces the old one (#461).
        let s = set_planning(&s, 0, "DEADLINE", Some((d("2026-10-23"), None, None, Some("-1w".into()))));
        assert_eq!(s, "* TODO A\nSCHEDULED: <2026-10-06 Tue 09:00 .+2d/3d> DEADLINE: <2026-10-23 Fri +1w -1w>\n");
        assert_eq!(set_planning("* TODO B\n", 0, "DEADLINE", Some((d("2026-10-23"), None, Some("+1m".into()), Some("-3d".into())))), "* TODO B\nDEADLINE: <2026-10-23 Fri +1m -3d>\n");
    }

    #[test]
    fn cycle_and_repeat() {
        let log = |from: &str| format!(":PROPERTIES:\n:LAST_REPEAT: [2026-10-02 Fri 14:30]\n:END:\n- State \"DONE\"       from \"{from}\"       [2026-10-02 Fri 14:30]");
        let t = "* TODO Water plants\nSCHEDULED: <2026-09-28 Mon +1w>\n";
        let next = cycle_todo(t, 0, &kw(), 1, at(), true);
        assert_eq!(next, "* NEXT Water plants\nSCHEDULED: <2026-09-28 Mon +1w>\n");
        let done = cycle_todo(&next, 1, &kw(), 1, at(), true);
        assert_eq!(done, format!("* NEXT Water plants\nSCHEDULED: <2026-10-05 Mon +1w>\n{}\n", log("NEXT")));
        let catchup = cycle_todo("* NEXT X\nDEADLINE: <2026-09-01 Tue ++1w>", 0, &kw(), 1, at(), true);
        assert_eq!(catchup, format!("* NEXT X\nDEADLINE: <2026-10-06 Tue ++1w>\n{}", log("NEXT")));
        // LAST_REPEAT goes into an existing drawer (replacing an old one); log after it, inside a LOGBOOK.
        let t = "* TODO H\nSCHEDULED: <2026-10-01 Thu .+1d/3d>\n:PROPERTIES:\n:STYLE: habit\n:LAST_REPEAT: [2026-09-30 Wed 08:00]\n:END:\n:LOGBOOK:\n- State \"DONE\"       from \"TODO\"       [2026-09-30 Wed]\n:END:\n";
        let done = set_keyword(t, 0, &kw(), Some("DONE"), at(), true);
        assert_eq!(done, "* TODO H\nSCHEDULED: <2026-10-03 Sat .+1d/3d>\n:PROPERTIES:\n:STYLE: habit\n:LAST_REPEAT: [2026-10-02 Fri 14:30]\n:END:\n:LOGBOOK:\n- State \"DONE\"       from \"TODO\"       [2026-10-02 Fri 14:30]\n- State \"DONE\"       from \"TODO\"       [2026-09-30 Wed]\n:END:\n");
        // Both timed and date-only entries are read.
        let h = &parse(Path::new("h.org"), &done, &kw()).headlines[0];
        assert_eq!(h.done_log, vec![d("2026-10-02"), d("2026-09-30")]);
        assert_eq!(h.scheduled.as_ref().unwrap().max, Some((3, 'd')));
        let t = "* TODO H\nSCHEDULED: <2026-10-01 Thu .+1d>\n:PROPERTIES:\n:STYLE: habit\n:END:\n";
        assert!(set_keyword(t, 0, &kw(), Some("DONE"), at(), true).contains(":STYLE: habit\n:LAST_REPEAT: [2026-10-02 Fri 14:30]\n:END:\n- State"));
    }

    #[test]
    fn closed_stamp() {
        let done = |t: &str| set_keyword(t, 0, &kw(), Some("DONE"), at(), true);
        let c = "CLOSED: [2026-10-02 Fri 14:30]";
        assert_eq!(done("* TODO A\nbody\n"), format!("* DONE A\n{c}\nbody\n"));
        assert_eq!(done("* A"), format!("* DONE A\n{c}"));
        assert_eq!(done("* TODO A\n  SCHEDULED: <2026-10-01 Thu>\n"), format!("* DONE A\n  {c} SCHEDULED: <2026-10-01 Thu>\n"));
        // A stale stamp is replaced; done -> done leaves it alone.
        assert_eq!(done("* TODO A\nSCHEDULED: <2026-10-01 Thu> CLOSED: [2026-01-01 Thu 09:00]\n"), format!("* DONE A\n{c} SCHEDULED: <2026-10-01 Thu>\n"));
        let old = "* CANCELLED A\nCLOSED: [2026-01-01 Thu 09:00]\n";
        let kw2 = Kw { done: vec!["DONE".into(), "CANCELLED".into()], ..kw() };
        assert_eq!(set_keyword(old, 0, &kw2, Some("DONE"), at(), true), "* DONE A\nCLOSED: [2026-01-01 Thu 09:00]\n");
        // Reopening removes it (even with logging off), dropping an emptied planning line.
        let dl = done("* TODO A\nDEADLINE: <2026-10-09 Fri>\n");
        assert_eq!(dl, format!("* DONE A\n{c} DEADLINE: <2026-10-09 Fri>\n"));
        assert_eq!(set_keyword(&dl, 0, &kw(), Some("TODO"), at(), false), "* TODO A\nDEADLINE: <2026-10-09 Fri>\n");
        assert_eq!(cycle_todo(&done("* TODO A\nbody\n"), 0, &kw(), 1, at(), true), "* A\nbody\n");
        // Off: no stamp.
        assert_eq!(set_keyword("* TODO A\n", 0, &kw(), Some("DONE"), at(), false), "* DONE A\n");
        // Parsed, with or without a time; only on the planning line.
        let f = parse(Path::new("x.org"), &format!("{}* DONE B\nCLOSED: [2026-09-30 Wed]\n* DONE C\nbody CLOSED: [2026-09-30 Wed]\n", done("* TODO A\n")), &kw());
        let cl: Vec<_> = f.headlines.iter().map(|h| h.closed).collect();
        assert_eq!(cl, vec![Some(at()), Some(d("2026-09-30").and_hms_opt(0, 0, 0).unwrap()), None]);
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
        assert_eq!(cycle_todo("* X :t:", 0, &kw(), 1, at(), false), "* TODO X :t:");
        assert_eq!(cycle_todo("* DONE X", 0, &kw(), 1, at(), false), "* X");
    }

    #[test]
    fn headline_edits() {
        let t = "* TODO [#B] Task :a:\nbody\n";
        assert_eq!(cycle_priority(t, 1, &kw(), 1), "* TODO [#A] Task :a:\nbody\n");
        assert_eq!(cycle_priority("* X", 0, &kw(), 1), "* [#C] X");
        assert_eq!(cycle_priority("* [#A] X", 0, &kw(), 1), "* X");
        assert_eq!(set_priority(t, 0, &kw(), None), "* TODO Task :a:\nbody\n");
        assert_eq!(set_tags(t, 0, &kw(), vec!["x".into(), "y".into()]), "* TODO [#B] Task :x:y:\nbody\n");
        assert_eq!(set_title(t, 0, &kw(), " New name "), "* TODO [#B] New name :a:\nbody\n");
        assert_eq!(set_title(t, 0, &kw(), "  "), t);
        assert_eq!(set_keyword(t, 0, &kw(), None, at(), false), "* [#B] Task :a:\nbody\n");
        assert_eq!(heading(t, 1, &kw()).unwrap().title, "Task");
        let s = "* TODO A\nSCHEDULED: <2026-10-02 Fri> DEADLINE: <2026-10-09 Fri +1w>\n";
        assert_eq!(shift_date(s, 0, "SCHEDULED", 1), "* TODO A\nSCHEDULED: <2026-10-03 Sat> DEADLINE: <2026-10-09 Fri +1w>\n");
        assert_eq!(shift_date(s, 0, "DEADLINE", -7), "* TODO A\nSCHEDULED: <2026-10-02 Fri> DEADLINE: <2026-10-02 Fri +1w>\n");
    }

    #[test]
    fn notes() {
        let n = "- Note taken on [2026-10-02 Fri 14:30] \\\\\n";
        assert_eq!(add_note("* A\nbody\n* B\n", 0, " Called Bob ", at()), format!("* A\n{n}  Called Bob\nbody\n* B\n"));
        // Below the planning line and both drawers; from a body line too.
        let t = "* TODO A\nSCHEDULED: <2026-10-02 Fri +1d>\n:PROPERTIES:\n:ID: x\n:END:\n:LOGBOOK:\n- State \"DONE\" from \"TODO\" [2026-10-01 Thu]\n:END:\nbody\n";
        assert_eq!(add_note(t, 8, "x", at()), t.replace(":END:\nbody", &format!(":END:\n{n}  x\nbody")));
        // Last heading, with and without a trailing newline; multi-line text.
        assert_eq!(add_note("* A\n** B\n", 1, "one\n\ntwo", at()), format!("* A\n** B\n{n}  one\n\n  two\n"));
        assert_eq!(add_note("* A", 0, "x", at()), format!("* A\n{n}  x"));
        // Newest first.
        let once = add_note("* A\n", 0, "old", at());
        let later = d("2026-10-03").and_hms_opt(9, 0, 0).unwrap();
        assert_eq!(add_note(&once, 0, "new", later), format!("* A\n- Note taken on [2026-10-03 Sat 09:00] \\\\\n  new\n{n}  old\n"));
        assert_eq!(add_note("no heading\n", 0, "x", at()), "no heading\n");
    }

    #[test]
    fn unclosed_drawer_stays_in_its_heading() {
        let n = "- Note taken on [2026-10-02 Fri 14:30] \\\\\n";
        assert_eq!(add_note("* A\n:LOGBOOK:\n- x\n* B\nbody\n", 0, "hi", at()), format!("* A\n{n}  hi\n:LOGBOOK:\n- x\n* B\nbody\n"));
        assert_eq!(add_note("* A\n:LOGBOOK:\n- x\n", 0, "hi", at()), format!("* A\n{n}  hi\n:LOGBOOK:\n- x\n"));
        // An :END: belonging to a later heading is not this drawer's.
        let t = "* A\n:PROPERTIES:\n* B\n:PROPERTIES:\n:END:\n";
        assert_eq!(add_note(t, 0, "hi", at()), format!("* A\n{n}  hi\n:PROPERTIES:\n* B\n:PROPERTIES:\n:END:\n"));
    }

    #[test]
    fn repeat_with_unclosed_properties_stays_put() {
        let t = "* TODO A\nSCHEDULED: <2026-10-02 Fri +1d>\n:PROPERTIES:\n:ID: x\n* B\n:END:\n";
        let r = set_keyword(t, 0, &kw(), Some("DONE"), at(), false);
        assert!(r.contains(":LAST_REPEAT:") && r.find(":LAST_REPEAT:") < r.find("* B"), "{r}");
    }

    #[test]
    fn sort_children_keys() {
        let s = |t: &str, line, key, desc| sort_children(t, line, &kw(), key, desc).unwrap();
        // Nested children travel with their parent; the body before the first child stays; text after the subtree stays.
        let src = "* P\nbody\n** b\n*** deep\n** C\n** a\n* Q\n";
        assert_eq!(s(src, Some(1), "title", false), "* P\nbody\n** a\n** b\n*** deep\n** C\n* Q\n");
        assert_eq!(s(src, Some(0), "title", true), "* P\nbody\n** C\n** b\n*** deep\n** a\n* Q\n");
        // Top level: no line, or a cursor above the first heading; the preamble stays.
        let top = "#+title: T\n* b\n* a";
        assert_eq!(s(top, None, "title", false), "#+title: T\n* a\n* b");
        assert_eq!(s(top, Some(0), "title", false), "#+title: T\n* a\n* b");
        // Missing keys go last, both directions; ties keep their order.
        let pri = "* x\n* [#B] b1\n* [#A] a\n* [#B] b2\n";
        assert_eq!(s(pri, None, "priority", false), "* [#A] a\n* [#B] b1\n* [#B] b2\n* x\n");
        assert_eq!(s(pri, None, "priority", true), "* [#B] b1\n* [#B] b2\n* [#A] a\n* x\n");
        assert_eq!(s("* DONE d\n* plain\n* NEXT n\n* TODO t\n", None, "status", false), "* TODO t\n* NEXT n\n* DONE d\n* plain\n");
        let dates = "* none\n* late\nSCHEDULED: <2026-10-05 Mon> DEADLINE: <2026-10-01 Thu>\n* early\nSCHEDULED: <2026-10-02 Fri 10:00>\n";
        assert_eq!(s(dates, None, "scheduled", false), "* early\nSCHEDULED: <2026-10-02 Fri 10:00>\n* late\nSCHEDULED: <2026-10-05 Mon> DEADLINE: <2026-10-01 Thu>\n* none\n");
        assert_eq!(s(dates, None, "deadline", false), "* late\nSCHEDULED: <2026-10-05 Mon> DEADLINE: <2026-10-01 Thu>\n* none\n* early\nSCHEDULED: <2026-10-02 Fri 10:00>\n");
        // Blank separators stay positional, so the last child doesn't glue onto the next.
        assert_eq!(s("* P\n** b\n\n** a\n* Q\n", Some(0), "title", false), "* P\n** a\n\n** b\n* Q\n");
        assert!(sort_children("* P\n** only\n", Some(0), &kw(), "title", false).is_err());
        // A deeper first child doesn't hide its shallower siblings.
        assert!(sort_children("* P\n*** x\n** y\n*** z\n", Some(0), &kw(), "title", true).is_err());
        assert_eq!(s("* P\n*** x\n** b\n*** z\n** a\n", Some(0), "title", false), "* P\n*** x\n** a\n** b\n*** z\n");
        assert!(sort_children("no headings\n", None, &kw(), "title", false).is_err());
        assert!(sort_children(src, Some(0), &kw(), "size", false).is_err());
    }

    #[test]
    fn extracts_subtree_into_note() {
        // (new source, title, body) for an extract at LINE.
        let ex = |text: &str, line| {
            let mut got = (String::new(), String::new());
            extract_subtree(text, line, &kw(), |t, b| {
                got = (t.into(), b.into());
                Ok("N1".into())
            })
            .map(|s| (s, got.0, got.1))
        };
        let src = "* Top\n** TODO [#A] Plan [1/2] :work:\nSCHEDULED: <2026-10-02 Fri>\n:PROPERTIES:\n:ID: old\n:END:\n\nintro\n*** Child\nc\n**** Deep\n\n** Next\n";
        let kept = "* Top\n** TODO [#A] Plan [1/2] :work:\nSCHEDULED: <2026-10-02 Fri>\n:PROPERTIES:\n:ID: old\n:END:\n[[id:N1][Plan]]\n\n** Next\n";
        assert_eq!(ex(src, 7).unwrap(), (kept.into(), "Plan".into(), "intro\n* Child\nc\n** Deep".into()));
        // Last heading, no trailing newline, no body.
        assert_eq!(ex("* A\n* B [50%]", 1).unwrap(), ("* A\n* B [50%]\n[[id:N1][B]]".into(), "B".into(), String::new()));
        assert_eq!(ex("text\n", 0).unwrap_err(), "not on a heading");
        assert_eq!(ex("* TODO :tag:\n", 0).unwrap_err(), "the heading has no title");
        assert_eq!(extract_subtree("* A\n", 0, &kw(), |_, _| Err("boom".into())).unwrap_err(), "boom");
    }

    #[test]
    fn archive_done_moves_top_most_done_subtrees_in_order() {
        let kw = Kw { todo: vec!["TODO".into()], done: vec!["DONE".into(), "CANCELLED".into()] };
        let now = chrono::NaiveDateTime::parse_from_str("2026-10-02 14:30", "%Y-%m-%d %H:%M").unwrap();
        let src = "#+title: x\n* DONE A\n** TODO A1\n* TODO B\n** DONE B1\nnote\n*** DONE B1a\n** TODO B2\n* DONE C :ARCHIVE:\n* Old :ARCHIVE:\n** DONE D\n* DONE R\nSCHEDULED: <2026-10-01 Thu +1w>\n* CANCELLED E\n";
        let (s, a, n) = archive_done(src, "* earlier\n", "/n/x.org", now, &kw);
        assert_eq!(n, 3);
        assert_eq!(s, "#+title: x\n* TODO B\n** TODO B2\n* DONE C :ARCHIVE:\n* Old :ARCHIVE:\n** DONE D\n* DONE R\nSCHEDULED: <2026-10-01 Thu +1w>\n");
        let p = ":PROPERTIES:\n:ARCHIVE_TIME: 2026-10-02 Fri 14:30\n:ARCHIVE_FILE: /n/x.org\n:END:\n";
        assert_eq!(a, format!("* earlier\n* DONE A\n{p}** TODO A1\n* DONE B1\n{p}note\n** DONE B1a\n* CANCELLED E\n{p}"));
        assert_eq!(archive_done("* TODO a\n", "", "/n/x.org", now, &kw), ("* TODO a\n".into(), "".into(), 0));
        assert!(archive_done("* DONE a\n", "", "/n/x.org", now, &kw).1.starts_with("#    -*- mode: org -*-\n\n\nArchived entries from file /n/x.org\n\n* DONE a\n"));
    }

    #[test]
    fn many_keep_source_order() {
        let now = chrono::NaiveDateTime::parse_from_str("2026-10-02 14:30", "%Y-%m-%d %H:%M").unwrap();
        let src = "* A
* X
* B
** B1
* C
";
        let (s, d) = refile_many(src, &[4, 0, 2], "* T
", Some(0)).unwrap();
        assert_eq!((s.as_str(), d.as_str()), ("* X
", "* T
** A
** B
*** B1
** C
"));
        let (s, a) = archive_many(src, &[4, 0, 2], "", "/n/x.org", now).unwrap();
        assert_eq!(s, "* X
");
        let heads: Vec<&str> = a.lines().filter(|l| l.starts_with('*')).collect();
        assert_eq!(heads, ["* A", "* B", "** B1", "* C"]);
        assert!(refile_many("text\n", &[0], "", None).is_err());
    }

    #[test]
    fn clone_with_time_shift() {
        let k = kw();
        // Weekly ×3: planning (time, repeater, warning kept, weekday recomputed), body and range stamps move; inactive ones don't.
        let t = "* A\n** TODO Check-in\nSCHEDULED: <2026-10-05 Mon 09:00 +1w> DEADLINE: <2026-10-06 Tue -1d>\n<2026-10-07 Wed>--<2026-10-08 Thu> [2026-10-01 Thu]\n* B\n";
        let r = clone_shift(t, 2, 3, "+1w", &k).unwrap();
        let want = |a: &str, b: &str, c: &str, d: &str| format!("** TODO Check-in\nSCHEDULED: <{a} 09:00 +1w> DEADLINE: <{b} -1d>\n<{c}>--<{d}> [2026-10-01 Thu]\n");
        assert_eq!(
            r,
            format!(
                "* A\n{}{}{}{}* B\n",
                want("2026-10-05 Mon", "2026-10-06 Tue", "2026-10-07 Wed", "2026-10-08 Thu"),
                want("2026-10-12 Mon", "2026-10-13 Tue", "2026-10-14 Wed", "2026-10-15 Thu"),
                want("2026-10-19 Mon", "2026-10-20 Tue", "2026-10-21 Wed", "2026-10-22 Thu"),
                want("2026-10-26 Mon", "2026-10-27 Tue", "2026-10-28 Wed", "2026-10-29 Thu"),
            )
        );
        // Monthly from the original date: Jan 31 → Feb 29 (leap year) → Mar 31, not clamped drift.
        let r = clone_shift("* TODO Pay\nDEADLINE: <2024-01-31 Wed>\n", 0, 2, "1m", &k).unwrap();
        assert_eq!(r, "* TODO Pay\nDEADLINE: <2024-01-31 Wed>\n* TODO Pay\nDEADLINE: <2024-02-29 Thu>\n* TODO Pay\nDEADLINE: <2024-03-31 Sun>\n");
        assert!(clone_shift("* TODO Pay\nDEADLINE: <2025-01-31 Fri>\n", 0, 1, "+1m", &k).unwrap().ends_with("DEADLINE: <2025-02-28 Fri>\n"));
        // Done → first todo keyword (children too); CLOSED, LOGBOOK, stray CLOCK lines and :ID: dropped from the copy only.
        let t = "* DONE [#A] Meet :work:\nCLOSED: [2026-10-02 Fri 10:00] SCHEDULED: <2026-10-02 Fri>\n:PROPERTIES:\n:ID: x\n:END:\n:LOGBOOK:\nCLOCK: [2026-10-02 Fri 09:00]--[2026-10-02 Fri 10:00] =>  1:00\n:END:\nCLOCK: [2026-10-01 Thu 09:00]--[2026-10-01 Thu 10:00] =>  1:00\nnotes\n** DONE Sub\nCLOSED: [2026-10-02 Fri 10:00]\n";
        let r = clone_shift(t, 9, 1, "1d", &k).unwrap();
        assert_eq!(r, format!("{t}* TODO [#A] Meet :work:\nSCHEDULED: <2026-10-03 Sat>\nnotes\n** TODO Sub\n"));
        // A task without stamps is copied as is; no trailing newline kept.
        assert_eq!(clone_shift("* x", 0, 2, "1d", &k).unwrap(), "* x\n* x\n* x");
        for (n, s) in [(1, "1h"), (1, "0d"), (1, "-1d"), (0, "1d"), (53, "1d"), (1, "w")] {
            assert!(clone_shift("* x\n", 0, n, s, &k).is_err(), "{n} {s}");
        }
        assert_eq!(clone_shift("text\n", 0, 1, "1d", &k).unwrap_err(), "not on a heading");
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
        // Copy: source untouched, :ID:s dropped (nested too), a drawer emptied by that goes, others stay.
        let src = "* Inbox\n** TODO Buy milk\n:PROPERTIES:\n:ID: a\n:END:\nnote\n*** sub\n:PROPERTIES:\n:id: b\n:EFFORT: 1h\n:END:\n* Other\n";
        let dst = "#+title: P\n* Projects\n** Home\n* Later\n";
        assert_eq!(refile_copy(src, 2, dst, Some(2)).unwrap(), "#+title: P\n* Projects\n** Home\n*** TODO Buy milk\nnote\n**** sub\n:PROPERTIES:\n:EFFORT: 1h\n:END:\n* Later\n");
        assert_eq!(refile_copy("* A\n:PROPERTIES:\n:END:\n", 0, "", None).unwrap(), "* A\n:PROPERTIES:\n:END:\n");
        assert_eq!(refile_copy("** A\n*** B\n", 0, "* X", None).unwrap(), "* X\n* A\n** B\n");
        assert_eq!(refile_copy("* A\n", 0, "x\n", Some(0)).unwrap_err(), "refile target is not a heading");
        assert_eq!(refile_copy("text\n", 0, "", None).unwrap_err(), "not on a heading");
        let t = "* A\n:PROPERTIES:\n:ID: a\n:END:\n** B\n* C\n";
        assert_eq!(refile_copy_same(t, 0, Some(5)).unwrap(), "* A\n:PROPERTIES:\n:ID: a\n:END:\n** B\n* C\n** A\n*** B\n");
        assert_eq!(refile_copy_same(t, 0, Some(0)).unwrap(), "* A\n:PROPERTIES:\n:ID: a\n:END:\n** B\n** A\n*** B\n* C\n");
        assert_eq!(refile_copy_same(t, 4, None).unwrap(), "* A\n:PROPERTIES:\n:ID: a\n:END:\n** B\n* C\n* B\n");
        let now = chrono::NaiveDateTime::parse_from_str("2026-10-02 14:30", "%Y-%m-%d %H:%M").unwrap();
        let (s, a) = archive("* DONE A\nSCHEDULED: <2026-10-01 Thu>\n* B\n", 0, "", "/n/x.org", now).unwrap();
        assert_eq!(s, "* B\n");
        assert!(a.ends_with("* DONE A\nSCHEDULED: <2026-10-01 Thu>\n:PROPERTIES:\n:ARCHIVE_TIME: 2026-10-02 Fri 14:30\n:ARCHIVE_FILE: /n/x.org\n:END:\n"), "{a}");
        assert_eq!(capture_insert("#+title: Inbox\n", Some("Tasks"), "* TODO new\n", &kw()), "#+title: Inbox\n* Tasks\n** TODO new\n");
        assert_eq!(capture_insert("* Tasks\n** old\n* Z\n", Some("Tasks"), "* TODO new", &kw()), "* Tasks\n** old\n** TODO new\n* Z\n");
        assert_eq!(capture_insert("", None, "* TODO x\n[2026-10-02 Fri]\n\n", &kw()), "* TODO x\n[2026-10-02 Fri]\n");
        assert_eq!(fmt_time(now, "daily/%Y-%m-%d.org").unwrap(), "daily/2026-10-02.org");
        assert_eq!(fmt_time(now, "%Q"), None);
        assert_eq!(task_entry("Call", Some("TODO"), Some("A"), vec!["home".into()], Some((d("2026-10-02"), None, None, None)), Some((d("2026-10-05"), Some("09:00".into()), None, None))),
            "* TODO [#A] Call :home:\nSCHEDULED: <2026-10-02 Fri> DEADLINE: <2026-10-05 Mon 09:00>\n");
        assert_eq!(task_entry("x", Some("TODO"), None, vec![], None, None), "* TODO x\n");
        assert_eq!(task_entry("x", Some("TODO"), None, vec![], Some((d("2026-10-02"), Some("09:00".into()), Some(".+2d/3d".into()), None)), None), "* TODO x\nSCHEDULED: <2026-10-02 Fri 09:00 .+2d/3d>\n");
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
        let s = set_planning(t, 1, "SCHEDULED", Some((d("2026-10-02"), None, None, None)));
        assert_eq!(s, "* TODO A\nSCHEDULED: <2026-10-02 Fri>\nbody\n");
        let s = set_planning(&s, 0, "DEADLINE", Some((d("2026-10-09"), Some("09:00".into()), None, None)));
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
            let (f, done) = query(s, t, &kw(), |_| None).unwrap();
            todos(&files, &kw(), done, f, t).into_iter().map(|i| i.title).collect::<Vec<_>>().join(", ")
        };
        assert_eq!(q(""), "Write report, Plan trip, Call mum, Report draft");
        assert_eq!(q("todo:next"), "Write report");
        assert_eq!(q("tag:work"), "Write report, Plan trip");
        assert_eq!(q("-tag:home"), "Write report, Report draft");
        // Inherited tags: #+FILETAGS and ancestors, popped on a sibling.
        let inh = [Arc::new(parse(Path::new("/n/acme.org"), "#+FILETAGS: :client:\n* Acme :work:\n** TODO Invoice :money:\n*** TODO Send\n\
* TODO Lunch\n", &kw()))];
        let h = &inh[0].headlines;
        assert_eq!((h[0].inherited.clone(), h[1].tags.clone(), h[1].inherited.clone()), (vec!["client".into()], vec!["money".into()], vec!["client".into(), "work".into()]));
        assert_eq!((h[2].inherited.clone(), h[3].inherited.clone()), (vec!["client".to_string(), "work".into(), "money".into()], vec!["client".to_string()]));
        let iq = |s: &str| todos(&inh, &kw(), false, query(s, t, &kw(), |_| None).unwrap().0, t).into_iter().map(|i| i.title).collect::<Vec<_>>().join(", ");
        assert_eq!(iq("tag:work"), "Invoice, Send");
        assert_eq!(iq("-tag:work"), "Lunch");
        assert_eq!(iq("tag:client"), "Invoice, Send, Lunch");
        assert_eq!(all_tags(&inh), vec!["client", "money", "work"]);
        // Subtrees archived in place (:ARCHIVE:) are skipped by todos() and agenda().
        let arc = [Arc::new(parse(Path::new("/n/a.org"), "* Old :ARCHIVE:\n** TODO Stale\nSCHEDULED: <2026-10-02 Fri>\n*** TODO Deeper\n\
* TODO Done here :ARCHIVE:\n* Live\n** TODO Fresh\nSCHEDULED: <2026-10-02 Fri>\n", &kw()))];
        let all = |_: &OrgFile, _: &Headline| true;
        assert_eq!(todos(&arc, &kw(), true, all, t).into_iter().map(|i| i.title).collect::<Vec<_>>(), vec!["Fresh"]);
        assert_eq!(agenda(&arc, &kw(), t, 1, t).into_iter().map(|i| i.title).collect::<Vec<_>>(), vec!["Fresh"]);
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
        assert_eq!(q("tag:\"work\" file:\"inbox\""), "Write report, Plan trip");
        assert_eq!(q("-tag:\"home\" todo:\"next\""), "Write report");
        assert_eq!(q("-file:\"my notes\" -tag:\"x y\" report"), "Write report, Report draft");
        let mine = [Arc::new(parse(Path::new("/n/my notes.org"), "* TODO My idea :idea:\n", &kw()))];
        let (f, _) = query("file:\"MY NOTES\" tag:\"idea\" -tag:\"x y\"", t, &kw(), |_| None).unwrap();
        assert_eq!(todos(&mine, &kw(), false, f, t).len(), 1);
        assert_eq!(q("todo:TODO tag:home scheduled:<+1m"), "Plan trip, Call mum");
        assert_eq!(q("report -pri:a"), "Report draft");
        assert!(query("due:<someday", t, &kw(), |_| None).is_err());
        // closed: (pre-CLOSED done tasks, like Old report, never match a date)
        let fin = [Arc::new(parse(Path::new("/n/log.org"), "* DONE Shipped :work:\nCLOSED: [2026-09-29 Tue 10:00]\n\
* DONE Ancient\nCLOSED: [2026-08-01 Sat]\n* DONE Today\nCLOSED: [2026-10-02 Fri 08:00]\n* DONE Unstamped\n* TODO Open\n", &kw()))];
        let c = |s: &str| {
            let (f, done) = query(s, t, &kw(), |_| None).unwrap();
            (todos(&fin, &kw(), done, f, t).into_iter().map(|i| i.title).collect::<Vec<_>>().join(", "), done)
        };
        assert_eq!(c("closed:>=-7d"), ("Shipped, Today".into(), true));
        assert_eq!(c("closed:>=-7d tag:work"), ("Shipped".into(), true));
        assert_eq!(c("closed:today"), ("Today".into(), true));
        assert_eq!(c("closed:<2026-09-01"), ("Ancient".into(), true));
        assert_eq!(c("-closed:any"), ("Open".into(), false));
        assert_eq!(c("closed:none"), ("Open".into(), false));
        assert_eq!(c("closed:none todo:DONE"), ("Unstamped".into(), true));
        // effort: unestimated tasks match only none / -any
        let est = [Arc::new(parse(Path::new("/n/e.org"), "* TODO Quick\n:PROPERTIES:\n:EFFORT: 0:20\n:END:\n* TODO Hour\n:PROPERTIES:\n:EFFORT: 1h\n:END:\n* TODO Long\n:PROPERTIES:\n:EFFORT: 3:00\n:END:\n* TODO Vague\n", &kw()))];
        let e = |s: &str| {
            let (f, done) = query(s, t, &kw(), |_| None).unwrap();
            assert!(!done);
            todos(&est, &kw(), done, f, t).into_iter().map(|i| i.title).collect::<Vec<_>>().join(", ")
        };
        assert_eq!(e("effort:<=30"), "Quick");
        assert_eq!(e("effort:>1h"), "Long");
        assert_eq!(e("effort:>=60"), "Hour, Long");
        assert_eq!(e("effort:1h"), "Hour");
        assert_eq!(e("effort:<=0:30"), "Quick");
        assert_eq!(e("effort:none"), "Vague");
        assert_eq!(e("-effort:any"), "Vague");
        assert_eq!(e("effort:any -effort:<1h"), "Hour, Long");
        assert!(query("effort:<soon", t, &kw(), |_| None).is_err());
        // spent: / over: against clocked minutes (Quick 25 of 20, Hour 30 of 60, Vague 10, Long none)
        let sp = |h: &Headline| match h.title.as_str() { "Quick" => Some(25), "Hour" => Some(30), "Vague" => Some(10), _ => None };
        let st = |s: &str| todos(&est, &kw(), false, query(s, t, &kw(), sp).unwrap().0, t).into_iter().map(|i| i.title).collect::<Vec<_>>().join(", ");
        assert_eq!(st("over:yes"), "Quick");
        assert_eq!(st("over:no"), "Hour, Long, Vague"); // unestimated never runs over
        assert_eq!(st("-over:yes effort:any"), "Hour, Long");
        assert_eq!(st("spent:>=0:25"), "Quick, Hour");
        assert_eq!(st("spent:<30"), "Quick, Vague");
        assert_eq!(st("spent:none"), "Long");
        assert_eq!(st("spent:any effort:none"), "Vague");
        assert!(query("over:maybe", t, &kw(), sp).is_err());
        assert!(query("spent:>lots", t, &kw(), sp).is_err());
    }

    #[test]
    fn dates() {
        let t = d("2026-10-02"); // Friday
        assert_eq!(read_date("", t), Some((t, None, None, None)));
        assert_eq!(read_date("+3d", t).unwrap().0, d("2026-10-05"));
        assert_eq!(read_date("+1w 14:00", t), Some((d("2026-10-09"), Some("14:00".into()), None, None)));
        assert_eq!(read_date("mon", t).unwrap().0, d("2026-10-05"));
        assert_eq!(read_date("fri", t).unwrap().0, d("2026-10-09"));
        assert_eq!(read_date("12-24", t).unwrap().0, d("2026-12-24"));
        assert_eq!(read_date("2027-01-01", t).unwrap().0, d("2027-01-01"));
        assert_eq!(read_date("15", t).unwrap().0, d("2026-10-15"));
        assert_eq!(read_date("nonsense", t), None);
        // Only a clock time (or range) may follow the date (#448).
        for bad in ["fri 2pm", "fri foo", "+1w 14:00 x", "fri 25:00", "fri 9:60", "fri 9:5", "fri 10:00-", "14:00 x"] {
            assert_eq!(read_date(bad, t), None, "{bad}");
        }
        assert_eq!(read_date("fri 9:30", t).unwrap().1.as_deref(), Some("09:30"));
        assert_eq!(read_date("fri 10:00-11:30", t).unwrap().1.as_deref(), Some("10:00-11:30"));
        assert_eq!(read_date("8:15", t), Some((t, Some("08:15".into()), None, None)));
        // One repeater may follow the date and/or time (#449).
        assert_eq!(read_date("fri +1w", t), Some((d("2026-10-09"), None, Some("+1w".into()), None)));
        assert_eq!(read_date("mon 9:00 .+1d", t), Some((d("2026-10-05"), Some("09:00".into()), Some(".+1d".into()), None)));
        assert_eq!(read_date("today ++1m", t), Some((t, None, Some("++1m".into()), None)));
        assert_eq!(read_date("today .+2d/3d", t).unwrap().2.as_deref(), Some(".+2d/3d"));
        for bad in ["fri +1w +1d", "fri +1x", "fri 9:00 9:30"] {
            assert_eq!(read_date(bad, t), None, "{bad}");
        }
        // One warning cookie may follow too (#461); a leading "-3d" stays a date.
        assert_eq!(read_date("fri -3d", t), Some((d("2026-10-09"), None, None, Some("-3d".into()))));
        assert_eq!(read_date("fri 14:00 +1w -2d", t), Some((d("2026-10-09"), Some("14:00".into()), Some("+1w".into()), Some("-2d".into()))));
        assert_eq!(read_date("-3d", t), Some((d("2026-09-29"), None, None, None)));
        for bad in ["fri -3d -1w", "fri -3x", "fri -3", "fri --3d"] {
            assert_eq!(read_date(bad, t), None, "{bad}");
        }
    }

    #[test]
    fn reads_past_dates() {
        let t = d("2026-10-05"); // Monday
        assert_eq!(read_past_date("fri", t).unwrap().0, d("2026-10-02"));
        assert_eq!(read_past_date("mon", t).unwrap().0, d("2026-09-28"));
        assert_eq!(read_past_date("tue 9:00", t).unwrap().0, d("2026-09-29"));
        assert_eq!(read_past_date("yesterday", t).unwrap().0, d("2026-10-04"));
        assert_eq!(read_past_date("today", t).unwrap().0, t);
        assert_eq!(read_past_date("-2d", t).unwrap().0, d("2026-10-03"));
        assert_eq!(read_past_date("2026-10-05", t).unwrap().0, t);
        assert_eq!(read_past_date("2026-10-06", t), None);
        assert_eq!(read_past_date("tomorrow", t), None);
    }

    #[test]
    fn heading_id() {
        let k = kw();
        let src = "intro
* TODO [#A] Parent :t:
SCHEDULED: <2026-10-02 Fri>
text
** Child
body
";
        // planning line: drawer goes below it, title is clean
        let (t, id, title) = ensure_heading_id(src, 3, &k);
        assert_eq!(title, "Parent");
        assert_eq!(t.lines().nth(3).unwrap(), ":PROPERTIES:");
        assert_eq!(t.lines().nth(4).unwrap(), format!(":ID: {id}"));
        assert!(t.contains("** Child
body") && !t.contains("Child
:PROPERTIES:"));
        // idempotent: existing ID reused, text unchanged
        let (t2, id2, _) = ensure_heading_id(&t, 1, &k);
        assert_eq!((t2.as_str(), id2.as_str()), (t.as_str(), id.as_str()));
        // nested heading gets its own; drawer without ID gets one added
        let (t3, id3, title3) = ensure_heading_id("* A
:PROPERTIES:
:CATEGORY: c
:END:
", 0, &k);
        assert!(t3.contains(&format!(":ID: {id3}")) && t3.matches(":PROPERTIES:").count() == 1);
        assert_eq!(title3, "A");
        // before the first heading: file-level ID
        let (t4, id4, _) = ensure_heading_id("intro
* A
", 0, &k);
        assert!(t4.starts_with(&format!(":PROPERTIES:
:ID:       {id4}")));
    }
}
