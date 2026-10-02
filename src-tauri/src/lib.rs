mod attach;
mod backup;
mod config;
mod export;
mod ics;
#[cfg(desktop)]
mod idle;
#[cfg(target_os = "linux")]
mod wayland_idle;
mod notes;
mod org;
mod remind;
mod timeclock;
mod tray;

use chrono::{Duration, NaiveDate, NaiveDateTime};
use config::Config;
use serde_json::{json, Value};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};
use notify_debouncer_mini::{new_debouncer, notify, DebounceEventResult, Debouncer};
use tauri::{AppHandle, Emitter, Manager, RunEvent, State, WindowEvent};
use timeclock::{format_hm, Project, Tc};

type R<T> = Result<T, String>;

struct App {
    cfg_path: PathBuf,
    cfg: Mutex<Config>,
    profile: Mutex<String>,
    cache: org::Cache,
    watcher: Mutex<Option<Debouncer<notify::RecommendedWatcher>>>,
}

impl App {
    fn cfg(&self) -> Config {
        self.cfg.lock().unwrap().clone()
    }
    fn profile(&self) -> String {
        self.profile.lock().unwrap().clone()
    }
    fn profile_file(&self) -> PathBuf {
        self.cfg().data().join("active-profile.txt")
    }
    fn tc(&self) -> Tc {
        let c = self.cfg();
        let p = self.profile().to_lowercase();
        Tc { dir: c.data().join(&p), diary: c.notes().join(format!("dagbok-{p}.org")), expected: c.expected_daily_hours }
    }
    fn kw(&self) -> org::Kw {
        let c = self.cfg();
        org::Kw { todo: c.todo_keywords, done: c.done_keywords }
    }
    /// Parsed notes, minus the capture-template files.
    fn files(&self) -> Vec<Arc<org::OrgFile>> {
        let c = self.cfg();
        let mut v = org::scan(&c.notes(), &self.kw(), &self.cache);
        if let Some(t) = c.templates_path() {
            v.retain(|f| !f.path.starts_with(&t));
        }
        v
    }
    fn backup(&self) {
        let c = self.cfg();
        backup::backup(&c.data(), true);
        backup::backup(&c.notes(), false);
    }
    /// Only let the frontend touch files under the notes, data or config dirs.
    fn check(&self, p: &Path) -> R<()> {
        let c = self.cfg();
        let roots = [c.notes(), c.data(), self.cfg_path.parent().unwrap().to_path_buf()];
        if p.components().any(|c| c == Component::ParentDir) || !roots.iter().any(|r| p.starts_with(r)) {
            return Err(format!("{} is outside the notes/data/config dirs", p.display()));
        }
        Ok(())
    }
}

fn today() -> NaiveDate {
    timeclock::now().date()
}

fn date(input: &str) -> R<NaiveDate> {
    org::read_date(input, today()).map(|d| d.0).ok_or_else(|| format!("can't parse date: {input}"))
}

// ---------------------------------------------------------------- config & files

/// The tutorial note, kept beside config.toml so its examples stay out of the agenda.
/// Written when missing; returns its path and whether it was just written (first run).
#[tauri::command]
fn tutorial(s: State<App>) -> R<(PathBuf, bool)> {
    let p = s.cfg_path.with_file_name("tutorial.org");
    let new = !p.exists();
    if new {
        std::fs::write(&p, include_str!("tutorial.org")).map_err(|e| e.to_string())?;
    }
    Ok((p, new))
}

#[tauri::command]
fn config(s: State<App>) -> Value {
    let c = s.cfg();
    let tc = s.tc();
    json!({
        "config": c, "config_path": s.cfg_path, "notes": c.notes(), "data": c.data(), "export": config::expand(&c.export_dir),
        "profile": s.profile(), "log_path": tc.log_path(), "diary_path": tc.diary,
    })
}

#[tauri::command]
fn reload_config(app: AppHandle, s: State<App>) -> R<()> {
    *s.cfg.lock().unwrap() = Config::load(&s.cfg_path)?;
    s.cache.clear();
    watch(&app);
    Ok(())
}

/// Add a `[[views]]` entry to config.toml (or RENAME the one with QUERY), leaving the rest of the file as written.
#[tauri::command]
fn save_view(app: AppHandle, s: State<App>, name: String, query: String, rename: bool) -> R<()> {
    let text = std::fs::read_to_string(&s.cfg_path).map_err(|e| e.to_string())?;
    let new = if rename { config::rename_view(&text, &query, &name)? } else { config::add_view(&text, &name, &query)? };
    config::write_atomic(&s.cfg_path, new).map_err(|e| e.to_string())?;
    reload_config(app, s)
}

/// Watch the notes and data dirs; the UI gets an "fs-changed" event (with the
/// changed paths) whenever anything there changes — our own writes, Emacs, sync tools.
fn watch(app: &AppHandle) {
    let s = app.state::<App>();
    let c = s.cfg();
    let handle = app.clone();
    let debouncer = new_debouncer(std::time::Duration::from_millis(150), move |res: DebounceEventResult| {
        let Ok(events) = res else { return };
        let mut paths: Vec<PathBuf> = events.into_iter().map(|e| e.path).filter(|p| !p.components().any(|c| c.as_os_str() == ".git")).collect();
        paths.sort();
        paths.dedup();
        if !paths.is_empty() {
            tray::refresh(&handle);
            write_calendar(&handle);
            let _ = handle.emit("fs-changed", paths);
        }
    });
    let mut d = match debouncer {
        Ok(d) => d,
        Err(e) => return eprintln!("file watching unavailable: {e}"),
    };
    for dir in [c.notes(), c.data()] {
        let _ = std::fs::create_dir_all(&dir);
        if let Err(e) = d.watcher().watch(&dir, notify::RecursiveMode::Recursive) {
            eprintln!("can't watch {}: {e}", dir.display());
        }
    }
    *s.watcher.lock().unwrap() = Some(d);
}

#[tauri::command]
fn list_files(s: State<App>) -> Vec<PathBuf> {
    let mut v = org::org_paths(&s.cfg().notes());
    v.sort();
    v
}

#[tauri::command]
fn read_file(s: State<App>, path: PathBuf) -> R<String> {
    s.check(&path)?;
    std::fs::read_to_string(&path).or_else(|e| if path.exists() { Err(e.to_string()) } else { Ok(String::new()) })
}

#[tauri::command]
fn write_file(app: AppHandle, s: State<App>, path: PathBuf, text: String) -> R<()> {
    s.check(&path)?;
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    config::write_atomic(&path, text).map_err(|e| e.to_string())?;
    if path == s.cfg_path {
        reload_config(app, s)?;
    }
    Ok(())
}

/// Copy SRC into NOTE's attachment folder; returns the link target relative to the note.
#[tauri::command]
fn attach_file(s: State<App>, note: PathBuf, src: PathBuf) -> R<String> {
    let name = src.file_name().ok_or("not a file")?.to_string_lossy().to_string();
    attach(&s, &note, &name, |dst| std::fs::copy(&src, dst).map(|_| ()))
}

/// Save the pasted bytes (raw body) as header `name` in header `note`'s attachment
/// folder; both headers percent-encoded. Returns the link target.
#[tauri::command]
fn attach_bytes(s: State<App>, request: tauri::ipc::Request) -> R<String> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else { return Err("expected raw bytes".into()) };
    let header = |k: &str| -> R<String> {
        let v = request.headers().get(k).ok_or(format!("missing {k}"))?.to_str().map_err(|e| e.to_string())?;
        Ok(percent_encoding::percent_decode_str(v).decode_utf8().map_err(|e| e.to_string())?.into_owned())
    };
    attach(&s, Path::new(&header("note")?), &header("name")?, |dst| std::fs::write(dst, bytes))
}

/// Reserves the name with `create_new`, so concurrent attaches of one name don't collide.
fn attach(s: &App, note: &Path, name: &str, write: impl FnOnce(&Path) -> std::io::Result<()>) -> R<String> {
    let c = s.cfg();
    loop {
        let (dst, link) = attach::target(&c.notes(), &c.attachments_dir, note, name, |p| p.exists())?;
        std::fs::create_dir_all(dst.parent().unwrap()).map_err(|e| e.to_string())?;
        match std::fs::File::create_new(&dst) {
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("{}: {e}", dst.display())),
            Ok(_) => {}
        }
        return write(&dst).map(|_| link).map_err(|e| {
            let _ = std::fs::remove_file(&dst);
            format!("{}: {e}", dst.display())
        });
    }
}

/// Files under the attachments folder (outside `.trash`) that no note links to.
#[tauri::command]
fn unused_attachments(s: State<App>) -> Vec<PathBuf> {
    let c = s.cfg();
    let files: Vec<PathBuf> = walkdir::WalkDir::new(c.notes().join(&c.attachments_dir))
        .into_iter()
        .filter_entry(|e| e.file_name() != ".trash")
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .collect();
    let notes = org::scan(&c.notes(), &s.kw(), &s.cache);
    let notes: Vec<(&Path, &str)> = notes.iter().map(|f| (f.path.as_path(), f.text.as_str())).collect();
    let mut v = attach::unused(&notes, &files);
    v.sort();
    v
}

/// Move attachment FILES to `<attachments_dir>/.trash/`, keeping their subpaths.
#[tauri::command]
fn trash_attachments(s: State<App>, files: Vec<PathBuf>) -> R<()> {
    let c = s.cfg();
    let root = c.notes().join(&c.attachments_dir);
    let trash = root.join(".trash");
    for f in files {
        let bad = || format!("{} is not an attachment", f.display());
        let rel = f.strip_prefix(&root).map_err(|_| bad())?;
        if rel.starts_with(".trash") || rel.components().any(|c| !matches!(c, Component::Normal(_))) {
            return Err(bad());
        }
        let mut dst = trash.join(rel);
        for i in 1.. {
            if !dst.exists() {
                break;
            }
            dst = trash.join(format!("{}.{i}", rel.display()));
        }
        std::fs::create_dir_all(dst.parent().unwrap()).map_err(|e| e.to_string())?;
        std::fs::rename(&f, &dst).map_err(|e| format!("{}: {e}", f.display()))?;
    }
    Ok(())
}

// ---------------------------------------------------------------- org

#[tauri::command]
fn agenda(s: State<App>, start: String, days: i64) -> R<Vec<org::Item>> {
    Ok(org::agenda(&s.files(), &s.kw(), date(&start)?, days, today()))
}

#[tauri::command]
fn todos(s: State<App>) -> Vec<org::Item> {
    org::todos(&s.files(), &s.kw(), false, |_, _| true, today())
}

/// Tasks matching QUERY (see org::query).
#[tauri::command]
fn search_todos(s: State<App>, query: String) -> R<Vec<org::Item>> {
    let kw = s.kw();
    let (keep, done) = org::query(&query, today(), &kw)?;
    Ok(org::todos(&s.files(), &kw, done, keep, today()))
}

#[tauri::command]
fn org_heading(s: State<App>, text: String, line: usize) -> Option<org::Parts> {
    org::heading(&text, line, &s.kw())
}

/// Headline edits. OP: cycle (VALUE ±1) | keyword (VALUE or none) | priority (letter or none)
/// | priority-cycle (±1) | tags ("a:b") | shift ("SCHEDULED 1").
#[tauri::command]
fn org_edit(s: State<App>, text: String, line: usize, op: String, value: Option<String>) -> R<String> {
    let kw = s.kw();
    let v = value.unwrap_or_default();
    let n = || v.trim().parse::<i32>().map_err(|_| format!("bad number: {v}"));
    let opt = || Some(v.trim()).filter(|x| !x.is_empty());
    Ok(match op.as_str() {
        "cycle" => org::cycle_todo(&text, line, &kw, n()?, timeclock::now()),
        "keyword" => org::set_keyword(&text, line, &kw, opt(), timeclock::now()),
        "priority" => org::set_priority(&text, line, &kw, opt()),
        "priority-cycle" => org::cycle_priority(&text, line, &kw, n()?),
        "tags" => org::set_tags(&text, line, &kw, v.split(':').map(|t| t.trim().to_string()).collect()),
        "shift" => {
            let (kind, days) = v.split_once(' ').ok_or("shift needs \"KIND DAYS\"")?;
            org::shift_date(&text, line, kind, days.trim().parse().map_err(|_| "bad day count")?)
        }
        o => return Err(format!("unknown org op {o}")),
    })
}

#[tauri::command]
fn org_targets(s: State<App>) -> Vec<org::Target> {
    org::targets(&s.files(), &s.cfg().notes())
}

#[tauri::command]
fn org_tags(s: State<App>) -> Vec<String> {
    org::all_tags(&s.files())
}

/// Returns [new source text, new destination text].
#[tauri::command]
fn org_refile(src: String, line: usize, dst: String, dst_line: Option<usize>) -> R<[String; 2]> {
    org::refile(&src, line, &dst, dst_line).map(|(a, b)| [a, b])
}

#[tauri::command]
fn org_refile_same(text: String, line: usize, dst_line: Option<usize>) -> R<String> {
    org::refile_same(&text, line, dst_line)
}

#[tauri::command]
fn org_archive(src: String, line: usize, dst: String, src_path: String) -> R<[String; 2]> {
    org::archive(&src, line, &dst, &src_path, timeclock::now()).map(|(a, b)| [a, b])
}

/// Date input → Some(date), "" → None, unreadable → error.
fn opt_date(input: &str) -> R<Option<(NaiveDate, Option<String>)>> {
    if input.trim().is_empty() {
        return Ok(None);
    }
    org::read_date(input, today()).map(Some).ok_or_else(|| format!("can't read date \"{input}\""))
}

#[tauri::command]
fn task_entry(s: State<App>, title: String, priority: Option<String>, tags: Vec<String>, scheduled: String, deadline: String) -> R<String> {
    if title.trim().is_empty() {
        return Err("the task needs a title".into());
    }
    let kw = s.cfg().todo_keywords.first().cloned();
    Ok(org::task_entry(&title, kw.as_deref(), priority.as_deref().filter(|p| !p.is_empty()), tags, opt_date(&scheduled)?, opt_date(&deadline)?))
}

/// Human preview of a date input, for live feedback while typing.
#[tauri::command]
fn date_preview(input: String) -> String {
    match opt_date(&input) {
        Ok(None) => String::new(),
        Ok(Some((d, t))) => format!("{}{}", d.format("%A %-d %B %Y"), t.map(|t| format!(", {t}")).unwrap_or_default()),
        Err(_) => "✗ can't read this date".into(),
    }
}

#[tauri::command]
fn capture_insert(s: State<App>, text: String, heading: Option<String>, entry: String) -> String {
    org::capture_insert(&text, heading.as_deref(), &entry, &s.kw())
}

/// Config templates, then one per .org file in <notes>/<templates_dir>/.
#[tauri::command]
fn capture_templates(s: State<App>) -> Vec<config::Template> {
    let c = s.cfg();
    let mut paths: Vec<PathBuf> = c.templates_path().and_then(|d| std::fs::read_dir(d).ok()).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "org")).collect();
    paths.sort();
    let files = paths.iter().filter_map(|p| Some(config::Template::from_file(&p.file_stem()?.to_string_lossy(), &std::fs::read_to_string(p).ok()?)));
    c.templates.into_iter().chain(files).collect()
}

#[tauri::command]
fn template_prompts(body: String) -> Vec<String> {
    org::template_prompts(&body)
}

/// Expand template BODY with ANSWERS and file it into TEXT: [new text, cursor line, cursor column].
#[tauri::command]
fn capture_template(s: State<App>, text: String, heading: Option<String>, body: String, answers: std::collections::HashMap<String, String>, selection: String) -> (String, usize, usize) {
    let entry = org::expand_template(&body, timeclock::now(), &selection, |p| answers.get(p).cloned().unwrap_or_default());
    org::capture_template(&text, heading.as_deref(), &entry, &s.kw())
}

/// Resolve a strftime file pattern under the notes dir for DATE_INPUT (default
/// now), creating it as an org-roam note titled after its name if missing.
#[tauri::command]
fn capture_path(s: State<App>, file: String, date_input: Option<String>) -> R<PathBuf> {
    let t = match date_input {
        Some(i) => date(&i)?.and_time(timeclock::now().time()),
        None => timeclock::now(),
    };
    let p = s.cfg().notes().join(org::fmt_time(t, &file).ok_or_else(|| format!("bad date format in {file}"))?);
    s.check(&p)?;
    if !p.exists() {
        let title = p.file_stem().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
        notes::write_note(&p, &title).map_err(|e| e.to_string())?;
    }
    Ok(p)
}

/// INPUT is org-read-date style; "rm" removes the entry.
#[tauri::command]
fn org_planning(text: String, line: usize, kind: String, input: String) -> R<String> {
    let d = if input.trim() == "rm" { None } else { Some(org::read_date(&input, today()).ok_or("can't parse date")?) };
    Ok(org::set_planning(&text, line, &kind, d))
}

#[tauri::command]
fn read_date(input: String) -> R<String> {
    date(&input).map(|d| d.to_string())
}

/// Category and title of the heading at LINE (for clock-in suggestions).
#[tauri::command]
fn org_context(s: State<App>, path: PathBuf, text: String, line: usize) -> Value {
    let f = org::parse(&path, &text, &s.kw());
    match f.headlines.iter().rev().find(|h| h.line <= line) {
        Some(h) => json!({ "category": h.category, "title": h.title }),
        None => json!({ "category": f.category, "title": f.title }),
    }
}

// ---------------------------------------------------------------- notes

#[tauri::command]
fn notes_new(s: State<App>, title: String) -> R<notes::Node> {
    let (path, id) = notes::new_note(&s.cfg().notes(), &title).map_err(|e| e.to_string())?;
    Ok(notes::Node { id, title, path, line: 0 })
}

#[tauri::command]
fn notes_nodes(s: State<App>) -> Vec<notes::Node> {
    notes::nodes(&s.files())
}

#[tauri::command]
fn notes_backlinks(s: State<App>, path: PathBuf) -> Vec<notes::Hit> {
    notes::backlinks(&s.files(), &path)
}

#[tauri::command]
fn notes_graph(s: State<App>, path: PathBuf) -> notes::Graph {
    notes::graph(&s.files(), &path, 2)
}

#[tauri::command]
fn notes_unlinked(s: State<App>, path: PathBuf) -> Vec<notes::Mention> {
    notes::unlinked(&s.files(), &path)
}

#[tauri::command]
fn notes_ensure_id(text: String) -> (String, String) {
    notes::ensure_id(&text)
}

#[tauri::command]
fn notes_search(s: State<App>, query: String) -> Vec<notes::Hit> {
    notes::search(&s.files(), &query, 300)
}

// ---------------------------------------------------------------- timeclock

#[tauri::command]
fn tc_status(s: State<App>) -> Value {
    let (cur, on_break, today) = s.tc().status();
    json!({
        "profile": s.profile(),
        "project": cur.as_ref().map(|c| &c.1), "since": cur.as_ref().map(|c| c.0), "task": cur.as_ref().map(|c| &c.2),
        "on_break": on_break, "today": format_hm(today), "backup_error": backup::last_failure(),
    })
}

/// Everything the time dashboard shows, for the current week (Mon–Sun).
#[tauri::command]
fn tc_dashboard(s: State<App>) -> Value {
    use chrono::Datelike;
    let tc = s.tc();
    let t = today();
    let sessions = tc.sessions();
    let projects = tc.projects();
    let (cur, on_break, today_h) = tc.status();
    let monday = t - Duration::days(t.weekday().num_days_from_monday() as i64);
    let sunday = monday + Duration::days(6);
    let in_week = |d: NaiveDate| monday <= d && d <= sunday;
    let week: Vec<Value> = (0..7)
        .map(|i| {
            let d = monday + Duration::days(i);
            let h: f64 = if d == t { today_h } else { sessions.iter().filter(|x| x.date == d).map(|x| x.hours).sum() };
            json!({ "date": d, "hours": h, "expected": timeclock::expected_hours(d, tc.expected) })
        })
        .collect();
    let merged = timeclock::prepare_report_sessions(&sessions);
    let rounded = timeclock::apply_carry(&merged, &projects).0;
    let mut by: std::collections::BTreeMap<String, (f64, f64)> = Default::default();
    merged.iter().filter(|x| in_week(x.date)).for_each(|x| by.entry(x.project.clone()).or_default().0 += x.hours);
    rounded.iter().filter(|x| in_week(x.date)).for_each(|x| by.entry(x.project.clone()).or_default().1 += x.hours);
    let mut per_project: Vec<Value> = by
        .into_iter()
        .map(|(p, (w, b))| json!({ "project": if p.is_empty() { "Other".to_string() } else { p.clone() }, "code": timeclock::export_code(&p, &projects), "worked": w, "billable": b }))
        .collect();
    per_project.sort_by(|a, b| b["worked"].as_f64().unwrap_or(0.0).total_cmp(&a["worked"].as_f64().unwrap_or(0.0)));
    let (flex_total, flex_week, _) = timeclock::flex(&sessions, &projects, tc.expected, Some((monday, t)));
    json!({
        "profile": s.profile(), "profiles": s.cfg().profiles,
        "project": cur.as_ref().map(|c| &c.1), "since": cur.as_ref().map(|c| c.0), "task": cur.as_ref().map(|c| &c.2), "on_break": on_break,
        "today_hours": today_h, "today": sessions.iter().filter(|x| x.date == t).collect::<Vec<_>>(),
        "week": week, "projects": per_project, "flex_total": flex_total, "flex_week": flex_week, "backup_error": backup::last_failure(),
    })
}

#[tauri::command]
fn tc_projects(s: State<App>) -> timeclock::Projects {
    s.tc().projects()
}

#[tauri::command]
fn tc_save_project(s: State<App>, name: String, project: Project) -> R<()> {
    let tc = s.tc();
    let mut p = tc.projects();
    p.insert(name, project);
    tc.save_projects(&p)
}

#[tauri::command]
fn tc_suggestions(s: State<App>, project: Option<String>) -> Vec<String> {
    timeclock::task_suggestions(&s.tc().sessions(), project.as_deref(), today())
}

#[tauri::command]
fn tc_in(s: State<App>, project: String, task: String, export_code: Option<String>) -> R<String> {
    let switched = s.tc().clock_in(&project, &task, export_code.as_deref())?;
    if switched {
        s.backup();
    }
    Ok(if project.trim().is_empty() { "⏱ Clocked in".into() } else { format!("⏱ Clocked in on: {project}") })
}

#[tauri::command]
fn tc_out(s: State<App>, note: String) -> R<String> {
    if !s.tc().clock_out(&note)? {
        return Ok("Not clocked in.".into());
    }
    s.backup();
    Ok(if note.trim().is_empty() { "⏱ Clocked out.".into() } else { format!("⏱ Clocked out. Task: {note}") })
}

/// Idle prompt: clock out at SINCE, and back in at BACK unless stopping.
#[tauri::command]
fn tc_idle(s: State<App>, since: NaiveDateTime, back: Option<NaiveDateTime>, note: String) -> R<String> {
    if !s.tc().discard_idle(since, back, &note)? {
        return Ok("Not clocked in.".into());
    }
    s.backup();
    Ok(if back.is_some() { "⏱ Idle time discarded.".into() } else { "⏱ Idle time discarded and clocked out.".into() })
}

#[tauri::command]
fn tc_break(s: State<App>) -> R<String> {
    Ok(match s.tc().take_break()? {
        Some(p) => format!("☕ Clock paused! Project '{p}' saved."),
        None => "You are not clocked in right now!".into(),
    })
}

#[tauri::command]
fn tc_resume(s: State<App>) -> R<String> {
    Ok(match s.tc().resume()? {
        Some(p) => format!("▶ Clock is ticking again on: {p}"),
        None => "No valid break found to resume from.".into(),
    })
}

#[tauri::command]
fn tc_adjust(s: State<App>, minutes: i64) -> R<String> {
    Ok(if s.tc().adjust_start(minutes)? { format!("⏪ Start time moved back by {minutes} minutes.") } else { "⚠ You must be clocked in to adjust the start time!".into() })
}

#[tauri::command]
fn tc_sessions_on(s: State<App>, date_input: String) -> R<Vec<timeclock::Session>> {
    let d = date(&date_input)?;
    Ok(s.tc().sessions().into_iter().filter(|x| x.date == d && x.line.is_some()).collect())
}

#[tauri::command]
fn tc_edit_session(s: State<App>, line: usize, note: String, old_hours: f64, new_hours: f64) -> R<String> {
    s.tc().edit_session(line, &note, old_hours, new_hours)?;
    Ok("✅ Session updated.".into())
}

/// Org-formatted report text. KIND: daily | weekly | holidays | flex | doctor | backup.
#[tauri::command]
fn tc_report(s: State<App>, kind: String, date_input: Option<String>) -> R<String> {
    let tc = s.tc();
    let profile = s.profile();
    Ok(match kind.as_str() {
        "daily" => timeclock::daily_report(&tc.sessions(), &tc.projects(), date(date_input.as_deref().unwrap_or(""))?, &profile),
        "weekly" => timeclock::weekly_report(&tc.sessions(), &tc.projects(), today() - Duration::days(7), today(), &profile),
        "holidays" => timeclock::holidays_report(date(date_input.as_deref().unwrap_or(""))?.format("%Y").to_string().parse().unwrap()),
        "flex" => {
            let t = today();
            let week = (t - Duration::days(7), t);
            let (total, period, days) = timeclock::flex(&tc.sessions(), &tc.projects(), tc.expected, Some(week));
            format!("Flex ({profile}): total {total:+.2} h, last 7 days {period:+.2} h over {days} worked days")
        }
        "doctor" => {
            let issues = timeclock::doctor(&tc.read_log(), timeclock::now());
            let body = if issues.is_empty() { "- ✅ No issues found.".to_string() } else { issues.iter().map(|i| format!("- ⚠ {i}")).collect::<Vec<_>>().join("\n") };
            format!("#+TITLE: Timeclock Doctor — {profile}\n\n{body}\n")
        }
        "backup" => format!("#+TITLE: Backup Log\n\n{}\n", backup::log_text()),
        k => return Err(format!("unknown report {k}")),
    })
}

#[tauri::command]
fn tc_csv(s: State<App>, start: String, end: String, path: PathBuf) -> R<String> {
    let tc = s.tc();
    std::fs::write(&path, timeclock::csv(&tc.sessions(), &tc.projects(), date(&start)?, date(&end)?)).map_err(|e| e.to_string())?;
    Ok(format!("✅ CSV exported to {}", path.display()))
}

// ---------------------------------------------------------------- export

/// NAME in export_dir. If it exists, OVERWRITE None refuses with `exists:<path>` (so the
/// frontend can ask), Some(true) reuses it, Some(false) picks a free `name (2).ext`.
fn export_path(s: &App, name: &str, overwrite: Option<bool>) -> R<PathBuf> {
    let dir = config::expand(&s.cfg().export_dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let p = dir.join(name);
    match overwrite {
        _ if !p.exists() => Ok(p),
        None => Err(format!("exists:{}", p.display())),
        Some(true) => Ok(p),
        Some(false) => Ok(dir.join(export::free_name(name, |n| dir.join(n).exists()))),
    }
}

fn write_export(s: &App, name: &str, body: String, overwrite: Option<bool>) -> R<PathBuf> {
    let p = export_path(s, name, overwrite)?;
    config::write_atomic(&p, body).map_err(|e| e.to_string())?;
    Ok(p)
}

fn stem(path: &Path) -> String {
    path.file_stem().map_or("note".into(), |x| x.to_string_lossy().to_string())
}

/// Export note TEXT (the buffer of PATH) to export_dir. FORMAT: html | md | pdf (HTML that opens the print dialog).
#[tauri::command]
fn export_note(s: State<App>, path: PathBuf, text: String, format: String, overwrite: Option<bool>) -> R<PathBuf> {
    let d = export::parse(&text, &s.kw());
    let stem = stem(&path);
    let base = path.parent().unwrap_or(Path::new(""));
    match format.as_str() {
        "md" => write_export(&s, &format!("{stem}.md"), export::markdown(&d, &stem, base), overwrite),
        f => write_export(&s, &format!("{stem}.html"), export::html(&d, &stem, base, &export::Ids::new(), f == "pdf"), overwrite),
    }
}

/// Export note TEXT (the buffer of PATH) and the notes it reaches by id: links within DEPTH hops
/// as HTML pages in `<export_dir>/<stem>/`, id: links between them made relative. Returns PATH's page.
#[tauri::command]
fn export_linked(s: State<App>, path: PathBuf, text: String, depth: usize, overwrite: Option<bool>) -> R<PathBuf> {
    let kw = s.kw();
    let root = org::parse(&path, &text, &kw);
    let files = s.files();
    let files: Vec<&org::OrgFile> = files.iter().map(|f| &**f).collect();
    let (pages, ids) = export::bundle(&files, &root, depth);
    let dir = export_path(&s, &stem(&path), overwrite)?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    for (f, name) in &pages {
        let html = export::html(&export::parse(&f.text, &kw), &stem(&f.path), f.path.parent().unwrap_or(Path::new("")), &ids, false);
        config::write_atomic(&dir.join(name), html).map_err(|e| e.to_string())?;
    }
    Ok(dir.join(&pages[0].1))
}

/// The time report for START..END as an HTML page in export_dir; PRINT opens the print dialog.
#[tauri::command]
fn export_report(s: State<App>, start: String, end: String, print: bool, overwrite: Option<bool>) -> R<PathBuf> {
    let tc = s.tc();
    let (start, end) = (date(&start)?, date(&end)?);
    let org = timeclock::weekly_report(&tc.sessions(), &tc.projects(), start, end, &s.profile());
    let html = export::html(&export::parse(&org, &s.kw()), "Time report", Path::new(""), &export::Ids::new(), print);
    write_export(&s, &format!("time_report_{}_{start}_{end}.html", s.profile().to_lowercase()), html, overwrite)
}

/// Open an exported file with its default app (only files in export_dir or a folder in it).
#[tauri::command]
fn export_open(app: AppHandle, s: State<App>, path: PathBuf) -> R<()> {
    use tauri_plugin_opener::OpenerExt;
    let dir = config::expand(&s.cfg().export_dir);
    let parent = path.parent();
    if path.components().any(|c| c == std::path::Component::ParentDir) || !(parent == Some(dir.as_path()) || parent.and_then(Path::parent) == Some(dir.as_path())) {
        return Err(format!("{} is not in the export folder", path.display()));
    }
    app.opener().open_path(path.to_string_lossy(), None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
fn tc_switch_profile(s: State<App>, name: String) -> R<String> {
    if !s.cfg().profiles.contains(&name) {
        return Err(format!("unknown profile {name}"));
    }
    if name == s.profile() {
        return Ok(format!("Already on {name}"));
    }
    if s.tc().clock_out(&format!("Auto-clockout (Switched to {name} profile)"))? {
        s.backup();
    }
    *s.profile.lock().unwrap() = name.clone();
    let _ = std::fs::create_dir_all(s.cfg().data());
    std::fs::write(s.profile_file(), &name).map_err(|e| e.to_string())?;
    Ok(format!("🔄 Profile switched to: {name}"))
}

#[tauri::command]
fn tc_import(s: State<App>, dir: String) -> R<String> {
    let n = s.tc().import_emacs(&config::expand(&dir), &s.profile())?;
    Ok(format!("Imported {n} events into {}", s.profile()))
}

#[tauri::command]
fn backup_now(s: State<App>) -> String {
    s.backup();
    backup::last_failure().map_or("☁ Backup done.".into(), |f| format!("⚠ Backup failed: {f}"))
}

/// Rewrite the calendar feed if it's enabled and changed (unchanged, so no watcher loop).
fn write_calendar(app: &AppHandle) {
    let s = app.state::<App>();
    let c = s.cfg();
    if c.calendar_file.is_empty() {
        return;
    }
    let path = config::expand(&c.calendar_file);
    let text = ics::feed(&s.files(), &s.kw());
    if std::fs::read_to_string(&path).ok().as_deref() != Some(&text) {
        if let Err(e) = std::fs::write(&path, text) {
            eprintln!("calendar {}: {e}", path.display());
        }
    }
}

/// Quick capture: show the new-task form, telling the frontend whether the window was hidden.
fn capture(app: &AppHandle) {
    let hidden = app.get_webview_window("main").is_some_and(|w| !w.is_visible().unwrap_or(true));
    tray::show(app);
    let _ = app.emit("capture", hidden);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // A second launch hands over to the running app; `margin --capture` is the
        // quick-capture hook for desktops where global shortcuts don't work (Wayland).
        .plugin(tauri_plugin_single_instance::init(|app, args, _| {
            if args.iter().any(|a| a == "--capture") {
                capture(app)
            } else {
                tray::show(app)
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            #[cfg(desktop)]
            app.handle().plugin(tauri_plugin_updater::Builder::new().build())?;
            let cfg_path = app.path().app_config_dir()?.join("config.toml");
            let cfg = Config::load(&cfg_path).unwrap_or_else(|e| {
                eprintln!("{e}; using defaults");
                Config::default()
            });
            let saved = std::fs::read_to_string(cfg.data().join("active-profile.txt")).unwrap_or_default();
            let profile = cfg.profiles.iter().find(|p| **p == saved.trim()).or(cfg.profiles.first()).cloned().unwrap_or("Work".into());
            app.manage(App { cfg_path, cfg: Mutex::new(cfg), profile: Mutex::new(profile), cache: Default::default(), watcher: Mutex::new(None) });
            watch(app.handle());
            write_calendar(app.handle());
            tray::setup(app.handle())?;
            app.handle().plugin(tauri_plugin_notification::init())?;
            remind::start(app.handle());
            #[cfg(desktop)]
            {
                idle::start(app.handle());
                use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_handler(|app, _, ev| {
                            if ev.state == ShortcutState::Pressed {
                                capture(app)
                            }
                        })
                        .build(),
                )?;
                let key = app.state::<App>().cfg().capture_shortcut;
                if !key.is_empty() {
                    if let Err(e) = app.global_shortcut().register(key.as_str()) {
                        eprintln!("quick capture shortcut {key}: {e}");
                    }
                }
            }
            Ok(())
        })
        .on_window_event(|w, ev| {
            if let WindowEvent::CloseRequested { api, .. } = ev {
                let app = w.app_handle();
                if app.state::<App>().cfg().close_to_tray && tray::tracking(app) {
                    api.prevent_close();
                    let _ = w.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            config, tutorial, reload_config, save_view, list_files, read_file, write_file, attach_file, attach_bytes, unused_attachments, trash_attachments,
            agenda, todos, search_todos, org_heading, org_edit, org_planning, read_date, org_context,
            org_targets, org_tags, org_refile, org_refile_same, org_archive, capture_insert, capture_path, capture_templates, template_prompts, capture_template, task_entry, date_preview, tc_dashboard,
            notes_new, notes_nodes, notes_backlinks, notes_search, notes_graph, notes_unlinked, notes_ensure_id,
            tc_status, tc_projects, tc_save_project, tc_suggestions, tc_in, tc_out, tc_idle, tc_break, tc_resume, tc_adjust,
            tc_sessions_on, tc_edit_session, tc_report, tc_csv, tc_switch_profile, tc_import, backup_now,
            export_note, export_linked, export_report, export_open
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, ev| {
            // Replaces kill-emacs-hook: commit on exit.
            match ev {
                RunEvent::ExitRequested { .. } => app.state::<App>().backup(),
                #[cfg(target_os = "macos")]
                RunEvent::Reopen { .. } => tray::show(app), // dock icon click after hiding to tray
                _ => {}
            }
        });
}
