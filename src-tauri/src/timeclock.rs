// Port of albin-timeclock-*.el. Data lives per profile in
// <data_dir>/<profile>/{timelog.jsonl,projects.toml}; the diary is an org
// file in the notes dir so it's searchable and linkable like any note.

use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
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
    /// Hours this project should get per `budget_period` ("week", else month); None for no budget.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget_hours: Option<f64>,
    #[serde(default = "month", skip_serializing_if = "is_month")]
    pub budget_period: String,
}
fn yes() -> bool {
    true
}
fn month() -> String {
    "month".into()
}
fn is_month(s: &str) -> bool {
    s == "month"
}

impl Project {
    pub fn new(export_code: &str) -> Self {
        Project { export_code: export_code.into(), rounding: Some(0.5), round_up: false, active: true, budget_hours: None, budget_period: month() }
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

/// Time off FROM..=TO (one line of leave.jsonl). It lowers each day's expected hours by
/// HOURS (None: the whole day), or with REST by whatever is left of the day after work.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Leave {
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hours: Option<f64>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub rest: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
}

/// Export settings of one kind of time off, in leave.toml keyed by the lowercased kind.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct LeaveKind {
    #[serde(default)]
    pub export_code: String,
}

pub type LeaveCodes = BTreeMap<String, LeaveKind>;

pub fn now() -> NaiveDateTime {
    chrono::Local::now().naive_local().with_nanosecond(0).unwrap()
}

fn hours(d: Duration) -> f64 {
    d.num_seconds() as f64 / 3600.0
}

/// Hours worked on NOW's day: SESSIONS dated today plus the open clock since CUR, counted
/// from midnight if it started yesterday. A closed session crossing midnight stays dated by its start.
pub fn today_hours(sessions: &[Session], cur: Option<NaiveDateTime>, now: NaiveDateTime) -> f64 {
    let open = cur.map_or(0.0, |t| hours(now - t.max(now.date().and_time(NaiveTime::MIN))));
    sessions.iter().filter(|s| s.date == now.date()).map(|s| s.hours).sum::<f64>() + open
}

static TC_WRITES: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Run F holding the time-log write lock. The `tc_*` commands that write the log use it, so an
/// append can't land between a rewrite's read and write (`adjust_start`, `edit_session`,
/// `add_session`) and be lost. Taken in the commands, not in `Tc`, since `clock_in` calls `clock_out`.
pub fn locked<T>(f: impl FnOnce() -> T) -> T {
    let _g = TC_WRITES.lock().unwrap_or_else(|e| e.into_inner());
    f()
}

type Stamp = (std::time::SystemTime, u64);
type Cached = HashMap<PathBuf, (Stamp, Vec<(usize, Event)>)>;
/// Parsed events per log path, keyed on the file's (mtime, len). `Tc` is rebuilt per command, so
/// it lives here. In-app writes also `forget`, for a rewrite that keeps both.
static EVENTS: std::sync::Mutex<Option<Cached>> = std::sync::Mutex::new(None);

fn events_cache<T>(f: impl FnOnce(&mut Cached) -> T) -> T {
    f(EVENTS.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashMap::new))
}

// ---------------------------------------------------------------- storage

pub struct Tc {
    pub dir: PathBuf,
    pub diary: PathBuf,
    pub cal: Calendar,
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
    /// Cached (see `EVENTS`): stat before reading, so a write racing the read only causes a miss.
    pub fn events(&self) -> Vec<(usize, Event)> {
        let path = self.log_path();
        let Some(stamp) = fs::metadata(&path).ok().and_then(|m| Some((m.modified().ok()?, m.len()))) else {
            return parse_events(&self.read_log());
        };
        if let Some(ev) = events_cache(|c| c.get(&path).filter(|(s, _)| *s == stamp).map(|(_, ev)| ev.clone())) {
            return ev;
        }
        let ev = parse_events(&self.read_log());
        events_cache(|c| c.insert(path, (stamp, ev.clone())));
        ev
    }

    /// Drop the cached events after writing the log.
    fn forget(&self) {
        events_cache(|c| c.remove(&self.log_path()));
    }

    fn append(&self, ev: &Event) -> Result<(), String> {
        fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
        let mut f = fs::OpenOptions::new().create(true).append(true).open(self.log_path()).map_err(|e| e.to_string())?;
        let r = writeln!(f, "{}", serde_json::to_string(ev).unwrap()).map_err(|e| e.to_string());
        self.forget();
        r
    }

    fn rewrite_line(&self, idx: usize, ev: &Event) -> Result<(), String> {
        self.rewrite_lines(&[(idx, ev.clone())])
    }

    /// Replace the log lines at the given indexes in one atomic write.
    fn rewrite_lines(&self, evs: &[(usize, Event)]) -> Result<(), String> {
        let text = self.try_read_log()?;
        let mut lines: Vec<String> = text.lines().map(String::from).collect();
        for (idx, ev) in evs {
            *lines.get_mut(*idx).ok_or("line not found")? = serde_json::to_string(ev).unwrap();
        }
        let r = crate::config::write_atomic(&self.log_path(), lines.join("\n") + "\n").map_err(|e| e.to_string());
        self.forget();
        r
    }

    /// Sessions clocked on task OLD per project (see `rename_task`), on CATEGORY when it is a project
    /// (as `spent_on` scopes it), renamed to NEW when APPLY. Hold `locked`.
    pub fn rename_task(&self, category: &str, old: &str, new: &str, apply: bool) -> Result<BTreeMap<String, usize>, String> {
        let c = norm(category);
        let scope = self.projects().keys().any(|p| norm(p) == c).then_some(c);
        let (lines, by) = rename_task(&self.events(), scope.as_deref(), old, new);
        if apply && !lines.is_empty() {
            self.rewrite_lines(&lines)?;
        }
        Ok(by)
    }

    /// Rewrite the log via `normalize_log` (after a pull); true if it changed. Hold `locked`.
    pub fn normalize_log(&self) -> Result<bool, String> {
        let r = rewrite_with(&self.log_path(), normalize_log);
        self.forget();
        r
    }

    /// Drop duplicate time off lines via `dedupe_lines` (after a pull); true if it changed.
    pub fn dedupe_leave(&self) -> Result<bool, String> {
        rewrite_with(&self.leave_path(), dedupe_lines)
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
        [self.load_projects().err(), self.leave_codes().err(), read_or_empty(&self.diary).err()].into_iter().flatten().collect()
    }

    pub fn save_projects(&self, p: &Projects) -> Result<(), String> {
        fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
        crate::config::write_atomic(&self.projects_path(), toml::to_string(p).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    pub fn sessions(&self) -> Vec<Session> {
        sessions(&self.events(), now())
    }

    pub fn leave_path(&self) -> PathBuf {
        self.dir.join("leave.jsonl")
    }

    /// Time off with its line index; malformed lines are skipped.
    pub fn leave(&self) -> Vec<(usize, Leave)> {
        let text = fs::read_to_string(self.leave_path()).unwrap_or_default();
        text.lines().enumerate().filter_map(|(i, l)| serde_json::from_str(l).ok().map(|e| (i, e))).collect()
    }

    /// Add time off; refuses bad ranges and hours, and overlaps with time off of the same kind.
    pub fn add_leave(&self, l: Leave) -> Result<(), String> {
        let l = Leave { kind: l.kind.trim().to_lowercase(), note: l.note.trim().into(), ..l };
        if l.from > l.to || l.kind.is_empty() {
            return Err("Time off needs a kind and must end on or after its first day.".into());
        }
        // The longest weekday in the range (red days aside) caps the hours; leave_on caps each day.
        let max = l.from.iter_days().take_while(|d| *d <= l.to).take(7).map(|d| self.cal.weekday(d)).fold(0.0, f64::max);
        if l.hours.is_some() && max <= 0.0 {
            return Err("Those days have no expected hours; leave the hours out.".into());
        }
        if l.hours.is_some_and(|h| !(h > 0.0 && h <= max)) || (l.rest && l.hours.is_some()) {
            return Err(format!("Hours per day must be more than 0 and at most {max}."));
        }
        let text = read_or_empty(&self.leave_path())?;
        let old: Vec<Leave> = text.lines().filter_map(|x| serde_json::from_str(x).ok()).collect();
        if old.iter().any(|o| o.kind == l.kind && o.from <= l.to && l.from <= o.to) {
            return Err(format!("That overlaps {} already logged.", l.kind));
        }
        let mut lines: Vec<String> = text.lines().map(String::from).collect();
        lines.push(serde_json::to_string(&l).unwrap());
        fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
        crate::config::write_atomic(&self.leave_path(), lines.join("\n") + "\n").map_err(|e| e.to_string())
    }

    /// Time off export codes (leave.toml): missing is empty, unreadable or malformed is an error.
    pub fn leave_codes(&self) -> Result<LeaveCodes, String> {
        let path = self.dir.join("leave.toml");
        toml::from_str(&read_or_empty(&path)?).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn save_leave_code(&self, kind: &str, code: &str) -> Result<(), String> {
        let mut c = self.leave_codes()?;
        c.insert(kind.trim().to_lowercase(), LeaveKind { export_code: code.trim().into() });
        fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
        crate::config::write_atomic(&self.dir.join("leave.toml"), toml::to_string(&c).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    pub fn remove_leave(&self, line: usize) -> Result<(), String> {
        let text = read_or_empty(&self.leave_path())?;
        let mut lines: Vec<&str> = text.lines().collect();
        if line >= lines.len() {
            return Err("line not found".into());
        }
        lines.remove(line);
        let out = if lines.is_empty() { String::new() } else { lines.join("\n") + "\n" };
        crate::config::write_atomic(&self.leave_path(), out).map_err(|e| e.to_string())
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
        let cur = self.current();
        let today = today_hours(&self.sessions(), cur.as_ref().map(|c| c.0), now);
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
        let r = crate::config::write_atomic(&self.log_path(), lines.join("\n") + "\n").map_err(|e| e.to_string());
        self.forget();
        r?;
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

    /// Clock out at AT (forgot to stop); refuses a time before the session started.
    pub fn clock_out_earlier(&self, at: NaiveDateTime, note: &str) -> Result<bool, String> {
        let Some((start, _, _)) = self.current() else { return Ok(false) };
        if at < start {
            return Err(format!("That is before this session started ({}).", start.format("%H:%M")));
        }
        self.clock_out_at(note, at)
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

    /// Move the current session's start back by MINUTES, but not before the previous session's end.
    pub fn adjust_start(&self, minutes: i64) -> Result<bool, String> {
        let mut ev = self.events();
        let Some((idx, Event::In { t, project, task })) = ev.pop() else { return Ok(false) };
        let t = t - Duration::minutes(minutes);
        if let Some(Event::Out { t: end, .. }) = ev.last().map(|e| &e.1) {
            if t < *end {
                return Err(format!("That start overlaps the previous session, which ended {}.", end.format("%H:%M")));
            }
        }
        self.rewrite_line(idx, &Event::In { t, project, task })?;
        Ok(true)
    }

    /// Change a completed session's description and duration. The end time
    /// moves by the duration delta, which stays correct across breaks and midnight.
    /// A moved end must stay after the previous event, not pass the next one or now.
    pub fn edit_session(&self, line: usize, note: &str, old_hours: f64, new_hours: f64) -> Result<(), String> {
        let events = self.events();
        let Some(pos) = events.iter().position(|(i, _)| *i == line) else { return Err("not a clock-out line".into()) };
        let Event::Out { t: old, .. } = &events[pos].1 else { return Err("not a clock-out line".into()) };
        let t = *old + Duration::seconds(((new_hours - old_hours) * 3600.0).round() as i64);
        if t != *old {
            if new_hours.is_nan() || new_hours <= 0.0 {
                return Err("A session must last longer than zero hours.".into());
            }
            if pos == 0 {
                return Err("This session has no start entry to measure from.".into());
            }
            let prev = events[pos - 1].1.t();
            if t <= prev {
                return Err(format!("This session started (or resumed after a break) at {}, so it can't end at or before that. Use Edit raw log (Space t E) to drop the break.", prev.format("%H:%M")));
            }
            if t > now() || events.get(pos + 1).is_some_and(|(_, e)| t > e.t()) {
                return Err("That would end in the future or overlap the next entry.".into());
            }
        }
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
    project_spans(events, now).into_iter().map(|(s, e, _)| (s, e)).collect()
}

/// `spans` with the project of the `in` that started each (a resume repeats it).
pub fn project_spans(events: &[(usize, Event)], now: NaiveDateTime) -> Vec<(NaiveDateTime, NaiveDateTime, &str)> {
    let (mut start, mut out) = (None, vec![]);
    for (_, ev) in events {
        match ev {
            Event::In { t, project, .. } => start = Some((*t, project.as_str())),
            Event::Break { t } | Event::Out { t, .. } => out.extend(start.take().map(|(s, p)| (s, *t, p))),
        }
    }
    out.extend(start.map(|(s, p)| (s, now, p)));
    out
}

/// Clocked time by weekday (Mon = 0) and hour of day over FROM..=TO (`rhythm`).
#[derive(Serialize, Debug, PartialEq)]
pub struct Rhythm {
    /// Average minutes per weekday-hour: the total over the times that weekday occurs in the range.
    pub minutes: [[f64; 24]; 7],
    /// The busiest cell (day, hour), None without clocked time.
    pub peak: Option<(usize, usize)>,
    /// Share of the clocked time from 17:00 on (0..1).
    pub evening: f64,
    pub hours: f64,
    /// Each span's part in each cell, for listing what's behind a cell.
    pub pieces: Vec<RhythmPiece>,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct RhythmPiece {
    pub day: usize,
    pub hour: usize,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub project: String,
    pub minutes: f64,
}

/// SPANS (`project_spans`) on PROJECT (all when None) folded into a `Rhythm` over FROM..=TO, split at
/// every hour boundary so midnight needs nothing special. Times are local wall clock without a zone,
/// so a DST day just counts the hours its clock shows.
pub fn rhythm(spans: &[(NaiveDateTime, NaiveDateTime, &str)], from: NaiveDate, to: NaiveDate, project: Option<&str>) -> Rhythm {
    let (lo, hi) = (from.and_time(NaiveTime::MIN), (to + Duration::days(1)).and_time(NaiveTime::MIN));
    let (mut total, mut pieces) = ([[0.0; 24]; 7], vec![]);
    for &(s, e, p) in spans.iter().filter(|x| project.is_none_or(|p| p == x.2)) {
        let (mut t, end) = (s.max(lo), e.min(hi));
        while t < end {
            let next = (t.date().and_hms_opt(t.hour(), 0, 0).unwrap() + Duration::hours(1)).min(end);
            let (day, hour, minutes) = (t.weekday().num_days_from_monday() as usize, t.hour() as usize, (next - t).num_seconds() as f64 / 60.0);
            total[day][hour] += minutes;
            pieces.push(RhythmPiece { day, hour, start: s, end: e, project: p.into(), minutes });
            t = next;
        }
    }
    let mut n = [0u32; 7];
    from.iter_days().take_while(|d| *d <= to).for_each(|d| n[d.weekday().num_days_from_monday() as usize] += 1);
    let minutes: [[f64; 24]; 7] = std::array::from_fn(|d| total[d].map(|m| if n[d] > 0 { m / n[d] as f64 } else { 0.0 }));
    let all: f64 = total.iter().flatten().sum();
    let late: f64 = total.iter().map(|r| r[17..].iter().sum::<f64>()).sum();
    let peak = (0..7).flat_map(|d| (0..24).map(move |h| (d, h))).filter(|&(d, h)| minutes[d][h] > 0.0).max_by(|a, b| minutes[a.0][a.1].total_cmp(&minutes[b.0][b.1]).then(b.cmp(a)));
    Rhythm { minutes, peak, evening: if all > 0.0 { late / all } else { 0.0 }, hours: all / 60.0, pieces }
}

pub fn parse_events(text: &str) -> Vec<(usize, Event)> {
    text.lines().enumerate().filter_map(|(i, l)| serde_json::from_str(l).ok().map(|e| (i, e))).collect()
}

/// The log in event-time order without blank or exact duplicate lines: after a `merge=union`
/// pull two machines' appends sit one after the other, but `sessions` pairs in file order.
/// The sort is stable (same-time events keep their order); an unparsable line keeps the time before it.
pub fn normalize_log(text: &str) -> String {
    let (text, mut last) = (dedupe_lines(text), NaiveDateTime::MIN);
    let mut lines: Vec<(NaiveDateTime, &str)> = text
        .lines()
        .map(|l| {
            if let Ok(ev) = serde_json::from_str::<Event>(l) {
                last = ev.t();
            }
            (last, l)
        })
        .collect();
    lines.sort_by_key(|x| x.0);
    lines.iter().map(|(_, l)| format!("{l}\n")).collect()
}

/// TEXT without blank or exact duplicate lines (first copy kept): a union merge of the same
/// append on two machines (`leave.jsonl`) keeps both copies.
pub fn dedupe_lines(text: &str) -> String {
    let mut seen = HashSet::new();
    text.lines().filter(|l| !l.trim().is_empty() && seen.insert(*l)).map(|l| format!("{l}\n")).collect()
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

/// KIND's export code, or the kind itself when it has none.
pub fn leave_code(kind: &str, codes: &LeaveCodes) -> String {
    match codes.get(kind).map(|c| c.export_code.trim()) {
        Some(c) if !c.is_empty() => c.into(),
        _ => kind.into(),
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

/// Clocked minutes per task, then per project (both trimmed and lowercased; see spent_by_task).
pub type Spent = HashMap<String, HashMap<String, u32>>;

fn norm(s: &str) -> String {
    s.trim().to_lowercase()
}

/// All time logged in EVENTS by task: a session's task is its clock-out note, else the task it was
/// clocked in on. Breaks don't count; the open clock counts up to NOW. Untitled sessions are left out.
pub fn spent_by_task(events: &[(usize, Event)], now: NaiveDateTime) -> Spent {
    let mut secs: HashMap<(String, String), i64> = HashMap::new();
    let (mut cur, mut start, mut acc): (Option<(String, String)>, Option<NaiveDateTime>, i64) = (None, None, 0);
    for (_, ev) in events {
        match ev {
            Event::In { t, project, task } => {
                // An In after a break resumes the session; one without an Out before it ends the last one.
                if let (Some(s), Some(k)) = (start, cur.take()) {
                    *secs.entry(k).or_default() += acc + (*t - s).num_seconds();
                    acc = 0;
                }
                cur = Some((norm(project), norm(task)));
                start = Some(*t);
            }
            Event::Break { t } => acc += start.take().map_or(0, |s| (*t - s).num_seconds()),
            Event::Out { t, note } => {
                acc += start.take().map_or(0, |s| (*t - s).num_seconds());
                if let Some((p, task)) = cur.take() {
                    *secs.entry((p, if note.trim().is_empty() { task } else { norm(note) })).or_default() += acc;
                }
                acc = 0;
            }
        }
    }
    if let Some(k) = cur {
        *secs.entry(k).or_default() += acc + start.map_or(0, |s| (now - s).num_seconds());
    }
    let mut out = Spent::new();
    for ((p, task), s) in secs {
        if !task.is_empty() && s > 0 {
            *out.entry(task).or_default().entry(p).or_default() += ((s + 30) / 60) as u32;
        }
    }
    out
}

/// Minutes clocked on a heading TITLE in CATEGORY: on that project when it is one in PROJECTS,
/// else on the task in any project. None when nothing was clocked.
pub fn spent_on(spent: &Spent, projects: &Projects, category: &str, title: &str) -> Option<u32> {
    let by = spent.get(&norm(title))?;
    let c = norm(category);
    let m = if projects.keys().any(|p| norm(p) == c) { by.get(&c).copied().unwrap_or(0) } else { by.values().sum() };
    (m > 0).then_some(m)
}

/// Lines of EVENTS to rewrite to rename task OLD to NEW where `spent_by_task` would see it: the clock-in
/// task and the clock-out note, on project SCOPE (normalised) only when given. Also the sessions touched,
/// by project. Nothing when NEW is empty or the same title.
pub fn rename_task(events: &[(usize, Event)], scope: Option<&str>, old: &str, new: &str) -> (Vec<(usize, Event)>, BTreeMap<String, usize>) {
    let (o, n) = (norm(old), new.trim());
    let (mut lines, mut by) = (Vec::new(), BTreeMap::new());
    if o.is_empty() || n.is_empty() || norm(n) == o {
        return (lines, by);
    }
    let inscope = |p: &str| scope.is_none_or(|s| norm(p) == s);
    // the open session: its project and whether it was renamed; a break keeps it open for the resuming In
    let (mut cur, mut resume): (Option<(String, bool)>, bool) = (None, false);
    let mut close = |cur: &mut Option<(String, bool)>| {
        if let Some((p, true)) = cur.take() {
            *by.entry(p).or_default() += 1;
        }
    };
    for (i, ev) in events {
        match ev {
            Event::In { t, project, task } => {
                if !resume {
                    close(&mut cur);
                }
                let hit = inscope(project) && norm(task) == o;
                if hit {
                    lines.push((*i, Event::In { t: *t, project: project.clone(), task: n.into() }));
                }
                cur.get_or_insert((project.clone(), false)).1 |= hit;
                resume = false;
            }
            Event::Break { .. } => resume = true,
            Event::Out { t, note } => {
                if let Some((p, hit)) = &mut cur {
                    if inscope(p) && norm(note) == o {
                        lines.push((*i, Event::Out { t: *t, note: n.into() }));
                        *hit = true;
                    }
                }
                close(&mut cur);
                resume = false;
            }
        }
    }
    close(&mut cur);
    (lines, by)
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

/// Expected hours per weekday, Monday first (config `weekday_hours`, or `expected_daily_hours` Mon–Fri).
#[derive(Clone, Debug, PartialEq)]
pub struct Calendar {
    pub hours: [f64; 7],
    /// Swedish red days count as days off (config `public_holidays = "se"`).
    pub swedish: bool,
    /// Extra days off (config `extra_days_off`).
    pub extra: Vec<NaiveDate>,
}

impl Calendar {
    /// WEEKDAY (7 values, Monday first) when set, else DAILY Monday to Friday.
    pub fn new(daily: f64, weekday: &[f64]) -> Calendar {
        Calendar { hours: weekday.try_into().unwrap_or([daily, daily, daily, daily, daily, 0.0, 0.0]), swedish: true, extra: vec![] }
    }

    /// Day-off rules: HOLIDAYS `"se"` or `"none"` (config `public_holidays`) plus EXTRA ISO dates (bad ones are skipped).
    pub fn with_days(mut self, holidays: &str, extra: &[String]) -> Calendar {
        self.swedish = holidays != "none";
        self.extra = extra.iter().filter_map(|d| d.parse().ok()).collect();
        self
    }

    /// Whether DATE is a public holiday or extra day off.
    pub fn is_day_off(&self, date: NaiveDate) -> bool {
        self.extra.contains(&date) || (self.swedish && swedish_red_days(date.year()).iter().any(|(d, _)| *d == date))
    }

    /// DATE's hours by weekday alone, red days included.
    pub fn weekday(&self, date: NaiveDate) -> f64 {
        self.hours[date.weekday().num_days_from_monday() as usize]
    }
}

/// DATE's expected hours: its weekday's in CAL, 0 on days off.
pub fn expected_hours(date: NaiveDate, cal: &Calendar) -> f64 {
    if cal.is_day_off(date) { 0.0 } else { cal.weekday(date) }
}

/// Time off covering DATE with the hours each credits, given EXPECTED (before leave) and WORKED.
pub fn leave_on(leave: &[Leave], date: NaiveDate, expected: f64, worked: f64) -> Vec<(&Leave, f64)> {
    if expected <= 0.0 {
        return vec![]; // weekends and red days: nothing to take off
    }
    let hours = |l: &Leave| if l.rest { (expected - worked).max(0.0) } else { l.hours.unwrap_or(expected).min(expected) };
    leave.iter().filter(|l| l.from <= date && date <= l.to).map(|l| (l, hours(l))).collect()
}

/// Hours DATE's expected time drops by, at most EXPECTED.
pub fn leave_credit(leave: &[Leave], date: NaiveDate, expected: f64, worked: f64) -> f64 {
    leave_on(leave, date, expected, worked).iter().map(|x| x.1).sum::<f64>().min(expected)
}

/// Seconds of DAY's expected hours (CAL, less LEAVE) left after WORKED; None without expected hours.
fn remaining_secs(day: NaiveDate, worked: f64, cal: &Calendar, leave: &[Leave]) -> Option<i64> {
    let e = expected_hours(day, cal);
    (e > 0.0).then(|| ((e - leave_credit(leave, day, e, worked) - worked) * 3600.0).round() as i64)
}

/// Whether the clock is running and today's expected hours are worked, for the "day done" notification.
/// False on a break, without expected hours, or with a rest/whole-day leave (nothing was worked off).
pub fn day_done(now: NaiveDateTime, worked: f64, cal: &Calendar, leave: &[Leave], running: bool, on_break: bool) -> bool {
    let day = now.date();
    let off = leave.iter().any(|l| l.from <= day && day <= l.to && (l.rest || l.hours.is_none()));
    running && !on_break && !off && remaining_secs(day, worked, cal, leave).is_some_and(|s| s <= 0)
}

/// Whether to nudge a break: the block that started at SINCE (the open `in`; a resume or switch starts
/// a new one) has run FOCUS_MIN minutes by NOW and wasn't nudged yet (LAST_NOTIFIED is its start). 0 = off.
pub fn focus_due(since: NaiveDateTime, now: NaiveDateTime, focus_min: i64, last_notified: Option<NaiveDateTime>) -> bool {
    focus_min > 0 && now - since >= Duration::minutes(focus_min) && last_notified != Some(since)
}

/// When to leave today, flex ignored: what is left of NOW's expected hours (CAL, less LEAVE) after
/// WORKED, rounded up to whole minutes; with the clock RUNNING, the time it runs out.
pub fn leave_time(now: NaiveDateTime, worked: f64, cal: &Calendar, leave: &[Leave], running: bool, on_break: bool) -> String {
    let (day, hm) = (now.date(), |m: i64| if m >= 60 { format!("{}h {:02}m", m / 60, m % 60) } else { format!("{m}m") });
    let Some(secs) = remaining_secs(day, worked, cal, leave) else {
        return "No expected hours today.".into();
    };
    if secs <= 0 {
        return if secs <= -60 { format!("Done for today, {} over.", hm(-secs / 60)) } else { "Done for today.".into() };
    }
    let left = (secs as f64 / 60.0).ceil() as i64;
    if running {
        format!("Leave at {} ({} left).", (now + Duration::minutes(left)).format("%H:%M"), hm(left))
    } else if on_break {
        format!("On a break: {} left; the leave time starts when you resume.", hm(left))
    } else {
        format!("{} left today (not clocked in).", hm(left))
    }
}

/// Port of `albin/calculate-flex`: (total, period, period_days). Only days with work count;
/// time off lowers that day's expected hours, so work on a vacation day is flex.
pub fn flex(sessions: &[Session], projects: &Projects, leave: &[Leave], cal: &Calendar, range: Option<(NaiveDate, NaiveDate)>) -> (f64, f64, usize) {
    let mut per_day: BTreeMap<NaiveDate, f64> = BTreeMap::new();
    for s in apply_carry(sessions, projects).0 {
        *per_day.entry(s.date).or_default() += s.hours;
    }
    let (mut total, mut period, mut n) = (0.0, 0.0, 0);
    for (date, h) in per_day {
        let e = expected_hours(date, cal);
        let f = h - e + leave_credit(leave, date, e, h);
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

/// "vacation [VAC] 8.00 h (note)" for each time off on DATE; the code only when one is set.
fn time_off((leave, codes): (&[Leave], &LeaveCodes), date: NaiveDate, cal: &Calendar, worked: f64) -> Vec<String> {
    let note = |l: &Leave| if l.note.is_empty() { String::new() } else { format!(" ({})", l.note) };
    let code = |l: &Leave| match leave_code(&l.kind, codes) {
        c if c == l.kind => String::new(),
        c => format!(" [{c}]"),
    };
    leave_on(leave, date, expected_hours(date, cal), worked).iter().map(|(l, h)| format!("{}{} {h:.2} h{}", l.kind, code(l), note(l))).collect()
}

pub fn daily_report(sessions: &[Session], projects: &Projects, leave: (&[Leave], &LeaveCodes), cal: &Calendar, date: NaiveDate, profile: &str) -> String {
    let merged = prepare_report_sessions(sessions);
    let rounded = apply_carry(&merged, projects).0;
    let raw: f64 = merged.iter().filter(|s| s.date == date).map(|s| s.hours).sum();
    let day: Vec<&Session> = rounded.iter().filter(|s| s.date == date).collect();
    let billable: f64 = day.iter().map(|s| s.hours).sum();
    let mut o = format!("#+TITLE: Daily Time Report ({profile})\n#+SUBTITLE: {date}\n\n* Summary\n");
    o += &format!("  - Hours worked: {raw:.2} h\n  - Billable: {billable:.2} h\n  - Sessions: {}\n", day.len());
    time_off(leave, date, cal, raw).iter().for_each(|t| o += &format!("  - Time off: {t}\n"));
    o += "\n* Detailed Log\n";
    if day.is_empty() {
        o += "  - No sessions found for this date.\n";
    }
    day.iter().for_each(|s| o += &session_line(s, "  "));
    o
}

pub fn weekly_report(sessions: &[Session], projects: &Projects, leave: (&[Leave], &LeaveCodes), cal: &Calendar, start: NaiveDate, end: NaiveDate, profile: &str) -> String {
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
    let off: Vec<String> = start
        .iter_days()
        .take_while(|d| *d <= end)
        .flat_map(|d| {
            let worked = by_day.get(&d).map_or(0.0, |ss| ss.iter().map(|s| s.hours).sum());
            time_off(leave, d, cal, worked).into_iter().map(move |t| format!("  - {d} {}: {t}\n", d.format("%a")))
        })
        .collect();
    if !off.is_empty() {
        o += &format!("\n* Time off\n{}", off.concat());
    }
    o += "\n* Detailed Log\n";
    for (date, ss) in by_day {
        o += &format!("** {date} ({:.2} h)\n", ss.iter().map(|s| s.hours).sum::<f64>());
        ss.iter().for_each(|s| o += &session_line(s, "   "));
    }
    o
}

/// Body of a `#+BEGIN: clocktable` block (#693): worked hours START..=END per project (only PROJECT when
/// given), a bold subtotal row each with its session descriptions below, and a total row.
pub fn clocktable(sessions: &[Session], project: Option<&str>, start: NaiveDate, end: NaiveDate) -> String {
    let mut by: BTreeMap<&str, BTreeMap<&str, f64>> = BTreeMap::new();
    let merged = prepare_report_sessions(sessions);
    for s in merged.iter().filter(|s| start <= s.date && s.date <= end && project.is_none_or(|p| s.project == p)) {
        *by.entry(&s.project).or_default().entry(s.desc.trim()).or_default() += s.hours;
    }
    let mut projs: Vec<_> = by.into_iter().map(|(p, m)| (p, m.values().sum::<f64>(), m.into_iter().collect::<Vec<_>>())).collect();
    projs.sort_by(|a, b| b.1.total_cmp(&a.1)); // stable sorts: ties keep name order
    projs.iter_mut().for_each(|p| p.2.sort_by(|a, b| b.1.total_cmp(&a.1)));
    let mut rows = vec![["Project", "Task (description)", "Hours"].map(String::from).to_vec()];
    for (p, h, tasks) in &projs {
        rows.push(vec![format!("*{}*", or_other(p)), String::new(), format!("*{h:.2}*")]);
        tasks.iter().for_each(|(d, h)| rows.push(vec![String::new(), if d.is_empty() { "(no description)".into() } else { d.to_string() }, format!("{h:.2}")]));
    }
    let total = projs.iter().fold(0.0, |a, p| a + p.1); // not sum(): an empty f64 sum is -0.0
    rows.push(vec!["*Total*".into(), String::new(), format!("*{total:.2}*")]);
    org_table(&rows)
}

pub fn holidays_report(year: i32, cal: &Calendar) -> String {
    let mut o = format!("#+TITLE: {}Public Holidays {year}\n\n", if cal.swedish { "Swedish " } else { "" });
    if cal.swedish {
        for (d, name) in swedish_red_days(year) {
            o += &format!("- {d}  {}  {name}\n", d.format("%a"));
        }
    }
    let mut extra: Vec<_> = cal.extra.iter().filter(|d| d.year() == year).collect();
    extra.sort();
    extra.dedup();
    if !extra.is_empty() {
        o += "\n* Extra days off\n";
        extra.iter().for_each(|d| o += &format!("- {d}  {}\n", d.format("%a")));
    }
    o + "\n(These days contribute 0h expected time in the flex calculation)\n"
}

fn csv_field(v: &str) -> String {
    format!("\"{}\"", v.replace('"', "\"\""))
}

/// Import-template columns; `Project` is the export code, hours use a decimal comma (`1,5`).
/// Time off adds a row per kind and working day: its code, note (or kind) and credited hours.
/// Also returns a summary: the time off rows written, and the covered days that wrote none and why.
pub fn csv(sessions: &[Session], projects: &Projects, (leave, codes): (&[Leave], &LeaveCodes), cal: &Calendar, start: NaiveDate, end: NaiveDate) -> (String, String) {
    let rounded = apply_carry(&prepare_report_sessions(sessions), projects).0;
    let mut worked: HashMap<NaiveDate, f64> = HashMap::new();
    rounded.iter().for_each(|s| *worked.entry(s.date).or_default() += s.hours);
    let mut rows: Vec<(NaiveDate, String, String, f64, bool)> =
        rounded.iter().filter(|s| start <= s.date && s.date <= end).map(|s| (s.date, export_code(&s.project, projects), s.desc.clone(), s.hours, false)).collect();
    let mut skipped = vec![];
    for date in start.iter_days().take_while(|d| *d <= end) {
        let e = expected_hours(date, cal);
        if e <= 0.0 && leave.iter().any(|l| l.from <= date && date <= l.to) {
            skipped.push(format!("{date} (no expected hours)"));
        }
        let mut left = e; // overlapping time off credits at most the day, as in flex
        for (l, h) in leave_on(leave, date, e, worked.get(&date).copied().unwrap_or(0.0)) {
            let h = h.min(left);
            left -= h;
            rows.push((date, leave_code(&l.kind, codes), if l.note.is_empty() { l.kind.clone() } else { l.note.clone() }, h, true));
        }
    }
    rows.sort_by_key(|r| r.0); // stable: a day's sessions, then its time off
    let (mut out, mut off) = (String::from("Project,Description,Date,Duration\n"), 0);
    for (date, code, desc, h, is_off) in rows {
        if (h * 100.0).round() <= 0.0 {
            if is_off {
                skipped.push(format!("{date} (nothing left to credit)"));
            }
            continue;
        }
        off += is_off as usize;
        let f = [code, desc, date.to_string(), format!("{h:.2}").trim_end_matches('0').trim_end_matches('.').replace('.', ",")];
        out += &(f.iter().map(|x| csv_field(x)).collect::<Vec<_>>().join(",") + "\n");
    }
    skipped.sort();
    skipped.dedup();
    let mut sum = format!("{off} time off row{}", if off == 1 { "" } else { "s" });
    if !skipped.is_empty() {
        sum += &format!("; none for {}", skipped[..skipped.len().min(3)].join(", "));
        if skipped.len() > 3 {
            sum += &format!(" and {} more", skipped.len() - 3);
        }
    }
    (out, sum)
}

/// Port of `albin/timeclock--find-log-issues`, plus overlapping sessions: two machines clocked
/// in at once read `in, in, out, out` after a sync (both kept, decision on #307).
pub fn doctor(text: &str, now: NaiveDateTime) -> Vec<String> {
    let mut issues = vec![];
    let mut open: Option<(usize, NaiveDateTime)> = None;
    // Line of the overlapped clock-in whose clock-out is still to come, and the latest clock-out
    // so far (not breaks: `adjust_start` after a resume may move it before the break).
    let (mut overlapped, mut ended): (Option<usize>, Option<NaiveDateTime>) = (None, None);
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
        let is_out = matches!(ev, Event::Out { .. });
        match ev {
            Event::In { t, .. } => {
                if let Some((m, o)) = open {
                    issues.push(format!("Line {n}: overlapping clock-ins (two machines?): clocked in at {t} while the session started {o} (line {m}) was still open; only the later one is counted"));
                    overlapped = Some(m);
                } else if let Some(e) = ended.filter(|e| t < *e) {
                    issues.push(format!("Line {n}: overlapping sessions: starts at {t}, before the previous session ended at {e}"));
                }
                open = Some((n, t));
            }
            Event::Out { t, .. } | Event::Break { t } => {
                match open.take() {
                    None => match overlapped.take() {
                        Some(m) => issues.push(format!("Line {n}: clock-out of the overlapped session started on line {m}")),
                        None => issues.push(format!("Line {n}: clock-out with no matching clock-in")),
                    },
                    Some((_, o)) if t < o => issues.push(format!("Line {n}: session ends before it starts ({o} -> {t})")),
                    _ => {}
                }
                if is_out {
                    ended = ended.max(Some(t));
                }
            }
        }
    }
    if let Some((_, o)) = open {
        let days = (now - o).num_seconds() as f64 / 86400.0;
        if days > 1.0 {
            issues.push(format!("Still clocked in since {o} ({days:.1} days ago) — did you forget to clock out?"));
        }
    }
    issues
}

/// Replace PATH's text with F(text) when that differs; true if it was written (missing: no-op).
fn rewrite_with(path: &Path, f: fn(&str) -> String) -> Result<bool, String> {
    let text = read_or_empty(path)?;
    let out = f(&text);
    if out == text {
        return Ok(false);
    }
    crate::config::write_atomic(path, out).map_err(|e| e.to_string())?;
    Ok(true)
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
                    ..Project::new("")
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
    fn w(daily: f64) -> Calendar {
        Calendar::new(daily, &[])
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
        assert_eq!(expected_hours(d("2026-04-06"), &w(8.0)), 0.0); // Easter Monday
        assert_eq!(expected_hours(d("2026-04-07"), &w(8.0)), 8.0);
    }

    #[test]
    fn day_off_rules() {
        let none = w(8.0).with_days("none", &[]);
        assert_eq!(expected_hours(d("2026-04-06"), &none), 8.0); // Easter Monday works
        let extra = w(8.0).with_days("se", &["2026-04-07".into(), "2026-04-07".into(), "bad".into(), "2026-04-11".into()]);
        assert_eq!((expected_hours(d("2026-04-07"), &extra), expected_hours(d("2026-04-06"), &extra)), (0.0, 0.0));
        assert_eq!(expected_hours(d("2026-04-08"), &extra), 8.0);
        let both = w(8.0).with_days("none", &["2026-04-07".into()]);
        assert_eq!((expected_hours(d("2026-04-07"), &both), expected_hours(d("2026-04-06"), &both)), (0.0, 8.0));
        let r = holidays_report(2026, &both);
        assert!(r.starts_with("#+TITLE: Public Holidays 2026") && !r.contains("Easter") && r.contains("* Extra days off\n- 2026-04-07  Tue"), "{r}");
        assert!(holidays_report(2026, &w(8.0)).contains("Swedish") && !holidays_report(2026, &extra).contains("2027"));
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
    fn clocktable_groups_by_project_and_description() {
        let s = |date: &str, p: &str, desc: &str, h| Session { date: d(date), project: p.into(), desc: desc.into(), hours: h, start: NaiveTime::MIN, end: NaiveTime::MIN, line: None };
        let log = [s("2026-10-01", "Acme", "fix", 1.0), s("2026-10-02", "Acme", " fix", 1.5), s("2026-10-02", "Acme", "docs", 0.5), s("2026-10-03", "Beta", "call", 4.0), s("2026-11-01", "Acme", "fix", 9.0)];
        let (from, to) = (d("2026-10-01"), d("2026-10-31"));
        let t = clocktable(&log, None, from, to);
        let rows: Vec<Vec<&str>> = t.lines().map(|l| l.split('|').map(str::trim).collect()).collect();
        assert_eq!(rows.len(), 8, "{t}");
        assert!(t.starts_with("| Project | Task (description) | Hours  |\n|---------+"), "{t}");
        assert_eq!(rows[2][1..4], ["*Beta*", "", "*4.00*"], "{t}"); // biggest project first
        assert_eq!(rows[4][1..4], ["*Acme*", "", "*3.00*"], "{t}");
        assert_eq!(rows[5][1..4], ["", "fix", "2.50"], "{t}"); // across days, trimmed, November left out
        assert_eq!(rows[6][1..4], ["", "docs", "0.50"], "{t}");
        assert!(t.ends_with("| *Total* |                    | *7.00* |\n"), "{t}");
        assert_eq!(clocktable(&log, None, from, to), t); // idempotent
        let acme = clocktable(&log, Some("Acme"), from, to);
        assert!(!acme.contains("Beta") && acme.contains("*Total*") && acme.contains("*3.00*"), "{acme}");
        let none = clocktable(&log, None, d("2020-01-01"), d("2020-01-31"));
        assert!(none.ends_with("| *Total* |                    | *0.00* |\n"), "{none}");
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
        assert_eq!(flex(&s, &Projects::new(), &[], &w(8.0), None).0, -6.0);
        assert_eq!(task_suggestions(&s, Some("Acme"), d("2026-10-02")), vec!["Review", "Did stuff"]);
    }

    #[test]
    fn when_to_leave() {
        let l = |hours, rest| Leave { from: d("2026-10-12"), to: d("2026-10-12"), kind: "vacation".into(), hours, rest, note: String::new() };
        let mon = dt("2026-10-12 14:24");
        assert_eq!(leave_time(mon, 5.7, &w(8.0), &[], true, false), "Leave at 16:42 (2h 18m left).");
        assert_eq!(leave_time(mon, 5.7, &w(8.0), &[], false, true), "On a break: 2h 18m left; the leave time starts when you resume.");
        assert_eq!(leave_time(mon, 7.5, &w(8.0), &[], false, false), "30m left today (not clocked in).");
        assert_eq!(leave_time(mon, 7.99, &w(8.0), &[], true, false), "Leave at 14:25 (1m left)."); // rounded up
        assert_eq!(leave_time(mon, 8.5, &w(8.0), &[], true, false), "Done for today, 30m over.");
        assert_eq!(leave_time(mon, 8.0, &w(8.0), &[], true, false), "Done for today.");
        assert!(day_done(mon, 8.0, &w(8.0), &[], true, false)); // exactly at the boundary
        assert!(day_done(mon, 8.0 + 59.0 / 3600.0, &w(8.0), &[], true, false));
        assert!(!day_done(mon, 7.99, &w(8.0), &[], true, false));
        assert!(!day_done(mon, 9.0, &w(8.0), &[], false, false)); // not running
        assert!(!day_done(mon, 9.0, &w(8.0), &[], false, true)); // on a break
        assert!(!day_done(dt("2026-10-10 10:00"), 1.0, &w(8.0), &[], true, false)); // no expected hours
        assert!(day_done(mon, 4.0, &w(8.0), &[l(Some(4.0), false)], true, false)); // half day off
        assert!(!day_done(mon, 3.0, &w(8.0), &[l(None, true)], true, false)); // rest of the day off
        assert!(!day_done(mon, 0.0, &w(8.0), &[l(None, false)], true, false)); // whole day off
        assert_eq!(leave_time(dt("2026-10-10 10:00"), 1.0, &w(8.0), &[], true, false), "No expected hours today."); // Saturday
        assert_eq!(leave_time(dt("2026-12-25 10:00"), 0.0, &w(8.0), &[], true, false), "No expected hours today."); // red day
        assert_eq!(leave_time(mon, 2.0, &w(8.0), &[l(Some(4.0), false)], true, false), "Leave at 16:24 (2h 00m left)."); // half day off
        assert_eq!(leave_time(mon, 3.0, &w(8.0), &[l(None, true)], true, false), "Done for today."); // rest of the day off
        assert_eq!(leave_time(mon, 0.0, &w(8.0), &[l(None, false)], false, false), "Done for today."); // whole day off
    }

    #[test]
    fn focus_due_once_per_block() {
        let since = dt("2026-10-12 23:30");
        assert!(!focus_due(since, dt("2026-10-13 00:19"), 50, None));
        assert!(focus_due(since, dt("2026-10-13 00:20"), 50, None)); // past midnight
        assert!(!focus_due(since, dt("2026-10-13 02:00"), 50, Some(since))); // no repeat
        assert!(focus_due(dt("2026-10-13 00:30"), dt("2026-10-13 01:20"), 50, Some(since))); // a resume starts a new block
        assert!(!focus_due(since, dt("2026-10-13 05:00"), 0, None)); // off
        assert_eq!(format!("{since:?}").parse::<NaiveDateTime>(), Ok(since)); // how remind::focus stores it
    }

    #[test]
    fn leave_lowers_expected_hours() {
        let l = |from: &str, to: &str, hours, rest| Leave { from: d(from), to: d(to), kind: "vacation".into(), hours, rest, note: String::new() };
        let s = |date: &str, h| Session { date: d(date), project: "A".into(), desc: "x".into(), hours: h, start: NaiveTime::MIN, end: NaiveTime::MIN, line: None };
        let p = Projects::new();
        // Mon 2026-10-05 .. Sun 10-11: full days, none credited on the weekend.
        let week = [l("2026-10-05", "2026-10-11", None, false)];
        assert_eq!(leave_credit(&week, d("2026-10-06"), 8.0, 0.0), 8.0);
        assert_eq!(leave_credit(&week, d("2026-10-10"), 0.0, 0.0), 0.0);
        assert_eq!(flex(&[s("2026-10-06", 2.0)], &p, &week, &w(8.0), None).0, 2.0); // work on vacation is flex
        assert_eq!(flex(&[], &p, &week, &w(8.0), None).0, 0.0); // and a vacation day without work is even
        assert_eq!(flex(&[s("2026-10-12", 8.0)], &p, &week, &w(8.0), None).0, 0.0); // after it, back to normal
        // Half a day off plus 4 h worked is a full day; rest-of-day tops up to the target, never past it.
        assert_eq!(flex(&[s("2026-10-06", 4.0)], &p, &[l("2026-10-06", "2026-10-06", Some(4.0), false)], &w(8.0), None).0, 0.0);
        let sick = [Leave { kind: "sick".into(), ..l("2026-10-06", "2026-10-06", None, true) }];
        assert_eq!(flex(&[s("2026-10-06", 3.0)], &p, &sick, &w(8.0), None).0, 0.0);
        assert_eq!(flex(&[s("2026-10-06", 9.0)], &p, &sick, &w(8.0), None).0, 1.0);
        // Overlapping entries never credit more than the day.
        assert_eq!(leave_credit(&[week[0].clone(), sick[0].clone()], d("2026-10-06"), 8.0, 3.0), 8.0);
        // Reports list it without counting it as worked.
        let codes = LeaveCodes::from([("vacation".into(), LeaveKind { export_code: "VAC".into() })]);
        let r = daily_report(&[], &p, (&sick, &codes), &w(8.0), d("2026-10-06"), "Work");
        assert!(r.contains("Sessions: 0\n  - Time off: sick 8.00 h\n"), "{r}");
        let r = weekly_report(&[], &p, (&week, &codes), &w(8.0), d("2026-10-09"), d("2026-10-12"), "Work");
        assert!(r.contains("- 2026-10-09 Fri: vacation [VAC] 8.00 h") && !r.contains("2026-10-10 Sat") && !r.contains("2026-10-12 Mon:"), "{r}");
    }

    #[test]
    fn csv_time_off_rows() {
        let l = |kind: &str, from: &str, to: &str, hours, rest| Leave { from: d(from), to: d(to), kind: kind.into(), hours, rest, note: String::new() };
        let s = |date: &str, h| Session { date: d(date), project: "A".into(), desc: "x".into(), hours: h, start: NaiveTime::MIN, end: NaiveTime::MIN, line: None };
        let codes = LeaveCodes::from([("vacation".into(), LeaveKind { export_code: "VAC".into() })]);
        // Fri 2026-10-09 .. Mon 10-12: no weekend rows; uncoded kinds fall back to the kind; work on the day stays its own row.
        let leave = [Leave { note: "trip".into(), ..l("vacation", "2026-10-09", "2026-10-12", None, false) }, l("sick", "2026-10-13", "2026-10-13", None, true)];
        let (out, sum) = csv(&[s("2026-10-12", 1.0), s("2026-10-13", 2.5)], &Projects::new(), (&leave, &codes), &w(8.0), d("2026-10-01"), d("2026-10-13"));
        assert_eq!(sum, "3 time off rows; none for 2026-10-10 (no expected hours), 2026-10-11 (no expected hours)");
        assert_eq!(
            out.lines().skip(1).collect::<Vec<_>>(),
            [
                r#""VAC","trip","2026-10-09","8""#,
                r#""A","x","2026-10-12","1""#,
                r#""VAC","trip","2026-10-12","8""#,
                r#""A","x","2026-10-13","2,5""#,
                r#""sick","sick","2026-10-13","5,5""#, // rest of the day after work
            ]
        );
        // Out of range: nothing; overlapping kinds never credit more than the day; partial hours.
        assert_eq!(csv(&[], &Projects::new(), (&leave, &codes), &w(8.0), d("2026-10-14"), d("2026-10-20")), ("Project,Description,Date,Duration\n".into(), "0 time off rows".into()));
        let both = [l("vacation", "2026-10-06", "2026-10-06", Some(6.0), false), l("other", "2026-10-06", "2026-10-06", Some(4.5), false)];
        let out = csv(&[], &Projects::new(), (&both, &codes), &w(8.0), d("2026-10-06"), d("2026-10-06")).0;
        assert!(out.ends_with("\"VAC\",\"vacation\",\"2026-10-06\",\"6\"\n\"other\",\"other\",\"2026-10-06\",\"2\"\n"), "{out}");
    }

    #[test]
    fn csv_says_why_time_off_wrote_no_row() {
        let l = |kind: &str, from: &str, to: &str, rest| Leave { from: d(from), to: d(to), kind: kind.into(), hours: None, rest, note: String::new() };
        let s = |date: &str, h| Session { date: d(date), project: "A".into(), desc: "x".into(), hours: h, start: NaiveTime::MIN, end: NaiveTime::MIN, line: None };
        let sum = |sessions: &[Session], leave: &[Leave], daily: f64, end| csv(sessions, &Projects::new(), (leave, &LeaveCodes::new()), &w(daily), d("2026-10-01"), d(end)).1;
        // A lone Saturday, a red day (Christmas), rest of the day after a full day, no daily hours.
        assert_eq!(sum(&[], &[l("vacation", "2026-10-10", "2026-10-10", false)], 8.0, "2026-10-31"), "0 time off rows; none for 2026-10-10 (no expected hours)");
        assert_eq!(sum(&[], &[l("vacation", "2026-12-25", "2026-12-25", false)], 8.0, "2026-12-31"), "0 time off rows; none for 2026-12-25 (no expected hours)");
        assert_eq!(sum(&[s("2026-10-13", 8.0)], &[l("sick", "2026-10-13", "2026-10-13", true)], 8.0, "2026-10-31"), "0 time off rows; none for 2026-10-13 (nothing left to credit)");
        assert_eq!(sum(&[], &[l("vacation", "2026-10-12", "2026-10-12", false)], 0.0, "2026-10-31"), "0 time off rows; none for 2026-10-12 (no expected hours)");
        // A full day on a working day is a row; long lists are cut short.
        assert_eq!(sum(&[], &[l("vacation", "2026-10-12", "2026-10-12", false)], 8.0, "2026-10-31"), "1 time off row");
        assert_eq!(
            sum(&[], &[l("vacation", "2026-10-01", "2026-10-31", false)], 8.0, "2026-10-31"),
            "22 time off rows; none for 2026-10-03 (no expected hours), 2026-10-04 (no expected hours), 2026-10-10 (no expected hours) and 6 more"
        );
    }

    #[test]
    fn leave_on_disk() {
        let dir = std::env::temp_dir().join(format!("tc-leave-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let tc = Tc { dir: dir.join("work"), diary: dir.join("dagbok.org"), cal: w(8.0) };
        let l = |from: &str, to: &str, kind: &str| Leave { from: d(from), to: d(to), kind: kind.into(), hours: None, rest: false, note: " trip ".into() };
        tc.add_leave(l("2026-07-01", "2026-07-14", " Vacation")).unwrap();
        tc.add_leave(l("2026-07-03", "2026-07-03", "sick")).unwrap(); // another kind may overlap
        assert_eq!(tc.leave()[0], (0, Leave { kind: "vacation".into(), note: "trip".into(), ..l("2026-07-01", "2026-07-14", "") }));
        assert!(tc.add_leave(l("2026-07-14", "2026-07-20", "vacation")).unwrap_err().contains("overlaps"));
        assert!(tc.add_leave(l("2026-07-02", "2026-07-01", "other")).is_err());
        assert!(tc.add_leave(Leave { hours: Some(9.0), ..l("2026-08-01", "2026-08-01", "other") }).is_err());
        assert!(tc.add_leave(Leave { hours: Some(0.0), ..l("2026-08-01", "2026-08-01", "other") }).is_err());
        tc.remove_leave(0).unwrap();
        assert_eq!(tc.leave().iter().map(|x| (x.0, x.1.kind.as_str())).collect::<Vec<_>>(), vec![(0, "sick")]);
        assert!(tc.remove_leave(5).is_err());
        // Export codes live apart in leave.toml, keyed like add_leave stores the kind.
        assert_eq!(tc.leave_codes().unwrap(), LeaveCodes::new());
        tc.save_leave_code(" Vacation ", " VAC ").unwrap();
        tc.save_leave_code("sick", "SJK").unwrap();
        assert_eq!(leave_code("vacation", &tc.leave_codes().unwrap()), "VAC");
        assert_eq!(fs::read_to_string(tc.dir.join("leave.toml")).unwrap(), "[sick]\nexport_code = \"SJK\"\n\n[vacation]\nexport_code = \"VAC\"\n");
        fs::write(tc.dir.join("leave.toml"), "[sick\n").unwrap();
        assert!(tc.save_leave_code("sick", "X").is_err() && tc.file_issues().iter().any(|i| i.contains("leave.toml")));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn project_budget_serde() {
        let p: Projects = toml::from_str("[Acme]\nexport_code = \"A\"\n").unwrap();
        assert_eq!((p["Acme"].budget_hours, p["Acme"].budget_period.as_str()), (None, "month"));
        assert_eq!(toml::to_string(&p).unwrap(), "[Acme]\nexport_code = \"A\"\nround_up = false\nactive = true\n"); // no budget keys
        let b = Projects::from([("B".into(), Project { budget_hours: Some(4.0), budget_period: "week".into(), ..Project::new("B") })]);
        assert_eq!(toml::from_str::<Projects>(&toml::to_string(&b).unwrap()).unwrap(), b);
    }

    #[test]
    fn eld_import() {
        let p = import_projects(r#"(("Acme" :export-code "AC-1" :rounding 0.25 :round-up t :active t) ("Old" . "OLD-9") ("Raw" :export-code "R" :rounding nil :round-up nil :active nil))"#);
        assert_eq!(p["Acme"], Project { export_code: "AC-1".into(), rounding: Some(0.25), round_up: true, ..Project::new("") });
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
    fn doctor_flags_overlapping_sessions() {
        let ev = |e: &str, t: &str| format!("{{\"ev\":\"{e}\",\"t\":\"2026-10-01T{t}:00\",\"project\":\"A\"}}");
        // Two machines clocked in at once, after a union merge and normalize_log.
        let log = [ev("in", "09:00"), ev("in", "10:00"), ev("out", "11:00"), ev("out", "12:00"), ev("in", "12:00"), ev("out", "13:00")].join("\n");
        let i = doctor(&normalize_log(&log), dt("2026-10-01 14:00"));
        assert_eq!(i.len(), 2, "{i:?}");
        assert!(i[0].starts_with("Line 2: overlapping clock-ins") && i[0].contains("(line 1)"), "{i:?}");
        assert_eq!(i[1], "Line 4: clock-out of the overlapped session started on line 1");
        // Unsorted, the overlap hides behind clean pairs; back-to-back sessions are fine.
        let i = doctor(&[ev("in", "09:00"), ev("out", "12:00"), ev("in", "10:00"), ev("out", "11:00")].join("\n"), dt("2026-10-01 14:00"));
        assert_eq!(i, ["Line 3: overlapping sessions: starts at 2026-10-01 10:00:00, before the previous session ended at 2026-10-01 12:00:00"]);
        assert!(doctor(&[ev("in", "09:00"), ev("break", "10:00"), ev("in", "10:00"), ev("out", "11:00")].join("\n"), dt("2026-10-01 14:00")).is_empty());
    }

    #[test]
    fn clock_flow_on_disk() {
        let dir = std::env::temp_dir().join(format!("tc-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let tc = Tc { dir: dir.join("work"), diary: dir.join("notes/dagbok-work.org"), cal: w(8.0) };
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
    fn today_counts_an_open_clock_from_midnight() {
        let now = dt("2026-10-02 01:00");
        let done = Session { date: d("2026-10-02"), project: "A".into(), desc: String::new(), hours: 0.5, start: NaiveTime::MIN, end: NaiveTime::MIN, line: Some(1) };
        let yday = Session { date: d("2026-10-01"), ..done.clone() };
        let sessions = [done, yday];
        assert_eq!(today_hours(&sessions, Some(dt("2026-10-01 22:00")), now), 1.5); // not 3.5
        assert_eq!(today_hours(&sessions, Some(dt("2026-10-02 00:15")), now), 1.25);
        assert_eq!(today_hours(&sessions, None, now), 0.5);
        // 1.5 h done at 01:00 of an 8 h day: 6.5 h left.
        assert_eq!(leave_time(now, today_hours(&sessions, Some(dt("2026-10-01 22:00")), now), &w(8.0), &[], true, false), "Leave at 07:30 (6h 30m left).");
    }

    #[test]
    fn locked_keeps_appends_during_a_rewrite() {
        for i in 0..20 {
            let dir = std::env::temp_dir().join(format!("tc-race-{}-{i}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            let tc = Tc { dir: dir.join("work"), diary: dir.join("dagbok.org"), cal: w(8.0) };
            tc.clock_in("Acme", "", None).unwrap();
            let go = std::sync::Barrier::new(2);
            std::thread::scope(|t| {
                t.spawn(|| {
                    go.wait();
                    for _ in 0..5 {
                        locked(|| tc.adjust_start(1)).unwrap();
                    }
                });
                t.spawn(|| {
                    go.wait();
                    for _ in 0..5 {
                        locked(|| tc.take_break()).unwrap();
                        locked(|| tc.resume()).unwrap();
                    }
                });
            });
            let ev = tc.events();
            assert_eq!(ev.len(), 11, "{ev:?}"); // in + 5 × (break, in)
            assert!(doctor(&tc.read_log(), now() + Duration::minutes(10)).is_empty());
            let _ = fs::remove_dir_all(&dir);
        }
    }

    #[test]
    fn locked_keeps_leave_added_during_a_dedupe() {
        let dir = std::env::temp_dir().join(format!("tc-leave-race-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let tc = Tc { dir: dir.join("work"), diary: dir.join("d.org"), cal: w(8.0) };
        let go = std::sync::Barrier::new(2);
        std::thread::scope(|t| {
            t.spawn(|| {
                go.wait();
                for d in 1..=20 {
                    let day = NaiveDate::from_ymd_opt(2026, 7, d).unwrap();
                    locked(|| tc.add_leave(Leave { from: day, to: day, kind: "vacation".into(), note: String::new(), hours: None, rest: false })).unwrap();
                }
            });
            t.spawn(|| {
                go.wait();
                for _ in 0..20 {
                    locked(|| tc.dedupe_leave()).unwrap();
                }
            });
        });
        assert_eq!(tc.leave().len(), 20);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn expected_hours_per_weekday() {
        let four = Calendar::new(8.0, &[8.0, 8.0, 8.0, 8.0, 0.0, 0.0, 0.0]);
        assert_eq!(Calendar::new(7.5, &[]).hours, [7.5, 7.5, 7.5, 7.5, 7.5, 0.0, 0.0]); // empty: Mon–Fri as before
        assert_eq!((expected_hours(d("2026-10-15"), &four), expected_hours(d("2026-10-16"), &four)), (8.0, 0.0)); // Friday off
        assert_eq!(expected_hours(d("2026-04-06"), &Calendar::new(8.0, &[8.0; 7])), 0.0); // red days stay 0
        let s = |date: &str, h| Session { date: d(date), project: "A".into(), desc: "x".into(), hours: h, start: NaiveTime::MIN, end: NaiveTime::MIN, line: None };
        let week: Vec<_> = ["2026-10-12", "2026-10-13", "2026-10-14", "2026-10-15"].iter().map(|x| s(x, 8.0)).chain([s("2026-10-16", 2.0)]).collect();
        assert_eq!(flex(&week, &Projects::new(), &[], &four, None).0, 2.0); // Friday's work is all flex
        assert_eq!(leave_time(dt("2026-10-16 10:00"), 0.0, &four, &[], true, false), "No expected hours today.");
        // A short Friday: 8 h a day off for the week credits Friday its 6 h, no more.
        let short = Calendar::new(8.0, &[8.0, 8.0, 8.0, 8.0, 6.0, 0.0, 0.0]);
        let l = |from: &str, to: &str, h| Leave { from: d(from), to: d(to), kind: "vacation".into(), hours: Some(h), rest: false, note: String::new() };
        let vac = [l("2026-10-12", "2026-10-16", 8.0)];
        assert_eq!(leave_on(&vac, d("2026-10-16"), expected_hours(d("2026-10-16"), &short), 0.0)[0].1, 6.0);
        assert_eq!(flex(&[s("2026-10-16", 1.0)], &Projects::new(), &vac, &short, None).0, 1.0);
        // Time off hours are capped by the range's longest day.
        let dir = std::env::temp_dir().join(format!("tc-weekday-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let tc = Tc { dir: dir.join("work"), diary: dir.join("d.org"), cal: short };
        assert!(tc.add_leave(l("2026-10-16", "2026-10-16", 7.0)).unwrap_err().contains("at most 6"));
        assert!(tc.add_leave(l("2026-10-17", "2026-10-18", 2.0)).unwrap_err().contains("no expected hours"));
        tc.add_leave(vac[0].clone()).unwrap(); // Monday's 8 h
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn adjust_start_refuses_to_overlap_the_previous_session() {
        let dir = std::env::temp_dir().join(format!("tc-adjust-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let tc = Tc { dir: dir.join("work"), diary: dir.join("d.org"), cal: w(8.0) };
        assert!(!tc.clock_in("A", "", None).unwrap());
        assert!(tc.clock_out("").unwrap());
        assert!(!tc.clock_in("A", "", None).unwrap());
        let err = tc.adjust_start(5).unwrap_err();
        assert!(err.contains("overlaps"), "{err}");
        assert!(tc.adjust_start(0).unwrap());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn clock_out_earlier_refuses_before_the_start() {
        let dir = std::env::temp_dir().join(format!("tc-outat-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let tc = Tc { dir: dir.join("work"), diary: dir.join("d.org"), cal: w(8.0) };
        assert!(!tc.clock_out_earlier(dt("2026-10-01 08:00"), "").unwrap()); // not clocked in
        tc.append(&Event::In { t: dt("2026-10-01 09:00"), project: "A".into(), task: String::new() }).unwrap();
        let err = tc.clock_out_earlier(dt("2026-10-01 08:00"), "").unwrap_err();
        assert!(err.contains("09:00"), "{err}");
        assert!(tc.current().is_some());
        assert!(tc.clock_out_earlier(dt("2026-10-01 10:00"), "").unwrap());
        assert!(tc.current().is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn malformed_projects_are_reported() {
        let dir = std::env::temp_dir().join(format!("tc-proj-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let tc = Tc { dir: dir.join("work"), diary: dir.join("dagbok.org"), cal: w(8.0) };
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
        let tc = Tc { dir: dir.join("work"), diary: dir.join("dagbok.org"), cal: w(8.0) };
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
        let tc = Tc { dir: dir.join("work"), diary: dir.join("dagbok.org"), cal: w(8.0) };
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
    fn edit_session_bounds() {
        let dir = std::env::temp_dir().join(format!("tc-edit-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let tc = Tc { dir: dir.join("work"), diary: dir.join("dagbok.org"), cal: w(8.0) };
        let i = |t: &str| Event::In { t: dt(t), project: "A".into(), task: String::new() };
        let o = |t: &str| Event::Out { t: dt(t), note: "x".into() };
        let log = [i("2024-10-01 09:00"), Event::Break { t: dt("2024-10-01 10:00") }, i("2024-10-01 11:00"), o("2024-10-01 12:00"), i("2024-10-01 13:00"), o("2024-10-01 14:00")];
        log.iter().for_each(|e| tc.append(e).unwrap());
        let out_t = |l| tc.events().into_iter().find(|(i, _)| *i == l).unwrap().1.t();
        for h in [0.0, -1.0] {
            assert!(tc.edit_session(3, "x", 2.0, h).unwrap_err().contains("longer than zero"), "{h}");
        }
        // 0.5 h ends at 10:30, before the 11:00 resume
        assert!(tc.edit_session(3, "x", 2.0, 0.5).unwrap_err().contains("11:00"));
        assert!(tc.edit_session(3, "x", 2.0, 3.5).is_err()); // past the 13:00 clock-in
        assert!(tc.edit_session(5, "x", 1.0, 1e6).is_err()); // future
        assert_eq!(out_t(3), dt("2024-10-01 12:00"));
        tc.edit_session(3, "x", 2.0, 1.5).unwrap(); // shortening
        assert_eq!(out_t(3), dt("2024-10-01 11:30"));
        tc.edit_session(3, "x", 1.5, 3.0).unwrap(); // back to back with the next in
        assert_eq!(out_t(3), dt("2024-10-01 13:00"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn unreadable_files_are_not_overwritten() {
        let dir = std::env::temp_dir().join(format!("tc-bad-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let tc = Tc { dir: dir.join("work"), diary: dir.join("dagbok.org"), cal: w(8.0) };
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
        let none = (&[][..], &LeaveCodes::new());
        let out = csv(&s, &Projects::new(), none, &w(8.0), d("2026-10-01"), d("2026-10-01")).0;
        assert_eq!(out.lines().nth(1).unwrap(), r#""A","say ""hi""","2026-10-01","1,5""#);
        let s = vec![Session { hours: 2.0, ..s[0].clone() }];
        assert!(csv(&s, &Projects::new(), none, &w(8.0), d("2026-10-01"), d("2026-10-01")).0.ends_with(",\"2\"\n"));
    }

    #[test]
    fn csv_skips_zero_rows() {
        let s = |h| vec![Session { date: d("2026-10-01"), project: "A".into(), desc: "x".into(), hours: h, start: NaiveTime::MIN, end: NaiveTime::MIN, line: None }];
        let rows = |h| csv(&s(h), &Projects::new(), (&[], &LeaveCodes::new()), &w(8.0), d("2026-10-01"), d("2026-10-01")).0.lines().count();
        assert_eq!((rows(0.0), rows(0.2), rows(0.3)), (1, 1, 2)); // default rounding is 0.5 h
    }

    #[test]
    fn events_cached_until_log_changes() {
        let dir = std::env::temp_dir().join(format!("tc-cache-{}", std::process::id()));
        let tc = Tc { dir: dir.clone(), diary: dir.join("d.org"), cal: w(8.0) };
        let _ = fs::remove_dir_all(&dir);
        assert!(tc.events().is_empty()); // no log: nothing cached
        fs::create_dir_all(&dir).unwrap();
        let line = |p: &str| format!("{{\"ev\":\"in\",\"t\":\"2026-10-01T08:00:00\",\"project\":\"{p}\"}}\n");
        fs::write(tc.log_path(), line("A")).unwrap();
        let proj = |tc: &Tc| tc.events().iter().map(|(_, e)| match e { Event::In { project, .. } => project.clone(), _ => "-".into() }).collect::<Vec<_>>();
        assert_eq!(proj(&tc), ["A"]);
        // Same length and mtime: a hit, the edit isn't seen.
        let mtime = fs::metadata(tc.log_path()).unwrap().modified().unwrap();
        fs::write(tc.log_path(), line("B")).unwrap();
        fs::File::options().write(true).open(tc.log_path()).unwrap().set_modified(mtime).unwrap();
        assert_eq!(proj(&tc), ["A"]);
        // A changed mtime is a miss.
        fs::File::options().write(true).open(tc.log_path()).unwrap().set_modified(mtime + std::time::Duration::from_secs(1)).unwrap();
        assert_eq!(proj(&tc), ["B"]);
        // In-app writes forget it even when (mtime, len) stays put.
        let mtime = fs::metadata(tc.log_path()).unwrap().modified().unwrap();
        tc.rewrite_line(0, &Event::In { t: dt("2026-10-01 08:00"), project: "C".into(), task: String::new() }).unwrap();
        fs::File::options().write(true).open(tc.log_path()).unwrap().set_modified(mtime).unwrap();
        assert_eq!(proj(&tc), ["C"]);
        tc.append(&Event::Out { t: dt("2026-10-01 09:00"), note: String::new() }).unwrap();
        assert_eq!(proj(&tc), ["C", "-"]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn normalize_log_sorts_and_dedupes() {
        let (a, b) = (r#"{"ev":"in","t":"2026-10-01T10:00:00","project":"A"}"#, r#"{"ev":"out","t":"2026-10-01T11:00:00"}"#);
        let (c, d_) = (r#"{"ev":"in","t":"2026-10-01T08:00:00","project":"B"}"#, r#"{"ev":"out","t":"2026-10-01T09:00:00"}"#);
        // Machine A's lines, then B's earlier ones and a duplicate (union merge), a junk line after B's in.
        let text = format!("{a}\n{b}\n{c}\njunk\n\n{d_}\n{a}\n");
        assert_eq!(normalize_log(&text), format!("{c}\njunk\n{d_}\n{a}\n{b}\n"));
        let s = sessions(&parse_events(&normalize_log(&text)), dt("2026-10-02 00:00"));
        assert_eq!(s.iter().map(|s| (s.project.as_str(), s.hours)).collect::<Vec<_>>(), [("B", 1.0), ("A", 1.0)]);
        // Same-time events keep their order (a project switch is `out` then `in` at one second).
        let sw = format!("{b}\n{}\n", r#"{"ev":"in","t":"2026-10-01T11:00:00","project":"C"}"#);
        assert_eq!(normalize_log(&sw), sw);
        let dir = std::env::temp_dir().join(format!("tc-norm-{}", std::process::id()));
        let tc = Tc { dir: dir.clone(), diary: dir.join("d.org"), cal: w(8.0) };
        assert_eq!(tc.normalize_log(), Ok(false)); // no log: nothing written
        assert!(!tc.log_path().exists());
        fs::create_dir_all(&dir).unwrap();
        fs::write(tc.log_path(), &text).unwrap();
        assert_eq!((tc.normalize_log(), tc.normalize_log()), (Ok(true), Ok(false)));
        assert_eq!(tc.read_log(), normalize_log(&text));
        // Time off: the same entry added on both machines is kept once, order untouched.
        let (v, s) = (r#"{"from":"2026-07-01","to":"2026-07-14","kind":"vacation"}"#, r#"{"from":"2026-06-01","to":"2026-06-01","kind":"sick"}"#);
        assert_eq!(tc.dedupe_leave(), Ok(false));
        fs::write(tc.leave_path(), format!("{v}\n{s}\n\n{v}\n")).unwrap();
        assert_eq!((tc.dedupe_leave(), tc.dedupe_leave()), (Ok(true), Ok(false)));
        assert_eq!(fs::read_to_string(tc.leave_path()).unwrap(), format!("{v}\n{s}\n"));
        assert_eq!(tc.leave().len(), 2);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn spent_by_task_sums_sessions_per_task_and_project() {
        let i = |t: &str, p: &str, task: &str| Event::In { t: dt(t), project: p.into(), task: task.into() };
        let o = |t: &str, note: &str| Event::Out { t: dt(t), note: note.into() };
        let ev: Vec<(usize, Event)> = [
            i("2026-10-01 09:00", "Acme", "Write report"),
            Event::Break { t: dt("2026-10-01 09:30") },
            i("2026-10-01 10:00", "Acme", "Write report"), // resumed: the break doesn't count
            o("2026-10-01 10:40", ""),                     // blank note: the clock-in task
            i("2026-10-01 23:30", "Acme", ""),
            o("2026-10-02 00:30", " write REPORT "), // across midnight, note wins, normalised
            i("2026-10-02 08:00", "Other", "Write report"),
            i("2026-10-02 08:15", "Other", "Mail"), // switched without an Out
            o("2026-10-02 08:20", ""),
            i("2026-10-02 09:00", "", ""), // untitled: left out
            o("2026-10-02 09:30", ""),
            i("2026-10-02 10:00", "Acme", "Write report"), // still running
        ]
        .into_iter()
        .enumerate()
        .collect();
        let s = spent_by_task(&ev, dt("2026-10-02 10:05"));
        assert_eq!(s["write report"], HashMap::from([("acme".into(), 30 + 40 + 60 + 5), ("other".into(), 15)]));
        assert_eq!(s["mail"], HashMap::from([("other".into(), 5)]));
        assert_eq!(s.len(), 2);
        let projects = Projects::from([("ACME".to_string(), Project::new("A"))]);
        assert_eq!(spent_on(&s, &projects, "acme", "Write report"), Some(135)); // a project: only its own time
        assert_eq!(spent_on(&s, &projects, "inbox", "write report "), Some(150)); // not a project: every project
        assert_eq!(spent_on(&s, &projects, "acme", "Mail"), None);
        assert_eq!(spent_on(&s, &projects, "inbox", "Plan"), None);
    }

    #[test]
    fn rename_task_keeps_the_clocked_time_on_the_new_title() {
        let dir = std::env::temp_dir().join(format!("tc-rename-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let tc = Tc { dir: dir.join("work"), diary: dir.join("d.org"), cal: w(8.0) };
        let i = |t: &str, p: &str, task: &str| Event::In { t: dt(t), project: p.into(), task: task.into() };
        let o = |t: &str, note: &str| Event::Out { t: dt(t), note: note.into() };
        for ev in [
            i("2026-10-01 09:00", "Acme", "Write report"),
            Event::Break { t: dt("2026-10-01 09:30") },
            i("2026-10-01 10:00", "Acme", "Write report"), // resumed: one session, two In lines
            o("2026-10-01 10:40", "write report"),         // clock-out pre-filled with the task
            i("2026-10-01 11:00", "Acme", ""),
            o("2026-10-01 11:30", " WRITE report "), // only the note names it
            i("2026-10-01 12:00", "Acme", "Mail"),
            o("2026-10-01 12:10", ""),
            i("2026-10-01 13:00", "Other", "Write report"),
            o("2026-10-01 13:30", ""),
            i("2026-10-01 14:00", "Acme", "write report"), // still running
        ] {
            tc.append(&ev).unwrap();
        }
        tc.save_projects(&Projects::from([("ACME".to_string(), Project::new("A"))])).unwrap();
        let before = fs::read_to_string(tc.log_path()).unwrap();
        let acme = BTreeMap::from([("Acme".to_string(), 3)]);
        assert_eq!(tc.rename_task("acme", "Write report", "Write the report", false), Ok(acme.clone())); // dry run
        assert_eq!(fs::read_to_string(tc.log_path()).unwrap(), before);
        assert_eq!(tc.rename_task("inbox", "Write report", "Write the report", false), Ok(BTreeMap::from([("Acme".to_string(), 3), ("Other".to_string(), 1)])));
        assert_eq!(tc.rename_task("acme", "Write report", " write REPORT", false), Ok(BTreeMap::new())); // same title
        assert_eq!(tc.rename_task("acme", "Write report", "Write the report", true), Ok(acme));
        let s = spent_by_task(&tc.events(), dt("2026-10-01 14:05"));
        assert_eq!(s["write the report"], HashMap::from([("acme".into(), 30 + 40 + 30 + 5)]));
        assert_eq!(s["write report"], HashMap::from([("other".into(), 30)])); // out of scope
        assert_eq!(s["mail"], HashMap::from([("acme".into(), 10)]));
        assert_eq!(tc.events().len(), 11);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rhythm_splits_at_hours_midnight_and_breaks() {
        let i = |t: &str, p: &str| Event::In { t: dt(t), project: p.into(), task: String::new() };
        let (o, b) = (|t: &str| Event::Out { t: dt(t), note: String::new() }, |t: &str| Event::Break { t: dt(t) });
        // 2026-10-19 is a Monday. 09:10-09:40 inside one hour; 10:30-12:00 with a break 11:15-11:45;
        // Tue 23:30 to Wed 00:30 over midnight; an open clock on Fri 14:00 until now 14:20 on project B.
        let ev: Vec<(usize, Event)> = [i("2026-10-19 09:10", "A"), o("2026-10-19 09:40"), i("2026-10-19 10:30", "A"), b("2026-10-19 11:15"), i("2026-10-19 11:45", "A"), o("2026-10-19 12:00"), i("2026-10-20 23:30", "A"), o("2026-10-21 00:30"), i("2026-10-23 14:00", "B")].into_iter().enumerate().collect();
        let sp = project_spans(&ev, dt("2026-10-23 14:20"));
        assert_eq!(sp[1], (dt("2026-10-19 10:30"), dt("2026-10-19 11:15"), "A"));
        let r = rhythm(&sp, d("2026-10-19"), d("2026-10-25"), None);
        assert_eq!((r.minutes[0][9], r.minutes[0][10], r.minutes[0][11], r.minutes[0][12]), (30.0, 30.0, 30.0, 0.0)); // 15 before the break + 15 after
        assert_eq!((r.minutes[1][23], r.minutes[2][0], r.minutes[4][14]), (30.0, 30.0, 20.0));
        assert_eq!((r.hours, r.peak, r.pieces.len()), (170.0 / 60.0, Some((0, 9)), 7)); // ties: the earliest cell
        assert!((r.evening - 30.0 / 170.0).abs() < 1e-9);
        let late = r.pieces.iter().find(|x| (x.day, x.hour) == (2, 0)).unwrap();
        assert_eq!((late.start, late.end, late.minutes), (dt("2026-10-20 23:30"), dt("2026-10-21 00:30"), 30.0));
        // Project filter; a range starting Wednesday clips the midnight span to its second half.
        let b_only = rhythm(&sp, d("2026-10-19"), d("2026-10-25"), Some("B"));
        assert_eq!((b_only.hours, b_only.peak, b_only.pieces[0].project.as_str()), (20.0 / 60.0, Some((4, 14)), "B"));
        let wed = rhythm(&sp, d("2026-10-21"), d("2026-10-25"), Some("A"));
        assert_eq!((wed.minutes[1][23], wed.minutes[2][0], wed.hours), (0.0, 30.0, 0.5));
        // Empty: no peak, no division by zero; a range with no Mondays averages Monday to 0.
        let none = rhythm(&[], d("2026-10-19"), d("2026-10-25"), None);
        assert_eq!((none.peak, none.evening, none.hours), (None, 0.0, 0.0));
    }

    #[test]
    fn rhythm_averages_by_weekday_occurrences() {
        // Four Mondays 08:00-09:00 plus one Monday 10:00-10:40, over four weeks: 60 and 10 min average.
        let mut sp: Vec<(NaiveDateTime, NaiveDateTime, &str)> = (0..4).map(|w| (dt("2026-10-05 08:00") + Duration::weeks(w), dt("2026-10-05 09:00") + Duration::weeks(w), "A")).collect();
        sp.push((dt("2026-10-12 10:00"), dt("2026-10-12 10:40"), "A"));
        let r = rhythm(&sp, d("2026-10-05"), d("2026-11-01"), None);
        assert_eq!((r.minutes[0][8], r.minutes[0][10], r.peak), (60.0, 10.0, Some((0, 8))));
        // Sweden's DST end (2026-10-25, a Sunday): wall clock 01:30-03:30 counts its two clock hours once.
        let dst = rhythm(&[(dt("2026-10-25 01:30"), dt("2026-10-25 03:30"), "A")], d("2026-10-19"), d("2026-10-25"), None);
        assert_eq!((dst.minutes[6][1], dst.minutes[6][2], dst.minutes[6][3], dst.hours), (30.0, 60.0, 30.0, 2.0));
    }
}
