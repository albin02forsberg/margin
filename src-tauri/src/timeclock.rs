// Port of albin-timeclock-*.el. Data lives per profile in
// <data_dir>/<profile>/{timelog.jsonl,projects.toml}; the diary is an org
// file in the notes dir so it's searchable and linkable like any note.

use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Weekday};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub const ONGOING: &str = "Ongoing session";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "ev", rename_all = "lowercase")]
pub enum Event {
    In {
        t: NaiveDateTime,
        project: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        task: String,
    },
    Out {
        t: NaiveDateTime,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        note: String,
    },
    Break {
        t: NaiveDateTime,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Project {
    #[serde(default)]
    pub export_code: String,
    #[serde(default)]
    pub rounding: Option<f64>,
    #[serde(default)]
    pub round_up: bool,
    #[serde(default = "yes")]
    pub active: bool,
}
fn yes() -> bool {
    true
}

impl Project {
    pub fn new(export_code: &str) -> Self {
        Project { export_code: export_code.into(), rounding: Some(0.5), round_up: false, active: true }
    }
}

pub type Projects = BTreeMap<String, Project>;

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Session {
    pub date: NaiveDate,
    pub project: String,
    pub desc: String,
    pub hours: f64,
    pub start: NaiveTime,
    pub end: NaiveTime,
    /// Line index of the closing `out` event, for editing.
    pub line: Option<usize>,
}

pub fn now() -> NaiveDateTime {
    chrono::Local::now().naive_local().with_nanosecond(0).unwrap()
}

fn hours(d: Duration) -> f64 {
    d.num_seconds() as f64 / 3600.0
}

// ---------------------------------------------------------------- storage

pub struct Tc {
    pub dir: PathBuf,
    pub diary: PathBuf,
    pub expected: f64,
}

impl Tc {
    pub fn log_path(&self) -> PathBuf {
        self.dir.join("timelog.jsonl")
    }

    pub fn projects_path(&self) -> PathBuf {
        self.dir.join("projects.toml")
    }

    /// The log for display; unreadable is empty.
    pub fn read_log(&self) -> String {
        fs::read_to_string(self.log_path()).unwrap_or_default()
    }

    /// The log before rewriting it: missing is empty, unreadable is an error.
    fn try_read_log(&self) -> Result<String, String> {
        read_or_empty(&self.log_path())
    }

    /// Events with their line index. Malformed lines are skipped (`doctor` reports them).
    pub fn events(&self) -> Vec<(usize, Event)> {
        parse_events(&self.read_log())
    }

    fn append(&self, ev: &Event) -> Result<(), String> {
        fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
        let mut f = fs::OpenOptions::new().create(true).append(true).open(self.log_path()).map_err(|e| e.to_string())?;
        writeln!(f, "{}", serde_json::to_string(ev).unwrap()).map_err(|e| e.to_string())
    }

    fn rewrite_line(&self, idx: usize, ev: &Event) -> Result<(), String> {
        let text = self.try_read_log()?;
        let mut lines: Vec<String> = text.lines().map(String::from).collect();
        let l = lines.get_mut(idx).ok_or("line not found")?;
        *l = serde_json::to_string(ev).unwrap();
        crate::config::write_atomic(&self.log_path(), lines.join("\n") + "\n").map_err(|e| e.to_string())
    }

    /// Projects for display; unreadable or malformed is empty.
    pub fn projects(&self) -> Projects {
        self.load_projects().unwrap_or_default()
    }

    /// Projects before saving them: missing is empty, unreadable or malformed is an error.
    pub fn load_projects(&self) -> Result<Projects, String> {
        let path = self.projects_path();
        toml::from_str(&read_or_empty(&path)?).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// Unreadable or malformed projects.toml / diary, which the lenient readers hide.
    pub fn file_issues(&self) -> Vec<String> {
        [self.load_projects().err(), read_or_empty(&self.diary).err()].into_iter().flatten().collect()
    }

    pub fn save_projects(&self, p: &Projects) -> Result<(), String> {
        fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
        crate::config::write_atomic(&self.projects_path(), toml::to_string(p).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    pub fn sessions(&self) -> Vec<Session> {
        sessions(&self.events(), now())
    }

    /// The open `in` event, if clocked in.
    pub fn current(&self) -> Option<(NaiveDateTime, String, String)> {
        match self.events().pop() {
            Some((_, Event::In { t, project, task })) => Some((t, project, task)),
            _ => None,
        }
    }

    /// Project of the `in` preceding a trailing `break`, if on a break.
    pub fn on_break(&self) -> Option<String> {
        let ev = self.events();
        if !matches!(ev.last(), Some((_, Event::Break { .. }))) {
            return None;
        }
        ev.iter().rev().find_map(|(_, e)| match e {
            Event::In { project, .. } => Some(project.clone()),
            _ => None,
        })
    }

    pub fn status(&self) -> (Option<(NaiveDateTime, String, String)>, Option<String>, f64) {
        let now = now();
        let mut today: f64 = self.sessions().iter().filter(|s| s.date == now.date()).map(|s| s.hours).sum();
        let cur = self.current();
        if let Some((t, _, _)) = &cur {
            today += hours(now - *t);
        }
        (cur, self.on_break(), today)
    }

    /// Clock in on PROJECT, auto clocking out first if needed. New projects
    /// get EXPORT_CODE (or their own name). Returns true if it auto clocked out.
    pub fn clock_in(&self, project: &str, task: &str, export_code: Option<&str>) -> Result<bool, String> {
        let project = project.trim();
        self.ensure_project(project, export_code)?;
        let switched = if self.current().is_some() {
            let note = if project.is_empty() { String::new() } else { format!("Automatically switched to {project}") };
            self.clock_out(&note)?
        } else {
            false
        };
        self.append(&Event::In { t: now(), project: project.into(), task: task.trim().into() })?;
        Ok(switched)
    }

    /// Add PROJECT if new, with EXPORT_CODE (or its own name).
    fn ensure_project(&self, project: &str, export_code: Option<&str>) -> Result<(), String> {
        let mut projects = self.load_projects()?;
        if !project.is_empty() && !projects.contains_key(project) {
            let code = export_code.map(str::trim).filter(|c| !c.is_empty()).unwrap_or(project);
            projects.insert(project.into(), Project::new(code));
            self.save_projects(&projects)?;
        }
        Ok(())
    }

    /// Log a finished session START–END after the fact. The `in`/`out` pair goes in at its
    /// chronological spot, since `sessions` pairs events in file order. Refuses overlaps with
    /// logged or running time, and spots inside a pause (the paused time would move into it).
    pub fn add_session(&self, start: NaiveDateTime, end: NaiveDateTime, project: &str, export_code: Option<&str>, note: &str) -> Result<(), String> {
        let now = now();
        if start >= end || end > now {
            return Err("A session must end after it starts, and not in the future.".into());
        }
        let text = self.try_read_log()?;
        let events = parse_events(&text);
        if spans(&events, now).iter().any(|&(s, e)| s < end && start < e) {
            return Err("That overlaps time already logged.".into());
        }
        let before = events.iter().rev().find(|(_, e)| e.t() <= start);
        if matches!(before, Some((_, Event::Break { .. }))) {
            return Err("That falls in a paused session; resume or clock it out first.".into());
        }
        let (project, note) = (project.trim(), note.trim());
        self.ensure_project(project, export_code)?;
        self.check_diary(note)?;
        let pos = before.map_or(0, |(i, _)| i + 1);
        let mut lines: Vec<String> = text.lines().map(String::from).collect();
        let pair = [Event::In { t: start, project: project.into(), task: String::new() }, Event::Out { t: end, note: note.into() }];
        lines.splice(pos..pos, pair.iter().map(|e| serde_json::to_string(e).unwrap()));
        fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
        crate::config::write_atomic(&self.log_path(), lines.join("\n") + "\n").map_err(|e| e.to_string())?;
        append_diary(&self.diary, project, note, hours(end - start), end).map_err(|e| e.to_string())
    }

    /// Returns false if not clocked in.
    pub fn clock_out(&self, note: &str) -> Result<bool, String> {
        self.clock_out_at(note, now())
    }

    fn clock_out_at(&self, note: &str, t: NaiveDateTime) -> Result<bool, String> {
        let Some((start, project, _)) = self.current() else { return Ok(false) };
        let note = note.trim();
        self.check_diary(note)?;
        self.append(&Event::Out { t, note: note.into() })?;
        append_diary(&self.diary, &project, note, hours(t - start), t).map_err(|e| e.to_string())?;
        Ok(true)
    }

    /// Fail before logging if the diary NOTE goes to can't be read, so a retry doesn't double-log.
    fn check_diary(&self, note: &str) -> Result<(), String> {
        if !note.is_empty() {
            read_or_empty(&self.diary)?;
        }
        Ok(())
    }

    /// Drop time spent away: clock out at SINCE (not before the session start), then
    /// back in on the same project and task at BACK, or stay out if None.
    pub fn discard_idle(&self, since: NaiveDateTime, back: Option<NaiveDateTime>, note: &str) -> Result<bool, String> {
        let Some((start, project, task)) = self.current() else { return Ok(false) };
        self.clock_out_at(note, since.max(start))?;
        if let Some(t) = back {
            self.append(&Event::In { t, project, task })?;
        }
        Ok(true)
    }

    pub fn take_break(&self) -> Result<Option<String>, String> {
        let Some((_, project, _)) = self.current() else { return Ok(None) };
        self.append(&Event::Break { t: now() })?;
        Ok(Some(project))
    }

    pub fn resume(&self) -> Result<Option<String>, String> {
        let Some(project) = self.on_break() else { return Ok(None) };
        let task = self.events().iter().rev().find_map(|(_, e)| match e {
            Event::In { task, .. } => Some(task.clone()),
            _ => None,
        });
        self.append(&Event::In { t: now(), project: project.clone(), task: task.unwrap_or_default() })?;
        Ok(Some(project))
    }

    /// Move the current session's start back by MINUTES.
    pub fn adjust_start(&self, minutes: i64) -> Result<bool, String> {
        let Some((idx, Event::In { t, project, task })) = self.events().pop() else { return Ok(false) };
        self.rewrite_line(idx, &Event::In { t: t - Duration::minutes(minutes), project, task })?;
        Ok(true)
    }

    /// Change a completed session's description and duration. The end time
    /// moves by the duration delta, which stays correct across breaks and midnight.
    pub fn edit_session(&self, line: usize, note: &str, old_hours: f64, new_hours: f64) -> Result<(), String> {
        let ev = self.events().into_iter().find(|(i, _)| *i == line).map(|(_, e)| e);
        let Some(Event::Out { t, .. }) = ev else { return Err("not a clock-out line".into()) };
        let t = t + Duration::seconds(((new_hours - old_hours) * 3600.0).round() as i64);
        self.rewrite_line(line, &Event::Out { t, note: note.trim().into() })
    }

    /// One-time import of the Emacs timelog-<profile> + timeclock-projects-<profile>.eld.
    pub fn import_emacs(&self, old_dir: &Path, profile: &str) -> Result<usize, String> {
        if !self.try_read_log()?.trim().is_empty() {
            return Err("timelog already has data; import only works on an empty profile".into());
        }
        let p = profile.to_lowercase();
        let log = fs::read_to_string(old_dir.join(format!("timelog-{p}"))).map_err(|e| format!("timelog-{p}: {e}"))?;
        let events = import_timelog(&log);
        for ev in &events {
            self.append(ev)?;
        }
        if let Ok(eld) = fs::read_to_string(old_dir.join(format!("timeclock-projects-{p}.eld"))) {
            let mut projects = self.load_projects()?;
            projects.extend(import_projects(&eld));
            self.save_projects(&projects)?;
        }
        Ok(events.len())
    }
}

impl Event {
    pub fn t(&self) -> NaiveDateTime {
        match self {
            Event::In { t, .. } | Event::Out { t, .. } | Event::Break { t } => *t,
        }
    }
}

/// Worked intervals: `in` until `break`/`out`, an open one until NOW.
pub fn spans(events: &[(usize, Event)], now: NaiveDateTime) -> Vec<(NaiveDateTime, NaiveDateTime)> {
    let (mut start, mut out) = (None, vec![]);
    for (_, ev) in events {
        match ev {
            Event::In { t, .. } => start = Some(*t),
            Event::Break { t } | Event::Out { t, .. } => out.extend(start.take().map(|s| (s, *t))),
        }
    }
    out.extend(start.map(|s| (s, now)));
    out
}

pub fn parse_events(text: &str) -> Vec<(usize, Event)> {
    text.lines().enumerate().filter_map(|(i, l)| serde_json::from_str(l).ok().map(|e| (i, e))).collect()
}

// ---------------------------------------------------------------- sessions

/// Port of `albin/timeclock--parse-sessions`: time before a break accumulates
/// into the session that eventually clocks out.
pub fn sessions(events: &[(usize, Event)], now: NaiveDateTime) -> Vec<Session> {
    let mut project = String::new();
    let mut start: Option<NaiveDateTime> = None;
    let mut last_start: Option<NaiveDateTime> = None;
    let mut acc = 0.0;
    let mut out = vec![];
    for (i, ev) in events {
        match ev {
            Event::In { t, project: p, .. } => {
                project = p.clone();
                start = Some(*t);
                last_start = Some(*t);
            }
            Event::Break { t } => {
                if let Some(s) = start.take() {
                    acc += hours(*t - s);
                }
            }
            Event::Out { t, note } => {
                if let Some(s) = start.take() {
                    acc += hours(*t - s);
                    if acc > 0.0 {
                        out.push(Session { date: s.date(), project: project.clone(), desc: note.clone(), hours: acc, start: s.time(), end: t.time(), line: Some(*i) });
                    }
                    acc = 0.0;
                }
            }
        }
    }
    if let (true, Some(s)) = (acc > 0.0, last_start) {
        out.push(Session { date: s.date(), project, desc: ONGOING.into(), hours: acc, start: s.time(), end: now.time(), line: None });
    }
    out
}

fn is_blank_desc(d: &str) -> bool {
    d.trim().is_empty() || d == ONGOING
}

/// Port of `albin/merge-empty-sessions`: undescribed sessions fold into the
/// next described session of the same day and project.
pub fn merge_empty_sessions(sessions: &[Session]) -> Vec<Session> {
    let mut merged: Vec<Session> = vec![];
    let mut active: HashMap<(NaiveDate, String), usize> = HashMap::new();
    for s in sessions.iter().rev() {
        let key = (s.date, s.project.clone());
        if is_blank_desc(&s.desc) {
            match active.get(&key) {
                Some(&i) => merged[i].hours += s.hours,
                None => merged.push(s.clone()),
            }
        } else {
            merged.push(s.clone());
            active.insert(key, merged.len() - 1);
        }
    }
    merged.reverse();
    merged
}

/// Port of `albin/merge-sessions-by-description`.
pub fn merge_sessions_by_description(sessions: &[Session]) -> Vec<Session> {
    let mut merged: Vec<Session> = vec![];
    let mut by_key: HashMap<(NaiveDate, String, String), usize> = HashMap::new();
    for s in sessions {
        let desc = s.desc.trim().to_string();
        let key = (s.date, s.project.clone(), desc.clone());
        match by_key.get(&key) {
            Some(&i) if !is_blank_desc(&desc) => {
                merged[i].hours += s.hours;
                merged[i].end = s.end;
            }
            _ => {
                merged.push(s.clone());
                if !is_blank_desc(&desc) {
                    by_key.insert(key, merged.len() - 1);
                }
            }
        }
    }
    merged
}

pub fn prepare_report_sessions(sessions: &[Session]) -> Vec<Session> {
    merge_sessions_by_description(&merge_empty_sessions(sessions))
}

pub fn round_hours(h: f64, resolution: Option<f64>, up: bool) -> f64 {
    match resolution {
        Some(r) if r > 0.0 => {
            let f = 1.0 / r;
            // Emacs `round' rounds half to even.
            if up { (h * f).ceil() / f } else { (h * f).round_ties_even() / f }
        }
        _ => h,
    }
}

/// Port of `albin/apply-time-carry`: round per project, carrying the remainder forward.
pub fn apply_carry(sessions: &[Session], projects: &Projects) -> (Vec<Session>, f64) {
    let mut carry = 0.0;
    let out = sessions
        .iter()
        .map(|s| {
            let (res, up) = projects.get(&s.project).map_or((Some(0.5), false), |p| (p.rounding, p.round_up));
            let exact = s.hours + carry;
            let rounded = round_hours(exact, res, up);
            carry = exact - rounded;
            Session { hours: rounded, ..s.clone() }
        })
        .collect();
    (out, carry)
}

pub fn export_code(project: &str, projects: &Projects) -> String {
    match projects.get(project).map(|p| p.export_code.trim()) {
        Some(c) if !c.is_empty() => c.into(),
        _ if project.trim().is_empty() => "Other".into(),
        _ => project.into(),
    }
}

/// Recent unique task descriptions, today's first (`albin/timeclock-task-suggestions`).
pub fn task_suggestions(sessions: &[Session], project: Option<&str>, today: NaiveDate) -> Vec<String> {
    let mut seen = HashSet::new();
    let (mut today_s, mut older) = (vec![], vec![]);
    for s in sessions.iter().rev() {
        let d = s.desc.trim();
        if is_blank_desc(d) || project.is_some_and(|p| p != s.project) || !seen.insert(d.to_string()) {
            continue;
        }
        if s.date == today { today_s.push(d.to_string()) } else { older.push(d.to_string()) }
    }
    today_s.extend(older);
    today_s
}

// ---------------------------------------------------------------- holidays & flex

pub fn easter(year: i32) -> NaiveDate {
    let a = year % 19;
    let (b, c) = (year / 100, year % 100);
    let (d, e) = (b / 4, b % 4);
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15).rem_euclid(30);
    let (i, k) = (c / 4, c % 4);
    let l = (32 + 2 * e + 2 * i - h - k).rem_euclid(7);
    let m = (a + 11 * h + 22 * l) / 451;
    let month = (h + l - 7 * m + 114) / 31;
    let day = (h + l - 7 * m + 114) % 31 + 1;
    NaiveDate::from_ymd_opt(year, month as u32, day as u32).unwrap()
}

pub fn swedish_red_days(year: i32) -> Vec<(NaiveDate, &'static str)> {
    let d = |m, day| NaiveDate::from_ymd_opt(year, m, day).unwrap();
    let e = easter(year);
    let days = |n| Duration::days(n);
    // Friday between June 19-25 / Saturday between Oct 31 - Nov 6.
    let midsummer = d(6, 19) + days((12 - d(6, 19).weekday().num_days_from_sunday() as i64) % 7);
    let all_saints = d(10, 31) + days((13 - d(10, 31).weekday().num_days_from_sunday() as i64) % 7);
    let mut v = vec![
        (d(1, 1), "New Year's Day"),
        (d(1, 6), "Epiphany"),
        (e - days(2), "Good Friday"),
        (e, "Easter Sunday"),
        (e + days(1), "Easter Monday"),
        (d(5, 1), "May 1st (Labour Day)"),
        (e + days(39), "Ascension Day"),
        (e + days(49), "Pentecost"),
        (d(6, 6), "National Day"),
        (midsummer, "Midsummer Eve"),
        (midsummer + days(1), "Midsummer Day"),
        (all_saints, "All Saints' Day"),
        (d(12, 24), "Christmas Eve"),
        (d(12, 25), "Christmas Day"),
        (d(12, 26), "Boxing Day"),
        (d(12, 31), "New Year's Eve"),
    ];
    v.sort();
    v
}

pub fn expected_hours(date: NaiveDate, daily: f64) -> f64 {
    let weekend = matches!(date.weekday(), Weekday::Sat | Weekday::Sun);
    if weekend || swedish_red_days(date.year()).iter().any(|(d, _)| *d == date) { 0.0 } else { daily }
}

/// Port of `albin/calculate-flex`: (total, period, period_days). Only days with work count.
pub fn flex(sessions: &[Session], projects: &Projects, daily: f64, range: Option<(NaiveDate, NaiveDate)>) -> (f64, f64, usize) {
    let mut per_day: BTreeMap<NaiveDate, f64> = BTreeMap::new();
    for s in apply_carry(sessions, projects).0 {
        *per_day.entry(s.date).or_default() += s.hours;
    }
    let (mut total, mut period, mut n) = (0.0, 0.0, 0);
    for (date, h) in per_day {
        let f = h - expected_hours(date, daily);
        total += f;
        if range.is_some_and(|(a, b)| a <= date && date <= b) {
            period += f;
            n += 1;
        }
    }
    (total, period, n)
}

// ---------------------------------------------------------------- formatting & reports

pub fn format_hm(h: f64) -> String {
    let whole = h.trunc() as i64;
    let m = ((h - h.trunc()) * 60.0).trunc() as i64;
    if whole > 0 { format!("{whole}h {m:02}m") } else { format!("{m}m") }
}

fn or_other(p: &str) -> &str {
    if p.trim().is_empty() { "Other" } else { p }
}

pub fn org_table(rows: &[Vec<String>]) -> String {
    let n = rows.iter().map(Vec::len).max().unwrap_or(0);
    let w: Vec<usize> = (0..n).map(|i| rows.iter().map(|r| r.get(i).map_or(0, |c| c.chars().count())).max().unwrap_or(0)).collect();
    let line = |r: &Vec<String>| {
        let cells: Vec<String> = (0..n).map(|i| format!("{:<1$}", r.get(i).map_or("", |c| c), w[i])).collect();
        format!("| {} |\n", cells.join(" | "))
    };
    let mut out = line(&rows[0]);
    out += &format!("|{}|\n", w.iter().map(|w| "-".repeat(w + 2)).collect::<Vec<_>>().join("+"));
    rows[1..].iter().for_each(|r| out += &line(r));
    out
}

fn session_line(s: &Session, indent: &str) -> String {
    format!("{indent}- [{} - {}] *{}* ({:.2} h) » {}\n", s.start.format("%H:%M:%S"), s.end.format("%H:%M:%S"), or_other(&s.project), s.hours, s.desc)
}

pub fn daily_report(sessions: &[Session], projects: &Projects, date: NaiveDate, profile: &str) -> String {
    let merged = prepare_report_sessions(sessions);
    let rounded = apply_carry(&merged, projects).0;
    let raw: f64 = merged.iter().filter(|s| s.date == date).map(|s| s.hours).sum();
    let day: Vec<&Session> = rounded.iter().filter(|s| s.date == date).collect();
    let billable: f64 = day.iter().map(|s| s.hours).sum();
    let mut o = format!("#+TITLE: Daily Time Report ({profile})\n#+SUBTITLE: {date}\n\n* Summary\n");
    o += &format!("  - Hours worked: {raw:.2} h\n  - Billable: {billable:.2} h\n  - Sessions: {}\n\n* Detailed Log\n", day.len());
    if day.is_empty() {
        o += "  - No sessions found for this date.\n";
    }
    day.iter().for_each(|s| o += &session_line(s, "  "));
    o
}

pub fn weekly_report(sessions: &[Session], projects: &Projects, start: NaiveDate, end: NaiveDate, profile: &str) -> String {
    let in_range = |s: &&Session| start <= s.date && s.date <= end;
    let merged = prepare_report_sessions(sessions);
    let rounded = apply_carry(&merged, projects).0;
    let mut raw_by: HashMap<&str, f64> = HashMap::new();
    let mut rnd_by: HashMap<&str, f64> = HashMap::new();
    let mut by_day: BTreeMap<NaiveDate, Vec<&Session>> = BTreeMap::new();
    for s in merged.iter().filter(in_range) {
        *raw_by.entry(&s.project).or_default() += s.hours;
        by_day.entry(s.date).or_default().push(s);
    }
    for s in rounded.iter().filter(in_range) {
        *rnd_by.entry(&s.project).or_default() += s.hours;
    }
    let total_raw: f64 = raw_by.values().sum();
    let total_rnd: f64 = rnd_by.values().sum();
    let count: usize = by_day.values().map(Vec::len).sum();
    let mut o = format!("#+TITLE: Time Report ({profile})\n#+SUBTITLE: {start} to {end}\n\n* Summary\n");
    o += &format!("  - Hours worked: {total_raw:.2} h\n  - Billable: {total_rnd:.2} h\n  - Sessions: {count}\n\n* Project Breakdown\n");
    let mut rows = vec![["Project", "Code", "Worked", "Billable", "Share"].map(String::from).to_vec()];
    let mut projs: Vec<(&str, f64)> = rnd_by.into_iter().collect();
    projs.sort_by(|a, b| b.1.total_cmp(&a.1));
    for (p, rnd) in projs {
        let share = if total_rnd > 0.0 { (rnd / total_rnd * 100.0) as i64 } else { 0 };
        rows.push(vec![or_other(p).into(), export_code(p, projects), format!("{:.2} h", raw_by.get(p).unwrap_or(&0.0)), format!("{rnd:.2} h"), format!("{share}%")]);
    }
    o += &org_table(&rows);
    o += "\n* Detailed Log\n";
    for (date, ss) in by_day {
        o += &format!("** {date} ({:.2} h)\n", ss.iter().map(|s| s.hours).sum::<f64>());
        ss.iter().for_each(|s| o += &session_line(s, "   "));
    }
    o
}

pub fn holidays_report(year: i32) -> String {
    let mut o = format!("#+TITLE: Swedish Public Holidays {year}\n\n");
    for (d, name) in swedish_red_days(year) {
        o += &format!("- {d}  {}  {name}\n", d.format("%a"));
    }
    o + "\n(These days contribute 0h expected time in the flex calculation)\n"
}

fn csv_field(v: &str) -> String {
    format!("\"{}\"", v.replace('"', "\"\""))
}

/// Import-template columns; `Project` is the export code, hours use a decimal comma (`1,5`).
pub fn csv(sessions: &[Session], projects: &Projects, start: NaiveDate, end: NaiveDate) -> String {
    let rounded = apply_carry(&prepare_report_sessions(sessions), projects).0;
    let mut out = String::from("Project,Description,Date,Duration\n");
    for s in rounded.iter().filter(|s| start <= s.date && s.date <= end && (s.hours * 100.0).round() > 0.0) {
        let f = [export_code(&s.project, projects), s.desc.clone(), s.date.to_string(), format!("{:.2}", s.hours).trim_end_matches('0').trim_end_matches('.').replace('.', ",")];
        out += &(f.iter().map(|x| csv_field(x)).collect::<Vec<_>>().join(",") + "\n");
    }
    out
}

/// Port of `albin/timeclock--find-log-issues`.
pub fn doctor(text: &str, now: NaiveDateTime) -> Vec<String> {
    let mut issues = vec![];
    let mut open: Option<NaiveDateTime> = None;
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        if line.trim().is_empty() {
            continue;
        }
        let ev: Event = match serde_json::from_str(line) {
            Ok(e) => e,
            Err(e) => {
                issues.push(format!("Line {n}: unrecognized entry ({e}): {line}"));
                continue;
            }
        };
        match ev {
            Event::In { t, .. } => {
                if let Some(o) = open {
                    issues.push(format!("Line {n}: clocked in again while session started {o} was still open (its time is dropped)"));
                }
                open = Some(t);
            }
            Event::Out { t, .. } | Event::Break { t } => match open.take() {
                None => issues.push(format!("Line {n}: clock-out with no matching clock-in")),
                Some(o) if t < o => issues.push(format!("Line {n}: session ends before it starts ({o} -> {t})")),
                _ => {}
            },
        }
    }
    if let Some(o) = open {
        let days = (now - o).num_seconds() as f64 / 86400.0;
        if days > 1.0 {
            issues.push(format!("Still clocked in since {o} ({days:.1} days ago) — did you forget to clock out?"));
        }
    }
    issues
}

/// PATH's text; missing is empty, any other read error is an error naming PATH.
pub fn read_or_empty(path: &Path) -> Result<String, String> {
    match fs::read_to_string(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        r => r.map_err(|e| format!("{}: {e}", path.display())),
    }
}

/// Port of `albin/append-to-diary`.
pub fn append_diary(path: &Path, project: &str, reason: &str, h: f64, now: NaiveDateTime) -> std::io::Result<()> {
    if reason.trim().is_empty() {
        return Ok(());
    }
    let heading = now.format("* %Y-%m-%d %A").to_string();
    let mut text = match fs::read_to_string(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        r => r?,
    };
    // End of that day's section (before the next heading and its blank lines), for past sessions.
    let (mut off, mut found, mut at) = (0, false, None);
    for l in text.split_inclusive('\n') {
        if found && l.starts_with("* ") {
            let t = text[..off].trim_end_matches(['\r', '\n']).len();
            at = Some(t + text[t..].find('\n').unwrap_or(0) + 1);
            break;
        }
        found |= l.trim_end() == heading;
        off += l.len();
    }
    if !found {
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        text += &format!("\n{heading}\n");
    } else if at.is_none() && !text.ends_with('\n') {
        text.push('\n');
    }
    let entry = format!("- [{}] *{}* ({}): {}\n", now.format("%H:%M"), or_other(project), format_hm(h), reason);
    text.insert_str(at.unwrap_or(text.len()), &entry);
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    crate::config::write_atomic(path, text)
}

// ---------------------------------------------------------------- emacs import

pub fn import_timelog(text: &str) -> Vec<Event> {
    let re = Regex::new(r"^([ioO]) (\d{4})/(\d{2})/(\d{2}) (\d{1,2}:\d{2}:\d{2})(?: (.*))?").unwrap();
    text.lines()
        .filter_map(|l| {
            let c = re.captures(l)?;
            let t = NaiveDateTime::parse_from_str(&format!("{}-{}-{} {}", &c[2], &c[3], &c[4], &c[5]), "%Y-%m-%d %H:%M:%S").ok()?;
            let text = c.get(6).map_or("", |m| m.as_str()).trim().to_string();
            Some(match &c[1] {
                "i" => Event::In { t, project: text, task: String::new() },
                _ if text.contains("---BREAK---") => Event::Break { t },
                _ => Event::Out { t, note: text },
            })
        })
        .collect()
}

/// Parse `(("Name" :export-code "X" :rounding 0.5 :round-up nil :active t) ("Old" . "CODE"))`.
pub fn import_projects(eld: &str) -> Projects {
    let entry = Regex::new(r#"\("((?:[^"\\]|\\.)*)"([^()]*)\)"#).unwrap();
    let old = Regex::new(r#"^\s*\.\s*"((?:[^"\\]|\\.)*)""#).unwrap();
    let field = |rest: &str, key: &str| -> Option<String> {
        Regex::new(&format!(r#":{key}\s+("(?:[^"\\]|\\.)*"|[^\s)]+)"#)).unwrap().captures(rest).map(|c| c[1].to_string())
    };
    let unquote = |s: String| s.trim_matches('"').replace("\\\"", "\"");
    entry
        .captures_iter(eld)
        .map(|c| {
            let name = unquote(c[1].to_string());
            let rest = &c[2];
            let p = match old.captures(rest) {
                Some(o) => Project::new(&unquote(o[1].to_string())),
                None => Project {
                    export_code: field(rest, "export-code").map(unquote).filter(|s| s != "nil").unwrap_or_default(),
                    rounding: field(rest, "rounding").and_then(|s| s.parse().ok()),
                    round_up: field(rest, "round-up").is_some_and(|s| s != "nil"),
                    active: field(rest, "active").is_some_and(|s| s != "nil"),
                },
            };
            (name, p)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dt(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap()
    }
    fn d(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    #[test]
    fn holidays() {
        assert_eq!(easter(2024), d("2024-03-31"));
        assert_eq!(easter(2025), d("2025-04-20"));
        assert_eq!(easter(2026), d("2026-04-05"));
        let r = swedish_red_days(2026);
        assert!(r.contains(&(d("2026-06-19"), "Midsummer Eve")));
        assert!(r.contains(&(d("2026-10-31"), "All Saints' Day")));
        assert!(swedish_red_days(2025).contains(&(d("2025-11-01"), "All Saints' Day")));
        assert_eq!(expected_hours(d("2026-04-06"), 8.0), 0.0); // Easter Monday
        assert_eq!(expected_hours(d("2026-04-07"), 8.0), 8.0);
    }

    #[test]
    fn rounding_and_carry() {
        assert_eq!(round_hours(1.25, Some(0.5), false), 1.0); // half-to-even like Emacs
        assert_eq!(round_hours(1.75, Some(0.5), false), 2.0);
        assert_eq!(round_hours(1.1, Some(0.5), true), 1.5);
        assert_eq!(round_hours(1.1, None, false), 1.1);
        let s = |h| Session { date: d("2026-10-01"), project: "A".into(), desc: "x".into(), hours: h, start: NaiveTime::MIN, end: NaiveTime::MIN, line: None };
        let (r, carry) = apply_carry(&[s(0.4), s(0.4), s(0.4)], &Projects::new());
        assert_eq!(r.iter().map(|s| s.hours).collect::<Vec<_>>(), vec![0.5, 0.5, 0.0]);
        assert!((carry - 0.2).abs() < 1e-9);
    }

    #[test]
    fn breaks_accumulate_and_flex() {
        let log = "i 2026/09/28 08:00:00 Acme\no 2026/09/28 12:00:00 ---BREAK---\ni 2026/09/28 13:00:00 Acme\no 2026/09/28 17:00:00 Did stuff\n\
                   i 2026/10/02 09:00:00 Acme\no 2026/10/02 10:00:00\ni 2026/10/02 10:00:00 Acme\no 2026/10/02 11:00:00 Review";
        let ev: Vec<_> = import_timelog(log).into_iter().enumerate().collect();
        assert_eq!(ev[1].1, Event::Break { t: dt("2026-09-28 12:00") });
        let s = sessions(&ev, dt("2026-10-02 12:00"));
        assert_eq!(s.len(), 3);
        assert_eq!((s[0].hours, s[0].desc.as_str(), s[0].start), (8.0, "Did stuff", NaiveTime::from_hms_opt(13, 0, 0).unwrap()));
        let merged = prepare_report_sessions(&s);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[1].hours, 2.0); // empty-desc session folded into "Review"
        // Mon 8h on an 8h day => 0, Fri 2h => -6
        assert_eq!(flex(&s, &Projects::new(), 8.0, None).0, -6.0);
        assert_eq!(task_suggestions(&s, Some("Acme"), d("2026-10-02")), vec!["Review", "Did stuff"]);
    }

    #[test]
    fn eld_import() {
        let p = import_projects(r#"(("Acme" :export-code "AC-1" :rounding 0.25 :round-up t :active t) ("Old" . "OLD-9") ("Raw" :export-code "R" :rounding nil :round-up nil :active nil))"#);
        assert_eq!(p["Acme"], Project { export_code: "AC-1".into(), rounding: Some(0.25), round_up: true, active: true });
        assert_eq!(p["Old"], Project::new("OLD-9"));
        assert_eq!(p["Raw"].rounding, None);
        assert!(!p["Raw"].active);
    }

    #[test]
    fn doctor_finds_issues() {
        let log = "{\"ev\":\"in\",\"t\":\"2026-10-01T09:00:00\",\"project\":\"A\"}\n{\"ev\":\"in\",\"t\":\"2026-10-01T10:00:00\",\"project\":\"A\"}\ngarbage\n{\"ev\":\"out\",\"t\":\"2026-10-01T08:00:00\"}\n{\"ev\":\"out\",\"t\":\"2026-10-01T11:00:00\"}";
        let i = doctor(log, dt("2026-10-01 12:00"));
        assert_eq!(i.len(), 4, "{i:?}");
    }

    #[test]
    fn clock_flow_on_disk() {
        let dir = std::env::temp_dir().join(format!("tc-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let tc = Tc { dir: dir.join("work"), diary: dir.join("notes/dagbok-work.org"), expected: 8.0 };
        assert!(!tc.clock_in("Acme", "Review", Some("AC-1")).unwrap());
        assert_eq!(tc.projects()["Acme"].export_code, "AC-1");
        assert_eq!(tc.take_break().unwrap().as_deref(), Some("Acme"));
        assert_eq!(tc.on_break().as_deref(), Some("Acme"));
        assert_eq!(tc.resume().unwrap().as_deref(), Some("Acme"));
        assert_eq!(tc.current().unwrap().2, "Review"); // task carried over the break
        assert!(tc.adjust_start(30).unwrap());
        assert!(tc.clock_in("Other", "", None).unwrap()); // auto clock-out
        assert!(tc.clock_out("done").unwrap());
        assert!(!tc.clock_out("again").unwrap());
        let diary = fs::read_to_string(&tc.diary).unwrap();
        assert!(diary.contains("*Acme* (30m): Automatically switched to Other"), "{diary}");
        assert!(diary.contains("*Other* (0m): done"));
        assert!(doctor(&tc.read_log(), now()).is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn malformed_projects_are_reported() {
        let dir = std::env::temp_dir().join(format!("tc-proj-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let tc = Tc { dir: dir.join("work"), diary: dir.join("dagbok.org"), expected: 8.0 };
        assert!(tc.file_issues().is_empty()); // missing files are fine
        fs::create_dir_all(&tc.dir).unwrap();
        fs::write(tc.projects_path(), "[Acme\nexport_code = ").unwrap();
        assert!(tc.load_projects().is_err());
        assert!(tc.projects().is_empty()); // display stays lenient
        let i = tc.file_issues();
        assert!(i.len() == 1 && i[0].contains("projects.toml"), "{i:?}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn discard_idle_sessions() {
        let dir = std::env::temp_dir().join(format!("tc-idle-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let tc = Tc { dir: dir.join("work"), diary: dir.join("dagbok.org"), expected: 8.0 };
        let start = |tc: &Tc| {
            let _ = fs::remove_file(tc.log_path());
            tc.append(&Event::In { t: dt("2026-10-01 09:00"), project: "Acme".into(), task: "Review".into() }).unwrap();
        };
        let at = |tc: &Tc| sessions(&tc.events(), dt("2026-10-01 12:00")).iter().map(|s| (s.start.to_string(), s.end.to_string(), s.desc.clone())).collect::<Vec<_>>();
        let s = |a: &str, b: &str, d: &str| (format!("{a}:00"), format!("{b}:00"), d.to_string());
        // Keep: nothing changes (the open session isn't in `sessions` yet).
        start(&tc);
        assert!(at(&tc).is_empty());
        // Discard: out at the idle start, back in on the same project and task on return.
        assert!(tc.discard_idle(dt("2026-10-01 10:00"), Some(dt("2026-10-01 10:30")), "").unwrap());
        assert_eq!(at(&tc), vec![s("09:00", "10:00", "")]);
        assert_eq!(tc.current().unwrap(), (dt("2026-10-01 10:30"), "Acme".into(), "Review".into()));
        // Discard and stop: out at the idle start, not before the session began.
        start(&tc);
        assert!(tc.discard_idle(dt("2026-10-01 08:00"), None, "done").unwrap());
        assert!(at(&tc).is_empty()); // zero-length session
        start(&tc);
        assert!(tc.discard_idle(dt("2026-10-01 10:00"), None, "done").unwrap());
        assert_eq!(at(&tc), vec![s("09:00", "10:00", "done")]);
        assert!(tc.current().is_none());
        assert!(fs::read_to_string(&tc.diary).unwrap().contains("[10:00] *Acme* (1h 00m): done"));
        assert!(!tc.discard_idle(dt("2026-10-01 10:00"), None, "").unwrap());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn add_past_sessions() {
        let dir = std::env::temp_dir().join(format!("tc-add-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let tc = Tc { dir: dir.join("work"), diary: dir.join("dagbok.org"), expected: 8.0 };
        let log = [
            Event::In { t: dt("2024-10-01 08:00"), project: "A".into(), task: String::new() },
            Event::Out { t: dt("2024-10-01 09:00"), note: "a".into() },
            Event::In { t: dt("2024-10-01 12:00"), project: "A".into(), task: String::new() },
            Event::Break { t: dt("2024-10-01 13:00") },
            Event::In { t: dt("2024-10-01 14:00"), project: "A".into(), task: String::new() },
            Event::Out { t: dt("2024-10-01 15:00"), note: "b".into() },
            Event::In { t: dt("2024-10-02 08:00"), project: "B".into(), task: String::new() },
        ];
        log.iter().for_each(|e| tc.append(e).unwrap());
        fs::write(&tc.diary, "* 2024-10-01 Tuesday\n- old\n\n* 2024-10-02 Wednesday\n- today\n").unwrap();
        let before = sessions(&tc.events(), dt("2024-10-02 09:00"));
        tc.add_session(dt("2024-10-01 10:00"), dt("2024-10-01 11:30"), "New", Some("N-1"), "planning").unwrap();
        let after = sessions(&tc.events(), dt("2024-10-02 09:00"));
        assert_eq!(after.len(), before.len() + 1);
        assert_eq!((after[1].project.as_str(), after[1].desc.as_str(), after[1].hours), ("New", "planning", 1.5));
        assert_eq!(after[2], Session { line: Some(7), ..before[1].clone() }); // the break session is untouched
        assert_eq!(tc.projects()["New"].export_code, "N-1");
        assert_eq!(fs::read_to_string(&tc.diary).unwrap(), "* 2024-10-01 Tuesday\n- old\n- [11:30] *New* (1h 30m): planning\n\n* 2024-10-02 Wednesday\n- today\n");
        let err = |a, b| tc.add_session(dt(a), dt(b), "X", None, "").unwrap_err();
        assert!(err("2024-10-01 08:30", "2024-10-01 08:45").contains("overlaps"));
        assert!(err("2024-10-01 11:00", "2024-10-01 12:00").contains("overlaps"));
        assert!(err("2024-10-01 13:15", "2024-10-01 13:45").contains("paused"));
        assert!(err("2024-10-02 07:00", "2024-10-02 08:01").contains("overlaps")); // the running session
        assert!(err("2024-10-01 11:00", "2024-10-01 10:00").contains("end after"));
        // Back to back is fine, and the running session stays last.
        tc.add_session(dt("2024-10-02 07:00"), dt("2024-10-02 08:00"), "A", None, "").unwrap();
        assert_eq!(tc.current().unwrap().1, "B");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn unreadable_files_are_not_overwritten() {
        let dir = std::env::temp_dir().join(format!("tc-bad-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let tc = Tc { dir: dir.join("work"), diary: dir.join("dagbok.org"), expected: 8.0 };
        fs::create_dir_all(&tc.dir).unwrap();
        // Malformed projects.toml: clock_in on a new project errors and leaves it alone.
        fs::write(tc.projects_path(), "[Acme\nexport_code = 1").unwrap();
        assert!(tc.clock_in("New", "", None).unwrap_err().contains("projects.toml"));
        assert_eq!(fs::read_to_string(tc.projects_path()).unwrap(), "[Acme\nexport_code = 1");
        assert!(tc.events().is_empty());
        fs::remove_file(tc.projects_path()).unwrap();
        // Invalid UTF-8 in the log: add_session errors and leaves it alone.
        let bad = b"{\"ev\":\"in\",\"t\":\"2024-10-01T08:00:00\",\"project\":\"A\"}\n\xff\n".to_vec();
        fs::write(tc.log_path(), &bad).unwrap();
        assert!(tc.add_session(dt("2024-10-02 08:00"), dt("2024-10-02 09:00"), "A", None, "x").is_err());
        assert_eq!(fs::read(tc.log_path()).unwrap(), bad);
        fs::remove_file(tc.log_path()).unwrap();
        // Unreadable diary: nothing gets logged, so a retry doesn't overlap.
        fs::write(&tc.diary, b"\xff").unwrap();
        assert!(tc.add_session(dt("2024-10-02 08:00"), dt("2024-10-02 09:00"), "A", None, "x").is_err());
        assert!(!tc.log_path().exists());
        tc.clock_in("A", "", None).unwrap();
        assert!(tc.clock_out("note").is_err());
        assert!(tc.current().is_some());
        assert_eq!(fs::read(&tc.diary).unwrap(), b"\xff");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn csv_quotes_and_comma() {
        let s = vec![Session { date: d("2026-10-01"), project: "A".into(), desc: "say \"hi\"".into(), hours: 1.5, start: NaiveTime::MIN, end: NaiveTime::MIN, line: None }];
        let out = csv(&s, &Projects::new(), d("2026-10-01"), d("2026-10-01"));
        assert_eq!(out.lines().nth(1).unwrap(), r#""A","say ""hi""","2026-10-01","1,5""#);
        let s = vec![Session { hours: 2.0, ..s[0].clone() }];
        assert!(csv(&s, &Projects::new(), d("2026-10-01"), d("2026-10-01")).ends_with(",\"2\"\n"));
    }

    #[test]
    fn csv_skips_zero_rows() {
        let s = |h| vec![Session { date: d("2026-10-01"), project: "A".into(), desc: "x".into(), hours: h, start: NaiveTime::MIN, end: NaiveTime::MIN, line: None }];
        let rows = |h| csv(&s(h), &Projects::new(), d("2026-10-01"), d("2026-10-01")).lines().count();
        assert_eq!((rows(0.0), rows(0.2), rows(0.3)), (1, 1, 2)); // default rounding is 0.5 h
    }
}
