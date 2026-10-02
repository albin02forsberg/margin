mod backup;
mod config;
mod notes;
mod org;
mod timeclock;

use chrono::{Duration, NaiveDate};
use config::Config;
use serde_json::{json, Value};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::{Manager, RunEvent, State};
use timeclock::{format_hm, Project, Tc};

type R<T> = Result<T, String>;

struct App {
    cfg_path: PathBuf,
    cfg: Mutex<Config>,
    profile: Mutex<String>,
    cache: org::Cache,
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
    fn files(&self) -> Vec<Arc<org::OrgFile>> {
        org::scan(&self.cfg().notes(), &self.kw(), &self.cache)
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
fn reload_config(s: State<App>) -> R<()> {
    *s.cfg.lock().unwrap() = Config::load(&s.cfg_path)?;
    s.cache.clear();
    Ok(())
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
fn write_file(s: State<App>, path: PathBuf, text: String) -> R<()> {
    s.check(&path)?;
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, text).map_err(|e| e.to_string())?;
    if path == s.cfg_path {
        reload_config(s)?;
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
    org::todos(&s.files(), &s.kw())
}

#[tauri::command]
fn org_cycle(s: State<App>, text: String, line: usize, dir: i32) -> String {
    org::cycle_todo(&text, line, &s.kw(), dir, today())
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
        "weekly" => timeclock::weekly_report(&tc.sessions(), &tc.projects(), today(), &profile),
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let cfg_path = app.path().app_config_dir()?.join("config.toml");
            let cfg = Config::load(&cfg_path).unwrap_or_else(|e| {
                eprintln!("{e}; using defaults");
                Config::default()
            });
            let saved = std::fs::read_to_string(cfg.data().join("active-profile.txt")).unwrap_or_default();
            let profile = cfg.profiles.iter().find(|p| **p == saved.trim()).or(cfg.profiles.first()).cloned().unwrap_or("Work".into());
            app.manage(App { cfg_path, cfg: Mutex::new(cfg), profile: Mutex::new(profile), cache: Default::default() });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            config, reload_config, list_files, read_file, write_file,
            agenda, todos, org_cycle, org_planning, read_date, org_context,
            notes_new, notes_nodes, notes_backlinks, notes_search,
            tc_status, tc_projects, tc_save_project, tc_suggestions, tc_in, tc_out, tc_break, tc_resume, tc_adjust,
            tc_sessions_on, tc_edit_session, tc_report, tc_csv, tc_switch_profile, tc_import, backup_now
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, ev| {
            // Replaces kill-emacs-hook: commit on exit.
            if let RunEvent::ExitRequested { .. } = ev {
                app.state::<App>().backup();
            }
        });
}
