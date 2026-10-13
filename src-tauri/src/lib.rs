mod activity;
mod ai;
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
mod watcher;

use chrono::{Duration, NaiveDate, NaiveDateTime};
use config::Config;
use serde_json::{json, Value};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use notify_debouncer_mini::{new_debouncer, notify, DebounceEventResult, Debouncer};
use tauri::{AppHandle, Emitter, Manager, RunEvent, State, WindowEvent};
use timeclock::{format_hm, Project, Tc};

type R<T> = Result<T, String>;

struct App {
    cfg_path: PathBuf,
    cfg: Mutex<Config>,
    /// Why config.toml couldn't be loaded (the last good settings, or the defaults at startup, stay); cleared by a good reload.
    cfg_error: Mutex<Option<String>>,
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
        check_in(&[c.notes(), c.data(), self.cfg_path.parent().unwrap().to_path_buf()], p)
    }
}

/// App::check's rule: P has no `..` and lies under one of ROOTS (compared as written, never resolved).
fn check_in(roots: &[PathBuf], p: &Path) -> R<()> {
    if p.components().any(|c| c == Component::ParentDir) || !roots.iter().any(|r| p.starts_with(r)) {
        return Err(format!("{} is outside the notes/data/config dirs", p.display()));
    }
    Ok(())
}

/// P lies under NOTES (the stricter scope of note renames; check_in also allows data and config).
fn in_notes(notes: &Path, p: &Path) -> R<()> {
    if !p.starts_with(notes) {
        return Err(format!("{} isn't in the notes folder", p.display()));
    }
    Ok(())
}

fn today() -> NaiveDate {
    timeclock::now().date()
}

fn date(input: &str) -> R<NaiveDate> {
    date_at(input, today())
}

fn date_at(input: &str, today: NaiveDate) -> R<NaiveDate> {
    org::read_date(input, today).map(|d| d.0).ok_or_else(|| format!("can't parse date: {input}"))
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
        "config": c, "defaults": Config::default(), "config_path": s.cfg_path, "notes": c.notes(), "data": c.data(), "export": config::expand(&c.export_dir),
        "profile": s.profile(), "log_path": tc.log_path(), "projects_path": tc.projects_path(), "diary_path": tc.diary, "config_error": s.cfg_error.lock().unwrap().clone(),
        "config_missing": matches!(s.cfg_path.try_exists(), Ok(false)),
    })
}

/// Write the current settings to a missing config.toml (never over one that exists again) and reload.
#[tauri::command]
fn config_recreate(app: AppHandle, s: State<App>) -> R<String> {
    config::recreate(&s.cfg_path, &s.cfg())?;
    reload_config(app, s)
}

/// Re-read config.toml and apply it. Before the swap the open session is clocked out of the
/// log it started in if the folders or profile change (whoever edited the file); afterwards the
/// profile falls back to the first one if gone. Returns a warning (or "") for a shortcut that
/// won't register and for files left behind in old folders. A broken or missing file keeps the current settings (and is reported via `cfg_error`).
#[tauri::command(async)]
fn reload_config(app: AppHandle, s: State<App>) -> R<String> {
    let new = Config::load(&s.cfg_path).inspect_err(|e| *s.cfg_error.lock().unwrap() = Some(e.clone()))?;
    let (old, profile) = (s.cfg(), s.profile());
    if clock_out_before_setup(&s.tc(), &old, &new, &profile)? {
        s.backup();
    }
    *s.cfg.lock().unwrap() = new.clone();
    *s.cfg_error.lock().unwrap() = None;
    s.cache.clear();
    watch(&app);
    // A shortcut that won't register is reported, but the rest still applies.
    let rebind = rebind_capture(&app, &old.capture_shortcut, &new.capture_shortcut).err();
    if let Some(p) = new.profiles.first().filter(|_| !new.profiles.contains(&profile)) {
        tc_switch_profile(s, p.clone())?;
    }
    Ok(rebind.into_iter().chain([orphan_warning(&old, &new)]).filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" "))
}

/// Move the quick-capture shortcut from OLD to NEW (either may be empty = none).
#[cfg(desktop)]
fn rebind_capture(app: &AppHandle, old: &str, new: &str) -> R<()> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    if old == new {
        return Ok(());
    }
    let gs = app.global_shortcut();
    if !old.is_empty() {
        let _ = gs.unregister(old); // may never have registered (e.g. Wayland)
    }
    if new.is_empty() {
        return Ok(());
    }
    gs.register(new).map_err(|e| format!("quick capture shortcut {new}: {e}"))
}
#[cfg(not(desktop))]
fn rebind_capture(_: &AppHandle, _: &str, _: &str) -> R<()> {
    Ok(())
}

/// Add a `[[views]]` entry to config.toml (or RENAME the one with QUERY), leaving the rest of the file as written.
/// Returns reload_config's warning (or "").
#[tauri::command]
fn save_view(app: AppHandle, s: State<App>, name: String, query: String, rename: bool) -> R<String> {
    let text = std::fs::read_to_string(&s.cfg_path).map_err(|e| e.to_string())?;
    let new = if rename { config::rename_view(&text, &query, &name)? } else { config::add_view(&text, &name, &query)? };
    config::write_atomic(&s.cfg_path, new).map_err(|e| e.to_string())?;
    reload_config(app, s)
}

/// The setup prompts' answers (see config::setup): create the folders, save, reload, and
/// move to the first profile if the current one is gone. Returns a warning (or "") when
/// the old folders' files are left behind; nothing is moved.
#[tauri::command]
fn setup(app: AppHandle, s: State<App>, notes_dir: String, data_dir: String, profiles: String) -> R<String> {
    let text = std::fs::read_to_string(&s.cfg_path).map_err(|e| e.to_string())?;
    let (text, c) = config::setup(&text, &notes_dir, &data_dir, &profiles)?;
    apply_config(app, s, text, c)
}

/// The settings page's changed VALUES, RESET keys (see config::save), saved VIEWS and capture
/// TEMPLATES (see config::set_tables), saved like setup's answers.
#[tauri::command]
fn save_config(app: AppHandle, s: State<App>, values: toml::Table, reset: Option<Vec<String>>, views: Option<Vec<toml::Table>>, templates: Option<Vec<toml::Table>>) -> R<String> {
    let text = std::fs::read_to_string(&s.cfg_path).map_err(|e| e.to_string())?;
    let (mut text, mut c) = config::save(&text, values, &reset.unwrap_or_default())?;
    for (key, items) in [("views", views), ("templates", templates)] {
        if let Some(v) = items {
            (text, c) = config::set_tables(&text, key, v)?;
        }
    }
    apply_config(app, s, text, c)
}

/// Write config TEXT (parsed: C) and reload; see setup and reload_config.
fn apply_config(app: AppHandle, s: State<App>, text: String, c: Config) -> R<String> {
    for d in [c.notes(), c.data()] {
        std::fs::create_dir_all(&d).map_err(|e| format!("{}: {e}", d.display()))?;
    }
    config::write_atomic(&s.cfg_path, text).map_err(|e| e.to_string())?;
    reload_config(app, s)
}

/// Clock out OLD_TC if the config moves the notes or data folder or drops PROFILE (the config
/// then points elsewhere, and the clock-out would land in the wrong log). An empty list drops nothing.
fn clock_out_before_setup(old_tc: &Tc, old: &Config, new: &Config, profile: &str) -> R<bool> {
    if old.notes() == new.notes() && old.data() == new.data() && (new.profiles.is_empty() || new.profiles.iter().any(|p| p == profile)) {
        return Ok(false);
    }
    old_tc.clock_out("Auto-clockout (Config changed folders or profiles)")
}

/// Which of the old notes/data folders and dropped profiles' time logs still hold files
/// that the new settings no longer point at (empty string if none).
fn orphan_warning(old: &Config, new: &Config) -> String {
    let has_files = |p: &Path| std::fs::read_dir(p).is_ok_and(|mut d| d.next().is_some());
    let mut left: Vec<PathBuf> = vec![];
    for (o, n) in [(old.notes(), new.notes()), (old.data(), new.data())] {
        if o != n && has_files(&o) && !has_files(&n) {
            left.push(o);
        }
    }
    if old.data() == new.data() {
        let dropped = old.profiles.iter().filter(|p| !new.profiles.contains(p));
        left.extend(dropped.map(|p| old.data().join(p.to_lowercase())).filter(|d| has_files(d)));
    }
    left.dedup();
    if left.is_empty() {
        return String::new();
    }
    let list: Vec<_> = left.iter().map(|p| p.display().to_string()).collect();
    format!("Files stay in {} (nothing was moved); copy them over if you need them.", list.join(", "))
}

/// Whether CFG (the file or, for a symlink, its target CFG_REAL) is among PATHS, and the PATHS under ROOTS (minus .git), sorted and deduplicated.
fn split_changes(paths: Vec<PathBuf>, cfg: &Path, cfg_real: &Path, roots: &[PathBuf]) -> (bool, Vec<PathBuf>) {
    let cfg_changed = paths.iter().any(|p| p == cfg || p == cfg_real);
    let mut rest: Vec<PathBuf> = paths.into_iter().filter(|p| roots.iter().any(|r| p.starts_with(r)) && !p.components().any(|c| c.as_os_str() == ".git")).collect();
    rest.sort();
    rest.dedup();
    (cfg_changed, rest)
}

/// Watch the notes and data dirs; the UI gets an "fs-changed" event (with the
/// changed paths) whenever anything there changes — our own writes, Emacs, sync tools.
/// config.toml edited elsewhere (so it no longer matches the loaded config) gets "config-changed".
fn watch(app: &AppHandle) {
    let s = app.state::<App>();
    let c = s.cfg();
    let handle = app.clone();
    let (cfg_path, roots) = (s.cfg_path.clone(), [c.notes(), c.data()]);
    let cfg_real = std::fs::canonicalize(&cfg_path).unwrap_or_else(|_| cfg_path.clone());
    let real = cfg_real.clone();
    let debouncer = new_debouncer(std::time::Duration::from_millis(150), move |res: DebounceEventResult| {
        let Ok(events) = res else { return };
        let (cfg_changed, paths) = split_changes(events.into_iter().map(|e| e.path).collect(), &cfg_path, &real, &roots);
        if cfg_changed {
            let disk = std::fs::read_to_string(&cfg_path).ok().and_then(|t| toml::from_str::<Config>(&t).ok());
            let s = handle.state::<App>();
            // A load error (cfg_error) means the file was broken or gone: a fix always reloads.
            if disk.and_then(|d| toml::Table::try_from(d).ok()) != toml::Table::try_from(s.cfg()).ok() || s.cfg_error.lock().unwrap().is_some() {
                let _ = handle.emit("config-changed", ());
            }
        }
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
    // A symlinked config.toml: edits to its target land in the target's directory.
    let mut dirs = vec![s.cfg_path.parent().unwrap(), cfg_real.parent().unwrap()];
    dirs.dedup();
    for dir in dirs {
        if let Err(e) = d.watcher().watch(dir, notify::RecursiveMode::NonRecursive) {
            eprintln!("can't watch {}: {e}", dir.display());
        }
    }
    *s.watcher.lock().unwrap() = Some(d);
}

/// Where nightly builds (nightly.yml) publish their update manifest.
const NIGHTLY_UPDATES: &str = "https://github.com/albin02forsberg/margin/releases/download/nightly/latest.json";

/// The updater plugin's `check`, on the nightly channel when config.toml asks for it (the JS `check`
/// can't change endpoints); the frontend wraps the result in the plugin's `Update`.
#[tauri::command]
async fn update_check(webview: tauri::Webview, s: State<'_, App>) -> R<Option<Value>> {
    use tauri_plugin_updater::UpdaterExt;
    let e = |e: tauri_plugin_updater::Error| e.to_string();
    let mut b = webview.updater_builder();
    if s.cfg().nightly_updates {
        b = b.endpoints(vec![NIGHTLY_UPDATES.parse().unwrap()]).map_err(e)?;
    }
    let Some(u) = b.build().map_err(e)?.check().await.map_err(e)? else { return Ok(None) };
    let mut meta = json!({ "currentVersion": u.current_version, "version": u.version, "body": u.body, "rawJson": u.raw_json });
    meta["rid"] = webview.resources_table().add(u).into();
    Ok(Some(meta))
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
    read_or_empty(&path)
}

/// PATH's text; a missing file reads as "", anything else unreadable (a folder) is an error.
fn read_or_empty(path: &Path) -> R<String> {
    std::fs::read_to_string(path).or_else(|e| if path.exists() { Err(e.to_string()) } else { Ok(String::new()) })
}

/// Whether a save of TEXT may replace DISK (None: no file) when the caller last saw EXPECT
/// (None: overwrite anyway): not if someone else changed the file since. CRLF counts as LF.
fn may_overwrite(disk: Option<&str>, expect: Option<&str>, text: &str) -> bool {
    let (Some(d), Some(e)) = (disk, expect) else { return true };
    let d = d.replace("\r\n", "\n").replace('\r', "\n");
    d == e || d == text
}

/// Held across a note save and a rename's or trash's file moves and link rewrites, so a rename can't
/// write back a linking note over an autosave that landed after it read the note.
static NOTE_WRITES: Mutex<()> = Mutex::new(());

fn note_writes() -> std::sync::MutexGuard<'static, ()> {
    NOTE_WRITES.lock().unwrap_or_else(|e| e.into_inner())
}

/// write_file's save, under NOTE_WRITES so the disk check and the write are one step.
fn write_checked(path: &Path, text: String, expect: Option<&str>) -> R<()> {
    let _lock = note_writes();
    if expect.is_some() && !may_overwrite(std::fs::read_to_string(path).ok().as_deref(), expect, &text) {
        return Err("changed-on-disk".into());
    }
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    config::write_atomic(path, text).map_err(|e| e.to_string())
}

/// Whether PATH exists (read_file reads a missing file as "", which an emptied file also gives).
#[tauri::command]
fn file_exists(s: State<App>, path: PathBuf) -> R<bool> {
    s.check(&path)?;
    Ok(path.exists())
}

/// Returns reload_config's warning when PATH is config.toml, else "". With EXPECT (the text the
/// frontend last saved or loaded), fails with "changed-on-disk" instead of overwriting another app's edit.
#[tauri::command]
fn write_file(app: AppHandle, s: State<App>, path: PathBuf, text: String, expect: Option<String>) -> R<String> {
    s.check(&path)?;
    write_checked(&path, text, expect.as_deref())?;
    if path == s.cfg_path {
        return reload_config(app, s);
    }
    Ok(String::new())
}

/// Copy SRC into NOTE's attachment folder; returns the link target relative to the note.
#[tauri::command]
fn attach_file(s: State<App>, note: PathBuf, src: PathBuf) -> R<String> {
    let name = src.file_name().ok_or("not a file")?.to_string_lossy().to_string();
    let c = s.cfg();
    attach_in(&c.notes(), &c.attachments_dir, &note, &name, |p| p.exists(), |dst| std::fs::copy(&src, dst).map(|_| ()))
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
    let c = s.cfg();
    attach_in(&c.notes(), &c.attachments_dir, Path::new(&header("note")?), &header("name")?, |p| p.exists(), |dst| std::fs::write(dst, bytes))
}

/// Reserves the name with `create_new`, so concurrent attaches of one name don't collide
/// (`taken` may be stale; an `AlreadyExists` retries).
fn attach_in(
    notes: &Path,
    dir: &str,
    note: &Path,
    name: &str,
    taken: impl Fn(&Path) -> bool,
    write: impl FnOnce(&Path) -> std::io::Result<()>,
) -> R<String> {
    loop {
        let (dst, link) = attach::target(notes, dir, note, name, &taken)?;
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

/// Files under ROOT, skipping `.trash`.
fn attachment_files(root: &Path) -> Vec<PathBuf> {
    walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| e.file_name() != ".trash")
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .collect()
}

/// Files under the attachments folder (outside `.trash`) that no note links to.
#[tauri::command]
fn unused_attachments(s: State<App>) -> Vec<PathBuf> {
    let c = s.cfg();
    let files = attachment_files(&c.notes().join(&c.attachments_dir));
    let notes = org::scan(&c.notes(), &s.kw(), &s.cache);
    let notes: Vec<(&Path, &str)> = notes.iter().map(|f| (f.path.as_path(), f.text.as_str())).collect();
    let mut v = attach::unused(&notes, &files);
    v.sort();
    v
}

/// Move FILE (a file under ROOT, outside `ROOT/.trash`) to `ROOT/.trash/<subpath>`, adding `.1`, `.2`… on a
/// name clash. Returns where it went.
fn trash_to(root: &Path, f: &Path) -> R<PathBuf> {
    let rel = f
        .strip_prefix(root)
        .ok()
        .filter(|r| f.is_file() && !r.starts_with(".trash") && r.components().all(|c| matches!(c, Component::Normal(_))))
        .ok_or_else(|| format!("{} can't be moved to {}", f.display(), root.join(".trash").display()))?;
    let trash = root.join(".trash");
    let mut dst = trash.join(rel);
    for i in 1.. {
        if !dst.exists() {
            break;
        }
        dst = trash.join(format!("{}.{i}", rel.display()));
    }
    std::fs::create_dir_all(dst.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::rename(f, &dst).map_err(|e| format!("{}: {e}", f.display()))?;
    Ok(dst)
}

/// Move attachment FILES to `<attachments_dir>/.trash/`, keeping their subpaths.
#[tauri::command]
fn trash_attachments(s: State<App>, files: Vec<PathBuf>) -> R<()> {
    let c = s.cfg();
    let root = c.notes().join(&c.attachments_dir);
    for f in files {
        trash_to(&root, &f)?;
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
    org_edit_op(&text, line, &op, value, &s.kw(), timeclock::now())
}

/// `org_edit` without app state or clock.
fn org_edit_op(text: &str, line: usize, op: &str, value: Option<String>, kw: &org::Kw, now: NaiveDateTime) -> R<String> {
    let v = value.unwrap_or_default();
    let n = || v.trim().parse::<i32>().map_err(|_| format!("bad number: {v}"));
    let opt = || Some(v.trim()).filter(|x| !x.is_empty());
    Ok(match op {
        "cycle" => org::cycle_todo(text, line, kw, n()?, now),
        "keyword" => org::set_keyword(text, line, kw, opt(), now),
        "priority" => org::set_priority(text, line, kw, opt()),
        "priority-cycle" => org::cycle_priority(text, line, kw, n()?),
        "tags" => org::set_tags(text, line, kw, v.split(':').map(|t| t.trim().to_string()).collect()),
        "shift" => {
            let (kind, days) = v.split_once(' ').ok_or("shift needs \"KIND DAYS\"")?;
            org::shift_date(text, line, kind, days.trim().parse().map_err(|_| "bad day count")?)
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
fn opt_date_at(input: &str, today: NaiveDate) -> R<Option<(NaiveDate, Option<String>)>> {
    if input.trim().is_empty() {
        return Ok(None);
    }
    org::read_date(input, today).map(Some).ok_or_else(|| format!("can't read date \"{input}\""))
}

#[tauri::command]
fn task_entry(s: State<App>, title: String, priority: Option<String>, tags: Vec<String>, scheduled: String, deadline: String) -> R<String> {
    task_text(&s.cfg().todo_keywords, &title, priority.as_deref(), tags, &scheduled, &deadline, today())
}

/// The heading `task_entry` files: the first TODO keyword, dates read relative to TODAY.
fn task_text(todo: &[String], title: &str, priority: Option<&str>, tags: Vec<String>, scheduled: &str, deadline: &str, today: NaiveDate) -> R<String> {
    if title.trim().is_empty() {
        return Err("the task needs a title".into());
    }
    Ok(org::task_entry(title, todo.first().map(String::as_str), priority.filter(|p| !p.is_empty()), tags, opt_date_at(scheduled, today)?, opt_date_at(deadline, today)?))
}

/// Human preview of a date input, for live feedback while typing.
#[tauri::command]
fn date_preview(input: String) -> String {
    date_preview_at(&input, today())
}

fn date_preview_at(input: &str, today: NaiveDate) -> String {
    match opt_date_at(input, today) {
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
    let p = capture_file(&s.cfg().notes(), &file, date_input.as_deref(), timeclock::now())?;
    s.check(&p)?;
    if !p.exists() {
        let title = p.file_stem().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
        notes::write_note(&p, &title).map_err(|e| e.to_string())?;
    }
    Ok(p)
}

/// FILE (a strftime pattern) under NOTES for DATE_INPUT at NOW's time, or for NOW.
fn capture_file(notes: &Path, file: &str, date_input: Option<&str>, now: NaiveDateTime) -> R<PathBuf> {
    let t = match date_input {
        Some(i) => date_at(i, now.date())?.and_time(now.time()),
        None => now,
    };
    Ok(notes.join(org::fmt_time(t, file).ok_or_else(|| format!("bad date format in {file}"))?))
}

/// INPUT is org-read-date style; "rm" removes the entry.
#[tauri::command]
fn org_planning(text: String, line: usize, kind: String, input: String) -> R<String> {
    planning_at(&text, line, &kind, &input, today())
}

fn planning_at(text: &str, line: usize, kind: &str, input: &str, today: NaiveDate) -> R<String> {
    let d = if input.trim() == "rm" { None } else { Some(org::read_date(input, today).ok_or("can't parse date")?) };
    Ok(org::set_planning(text, line, kind, d))
}

#[tauri::command]
fn read_date(input: String) -> R<String> {
    date(&input).map(|d| d.to_string())
}

/// Category and title of the heading at LINE (for clock-in suggestions).
#[tauri::command]
fn org_context(s: State<App>, path: PathBuf, text: String, line: usize) -> Value {
    context_of(&org::parse(&path, &text, &s.kw()), line)
}

fn context_of(f: &org::OrgFile, line: usize) -> Value {
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

/// Every note's title (`#+title`, else the file name), with or without an ID.
#[tauri::command]
fn note_titles(s: State<App>) -> std::collections::HashMap<PathBuf, String> {
    s.files().iter().map(|f| (f.path.clone(), f.title.clone())).collect()
}

#[tauri::command]
fn notes_nodes(s: State<App>) -> Vec<notes::Node> {
    notes::nodes(&s.files())
}

#[tauri::command(async)]
fn notes_backlinks(s: State<App>, path: PathBuf) -> Vec<notes::Hit> {
    notes::backlinks(&s.files(), &path)
}

/// Rename note FROM to NAME in its folder and fix the `[[file:]]` links to it (see notes::rename),
/// after a backup. Returns the new path and the notes whose links changed.
#[tauri::command(async)]
fn rename_note(s: State<App>, from: PathBuf, name: String) -> R<(PathBuf, Vec<PathBuf>)> {
    let notes = s.cfg().notes();
    for p in [from.clone(), from.with_file_name(name.trim())] {
        s.check(&p)?;
        in_notes(&notes, &p)?;
    }
    s.backup();
    let _lock = note_writes();
    notes::rename(&s.files(), &from, &name)
}

/// Move the note PATH to `<notes>/.trash/`, keeping its subpath (after a backup). Returns where it went.
#[tauri::command(async)]
fn trash_note(s: State<App>, path: PathBuf) -> R<PathBuf> {
    s.check(&path)?;
    if path.extension().and_then(|e| e.to_str()) != Some("org") {
        return Err(format!("{} isn't a note", path.display()));
    }
    s.backup();
    let _lock = note_writes();
    trash_to(&s.cfg().notes(), &path)
}

#[tauri::command(async)]
fn notes_graph(s: State<App>, path: PathBuf) -> notes::Graph {
    notes::graph(&s.files(), &path, 2)
}

#[tauri::command(async)]
fn notes_unlinked(s: State<App>, path: PathBuf) -> Vec<notes::Mention> {
    notes::unlinked(&s.files(), &path)
}

#[tauri::command]
fn notes_ensure_id(text: String) -> (String, String) {
    notes::ensure_id(&text)
}

#[tauri::command(async)]
fn notes_search(s: State<App>, query: String) -> Vec<notes::Hit> {
    notes::search(&s.files(), &query, 300)
}

// ---------------------------------------------------------------- timeclock

#[tauri::command]
fn tc_status(s: State<App>) -> Value {
    status_json(&s.profile(), s.tc().status(), backup::last_failure())
}

type Status = (Option<(NaiveDateTime, String, String)>, Option<String>, f64);

/// The sidebar clock: PROFILE, Tc::status and the last backup failure.
fn status_json(profile: &str, (cur, on_break, today): Status, backup_error: Option<String>) -> Value {
    json!({
        "profile": profile,
        "project": cur.as_ref().map(|c| &c.1), "since": cur.as_ref().map(|c| c.0), "task": cur.as_ref().map(|c| &c.2),
        "on_break": on_break, "today": format_hm(today), "backup_error": backup_error,
    })
}

/// When to leave today (see timeclock::leave_time), for the open clock and today's hours.
fn leave_time(tc: &Tc, running: bool, on_break: bool, today_h: f64) -> String {
    let leave: Vec<_> = tc.leave().into_iter().map(|x| x.1).collect();
    timeclock::leave_time(timeclock::now(), today_h, tc.expected, &leave, running, on_break)
}

#[tauri::command]
fn tc_leave_time(s: State<App>) -> String {
    let tc = s.tc();
    let (cur, on_break, today_h) = tc.status();
    leave_time(&tc, cur.is_some(), on_break.is_some(), today_h)
}

/// Everything the time dashboard shows, for the current week (Mon–Sun).
#[tauri::command]
fn tc_dashboard(s: State<App>) -> Value {
    let tc = s.tc();
    let (cur, on_break, today_hours) = tc.status();
    let leave_time = leave_time(&tc, cur.is_some(), on_break.is_some(), today_hours);
    dashboard(DashIn {
        today: today(), today_hours, sessions: tc.sessions(), projects: tc.projects(), leave: tc.leave(), expected: tc.expected,
        status: (cur, on_break), profile: s.profile(), profiles: s.cfg().profiles, leave_time, backup_error: backup::last_failure(),
    })
}

/// What `dashboard` is built from, gathered by `tc_dashboard`.
struct DashIn {
    today: NaiveDate,
    today_hours: f64,
    sessions: Vec<timeclock::Session>,
    projects: timeclock::Projects,
    leave: Vec<(usize, timeclock::Leave)>,
    expected: f64,
    status: (Option<(NaiveDateTime, String, String)>, Option<String>),
    profile: String,
    profiles: Vec<String>,
    leave_time: String,
    backup_error: Option<String>,
}

fn dashboard(inp: DashIn) -> Value {
    use chrono::Datelike;
    let DashIn { today: t, today_hours: today_h, sessions, projects, leave, expected, status: (cur, on_break), .. } = &inp;
    let (t, today_h, expected) = (*t, *today_h, *expected);
    let leave_only: Vec<_> = leave.iter().map(|x| x.1.clone()).collect();
    let monday = t - Duration::days(t.weekday().num_days_from_monday() as i64);
    let sunday = monday + Duration::days(6);
    let in_week = |d: NaiveDate| monday <= d && d <= sunday;
    let week: Vec<Value> = (0..7)
        .map(|i| {
            let d = monday + Duration::days(i);
            let h: f64 = if d == t { today_h } else { sessions.iter().filter(|x| x.date == d).map(|x| x.hours).sum() };
            let e = timeclock::expected_hours(d, expected);
            let off = timeclock::leave_on(&leave_only, d, e, h);
            let kinds: Vec<&str> = off.iter().map(|x| x.0.kind.as_str()).collect();
            json!({ "date": d, "hours": h, "expected": e - timeclock::leave_credit(&leave_only, d, e, h), "leave": kinds.join(", "), "leave_hours": off.iter().map(|x| x.1).sum::<f64>() })
        })
        .collect();
    let merged = timeclock::prepare_report_sessions(sessions);
    let rounded = timeclock::apply_carry(&merged, projects).0;
    let mut by: std::collections::BTreeMap<String, (f64, f64)> = Default::default();
    merged.iter().filter(|x| in_week(x.date)).for_each(|x| by.entry(x.project.clone()).or_default().0 += x.hours);
    rounded.iter().filter(|x| in_week(x.date)).for_each(|x| by.entry(x.project.clone()).or_default().1 += x.hours);
    let mut per_project: Vec<Value> = by
        .into_iter()
        .map(|(p, (w, b))| json!({ "project": if p.is_empty() { "Other".to_string() } else { p.clone() }, "code": timeclock::export_code(&p, projects), "worked": w, "billable": b }))
        .collect();
    per_project.sort_by(|a, b| b["worked"].as_f64().unwrap_or(0.0).total_cmp(&a["worked"].as_f64().unwrap_or(0.0)));
    let (flex_total, flex_week, _) = timeclock::flex(sessions, projects, &leave_only, expected, Some((monday, t)));
    json!({
        "profile": inp.profile, "profiles": inp.profiles,
        "project": cur.as_ref().map(|c| &c.1), "since": cur.as_ref().map(|c| c.0), "task": cur.as_ref().map(|c| &c.2), "on_break": on_break,
        "today_hours": today_h, "today": sessions.iter().filter(|x| x.date == t).collect::<Vec<_>>(),
        "week": week, "leave": leave, "projects": per_project, "flex_total": flex_total, "flex_week": flex_week, "backup_error": inp.backup_error,
        "leave_time": inp.leave_time,
    })
}

/// Log time off (vacation, sick, …) FROM..=TO.
#[tauri::command(async)]
fn tc_leave_add(s: State<App>, leave: timeclock::Leave) -> R<()> {
    s.tc().add_leave(leave)?;
    s.backup();
    Ok(())
}

#[tauri::command(async)]
fn tc_leave_remove(s: State<App>, line: usize) -> R<()> {
    s.tc().remove_leave(line)?;
    s.backup();
    Ok(())
}

/// Export codes of every kind of time off: saved ones, kinds in leave.jsonl and the defaults.
#[tauri::command]
fn tc_leave_codes(s: State<App>) -> R<timeclock::LeaveCodes> {
    let tc = s.tc();
    let mut c = tc.leave_codes()?;
    for k in ["vacation", "sick", "other"].map(String::from).into_iter().chain(tc.leave().into_iter().map(|x| x.1.kind)) {
        c.entry(k).or_default();
    }
    Ok(c)
}

#[tauri::command]
fn tc_save_leave_code(s: State<App>, kind: String, code: String) -> R<()> {
    s.tc().save_leave_code(&kind, &code)
}

#[tauri::command]
fn tc_projects(s: State<App>) -> R<timeclock::Projects> {
    s.tc().load_projects()
}

#[tauri::command]
fn tc_save_project(s: State<App>, name: String, project: Project) -> R<()> {
    let tc = s.tc();
    let mut p = tc.load_projects()?;
    p.insert(name, project);
    tc.save_projects(&p)
}

#[tauri::command]
fn tc_suggestions(s: State<App>, project: Option<String>) -> Vec<String> {
    timeclock::task_suggestions(&s.tc().sessions(), project.as_deref(), today())
}

#[tauri::command(async)]
fn tc_in(s: State<App>, project: String, task: String, export_code: Option<String>) -> R<String> {
    let switched = s.tc().clock_in(&project, &task, export_code.as_deref())?;
    if switched {
        s.backup();
    }
    Ok(if project.trim().is_empty() { "⏱ Clocked in".into() } else { format!("⏱ Clocked in on: {project}") })
}

#[tauri::command(async)]
fn tc_out(s: State<App>, note: String) -> R<String> {
    if !s.tc().clock_out(&note)? {
        return Ok("Not clocked in.".into());
    }
    s.backup();
    Ok(if note.trim().is_empty() { "⏱ Clocked out.".into() } else { format!("⏱ Clocked out. Task: {note}") })
}

/// Idle prompt: clock out at SINCE, and back in at BACK unless stopping.
#[tauri::command(async)]
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

/// Log a finished session after the fact (an accepted suggestion).
#[tauri::command(async)]
fn tc_add_session(s: State<App>, start: NaiveDateTime, end: NaiveDateTime, project: String, export_code: Option<String>, note: String) -> R<String> {
    s.tc().add_session(start, end, &project, export_code.as_deref(), &note)?;
    s.backup();
    Ok(format!("✅ Logged {}–{} on {}", start.format("%H:%M"), end.format("%H:%M"), if project.trim().is_empty() { "Other" } else { project.trim() }))
}

/// Suggested sessions for DATE_INPUT as `{source, items}`: from ActivityWatch when `activitywatch_url`
/// is set and answers, else from the built-in watcher's log when it's on; null when both are off.
#[tauri::command(async)]
fn activity_suggestions(s: State<App>, date_input: String) -> R<Value> {
    let c = s.cfg();
    let aw = !c.activitywatch_url.trim().is_empty();
    if !aw && !c.activity_watcher {
        return Ok(Value::Null);
    }
    // Bad patterns are skipped and reported, so one typo doesn't hide all suggestions.
    let (d, (exclude, mut warnings)) = (date(&date_input)?, activity::lenient("activity_exclude", &c.activity_exclude));
    let (source, (window, afk)) = match aw.then(|| activity::fetch(&c.activitywatch_url, d, &exclude)) {
        Some(Ok(x)) => ("ActivityWatch", x),
        Some(Err(e)) if !c.activity_watcher => return Err(e),
        _ => ("Margin", (watcher::spans(&c.data().join("activity"), d), vec![])),
    };
    let tc = s.tc();
    let projects: Vec<String> = tc.projects().into_iter().filter(|(_, p)| p.active).map(|(n, _)| n).collect();
    let tracked = timeclock::spans(&tc.events(), timeclock::now());
    let tasks: Vec<activity::Task> = org::todos(&s.files(), &s.kw(), false, |_, _| true, today()).into_iter().map(|t| (t.title, t.category)).collect();
    let (meetings, calendar, w) = meetings_and_calendar(&c, d, |p| ics::calendar(p, &ics::CALENDAR, std::time::Instant::now(), ics::get));
    warnings.extend(w);
    let (rules, bad) = activity::parse_rules(&timeclock::read_or_empty(&tc.dir.join("activity_rules.toml"))?);
    warnings.extend(bad);
    let all = activity::suggest(&window, &afk, &tracked, &exclude, activity::Known { projects: &projects, tasks: &tasks, meetings: &meetings, calendar: &calendar, rules: &rules }, d);
    Ok(json!({ "source": source, "items": activity::undismissed(all, &dismissed(&tc).unwrap_or_default()), "warnings": warnings }))
}

/// The meeting patterns that compile and the calendar's events on D (via LOAD, the .ics text of the configured source);
/// a bad pattern or an unreachable calendar is a warning, not a reason to hide the other suggestions.
fn meetings_and_calendar(c: &Config, d: NaiveDate, load: impl FnOnce(&str) -> R<String>) -> (Vec<regex::Regex>, Vec<ics::Event>, Vec<String>) {
    let (meetings, mut warnings) = activity::lenient("activity_meetings", &c.activity_meetings);
    let calendar = match c.activity_calendar.trim() {
        "" => vec![],
        p => match load(p) {
            Ok(t) => ics::events_on(&t, d),
            Err(e) => {
                warnings.push(e);
                vec![]
            }
        },
    };
    (meetings, calendar, warnings)
}

/// The profile's `activity_dismissed.json`; missing is empty, unreadable or malformed is an error.
fn dismissed(tc: &Tc) -> R<activity::Dismissed> {
    let path = tc.dir.join("activity_dismissed.json");
    match timeclock::read_or_empty(&path)? {
        t if t.is_empty() => Ok(Default::default()),
        t => serde_json::from_str(&t).map_err(|e| format!("{}: {e}", path.display())),
    }
}

/// Hide the suggestion START–END for good (well, 30 days).
#[tauri::command]
fn activity_dismiss(s: State<App>, start: NaiveDateTime, end: NaiveDateTime) -> R<()> {
    let tc = s.tc();
    let d = activity::dismiss(dismissed(&tc)?, start, end, today());
    std::fs::create_dir_all(&tc.dir).map_err(|e| e.to_string())?;
    config::write_atomic(&tc.dir.join("activity_dismissed.json"), serde_json::to_string(&d).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

/// Remember that blocks titled TITLE (an EDITOR label) go to PROJECT (the user picked another project than the guess).
#[tauri::command]
fn activity_learn(s: State<App>, title: String, project: String, editor: bool) -> R<()> {
    activity::learn_file(&s.tc().dir.join("activity_rules.toml"), &title, &project, editor)
}

/// The learned rules as `[{pattern, project, missing}]`; MISSING when the project isn't an active one, so the rule is skipped.
#[tauri::command]
fn activity_rules_list(s: State<App>) -> R<Value> {
    let tc = s.tc();
    let projects = tc.projects();
    let rules = activity::rule_list(&timeclock::read_or_empty(&tc.dir.join("activity_rules.toml"))?)?;
    Ok(rules.into_iter().map(|(p, project)| json!({ "pattern": p, "missing": !projects.get(&project).is_some_and(|x| x.active), "project": project })).collect())
}

/// Forget the learned rule for PATTERN.
#[tauri::command]
fn activity_rules_delete(s: State<App>, pattern: String) -> R<()> {
    activity::forget_file(&s.tc().dir.join("activity_rules.toml"), &pattern)
}

/// What the model gets to draft a diary note for an accepted suggestion.
#[tauri::command]
fn ai_note_prompt(s: State<App>, project: Option<String>, task: Option<String>, apps: Vec<String>, titles: Vec<String>) -> R<String> {
    Ok(ai::note_prompt(project.as_deref(), task.as_deref(), &apps, &titles, &activity::exclude_rules(&s.cfg().activity_exclude)?))
}

/// What the model gets to summarize DATE_INPUT: logged sessions, plus ActivityWatch's titles when it's on and up.
#[tauri::command(async)]
fn ai_day_prompt(s: State<App>, date_input: String) -> R<String> {
    let (c, d) = (s.cfg(), date(&date_input)?);
    let exclude = activity::exclude_rules(&c.activity_exclude)?;
    let aw = Some(&c.activitywatch_url).filter(|u| !u.trim().is_empty()).and_then(|u| activity::fetch(u, d, &exclude).ok());
    let blocks = aw.map(|(w, a)| activity::suggest(&w, &a, &[], &exclude, Default::default(), d)).unwrap_or_default();
    let sessions: Vec<_> = s.tc().sessions().into_iter().filter(|x| x.date == d).collect();
    Ok(ai::day_prompt(d, &sessions, &blocks, &exclude))
}

/// The model's draft for PROMPT (the exact text the user was shown); ONE_LINE for a diary note.
#[tauri::command(async)]
fn ai_draft(app: AppHandle, s: State<App>, prompt: String, one_line: bool) -> R<String> {
    let c = s.cfg();
    if c.ai_model.trim().is_empty() {
        return Err("Pick a model with AI drafts: choose model… to draft with a local model.".into());
    }
    if prompt.len() > ai::MAX_PROMPT + 200 {
        return Err("That prompt is too long to send.".into());
    }
    ai::generate(c.ai_backend, &c.ai_url, &models_dir(&app)?, c.ai_model.trim(), &prompt, one_line)
}

/// The model's answer to the chat MESSAGES, about the open NOTE (name, text) if given; the raw answer
/// streams to ON_PIECE as it comes, and the cleaned one is returned.
#[tauri::command(async)]
fn ai_chat(app: AppHandle, s: State<App>, messages: Vec<ai::Msg>, note: Option<(String, String)>, on_piece: tauri::ipc::Channel<String>) -> R<String> {
    let c = s.cfg();
    if c.ai_model.trim().is_empty() {
        return Err("Pick a model with AI drafts: choose model… (Space f a) to chat with a local model.".into());
    }
    ai::chat(c.ai_backend, &c.ai_url, &models_dir(&app)?, c.ai_model.trim(), note.as_ref().map(|(n, t)| (n.as_str(), t.as_str())), &messages, &mut |p| {
        let _ = on_piece.send(p.into());
    })
}

/// Stop the built-in model's running draft or a chat answer; it returns an error.
#[tauri::command]
fn ai_draft_cancel() {
    ai::cancel_draft();
}

/// Where downloaded models live: the app's local data dir, never data_dir (backups commit that).
fn models_dir(app: &AppHandle) -> R<PathBuf> {
    Ok(app.path().app_local_data_dir().map_err(|e| e.to_string())?.join("models"))
}

fn model_file(app: &AppHandle, id: &str) -> R<(&'static ai::Model, PathBuf)> {
    let m = ai::MODELS.iter().find(|m| m.id == id).ok_or_else(|| format!("unknown built-in model {id}"))?;
    Ok((m, m.path(&models_dir(app)?)))
}

#[derive(serde::Serialize)]
struct LocalModel {
    #[serde(flatten)]
    model: &'static ai::Model,
    path: PathBuf,
    /// Bytes on disk: SIZE once downloaded and checked, else what a paused download has.
    have: u64,
    ready: bool,
    /// Whether this build can run it (ai::RUNS).
    runs: bool,
}

/// The models Margin can download and run itself, and how much of each is here.
#[tauri::command]
fn ai_models(app: AppHandle) -> R<Vec<LocalModel>> {
    ai::MODELS.iter().map(|m| {
        let (_, path) = model_file(&app, m.id)?;
        let ready = path.is_file();
        let have = if ready { m.size } else { std::fs::metadata(ai::part(&path)).map(|x| x.len()).unwrap_or(0) };
        Ok(LocalModel { model: m, path, have, ready, runs: ai::RUNS })
    }).collect()
}

static DOWNLOADING: AtomicBool = AtomicBool::new(false);
static CANCEL: AtomicBool = AtomicBool::new(false);

/// Download model ID (resuming a paused one), emitting `ai-download` {id, have, size} as it goes.
#[tauri::command]
async fn ai_download(app: AppHandle, id: String) -> R<()> {
    let (m, path) = model_file(&app, &id)?;
    if DOWNLOADING.swap(true, Ordering::SeqCst) {
        return Err("A model is already downloading.".into());
    }
    CANCEL.store(false, Ordering::SeqCst);
    let mut next = 0;
    let r = ai::download(m.url, &path, m.size, m.sha256, &CANCEL, |have| {
        if have >= next || have == m.size {
            next = have + m.size / 200;
            let _ = app.emit("ai-download", json!({ "id": m.id, "have": have, "size": m.size }));
        }
    }).await;
    DOWNLOADING.store(false, Ordering::SeqCst);
    r
}

/// Stop the running download; its partial file stays, so the next one resumes.
#[tauri::command]
fn ai_download_cancel() {
    CANCEL.store(true, Ordering::SeqCst);
}

/// Delete model ID from disk, partial download included.
#[tauri::command]
fn ai_model_delete(app: AppHandle, id: String) -> R<()> {
    let (_, path) = model_file(&app, &id)?;
    if DOWNLOADING.load(Ordering::SeqCst) {
        return Err("Cancel the download first.".into());
    }
    for p in [ai::part(&path), path] {
        match std::fs::remove_file(&p) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(format!("{}: {e}", p.display())),
            _ => {}
        }
    }
    Ok(())
}

/// Save the draft BACKEND and MODEL ("" = off) to config.toml, leaving the rest as written.
/// Returns reload_config's warning (or "").
#[tauri::command]
fn ai_set(app: AppHandle, s: State<App>, backend: ai::Backend, model: String) -> R<String> {
    let model = model.trim();
    if backend == ai::Backend::Embedded && !model.is_empty() && !ai::MODELS.iter().any(|m| m.id == model) {
        return Err(format!("unknown built-in model {model}"));
    }
    let text = std::fs::read_to_string(&s.cfg_path).map_err(|e| e.to_string())?;
    let text = config::set_key(&text, "ai_backend", toml::Value::try_from(backend).map_err(|e| e.to_string())?)?;
    let text = config::set_key(&text, "ai_model", model.into())?;
    config::write_atomic(&s.cfg_path, text).map_err(|e| e.to_string())?;
    reload_config(app, s)
}

/// Org-formatted report text. KIND: daily | weekly | holidays | flex | doctor | backup.
#[tauri::command]
fn tc_report(s: State<App>, kind: String, date_input: Option<String>) -> R<String> {
    let tc = s.tc();
    let profile = s.profile();
    let leave: Vec<_> = tc.leave().into_iter().map(|x| x.1).collect();
    let codes = tc.leave_codes().unwrap_or_default();
    Ok(match kind.as_str() {
        "daily" => timeclock::daily_report(&tc.sessions(), &tc.projects(), (&leave, &codes), tc.expected, date(date_input.as_deref().unwrap_or(""))?, &profile),
        "weekly" => timeclock::weekly_report(&tc.sessions(), &tc.projects(), (&leave, &codes), tc.expected, today() - Duration::days(7), today(), &profile),
        "holidays" => timeclock::holidays_report(date(date_input.as_deref().unwrap_or(""))?.format("%Y").to_string().parse().unwrap()),
        "flex" => {
            let t = today();
            let week = (t - Duration::days(7), t);
            let (total, period, days) = timeclock::flex(&tc.sessions(), &tc.projects(), &leave, tc.expected, Some(week));
            format!("Flex ({profile}): total {total:+.2} h, last 7 days {period:+.2} h over {days} worked days")
        }
        "doctor" => {
            let mut issues = timeclock::doctor(&tc.read_log(), timeclock::now());
            issues.extend(tc.file_issues());
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
    let leave: Vec<_> = tc.leave().into_iter().map(|x| x.1).collect();
    let csv = timeclock::csv(&tc.sessions(), &tc.load_projects()?, (&leave, &tc.leave_codes()?), tc.expected, date(&start)?, date(&end)?);
    std::fs::write(&path, csv).map_err(|e| e.to_string())?;
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
        "md" => write_export(&s, &format!("{stem}.md"), export::markdown(&d, &stem, base, &export::Ids::new()), overwrite),
        f => write_export(&s, &format!("{stem}.html"), export::html(&d, &stem, base, &export::Ids::new(), f == "pdf"), overwrite),
    }
}

/// Export note TEXT (the buffer of PATH) and the notes it reaches by id: links within DEPTH hops
/// as FORMAT (html | md) pages in `<export_dir>/<stem>/`, id: links between them made relative,
/// pages of an earlier export no longer included moved into `old/`; pdf: one `<stem> (linked).html` print page. Returns PATH's page.
#[tauri::command]
fn export_linked(s: State<App>, path: PathBuf, text: String, depth: usize, format: String, overwrite: Option<bool>) -> R<PathBuf> {
    let kw = s.kw();
    let root = org::parse(&path, &text, &kw);
    let files = s.files();
    let files: Vec<&org::OrgFile> = files.iter().map(|f| &**f).collect();
    let ext = if format == "md" { "md" } else { "html" };
    let (pages, ids) = export::bundle(&files, &root, depth, ext);
    if format == "pdf" {
        return write_export(&s, &format!("{} (linked).html", stem(&path)), export::combined(&pages, &ids, &kw), overwrite);
    }
    let dir = export_path(&s, &stem(&path), overwrite)?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let names: Vec<String> = pages.iter().map(|(_, n)| n.clone()).collect();
    let existing: Vec<String> = std::fs::read_dir(&dir).map_err(|e| e.to_string())?.flatten().filter(|e| e.path().is_file()).map(|e| e.file_name().to_string_lossy().to_string()).collect();
    for n in export::stale(&existing, &names, ext) {
        let old = dir.join("old");
        std::fs::create_dir_all(&old).map_err(|e| e.to_string())?;
        let _ = std::fs::remove_file(old.join(&n)); // Windows' rename won't replace
        std::fs::rename(dir.join(&n), old.join(&n)).map_err(|e| e.to_string())?;
    }
    for (f, name) in &pages {
        let (d, title, base) = (export::parse(&f.text, &kw), stem(&f.path), f.path.parent().unwrap_or(Path::new("")));
        let body = if ext == "md" { export::markdown(&d, &title, base, &ids) } else { export::html(&d, &title, base, &ids, false) };
        config::write_atomic(&dir.join(name), body).map_err(|e| e.to_string())?;
    }
    Ok(dir.join(&pages[0].1))
}

/// The time report for START..END as an HTML page in export_dir; PRINT opens the print dialog.
#[tauri::command]
fn export_report(s: State<App>, start: String, end: String, print: bool, overwrite: Option<bool>) -> R<PathBuf> {
    let tc = s.tc();
    let (start, end) = (date(&start)?, date(&end)?);
    let leave: Vec<_> = tc.leave().into_iter().map(|x| x.1).collect();
    let org = timeclock::weekly_report(&tc.sessions(), &tc.load_projects()?, (&leave, &tc.leave_codes()?), tc.expected, start, end, &s.profile());
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

#[tauri::command(async)]
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

#[tauri::command(async)]
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
            let (cfg, cfg_error) = match Config::load_or_create(&cfg_path) {
                Ok(c) => (c, None),
                Err(e) => {
                    eprintln!("{e}; using defaults");
                    (Config::default(), Some(e))
                }
            };
            let saved = std::fs::read_to_string(cfg.data().join("active-profile.txt")).unwrap_or_default();
            let profile = cfg.profiles.iter().find(|p| **p == saved.trim()).or(cfg.profiles.first()).cloned().unwrap_or("Work".into());
            app.manage(App { cfg_path, cfg: Mutex::new(cfg), cfg_error: Mutex::new(cfg_error), profile: Mutex::new(profile), cache: Default::default(), watcher: Mutex::new(None) });
            watch(app.handle());
            write_calendar(app.handle());
            tray::setup(app.handle())?;
            app.handle().plugin(tauri_plugin_notification::init())?;
            remind::start(app.handle());
            #[cfg(desktop)]
            {
                idle::start(app.handle());
                watcher::start(app.handle());
                use tauri_plugin_global_shortcut::ShortcutState;
                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_handler(|app, _, ev| {
                            if ev.state == ShortcutState::Pressed {
                                capture(app)
                            }
                        })
                        .build(),
                )?;
                if let Err(e) = rebind_capture(app.handle(), "", &app.state::<App>().cfg().capture_shortcut) {
                    eprintln!("{e}");
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
            watcher::gnome_extension, watcher::screen_recording,
            config, config_recreate, tutorial, update_check, reload_config, save_view, setup, save_config, list_files, read_file, file_exists, write_file, attach_file, attach_bytes, unused_attachments, trash_attachments,
            agenda, todos, search_todos, org_heading, org_edit, org_planning, read_date, org_context,
            org_targets, org_tags, org_refile, org_refile_same, org_archive, capture_insert, capture_path, capture_templates, template_prompts, capture_template, task_entry, date_preview, tc_dashboard,
            notes_new, note_titles, notes_nodes, notes_backlinks, rename_note, trash_note, notes_search, notes_graph, notes_unlinked, notes_ensure_id,
            tc_status, tc_leave_time, tc_projects, tc_save_project, tc_suggestions, tc_in, tc_out, tc_idle, tc_break, tc_resume, tc_adjust,
            tc_sessions_on, tc_edit_session, tc_add_session, tc_leave_add, tc_leave_remove, tc_leave_codes, tc_save_leave_code,activity_suggestions, activity_dismiss, activity_learn, activity_rules_list, activity_rules_delete, ai_note_prompt, ai_day_prompt, ai_draft, ai_chat, ai_draft_cancel, ai_models, ai_set, ai_download, ai_download_cancel, ai_model_delete, tc_report, tc_csv, tc_switch_profile, tc_import, backup_now,
            export_note, export_linked, export_report, export_open
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, ev| {
            // Replaces kill-emacs-hook: commit on exit.
            match ev {
                RunEvent::ExitRequested { .. } => {
                    watcher::flush(&app.state::<App>().cfg().data().join("activity"));
                    app.state::<App>().backup()
                }
                #[cfg(target_os = "macos")]
                RunEvent::Reopen { .. } => tray::show(app), // dock icon click after hiding to tray
                _ => {}
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_refuses_to_overwrite_an_external_edit() {
        assert!(may_overwrite(Some("a\n"), Some("a\n"), "b\n")); // unchanged since we saw it
        assert!(!may_overwrite(Some("theirs\n"), Some("a\n"), "b\n"));
        assert!(may_overwrite(Some("b\n"), Some("a\n"), "b\n")); // already holds our text
        assert!(may_overwrite(Some("a\r\nb\r\n"), Some("a\nb\n"), "c\n")); // CRLF file
        assert!(may_overwrite(None, Some("a\n"), "b\n")); // missing: a new note
        assert!(may_overwrite(Some("theirs\n"), None, "b\n")); // :w forces
    }

    #[test]
    fn rename_never_drops_a_concurrent_save() {
        let kw = org::Kw { todo: vec![], done: vec![] };
        for i in 0..20 {
            let dir = std::env::temp_dir().join(format!("margin-race-{}-{i}", notes::gen_id()));
            std::fs::create_dir_all(&dir).unwrap();
            let (a, b) = (dir.join("a.org"), dir.join("b.org"));
            let old = "see [[file:a.org][A]]\n";
            std::fs::write(&a, "a\n").unwrap();
            std::fs::write(&b, old).unwrap();
            let files = vec![Arc::new(org::parse(&b, old, &kw))];
            let go = std::sync::Barrier::new(2);
            let (renamed, saved) = std::thread::scope(|t| {
                let r = t.spawn(|| {
                    go.wait();
                    let _lock = note_writes();
                    notes::rename(&files, &a, "c.org")
                });
                let w = t.spawn(|| {
                    go.wait();
                    write_checked(&b, format!("{old}typed\n"), Some(old))
                });
                (r.join().unwrap(), w.join().unwrap())
            });
            renamed.unwrap();
            let disk = std::fs::read_to_string(&b).unwrap();
            assert!(disk.contains("file:c.org"), "{disk}");
            // The save either landed first (and the rename kept it) or was refused, never overwritten.
            match saved {
                Ok(()) => assert!(disk.contains("typed"), "{disk}"),
                Err(e) => assert_eq!(e, "changed-on-disk"),
            }
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    fn kw() -> org::Kw {
        org::Kw { todo: vec!["TODO".into()], done: vec!["DONE".into()] }
    }

    fn edit(text: &str, op: &str, v: Option<&str>) -> R<String> {
        let now = "2026-01-08T09:00:00".parse::<NaiveDateTime>().unwrap();
        org_edit_op(text, 0, op, v.map(String::from), &kw(), now)
    }

    #[test]
    fn org_edit_ops() {
        assert_eq!(edit("* Task\n", "cycle", Some("1")).unwrap(), "* TODO Task\n");
        assert_eq!(edit("* TODO Task\n", "cycle", Some(" 1 ")).unwrap(), "* DONE Task\n");
        assert_eq!(edit("* TODO Task\n", "cycle", Some("-1")).unwrap(), "* Task\n");
        assert_eq!(edit("* Task\n", "keyword", Some("TODO")).unwrap(), "* TODO Task\n");
        assert_eq!(edit("* TODO Task\n", "keyword", Some("")).unwrap(), "* Task\n");
        assert_eq!(edit("* TODO Task\n", "keyword", None).unwrap(), "* Task\n");
        assert_eq!(edit("* Task\n", "priority", Some("b")).unwrap(), "* [#B] Task\n");
        assert_eq!(edit("* [#B] Task\n", "priority", None).unwrap(), "* Task\n");
        assert_eq!(edit("* Task\n", "priority-cycle", Some("1")).unwrap(), "* [#C] Task\n"); // none -> C -> B -> A -> none
        assert_eq!(edit("* [#B] Task\n", "priority-cycle", Some("1")).unwrap(), "* [#A] Task\n");
        assert_eq!(edit("* [#A] Task\n", "priority-cycle", Some("1")).unwrap(), "* Task\n");
        assert_eq!(edit("* [#A] Task\n", "priority-cycle", Some("-1")).unwrap(), "* [#B] Task\n");
        assert_eq!(edit("* Task\n", "tags", Some("a:b")).unwrap(), "* Task :a:b:\n");
        // An empty value reaches set_tags as one empty tag, which it drops: clears the tags.
        assert_eq!(edit("* Task :a:\n", "tags", Some("")).unwrap(), "* Task\n");
        assert_eq!(edit("* Task :a:\n", "tags", None).unwrap(), "* Task\n");
        let sched = "* Task\nSCHEDULED: <2026-01-08 Thu>\n";
        assert_eq!(edit(sched, "shift", Some("SCHEDULED 3")).unwrap(), "* Task\nSCHEDULED: <2026-01-11 Sun>\n");
        assert_eq!(edit(sched, "shift", Some("SCHEDULED -1")).unwrap(), "* Task\nSCHEDULED: <2026-01-07 Wed>\n");
        assert_eq!(edit(sched, "shift", Some("scheduled 3")).unwrap(), sched); // the kind is case-sensitive
        assert_eq!(edit(sched, "shift", Some("DEADLINE 3")).unwrap(), sched); // no such date: untouched
    }

    #[test]
    fn org_edit_errors() {
        assert_eq!(edit("* T\n", "cycle", Some("x")).unwrap_err(), "bad number: x");
        assert_eq!(edit("* T\n", "priority-cycle", None).unwrap_err(), "bad number: ");
        assert_eq!(edit("* T\n", "shift", Some("scheduled")).unwrap_err(), "shift needs \"KIND DAYS\"");
        assert_eq!(edit("* T\n", "shift", None).unwrap_err(), "shift needs \"KIND DAYS\"");
        assert_eq!(edit("* T\n", "shift", Some("SCHEDULED two")).unwrap_err(), "bad day count");
        assert_eq!(edit("* T\n", "nope", None).unwrap_err(), "unknown org op nope");
    }

    #[test]
    fn org_context_falls_back_to_the_file() {
        let text = "#+title: Doc\n#+category: Work\npreamble\n* A\n** B\ntext\n* C\n";
        let f = org::parse(Path::new("doc.org"), text, &kw());
        let c = |l| context_of(&f, l);
        assert_eq!(c(2), json!({ "category": "Work", "title": "Doc" })); // before the first heading
        assert_eq!(c(3), json!({ "category": "Work", "title": "A" }));
        assert_eq!(c(5), json!({ "category": "Work", "title": "B" })); // under B
        assert_eq!(c(6), json!({ "category": "Work", "title": "C" }));
    }

    #[test]
    fn refile_and_archive_pass_errors_through() {
        let src = "* A\n* B\n";
        assert_eq!(org_refile(src.into(), 0, "* X\n".into(), None).unwrap(), ["* B\n".to_string(), "* X\n* A\n".to_string()]);
        assert_eq!(org_refile("text\n".into(), 0, "".into(), None).unwrap_err(), "not on a heading");
        assert_eq!(org_refile(src.into(), 0, "plain\n".into(), Some(0)).unwrap_err(), "refile target is not a heading");
        assert_eq!(org_refile_same("* A\n** B\n".into(), 0, Some(1)).unwrap_err(), "can't refile a subtree under itself");
        assert_eq!(org_refile_same("text\n".into(), 0, None).unwrap_err(), "not on a heading");
        assert_eq!(org_refile_same(src.into(), 0, Some(1)).unwrap(), "* B\n** A\n");
        assert!(org_archive("text\n".into(), 0, "".into(), "a.org".into()).is_err());
        let [left, arch] = org_archive(src.into(), 1, "".into(), "a.org".into()).unwrap();
        assert_eq!(left, "* A\n");
        assert!(arch.contains("Archived entries from file a.org") && arch.contains(":ARCHIVE_FILE: a.org") && arch.contains("* B"), "{arch}");
    }

    #[test]
    fn bad_meeting_pattern_and_calendar_only_warn() {
        let c: Config = toml::from_str("activity_meetings = [\"(\", \"standup\"]\nactivity_calendar = \"x.ics\"").unwrap();
        let d = NaiveDate::from_ymd_opt(2026, 1, 5).unwrap();
        let (m, cal, w) = meetings_and_calendar(&c, d, |_| Err("activity_calendar (URL): offline".into()));
        assert_eq!((m.len(), cal.len(), w.len()), (1, 0, 2));
        assert!(w[0].starts_with("activity_meetings:") && w[1].starts_with("activity_calendar"));
    }

    #[test]
    fn config_changes_split_out() {
        let (cfg, roots) = (PathBuf::from("/c/config.toml"), [PathBuf::from("/n"), PathBuf::from("/d")]);
        let p = |v: &[&str]| v.iter().map(PathBuf::from).collect::<Vec<_>>();
        let changes = p(&["/n/b.org", "/c/.config.toml.margin-tmp", "/c/config.toml", "/n/a.org", "/n/.git/x", "/d/w/log", "/n/a.org"]);
        assert_eq!(split_changes(changes, &cfg, &cfg, &roots), (true, p(&["/d/w/log", "/n/a.org", "/n/b.org"])));
        assert_eq!(split_changes(p(&["/c/tutorial.org", "/n/a.org"]), &cfg, &cfg, &roots), (false, p(&["/n/a.org"])));
        // A symlinked config.toml: the target's events count too.
        let real = PathBuf::from("/dots/config.toml");
        assert_eq!(split_changes(p(&["/dots/config.toml"]), &cfg, &real, &roots), (true, vec![]));
    }

    #[test]
    fn nightly_versions_order_between_releases() {
        // nightly.yml names builds <next patch>-nightly.<yyyymmddhhmm>; the updater compares semver.
        let v = |s: &str| semver::Version::parse(s).unwrap();
        assert!(v("0.2.2") < v("0.2.3-nightly.202610110300"));
        assert!(v("0.2.3-nightly.202610110300") < v("0.2.3-nightly.202610120300"));
        assert!(v("0.2.3-nightly.202612312359") < v("0.2.3"));
    }

    #[test]
    fn trash_to_keeps_subpath_and_numbers_clashes() {
        let root = std::env::temp_dir().join(format!("margin-trash-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let put = |p: &str| {
            let p = root.join(p);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, p.to_str().unwrap()).unwrap();
            p
        };
        let a = put("sub/a.org");
        assert_eq!(trash_to(&root, &a).unwrap(), root.join(".trash/sub/a.org"));
        assert!(!a.exists());
        let a = put("sub/a.org");
        assert_eq!(trash_to(&root, &a).unwrap(), root.join(".trash/sub/a.org.1"));
        assert_eq!(std::fs::read_to_string(root.join(".trash/sub/a.org.1")).unwrap(), a.to_str().unwrap());
        // Outside ROOT, already in .trash, `..` in the path, a folder, a missing file: refused.
        let out = std::env::temp_dir().join(format!("margin-trash-out-{}.org", std::process::id()));
        std::fs::write(&out, "").unwrap();
        put("x.org");
        for bad in [out.clone(), root.join(".trash/sub/a.org"), root.join("sub/../x.org"), root.join("sub"), root.join("nope.org")] {
            assert!(trash_to(&root, &bad).is_err(), "{}", bad.display());
        }
        assert!(out.exists());
        let _ = std::fs::remove_file(out);
        let _ = std::fs::remove_dir_all(&root);
    }

    fn keys(v: &Value) -> Vec<&str> {
        let mut k: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
        k.sort();
        k
    }

    fn dash_fixture(today: &str, status: (Option<(NaiveDateTime, String, String)>, Option<String>)) -> DashIn {
        use timeclock::{Leave, Session};
        let d = |s: &str| s.parse::<NaiveDate>().unwrap();
        let t = chrono::NaiveTime::from_hms_opt(9, 0, 0).unwrap();
        let ses = |date, project: &str, desc: &str, hours| Session { date: d(date), project: project.into(), desc: desc.into(), hours, start: t, end: t, line: None };
        let off = |date, kind: &str| Leave { from: d(date), to: d(date), kind: kind.into(), hours: None, rest: false, note: String::new() };
        DashIn {
            today: d(today),
            today_hours: 1.5, // the running clock, more than the 1.0 the log holds for 01-08
            sessions: vec![
                ses("2025-12-29", "Acme", "old", 9.0), // last week: flex_total only
                ses("2026-01-05", "Acme", "a", 3.2),
                ses("2026-01-05", "", "x", 1.0),
                ses("2026-01-06", "Acme", "red", 1.0), // Epiphany
                ses("2026-01-07", "Beta", "b", 2.0),
                ses("2026-01-08", "Acme", "c", 1.0),
            ],
            projects: [("Acme".to_string(), Project::new("A-1"))].into(),
            leave: vec![(0, off("2026-01-09", "Vacation")), (1, off("2026-01-06", "Vab"))],
            expected: 8.0,
            status,
            profile: "Work".into(),
            profiles: vec!["Work".into(), "Home".into()],
            leave_time: "7h 00m left today (not clocked in).".into(),
            backup_error: Some("disk full".into()),
        }
    }

    #[test]
    fn dashboard_builds_the_week() {
        let v = dashboard(dash_fixture("2026-01-08", (None, None)));
        // The frontend contract (Time.svelte).
        assert_eq!(keys(&v), ["backup_error", "flex_total", "flex_week", "leave", "leave_time", "on_break", "profile", "profiles", "project", "projects", "since", "task", "today", "today_hours", "week"]);
        assert_eq!(keys(&v["week"][0]), ["date", "expected", "hours", "leave", "leave_hours"]);
        assert_eq!(keys(&v["projects"][0]), ["billable", "code", "project", "worked"]);
        let day = |i: usize, k: &str| v["week"][i][k].clone();
        let dates: Vec<_> = (0..7).map(|i| day(i, "date")).collect();
        assert_eq!(dates, (5..=11).map(|n| json!(format!("2026-01-{n:02}"))).collect::<Vec<_>>());
        let f = |x: Value| x.as_f64().unwrap();
        assert!((f(day(0, "hours")) - 4.2).abs() < 1e-9);
        assert_eq!((f(day(1, "expected")), day(1, "leave"), f(day(1, "leave_hours"))), (0.0, json!(""), 0.0)); // red day: no leave taken
        assert_eq!((f(day(3, "hours")), f(day(3, "expected"))), (1.5, 8.0)); // today: today_hours, not the log
        assert_eq!((f(day(4, "expected")), day(4, "leave"), f(day(4, "leave_hours"))), (0.0, json!("Vacation"), 8.0));
        assert_eq!((f(day(5, "expected")), f(day(6, "expected"))), (0.0, 0.0));
        // Worked descending; billable rounds with the carry across sessions (3.2 → 3.0, then 1.2 → 1.0 twice).
        let p: Vec<_> = v["projects"].as_array().unwrap().iter().map(|p| (p["project"].as_str().unwrap(), p["code"].as_str().unwrap(), f(p["worked"].clone()), f(p["billable"].clone()))).collect();
        assert_eq!(p.iter().map(|x| (x.0, x.1)).collect::<Vec<_>>(), [("Acme", "A-1"), ("Beta", "Beta"), ("Other", "Other")]);
        assert!((p[0].2 - 5.2).abs() < 1e-9 && p[0].3 == 5.0 && p[1].3 == 2.0 && p[2].3 == 1.0, "{p:?}");
        // Flex by hand on the rounded days: 12-29 +1, 01-05 4-8, 01-06 +1 (red), 01-07 2-8, 01-08 1-8.
        assert_eq!((v["flex_total"].clone(), v["flex_week"].clone()), (json!(-15.0), json!(-16.0)));
        let fx = dash_fixture("2026-01-08", (None, None));
        let leave: Vec<_> = fx.leave.iter().map(|x| x.1.clone()).collect();
        let (total, week, _) = timeclock::flex(&fx.sessions, &fx.projects, &leave, 8.0, Some((fx.today - Duration::days(3), fx.today)));
        assert_eq!((f(v["flex_total"].clone()), f(v["flex_week"].clone())), (total, week));
        let today: Vec<_> = v["today"].as_array().unwrap().iter().map(|s| s["desc"].clone()).collect();
        assert_eq!(today, [json!("c")]);
        assert_eq!((v["profile"].clone(), v["profiles"].clone(), v["backup_error"].clone()), (json!("Work"), json!(["Work", "Home"]), json!("disk full")));
        assert_eq!(v["leave_time"], json!("7h 00m left today (not clocked in)."));
        assert_eq!(v["leave"][0], json!([0, { "from": "2026-01-09", "to": "2026-01-09", "kind": "Vacation" }]));
        assert_eq!((v["project"].clone(), v["on_break"].clone()), (Value::Null, Value::Null));
    }

    #[test]
    fn dashboard_week_runs_monday_to_sunday() {
        for (t, today_col) in [("2026-01-05", 0), ("2026-01-11", 6)] {
            let v = dashboard(dash_fixture(t, (None, None)));
            assert_eq!((v["week"][0]["date"].clone(), v["week"][6]["date"].clone()), (json!("2026-01-05"), json!("2026-01-11")), "{t}");
            assert_eq!(v["week"][today_col]["hours"], json!(1.5), "{t}");
        }
        // Monday: the week's flex is that one day; Sunday: the whole week.
        assert_eq!(dashboard(dash_fixture("2026-01-05", (None, None)))["flex_week"], json!(-4.0));
        assert_eq!(dashboard(dash_fixture("2026-01-11", (None, None)))["flex_week"], json!(-16.0));
    }

    #[test]
    fn dashboard_and_status_show_the_clock() {
        let since = "2026-01-08T09:00:00".parse::<NaiveDateTime>().unwrap();
        let v = dashboard(dash_fixture("2026-01-08", (Some((since, "Acme".into(), "c".into())), None)));
        assert_eq!((v["project"].clone(), v["since"].clone(), v["task"].clone(), v["on_break"].clone()), (json!("Acme"), json!("2026-01-08T09:00:00"), json!("c"), Value::Null));
        let v = dashboard(dash_fixture("2026-01-08", (None, Some("Lunch".into()))));
        assert_eq!((v["project"].clone(), v["since"].clone(), v["on_break"].clone()), (Value::Null, Value::Null, json!("Lunch")));

        let v = status_json("Work", (Some((since, "Acme".into(), "c".into())), None, 1.5), None);
        assert_eq!(keys(&v), ["backup_error", "on_break", "profile", "project", "since", "task", "today"]);
        assert_eq!(v, json!({ "profile": "Work", "project": "Acme", "since": "2026-01-08T09:00:00", "task": "c", "on_break": null, "today": "1h 30m", "backup_error": null }));
        let v = status_json("Home", (None, Some("Lunch".into()), 0.25), Some("disk full".into()));
        assert_eq!(v, json!({ "profile": "Home", "project": null, "since": null, "task": null, "on_break": "Lunch", "today": "15m", "backup_error": "disk full" }));
    }

    fn iso(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }
    fn mon() -> NaiveDate {
        iso("2026-10-12")
    }

    #[test]
    fn dates_read_relative_to_a_given_today() {
        let t = mon();
        assert_eq!(date_at("+1d", t), Ok(iso("2026-10-13")));
        assert_eq!(date_at("fri", t), Ok(iso("2026-10-16")));
        assert_eq!(date_at("2026-12-24", t), Ok(iso("2026-12-24")));
        assert_eq!(date_at("", t), Ok(t)); // blank is today here, unlike opt_date_at
        assert_eq!(date_at("soon", t), Err("can't parse date: soon".into()));
        assert_eq!((opt_date_at("", t), opt_date_at("  ", t)), (Ok(None), Ok(None)));
        assert_eq!(opt_date_at("+1d 14:00", t), Ok(Some((iso("2026-10-13"), Some("14:00".into())))));
        assert_eq!(opt_date_at("soon", t), Err("can't read date \"soon\"".into()));
        assert_eq!(date_preview_at(" ", t), "");
        assert_eq!(date_preview_at("fri", t), "Friday 16 October 2026");
        assert_eq!(date_preview_at("fri 14:00", t), "Friday 16 October 2026, 14:00");
        assert_eq!(date_preview_at("soon", t), "✗ can't read this date");
    }

    #[test]
    fn task_text_and_planning() {
        let (t, todo) = (mon(), vec!["NEXT".to_string(), "TODO".into()]);
        assert_eq!(task_text(&todo, "  ", None, vec![], "", "", t), Err("the task needs a title".into()));
        assert_eq!(task_text(&todo, "Call", Some(""), vec![], "", "", t), Ok("* NEXT Call\n".into())); // first keyword, empty priority dropped
        assert_eq!(task_text(&[], "Call", Some("A"), vec!["home".into()], "+1d", "fri 09:00", t), Ok("* [#A] Call :home:\nSCHEDULED: <2026-10-13 Tue> DEADLINE: <2026-10-16 Fri 09:00>\n".into()));
        assert_eq!(task_text(&todo, "Call", None, vec![], "", "soon", t), Err("can't read date \"soon\"".into()));
        let set = "* TODO A\nDEADLINE: <2026-10-13 Tue>\n";
        assert_eq!(planning_at("* TODO A\n", 0, "DEADLINE", "+1d", t), Ok(set.into()));
        assert_eq!(planning_at(set, 0, "DEADLINE", " rm ", t), Ok("* TODO A\n".into()));
        assert_eq!(planning_at(set, 0, "DEADLINE", "soon", t), Err("can't parse date".into()));
    }

    #[test]
    fn capture_file_fills_the_pattern_under_notes() {
        let (n, now) = (Path::new("/n"), mon().and_hms_opt(14, 30, 0).unwrap());
        assert_eq!(capture_file(n, "daily/%Y-%m-%d.org", None, now), Ok(PathBuf::from("/n/daily/2026-10-12.org")));
        assert_eq!(capture_file(n, "daily/%Y-%m-%d.org", Some("+1d"), now), Ok(PathBuf::from("/n/daily/2026-10-13.org")));
        assert_eq!(capture_file(n, "%a-%H%M.org", Some("fri"), now), Ok(PathBuf::from("/n/Fri-1430.org"))); // now's time kept
        assert_eq!(capture_file(n, "%Q.org", None, now), Err("bad date format in %Q.org".into()));
        assert_eq!(capture_file(n, "x.org", Some("soon"), now), Err("can't parse date: soon".into()));
    }

    #[test]
    fn guard_keeps_paths_under_the_roots() {
        let roots = [PathBuf::from("/n/notes"), PathBuf::from("/d"), PathBuf::from("/c")];
        for ok in ["/n/notes/a.org", "/n/notes/sub/b.org", "/n/notes", "/d/work/timelog", "/c/config.toml"] {
            assert_eq!(check_in(&roots, Path::new(ok)), Ok(()), "{ok}");
        }
        // `..` even when it would land inside; prefixes count per component (notes2 isn't notes); relative paths.
        for bad in ["/etc/passwd", "/n/notes/../notes/a.org", "/n/notes/sub/../b.org", "/n/notes2/a.org", "/n/a.org", "n/notes/a.org", "a.org", ""] {
            assert_eq!(check_in(&roots, Path::new(bad)), Err(format!("{bad} is outside the notes/data/config dirs")), "{bad}");
        }
        // Renames are scoped to the notes folder, though check_in allows data and config.
        let notes = Path::new("/n/notes");
        assert_eq!(in_notes(notes, Path::new("/n/notes/sub/a.org")), Ok(()));
        for p in ["/d/work/timelog", "/c/config.toml", "/n/notes2/a.org"] {
            assert_eq!(in_notes(notes, Path::new(p)), Err(format!("{p} isn't in the notes folder")));
        }
        assert!(check_in(&roots, Path::new("/d/work/timelog")).is_ok() && check_in(&roots, Path::new("/c/config.toml")).is_ok());
    }

    /// Pins today's behaviour: the paths are compared as written, symlinks are not resolved.
    #[cfg(unix)]
    #[test]
    fn guard_compares_paths_as_written() {
        let dir = std::env::temp_dir().join(format!("margin-guard-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (notes, outside) = (dir.join("notes"), dir.join("outside"));
        std::fs::create_dir_all(&notes).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&notes, dir.join("link")).unwrap();
        std::os::unix::fs::symlink(&outside, notes.join("out")).unwrap();
        let roots = [notes.clone()];
        assert!(check_in(&roots, &dir.join("link/a.org")).is_err()); // the notes folder by another name
        assert!(check_in(&roots, &notes.join("out/a.org")).is_ok()); // a link inside pointing out
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_or_empty_and_ensure_id() {
        let dir = std::env::temp_dir().join(format!("margin-read-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.org"), "x\n").unwrap();
        assert_eq!(read_or_empty(&dir.join("a.org")), Ok("x\n".into()));
        assert_eq!(read_or_empty(&dir.join("missing.org")), Ok(String::new()));
        assert!(read_or_empty(&dir).is_err()); // a folder
        let _ = std::fs::remove_dir_all(&dir);
        let (text, id) = notes_ensure_id("* A\n".into());
        assert!(text.contains(&id));
        assert_eq!(notes_ensure_id(text.clone()), (text, id));
    }

    #[test]
    fn attach_in_numbers_clashes_retries_races_and_rolls_back() {
        let notes = std::env::temp_dir().join(format!("margin-attach-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&notes);
        let note = notes.join("n.org");
        let put = |dst: &Path| std::fs::write(dst, "x");
        let exists = |p: &Path| p.exists();
        let at = |taken: &dyn Fn(&Path) -> bool, w: &dyn Fn(&Path) -> std::io::Result<()>| {
            attach_in(&notes, "att", &note, "a.png", taken, w)
        };
        assert_eq!(at(&exists, &put).unwrap(), "att/n/a.png");
        assert_eq!(at(&exists, &put).unwrap(), "att/n/a-1.png");
        // Stale `taken` says free for an existing file once: create_new fails and the loop asks again.
        let calls = std::cell::Cell::new(0);
        let stale = |p: &Path| {
            calls.set(calls.get() + 1);
            calls.get() > 1 && p.exists()
        };
        assert_eq!(at(&stale, &put).unwrap(), "att/n/a-2.png");
        // A failing write removes the reserved file and reports it.
        let err = at(&exists, &|_| Err(std::io::Error::other("disk full"))).unwrap_err();
        assert!(err.contains("disk full"), "{err}");
        assert!(!notes.join("att/n/a-3.png").exists());
        let _ = std::fs::remove_dir_all(&notes);
    }

    #[test]
    fn attachment_files_skips_trash_and_unused_ignores_linked() {
        let root = std::env::temp_dir().join(format!("margin-attfiles-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for f in ["att/a/x.png", "att/a/y.png", "att/b/deep/z.png", "att/.trash/a/old.png"] {
            std::fs::create_dir_all(root.join(f).parent().unwrap()).unwrap();
            std::fs::write(root.join(f), "").unwrap();
        }
        let att = root.join("att");
        let mut files = attachment_files(&att);
        files.sort();
        assert_eq!(files, [att.join("a/x.png"), att.join("a/y.png"), att.join("b/deep/z.png")]);
        let (n1, n2) = (root.join("one.org"), root.join("two.org"));
        let notes = [(n1.as_path(), "see [[file:att/a/x.png]]"), (n2.as_path(), "[[att/b/deep/z.png][pic]]")];
        assert_eq!(attach::unused(&notes, &files), [att.join("a/y.png")]);
        let _ = std::fs::remove_dir_all(&root);
    }

    fn cfg(notes: &Path, data: &Path, profiles: &str) -> Config {
        toml::from_str(&format!("notes_dir = {:?}\ndata_dir = {:?}\nprofiles = [{profiles}]", notes, data)).unwrap()
    }

    #[test]
    fn setup_clocks_out_in_old_folder_and_warns_about_orphans() {
        let dir = std::env::temp_dir().join(format!("margin-setup-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (n1, d1, n2, d2) = (dir.join("n1"), dir.join("d1"), dir.join("n2"), dir.join("d2"));
        let old = cfg(&n1, &d1, "\"Work\", \"Home\"");
        let tc = Tc { dir: d1.join("work"), diary: n1.join("dagbok-work.org"), expected: 8.0 };
        tc.clock_in("Acme", "", None).unwrap();
        // Same folders, profile kept: nothing happens.
        assert!(!clock_out_before_setup(&tc, &old, &cfg(&n1, &d1, "\"Work\""), "Work").unwrap());
        assert!(tc.current().is_some());
        // An empty profile list (hand-edited) is no constraint either; a folder change still closes.
        assert!(!clock_out_before_setup(&tc, &old, &cfg(&n1, &d1, ""), "Work").unwrap());
        assert!(tc.current().is_some());
        // Data dir changes: the old log is closed.
        let new = cfg(&n2, &d2, "\"Work\"");
        assert!(clock_out_before_setup(&tc, &old, &new, "Work").unwrap());
        assert!(tc.current().is_none());
        // Old folders hold files, new ones are empty: warned; unchanged folders: not.
        std::fs::create_dir_all(&n2).unwrap();
        std::fs::create_dir_all(&d2).unwrap();
        let w = orphan_warning(&old, &new);
        assert!(w.contains(n1.to_str().unwrap()) && w.contains(d1.to_str().unwrap()), "{w}");
        assert_eq!(orphan_warning(&old, &old), "");
        // Dropping the open profile closes it too, and empty old folders warn about nothing.
        tc.clock_in("Acme", "", None).unwrap();
        assert!(clock_out_before_setup(&tc, &old, &cfg(&n1, &d1, "\"Home\""), "Work").unwrap());
        assert!(tc.current().is_none());
        let (n3, d3) = (dir.join("n3"), dir.join("d3")); // fresh defaults: created but empty
        std::fs::create_dir_all(&n3).unwrap();
        std::fs::create_dir_all(&d3).unwrap();
        assert_eq!(orphan_warning(&cfg(&n3, &d3, "\"Work\""), &old), "");
        // Dropped profile keeps its log folder.
        let w = orphan_warning(&old, &cfg(&n1, &d1, "\"Home\""));
        assert!(w.contains(d1.join("work").to_str().unwrap()), "{w}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
