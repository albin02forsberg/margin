mod activity;
mod ai;
mod attach;
mod babel;
mod backup;
mod config;
mod export;
mod ics;
mod ledger;
mod media;
#[cfg(desktop)]
mod idle;
#[cfg(target_os = "linux")]
mod wayland_idle;
mod notes;
mod org;
mod quit;
mod remind;
mod retrieve;
mod timeclock;
mod tray;
mod watcher;
mod weblink;

use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime};
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
        Tc { dir: c.data().join(&p), diary: c.notes().join(format!("dagbok-{p}.org")), cal: timeclock::Calendar::new(c.expected_daily_hours, &c.weekday_hours).with_days(&c.public_holidays, &c.extra_days_off) }
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

/// A CSV export may go anywhere the user picks (export_dir defaults to ~/Desktop), but only to an
/// absolute `.csv` path with no `..`, and never into the data/config dirs (time logs, config.toml).
fn csv_dest(p: &Path, forbidden: &[PathBuf]) -> R<()> {
    let csv = p.extension().is_some_and(|e| e.eq_ignore_ascii_case("csv"));
    if !p.is_absolute() || p.components().any(|c| c == Component::ParentDir) || !csv {
        return Err(format!("{} must be an absolute path to a .csv file", p.display()));
    }
    if forbidden.iter().any(|r| p.starts_with(r)) {
        return Err(format!("{} is inside the app's data or config folder", p.display()));
    }
    Ok(())
}

/// A relative CSV name (e.g. `report.csv`) lands in `export_dir`; absolute paths pass through.
fn csv_target(p: PathBuf, export_dir: &Path) -> PathBuf {
    if p.is_relative() { export_dir.join(p) } else { p }
}

fn today() -> NaiveDate {
    timeclock::now().date()
}

fn date(input: &str) -> R<NaiveDate> {
    date_at(input, today())
}

fn past_date(input: &str) -> R<NaiveDate> {
    org::read_past_date(input, today()).map(|d| d.0).ok_or_else(|| format!("can't read a past date: {input}"))
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
    if timeclock::locked(|| clock_out_before_setup(&s.tc(), &old, &new, &profile))? {
        s.backup();
    }
    *s.cfg.lock().unwrap() = new.clone();
    *s.cfg_error.lock().unwrap() = None;
    s.cache.clear();
    watch(&app);
    apply_spellcheck(&app, new.spellcheck, &new.spellcheck_languages);
    // A shortcut that won't register is reported, but the rest still applies.
    let rebind = rebind_capture(&app, &old.capture_shortcut, &new.capture_shortcut).err();
    if let Some(p) = new.profiles.first().filter(|_| !new.profiles.contains(&profile)) {
        tc_switch_profile(s, p.clone())?;
    }
    Ok(rebind.into_iter().chain([orphan_warning(&old, &new)]).filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" "))
}

/// Turn WebKitGTK's spell checker on or off for the main window (#514); it is off by default
/// and ignores the `spellcheck` attribute until then. WebKit on macOS and WebView2 need nothing.
#[cfg(target_os = "linux")]
fn apply_spellcheck(app: &AppHandle, on: bool, langs: &[String]) {
    use webkit2gtk::{WebContextExt, WebViewExt};
    let Some(w) = app.get_webview_window("main") else { return };
    let langs = langs.to_vec();
    if let Err(e) = w.with_webview(move |v| {
        if let Some(c) = v.inner().context() {
            c.set_spell_checking_enabled(on);
            // An empty list would leave WebKit's previous languages (or none) in place, so an
            // unset config falls back to the system locale (#565).
            let langs = if langs.is_empty() {
                let names = webkit2gtk::glib::language_names();
                system_spell_langs(&names.iter().map(|n| n.as_str()).collect::<Vec<_>>())
            } else {
                langs
            };
            c.set_spell_checking_languages(&langs.iter().map(String::as_str).collect::<Vec<_>>());
        }
    }) {
        eprintln!("spellcheck: {e}");
    }
}
#[cfg(not(target_os = "linux"))]
fn apply_spellcheck(_: &AppHandle, _: bool, _: &[String]) {}

/// Spell-check languages from locale names (`sv_SE.UTF-8`, `sv`, `C`): keep `ll_CC` ones with the
/// encoding/modifier stripped, deduped; `["en_US"]` when none qualify.
#[cfg(any(target_os = "linux", test))]
fn system_spell_langs(names: &[&str]) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for n in names {
        let l = n.split(['.', '@']).next().unwrap_or("");
        if l.split_once('_').is_some_and(|(a, b)| !a.is_empty() && !b.is_empty()) && !out.iter().any(|o| o == l) {
            out.push(l.to_string());
        }
    }
    if out.is_empty() {
        out.push("en_US".into());
    }
    out
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
/// TEMPLATES and editor SNIPPETS (see config::set_tables), saved like setup's answers.
#[tauri::command]
fn save_config(app: AppHandle, s: State<App>, values: toml::Table, reset: Option<Vec<String>>, views: Option<Vec<toml::Table>>, templates: Option<Vec<toml::Table>>, snippets: Option<Vec<toml::Table>>) -> R<String> {
    let text = std::fs::read_to_string(&s.cfg_path).map_err(|e| e.to_string())?;
    let (mut text, mut c) = config::save(&text, values, &reset.unwrap_or_default())?;
    for (key, items) in [("views", views), ("templates", templates), ("snippets", snippets)] {
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

/// Whether CFG (the file or, for a symlink, its target CFG_REAL) is among PATHS, and the PATHS under ROOTS (minus .git),
/// sorted and deduplicated. Under the first root (the notes) dot entries (.trash, lock files) and Emacs `#…#` autosaves are noise and dropped too.
fn split_changes(paths: Vec<PathBuf>, cfg: &Path, cfg_real: &Path, roots: &[PathBuf]) -> (bool, Vec<PathBuf>) {
    let cfg_changed = paths.iter().any(|p| p == cfg || p == cfg_real);
    let noise = |p: &Path| p.strip_prefix(&roots[0]).is_ok_and(|r| r.components().any(|c| c.as_os_str().to_str().is_some_and(|n| n.starts_with('.') || (n.len() > 1 && n.starts_with('#') && n.ends_with('#')))));
    let mut rest: Vec<PathBuf> = paths.into_iter().filter(|p| roots.iter().any(|r| p.starts_with(r)) && !p.components().any(|c| c.as_os_str() == ".git") && !noise(p)).collect();
    rest.sort();
    rest.dedup();
    (cfg_changed, rest)
}

/// Whether a change to P can alter notes, agenda or tray: anything outside the notes folder NOTES, a `.org` file, a folder (also a dotted one, or a gone one: it can't be stat-ed) or a data file.
/// Other existing files in the notes (images, attachments) only matter to an open tab.
fn needs_refresh(p: &Path, notes: &Path) -> bool {
    !p.starts_with(notes) || !p.exists() || p.is_dir() || p.extension().is_none_or(|e| ["org", "jsonl", "json", "toml"].contains(&e.to_str().unwrap_or("org")))
}

/// Watch the notes and data dirs; the UI gets an "fs-changed" event (with the
/// changed paths and which of them `needs_refresh`) whenever anything there changes — our own writes, Emacs, sync tools.
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
            let refresh: Vec<_> = paths.iter().filter(|p| needs_refresh(p, &roots[0])).collect();
            if !refresh.is_empty() {
                tray::refresh(&handle);
                write_calendar(&handle);
            }
            let _ = handle.emit("fs-changed", serde_json::json!({ "paths": paths, "refresh": refresh }));
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
    let mut meta = json!({ "currentVersion": u.current_version, "version": u.version, "body": u.body, "rawJson": u.raw_json, "pacman": pacman_owned() });
    meta["rid"] = webview.resources_table().add(u).into();
    Ok(Some(meta))
}

/// Whether pacman installed this copy (packaging/arch, #127): it can't install the .deb update itself.
/// An AppImage runs from /tmp/.mount_*, so it still updates itself.
fn pacman_owned() -> bool {
    cfg!(target_os = "linux")
        && Path::new("/usr/bin/pacman").exists()
        && std::env::current_exe().is_ok_and(|p| p.starts_with("/usr/"))
}

#[tauri::command(async)]
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
    attach_in(&c.notes(), &c.attachments_dir, &c.images_dir, &note, &name, |p| p.exists(), |dst| std::fs::copy(&src, dst).map(|_| ()))
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
    attach_in(&c.notes(), &c.attachments_dir, &c.images_dir, Path::new(&header("note")?), &header("name")?, |p| p.exists(), |dst| std::fs::write(dst, bytes))
}

/// Reserves the name with `create_new`, so concurrent attaches of one name don't collide
/// (`taken` may be stale; an `AlreadyExists` retries).
fn attach_in(
    notes: &Path,
    dir: &str,
    images: &str,
    note: &Path,
    name: &str,
    taken: impl Fn(&Path) -> bool,
    write: impl FnOnce(&Path) -> std::io::Result<()>,
) -> R<String> {
    loop {
        let (dst, link) = attach::target(notes, dir, images, note, name, &taken)?;
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

/// The IMAGES folders (see `Config::images_dir`) anywhere under NOTES, skipping dot folders.
fn image_dirs(notes: &Path, images: &str) -> Vec<PathBuf> {
    if images.is_empty() {
        return vec![];
    }
    walkdir::WalkDir::new(notes)
        .into_iter()
        .filter_entry(|e| e.depth() == 0 || !e.file_name().to_string_lossy().starts_with('.'))
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir() && e.depth() > 0 && e.path().ends_with(images))
        .map(|e| e.into_path())
        .collect()
}

/// Files in the attachments and images folders (outside `.trash`) that no note links to.
#[tauri::command]
fn unused_attachments(s: State<App>) -> Vec<PathBuf> {
    let c = s.cfg();
    let mut files = attachment_files(&c.notes().join(&c.attachments_dir));
    files.extend(image_dirs(&c.notes(), &c.images_dir).into_iter().flat_map(|d| attachment_files(&d)));
    files.sort();
    files.dedup();
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

/// Where the trashed note TRASHED (under `ROOT/.trash`) goes back to: its subpath under ROOT, minus the
/// `.N` that `trash_to` appended on a name clash, and minus a tag rename's `tag-rename-*` folder (#816).
fn restore_target(root: &Path, trashed: &Path) -> R<PathBuf> {
    let bad = || format!("{} isn't a note in {}", trashed.display(), root.join(".trash").display());
    let mut rel = trashed
        .strip_prefix(root.join(".trash"))
        .ok()
        .filter(|r| r.components().all(|c| matches!(c, Component::Normal(_))))
        .ok_or_else(bad)?;
    if rel.components().count() > 1 && rel.iter().next().and_then(|c| c.to_str()).is_some_and(|c| c.starts_with("tag-rename-")) {
        rel = rel.strip_prefix(rel.iter().next().unwrap()).unwrap();
    }
    let name = rel.file_name().and_then(|n| n.to_str()).ok_or_else(bad)?;
    let base = match name.rsplit_once('.') {
        Some((b, n)) if !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit()) && b.ends_with(".org") => b,
        _ => name,
    };
    if !base.ends_with(".org") {
        return Err(bad());
    }
    Ok(root.join(rel.with_file_name(base)))
}

/// Move the trashed note TRASHED back to its original place under ROOT; refuses to replace a file.
fn restore_to(root: &Path, trashed: &Path) -> R<PathBuf> {
    let to = restore_target(root, trashed)?;
    if !trashed.is_file() {
        return Err(format!("{} is gone", trashed.display()));
    }
    if to.exists() {
        return Err(format!("{} already exists; rename it first", to.strip_prefix(root).unwrap_or(&to).display()));
    }
    std::fs::create_dir_all(to.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::rename(trashed, &to).map_err(|e| format!("{}: {e}", trashed.display()))?;
    Ok(to)
}

/// The notes in `ROOT/.trash` (recursively) with their modification time (unix seconds), newest first.
fn trashed_in(root: &Path) -> Vec<(PathBuf, u64)> {
    fn walk(dir: &Path, out: &mut Vec<(PathBuf, u64)>) {
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let p = e.path();
            let Ok(m) = e.metadata() else { continue };
            if m.is_dir() {
                walk(&p, out);
            } else {
                let t = m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok());
                out.push((p, t.map_or(0, |d| d.as_secs())));
            }
        }
    }
    let mut v = Vec::new();
    walk(&root.join(".trash"), &mut v);
    v.retain(|(p, _)| restore_target(root, p).is_ok());
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    v
}

/// The folder whose `.trash` attachment F goes to: ATTACH (the attachments folder) when F is in it,
/// else the nearest IMAGES folder under NOTES holding F.
fn trash_root(notes: &Path, attach: &Path, images: &str, f: &Path) -> R<PathBuf> {
    if f.starts_with(attach) {
        return Ok(attach.to_path_buf());
    }
    f.ancestors()
        .skip(1)
        .take_while(|a| a.starts_with(notes) && *a != notes)
        .find(|a| !images.is_empty() && a.ends_with(images))
        .map(Path::to_path_buf)
        .ok_or_else(|| format!("{} isn't an attachment", f.display()))
}

/// Move attachment FILES to `<attachments_dir>/.trash/` (images to `<their images folder>/.trash/`),
/// keeping their subpaths.
#[tauri::command]
fn trash_attachments(s: State<App>, files: Vec<PathBuf>) -> R<()> {
    let c = s.cfg();
    for f in files {
        trash_to(&trash_root(&c.notes(), &c.notes().join(&c.attachments_dir), &c.images_dir, &f)?, &f)?;
    }
    Ok(())
}

// ---------------------------------------------------------------- org

type SpentFn = std::rc::Rc<dyn Fn(&str, &str) -> Option<u32>>;

/// Minutes clocked on a heading by (category, title), from the time log (see timeclock::spent_on).
fn spent_lookup(s: &App) -> SpentFn {
    let tc = s.tc();
    let (spent, projects) = (timeclock::spent_by_task(&tc.events(), timeclock::now()), tc.projects());
    std::rc::Rc::new(move |cat, title| timeclock::spent_on(&spent, &projects, cat, title))
}

/// ITEMS with their clocked minutes (from SPENT, a spent_lookup) filled in.
fn with_spent(spent: &dyn Fn(&str, &str) -> Option<u32>, items: Vec<org::Item>) -> Vec<org::Item> {
    items.into_iter().map(|i| org::Item { spent: spent(&i.category, &i.title), ..i }).collect()
}

#[tauri::command(async)]
fn agenda(s: State<App>, start: String, days: i64) -> R<Vec<org::Item>> {
    Ok(with_spent(&*spent_lookup(&s), org::agenda(&s.files(), &s.kw(), date(&start)?, days, today())))
}

/// Stuck projects: open tasks with open subtasks but no next action (see org::stuck).
#[tauri::command(async)]
fn org_stuck(s: State<App>) -> Vec<org::Stuck> {
    org::stuck(&s.files(), &s.kw(), &s.cfg().stuck_next_keywords)
}

/// `:GOAL:` headings with the progress of their subtasks (see org::goals).
#[tauri::command(async)]
fn org_goals(s: State<App>) -> Vec<org::Goal> {
    org::goals(&s.files(), &s.kw(), today(), &*spent_lookup(&s))
}

/// (path, line) of open tasks scheduled before today (see org::overdue_scheduled).
#[tauri::command(async)]
fn overdue_scheduled(s: State<App>) -> Vec<(PathBuf, usize)> {
    org::overdue_scheduled(&s.files(), &s.kw(), today())
}

#[tauri::command(async)]
fn todos(s: State<App>) -> Vec<org::Item> {
    with_spent(&*spent_lookup(&s), org::todos(&s.files(), &s.kw(), false, |_, _| true, today()))
}

/// The habits overview (`Space x H`): open habits over the last DAYS days, worst completion rate first.
#[tauri::command(async)]
fn habit_overview(s: State<App>, days: i64) -> Vec<org::Item> {
    org::habits(&s.files(), &s.kw(), today(), days.clamp(1, 366))
}

/// Tasks matching QUERY (see org::query).
#[tauri::command(async)]
fn search_todos(s: State<App>, query: String) -> R<Vec<org::Item>> {
    query_todos(&s, &s.files(), &query)
}

/// search_todos over FILES: the saved views' query path, shared with tasks blocks.
fn query_todos(s: &App, files: &[Arc<org::OrgFile>], query: &str) -> R<Vec<org::Item>> {
    let (kw, f) = (s.kw(), spent_lookup(s));
    let (g, bl) = (f.clone(), std::rc::Rc::new(org::blocked(files, &kw)));
    let b2 = bl.clone();
    let (keep, done) = org::query(query, today(), &kw, move |h| g(&h.category, &h.title), move |o, h| b2.contains_key(&(o.path.clone(), h.line)))?;
    Ok(with_spent(&*f, org::todos_with(files, &bl, &kw, done, keep, today())))
}

/// Open tasks tagged (own or inherited) `@NAME` or naming `@NAME` in the title, and other lines mentioning it.
fn person(files: &[Arc<org::OrgFile>], root: &Path, kw: &org::Kw, name: &str) -> (Vec<org::Item>, Vec<notes::Mention>) {
    let Some(re) = notes::person_re(name) else { return (vec![], vec![]) };
    let tag = format!("@{}", name.trim().trim_start_matches('@'));
    let keep = |_: &org::OrgFile, h: &org::Headline| h.tags.iter().chain(&h.inherited).any(|t| t.eq_ignore_ascii_case(&tag)) || re.is_match(&h.title);
    (org::todos(files, kw, false, keep, today()), notes::mentions_of(files, root, name))
}

/// The person view (`Space n @`): `{ tasks, notes, payments }` for `@NAME`; PAYMENTS are their ledger entries (#790)
/// with the ledger's path.
#[tauri::command(async)]
fn person_view(s: State<App>, name: String) -> Value {
    let (tasks, notes) = person(&s.files(), &s.cfg().notes(), &s.kw(), &name);
    let payments: Vec<Value> = ledger_text(&s).map(|(p, t)| ledger::payments(&t, &name).into_iter().map(|r| json!({ "path": p, "line": r.line, "date": r.date, "cents": r.cents, "currency": r.currency, "text": r.text })).collect()).unwrap_or_default();
    json!({ "tasks": with_spent(&*spent_lookup(&s), tasks), "notes": notes, "payments": payments })
}

#[tauri::command]
fn org_heading(s: State<App>, text: String, line: usize) -> Option<org::Parts> {
    org::heading(&text, line, &s.kw())
}

/// Headline edits. OP: cycle (VALUE ±1) | keyword (VALUE or none) | priority (letter or none)
/// | priority-cycle (±1) | tags ("a:b") | title (new text) | shift ("SCHEDULED 1")
/// | effort ("0:30", "2h"; empty clears) | prop (a "KEY VALUE" per line; no VALUE removes KEY) | move ("SCHEDULED tomorrow": absolute date, keeps time and repeater)
/// | blocker ("ID TITLE": append to `:BLOCKER:`) | unblocker (ID: drop from `:BLOCKER:`) | ordered (toggle `:ORDERED:`).
#[tauri::command]
fn org_edit(s: State<App>, text: String, line: usize, op: String, value: Option<String>) -> R<String> {
    org_edit_op(&text, line, &op, value, &s.kw(), timeclock::now(), s.cfg().log_done)
}

/// `org_edit` without app state or clock; LOG stamps `CLOSED:` on done (config `log_done`).
fn org_edit_op(text: &str, line: usize, op: &str, value: Option<String>, kw: &org::Kw, now: NaiveDateTime, log: bool) -> R<String> {
    let v = value.unwrap_or_default();
    let n = || v.trim().parse::<i32>().map_err(|_| format!("bad number: {v}"));
    let opt = || Some(v.trim()).filter(|x| !x.is_empty());
    Ok(match op {
        "cycle" => org::cycle_todo(text, line, kw, n()?, now, log),
        "keyword" => org::set_keyword(text, line, kw, opt(), now, log),
        "priority" => org::set_priority(text, line, kw, opt()),
        "priority-cycle" => org::cycle_priority(text, line, kw, n()?),
        "title" if v.trim().is_empty() => return Err("A task needs a title.".into()),
        "title" => org::set_title(text, line, kw, &v),
        "note" if v.trim().is_empty() => return Err("The note is empty.".into()),
        "note" => org::add_note(text, line, &v, now),
        "effort" => org::set_effort(text, line, opt().map(|e| org::parse_effort(e).ok_or_else(|| format!("Not a duration: {e} (try 0:30, 2h or 45min)"))).transpose()?),
        "file-prop" => {
            let (k, val) = v.split_once('=').ok_or("file-prop needs \"KEY=VALUE\"")?;
            org::set_file_prop(text, k, val)?
        }
        "prop" => v.lines().filter(|l| !l.trim().is_empty()).try_fold(text.to_string(), |t, l| {
            let (k, val) = l.trim().split_once(' ').map_or((l.trim(), ""), |(k, v)| (k, v.trim()));
            if k.contains(':') { return Err(format!("bad property name {k}")) }
            Ok(org::set_prop(&t, line, k, Some(val).filter(|v| !v.is_empty())))
        })?,
        "blocker" => {
            let (id, title) = v.split_once(' ').unwrap_or((&v, ""));
            org::add_blocker(text, line, id, title.trim())?
        }
        "unblocker" => org::remove_blocker(text, line, v.trim()),
        "ordered" => org::toggle_ordered(text, line)?,
        "tags" => org::set_tags(text, line, kw, v.split(':').map(|t| t.trim().to_string()).collect()),
        "shift" => {
            let (kind, days) = v.split_once(' ').ok_or("shift needs \"KIND DAYS\"")?;
            org::shift_date(text, line, kind, days.trim().parse().map_err(|_| "bad day count")?)
        }
        "move" => {
            let (kind, input) = v.split_once(' ').ok_or("move needs \"KIND DATE\"")?;
            org::move_planning(text, line, kind, date_at(input.trim(), now.date())?)
        }
        o => return Err(format!("unknown org op {o}")),
    })
}

/// Sort the children of the heading at or above LINE (None: top level) by KEY; `KEY-` sorts descending.
#[tauri::command]
fn org_sort(s: State<App>, text: String, line: Option<usize>, key: String) -> R<String> {
    org::sort_children(&text, line, &s.kw(), key.trim_end_matches('-'), key.ends_with('-'))
}

#[tauri::command(async)]
fn org_targets(s: State<App>) -> Vec<org::Target> {
    org::targets(&s.files(), &s.cfg().notes())
}

/// Flashcards: headings tagged `:drill:` (see org::drill_cards).
#[tauri::command(async)]
fn drill_cards(s: State<App>) -> Vec<org::Card> {
    org::drill_cards(&s.files())
}

#[tauri::command(async)]
fn org_tags(s: State<App>) -> Vec<String> {
    org::all_tags(&s.files())
}

/// Returns [new source text, new destination text].
#[tauri::command]
fn org_refile(src: String, line: usize, dst: String, dst_line: Option<usize>) -> R<[String; 2]> {
    org::refile(&src, line, &dst, dst_line).map(|(a, b)| [a, b])
}

/// `org_refile` for several (non-nested) LINES, kept in source order.
#[tauri::command]
fn org_refile_many(src: String, lines: Vec<usize>, dst: String, dst_line: Option<usize>) -> R<[String; 2]> {
    org::refile_many(&src, &lines, &dst, dst_line).map(|(a, b)| [a, b])
}

#[tauri::command]
fn org_refile_same(text: String, line: usize, dst_line: Option<usize>) -> R<String> {
    org::refile_same(&text, line, dst_line)
}

/// `org_refile_same` for several (non-nested) LINES; also returns the target line after the cut.
#[tauri::command]
fn org_refile_same_many(text: String, lines: Vec<usize>, dst_line: Option<usize>) -> R<(String, Option<usize>)> {
    org::refile_same_many(&text, &lines, dst_line)
}

/// Returns the new destination text (the source is unchanged).
#[tauri::command]
fn org_refile_copy(src: String, line: usize, dst: String, dst_line: Option<usize>) -> R<String> {
    org::refile_copy(&src, line, &dst, dst_line)
}

#[tauri::command]
fn org_refile_copy_same(text: String, line: usize, dst_line: Option<usize>) -> R<String> {
    org::refile_copy_same(&text, line, dst_line)
}

/// N copies of the subtree at LINE after it, each with its dates moved one more STEP.
#[tauri::command]
fn org_clone(s: State<App>, text: String, line: usize, n: u32, step: String) -> R<String> {
    org::clone_shift(&text, line, n, &step, &s.kw())
}

#[tauri::command]
fn org_archive(src: String, line: usize, dst: String, src_path: String) -> R<[String; 2]> {
    org::archive(&src, line, &dst, &src_path, timeclock::now()).map(|(a, b)| [a, b])
}

/// `org_archive` for several (non-nested) LINES, kept in source order.
#[tauri::command]
fn org_archive_many(src: String, lines: Vec<usize>, dst: String, src_path: String) -> R<[String; 2]> {
    org::archive_many(&src, &lines, &dst, &src_path, timeclock::now()).map(|(a, b)| [a, b])
}

/// Every done subtree of SRC moved to the archive text DST: (new src, new dst, count).
#[tauri::command]
fn org_archive_done(s: State<App>, src: String, dst: String, src_path: String) -> (String, String, usize) {
    org::archive_done(&src, &dst, &src_path, timeclock::now(), &s.kw())
}

/// Date input → Some(date), "" → None, unreadable → error.
fn opt_date_at(input: &str, today: NaiveDate) -> R<Option<org::When>> {
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
fn date_preview(input: String, kind: Option<String>) -> String {
    if kind.as_deref() == Some("past") {
        return preview_when(if input.trim().is_empty() { Ok(None) } else { org::read_past_date(&input, today()).map(Some).ok_or(String::new()) }, false);
    }
    let scheduled = kind.as_deref() == Some("SCHEDULED");
    // Planning prompts (kind given) also take a `norepeat` word.
    if kind.is_some() && input.split_whitespace().any(|w| w == org::NO_REPEAT) {
        let rest = input.split_whitespace().filter(|w| *w != org::NO_REPEAT).collect::<Vec<_>>().join(" ");
        let d = date_preview_at(&rest, today(), scheduled);
        return if d.is_empty() { "no repeat, keeps the date".into() } else if d.starts_with('✗') { d } else { format!("{d}, no repeat") };
    }
    date_preview_at(&input, today(), scheduled)
}

/// SCHEDULED: a `-3d` cookie delays the entry instead of warning ahead.
fn date_preview_at(input: &str, today: NaiveDate, scheduled: bool) -> String {
    preview_when(opt_date_at(input, today), scheduled)
}

fn preview_when(when: R<Option<org::When>>, scheduled: bool) -> String {
    match when {
        Ok(None) => String::new(),
        Ok(Some((d, t, r, w))) => [Some(d.format("%A %-d %B %Y").to_string()), t, r.as_deref().and_then(repeat_text), w.as_deref().and_then(|w| warn_text(w, scheduled))]
            .into_iter().flatten().collect::<Vec<_>>().join(", "),
        Err(_) => "✗ can't read this date".into(),
    }
}

/// "+1w" -> "every week", "++2d" -> "every 2 days, catching up", ".+1m" -> "1 month after done".
fn repeat_text(r: &str) -> Option<String> {
    let c = org::REPEATER.captures(r)?;
    let unit = match &c[3] { "h" => "hour", "d" => "day", "w" => "week", "m" => "month", _ => "year" };
    let n = &c[2];
    let every = if n == "1" { unit.to_string() } else { format!("{n} {unit}s") };
    Some(match &c[1] {
        "+" => format!("every {every}"),
        "++" => format!("every {every}, catching up"),
        _ => format!("{n} {unit}{} after done", if n == "1" { "" } else { "s" }),
    })
}

/// "-3d" -> "warn 3 days before", "-1w" -> "warn 1 week before"; SCHEDULED: "show 3 days later".
fn warn_text(w: &str, scheduled: bool) -> Option<String> {
    let w = w.strip_prefix('-')?;
    let (n, u) = w.split_at(w.len().checked_sub(1)?);
    let unit = match u { "d" => "day", "w" => "week", "m" => "month", _ => "year" };
    let (verb, rel) = if scheduled { ("show", "later") } else { ("warn", "before") };
    Some(format!("{verb} {n} {unit}{} {rel}", if n == "1" { "" } else { "s" }))
}

#[tauri::command]
fn capture_insert(s: State<App>, text: String, heading: Option<String>, entry: String) -> String {
    org::capture_insert(&text, heading.as_deref(), &entry, &s.kw())
}

/// TEXT (the ledger file) with quick-add INPUT filed under its month: [new text, the entry line].
#[tauri::command]
fn ledger_add(s: State<App>, text: String, input: String) -> R<(String, String)> {
    let e = ledger::parse_quick(&input, today(), &s.cfg().ledger_currency)?;
    Ok((ledger::add(&text, &e, &s.kw()), ledger::format_entry(&e)))
}

/// MONTH (`YYYY-MM`) of the ledger file: its path (None while the file doesn't exist), totals per currency and entries.
#[tauri::command(async)]
fn ledger_month(s: State<App>, month: String) -> R<(Option<PathBuf>, Vec<ledger::Total>, Vec<ledger::Row>)> {
    let (p, text) = ledger_text(&s)?;
    let (totals, rows) = ledger::summarize(&text, &month, &s.cfg().ledger_currency);
    Ok((p.exists().then_some(p), totals, rows))
}

/// The ledger file's path and text ("" while it doesn't exist).
fn ledger_text(s: &App) -> R<(PathBuf, String)> {
    let c = s.cfg();
    let p = capture_file(&c.notes(), &c.ledger_file, None, timeclock::now())?;
    s.check(&p)?;
    let t = read_or_empty(&p)?;
    Ok((p, t))
}

/// All-time ledger income and expense per currency of each project in NAMES (#790), for the project cards in one call.
#[tauri::command(async)]
fn ledger_projects(s: State<App>, names: Vec<String>) -> R<std::collections::BTreeMap<String, Vec<ledger::Total>>> {
    Ok(ledger::project_totals(&ledger_text(&s)?.1, &names, &ledger::id_categories(&s.files())))
}

/// Ledger entries dated START..=END as CSV at PATH (a bare name lands in `export_dir`), guarded like `tc_csv` (#791).
#[tauri::command(async)]
fn ledger_csv(s: State<App>, start: String, end: String, path: PathBuf) -> R<String> {
    let path = csv_target(path, &config::expand(&s.cfg().export_dir));
    csv_dest(&path, &[s.cfg().data(), s.cfg_path.parent().unwrap().to_path_buf()])?;
    let csv = ledger::to_csv(&ledger_text(&s)?.1, date(&start)?, date(&end)?);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    config::write_atomic(&path, &csv).map_err(|e| e.to_string())?;
    Ok(format!("✅ {} ledger entries exported to {}", csv.lines().count() - 1, path.display()))
}

/// A bank CSV at PATH (absolute `.csv`, not in the data or config folder; UTF-8, else read as Latin-1) and the mapping
/// SPEC (None: guessed) it is read with.
fn bank_csv(s: &App, path: &Path, spec: Option<&str>) -> R<(String, ledger::Mapping)> {
    csv_dest(path, &[s.cfg().data(), s.cfg_path.parent().unwrap().to_path_buf()])?;
    let b = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let csv = String::from_utf8(b).unwrap_or_else(|e| e.into_bytes().iter().map(|&b| b as char).collect());
    let m = match spec.filter(|s| !s.trim().is_empty()) {
        Some(sp) => ledger::Mapping::parse(sp)?,
        None => ledger::Mapping::guess(&csv),
    };
    Ok((csv, m))
}

/// What importing the bank CSV at PATH with SPEC (None: a guess) would propose, against the ledger and the inbox on disk:
/// `{ spec, rows: [[entry line, duplicate]] (the first 15), new, dupes, skipped }`.
#[tauri::command(async)]
fn ledger_import_preview(s: State<App>, path: PathBuf, spec: Option<String>) -> R<Value> {
    let (csv, m) = bank_csv(&s, &path, spec.as_deref())?;
    let c = s.cfg();
    let inbox = std::fs::read_to_string(c.notes().join(&c.inbox)).unwrap_or_default();
    let (es, skipped) = ledger::read_csv(&csv, &m, &c.ledger_currency, &[&ledger_text(&s)?.1, &inbox]);
    let dupes = es.iter().filter(|x| x.1).count();
    let rows: Vec<(String, bool)> = es.iter().take(15).map(|(e, d)| (ledger::format_entry(e), *d)).collect();
    Ok(json!({ "spec": m.spec(), "rows": rows, "new": es.len() - dupes, "dupes": dupes, "skipped": skipped }))
}

/// TEXT (the inbox) with the new entries of the bank CSV at PATH proposed for review: [new text, added, duplicates].
#[tauri::command(async)]
fn ledger_import(s: State<App>, text: String, path: PathBuf, spec: String) -> R<(String, usize, usize)> {
    let (csv, m) = bank_csv(&s, &path, Some(&spec))?;
    let (es, _) = ledger::read_csv(&csv, &m, &s.cfg().ledger_currency, &[&ledger_text(&s)?.1, &text]);
    let new: Vec<ledger::Entry> = es.iter().filter(|x| !x.1).map(|x| x.0.clone()).collect();
    Ok((ledger::propose(&text, &new, &s.kw()), new.len(), es.len() - new.len()))
}

/// The entry under the heading at LINE of TEXT (an inbox proposal) filed into LEDGER: [new text, new ledger, entry line].
#[tauri::command]
fn ledger_accept(s: State<App>, text: String, line: usize, ledger: String) -> R<(String, String, String)> {
    ledger::accept(&text, line, &ledger, &s.kw())
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
/// now), creating it as an org-roam note titled after its name if missing. With JOURNAL,
/// a new file also gets the `journal_template` expanded for that day, as does any new
/// file directly in the daily folder.
#[tauri::command]
fn capture_path(s: State<App>, file: String, date_input: Option<String>, journal: Option<bool>) -> R<PathBuf> {
    let now = timeclock::now();
    let p = capture_file(&s.cfg().notes(), &file, date_input.as_deref(), now)?;
    s.check(&p)?;
    if !p.exists() {
        let title = p.file_stem().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
        notes::write_note(&p, &title).map_err(|e| e.to_string())?;
        let (notes, daily) = (s.cfg().notes(), s.cfg().daily_dir.clone());
        if journal == Some(true) || is_daily_path(&notes, &daily, &p) {
            let day = match date_input.as_deref() {
                Some(i) => date_at(i, now.date())?,
                None => now.date(),
            };
            append_journal_template(&p, &s.cfg().journal_template, day)?;
        }
    }
    Ok(p)
}

/// Whether P is a direct child of the daily folder DAILY_DIR under NOTES.
fn is_daily_path(notes: &Path, daily_dir: &str, p: &Path) -> bool {
    !daily_dir.trim_matches('/').is_empty() && p.parent() == Some(notes.join(daily_dir).as_path())
}

/// Append TEMPLATE, expanded for DAY at midnight (prompts and `%^{}` empty), to the new note P.
fn append_journal_template(p: &Path, template: &str, day: NaiveDate) -> R<()> {
    if template.trim().is_empty() {
        return Ok(());
    }
    let mut body = org::expand_template(template, day.and_hms_opt(0, 0, 0).unwrap(), "", |_| String::new()).replace('\u{1}', ""); // %? marker
    if !body.ends_with('\n') {
        body.push('\n');
    }
    let mut f = std::fs::OpenOptions::new().append(true).open(p).map_err(|e| e.to_string())?;
    std::io::Write::write_all(&mut f, body.as_bytes()).map_err(|e| e.to_string())
}

/// FILE (a strftime pattern) under NOTES for DATE_INPUT at NOW's time, or for NOW.
fn capture_file(notes: &Path, file: &str, date_input: Option<&str>, now: NaiveDateTime) -> R<PathBuf> {
    let t = match date_input {
        Some(i) => date_at(i, now.date())?.and_time(now.time()),
        None => now,
    };
    Ok(notes.join(org::fmt_time(t, file).ok_or_else(|| format!("bad date format in {file}"))?))
}

/// INPUT is org-read-date style; "rm" removes the entry; a `norepeat` word drops the
/// repeater (alone: keeps the date and time).
#[tauri::command]
fn org_planning(text: String, line: usize, kind: String, input: String) -> R<String> {
    planning_at(&text, line, &kind, &input, today())
}

fn planning_at(text: &str, line: usize, kind: &str, input: &str, today: NaiveDate) -> R<String> {
    let stop = org::NO_REPEAT;
    let words: Vec<&str> = input.split_whitespace().collect();
    let d = if input.trim() == "rm" {
        None
    } else if words.contains(&stop) {
        let rest = words.iter().filter(|w| **w != stop).copied().collect::<Vec<_>>().join(" ");
        let (date, time, _, warn) = if rest.is_empty() {
            let old = org::planning_ts(text, line, kind);
            match old {
                Some(o) => (o.date, o.span(), None, None),
                None => return Ok(text.to_string()),
            }
        } else {
            org::read_date(&rest, today).ok_or("can't parse date")?
        };
        Some((date, time, Some(stop.to_string()), warn))
    } else {
        Some(org::read_date(input, today).ok_or("can't parse date")?)
    };
    Ok(org::set_planning(text, line, kind, d))
}

/// Active timestamp for INPUT (blank = today), keeping time range, repeater and warning.
fn date_stamp_at(input: &str, today: NaiveDate) -> R<String> {
    let (d, time, rep, warn) = opt_date_at(input, today)?.unwrap_or((today, None, None, None));
    Ok(org::ts_string(d, time.as_deref(), rep.as_deref(), warn.as_deref()))
}

#[tauri::command]
fn date_stamp(input: String) -> R<String> {
    date_stamp_at(&input, today())
}

#[tauri::command]
fn read_date(input: String) -> R<String> {
    date(&input).map(|d| d.to_string())
}

/// Like `read_date` for past days: weekday names look back, the future is refused.
#[tauri::command]
fn read_past_date(input: String) -> R<String> {
    past_date(&input).map(|d| d.to_string())
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
    let (path, id) = notes::new_note(&s.cfg().notes(), &title, "").map_err(|e| e.to_string())?;
    Ok(notes::Node { id, title, path, line: 0 })
}

/// Move the subtree at LINE of TEXT (the live text of PATH) into a new note
/// next to PATH, leaving a link. Returns [new text of PATH, new note's path].
#[tauri::command]
fn note_extract(s: State<App>, path: PathBuf, text: String, line: usize) -> R<[String; 2]> {
    s.check(&path)?;
    let dir = path.parent().ok_or("no folder")?;
    let mut new = PathBuf::new();
    let src = org::extract_subtree(&text, line, &s.kw(), |title, body| {
        let (p, id) = notes::new_note(dir, title, body).map_err(|e| e.to_string())?;
        new = p;
        Ok(id)
    })?;
    Ok([src, new.to_string_lossy().into_owned()])
}

/// Copy the live TEXT of note PATH into a new note TITLE beside it (fresh IDs;
/// the original's file properties and `:ID:`s are not copied). Returns (path, id).
#[tauri::command]
fn note_duplicate(s: State<App>, path: PathBuf, text: String, title: String) -> R<(PathBuf, String)> {
    s.check(&path)?;
    in_notes(&s.cfg().notes(), &path)?;
    let title = title.trim();
    if title.is_empty() {
        return Err("title is empty".into());
    }
    let dir = path.parent().ok_or("no folder")?;
    notes::new_note(dir, title, &org::duplicate_body(&text)).map_err(|e| e.to_string())
}

/// Every note's title (`#+title`, else the file name), with or without an ID.
#[tauri::command(async)]
fn note_titles(s: State<App>) -> std::collections::HashMap<PathBuf, String> {
    s.files().iter().map(|f| (f.path.clone(), f.title.clone())).collect()
}

#[tauri::command(async)]
fn notes_nodes(s: State<App>) -> Vec<notes::Node> {
    notes::nodes(&s.files())
}

#[tauri::command(async)]
fn notes_backlinks(s: State<App>, path: PathBuf) -> Vec<notes::Hit> {
    notes::backlinks(&s.files(), &path)
}

/// Notes that may link to a heading of note PATH by `[[file:…::*Title]]` (read-only).
#[tauri::command(async)]
fn retitle_candidates(s: State<App>, path: PathBuf) -> Vec<PathBuf> {
    notes::retitle_candidates(&s.files(), &path)
}

/// TEXT of NOTE with its links to heading OLD of TARGET pointed at NEW (see notes::retitle_links).
/// Pure: the frontend writes the result, so the change is undoable and open tabs stay in sync.
#[tauri::command]
fn retitle_links(text: String, note: PathBuf, target: PathBuf, old: String, new: String) -> Option<(String, usize)> {
    notes::retitle_links(&text, &note, &target, &old, &new)
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

/// Move note FROM into folder DIR under the notes folder (not a dot folder or the attachments
/// folder), fixing the links to and from it (see notes::move_note), after a backup.
#[tauri::command(async)]
fn move_note(s: State<App>, from: PathBuf, dir: PathBuf) -> R<(PathBuf, Vec<PathBuf>)> {
    let c = s.cfg();
    for p in [&from, &dir] {
        s.check(p)?;
        in_notes(&c.notes(), p)?;
    }
    notes::folder_ok(&c.notes(), &c.attachments_dir, &c.images_dir, &dir)?;
    s.backup();
    let _lock = note_writes();
    notes::move_note(&s.files(), &from, &dir)
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

/// The notes in `<notes>/.trash`, newest first, as (path, trashed unix seconds).
#[tauri::command(async)]
fn trashed_notes(s: State<App>) -> Vec<(PathBuf, u64)> {
    trashed_in(&s.cfg().notes())
}

/// Move the trashed note PATH back to where it came from (after a backup). Returns the restored path.
#[tauri::command(async)]
fn restore_note(s: State<App>, path: PathBuf) -> R<PathBuf> {
    s.check(&path)?;
    s.backup();
    let _lock = note_writes();
    restore_to(&s.cfg().notes(), &path)
}

#[tauri::command(async)]
fn notes_graph(s: State<App>, path: PathBuf) -> notes::Graph {
    notes::graph(&s.files(), &path, 2)
}

/// Whether PATH is the inbox, a journal file or trashed: not a note for orphans and tags.
fn not_a_note(c: &Config) -> impl Fn(&Path) -> bool {
    let (notes, daily, inbox) = (c.notes(), c.daily_dir.clone(), c.notes().join(&c.inbox));
    move |p: &Path| p == inbox || is_daily_path(&notes, &daily, p) || p.starts_with(notes.join(".trash"))
}

/// Notes with no links in or out (journal and inbox excluded), most recently changed first, as (path, title).
#[tauri::command(async)]
fn notes_orphans(s: State<App>) -> Vec<(PathBuf, String)> {
    let mut v: Vec<_> = notes::orphans(&s.files(), not_a_note(&s.cfg()))
        .into_iter()
        .map(|f| (std::fs::metadata(&f.path).and_then(|m| m.modified()).ok(), f.path.clone(), f.title.clone()))
        .collect();
    v.sort_by_key(|a| std::cmp::Reverse(a.0));
    v.into_iter().map(|(_, p, t)| (p, t)).collect()
}

/// Links that lead nowhere (unknown ID, missing file, heading or CUSTOM_ID), in notes outside journal, inbox and trash.
#[tauri::command(async)]
fn notes_broken_links(s: State<App>) -> Vec<notes::Broken> {
    notes::broken_links(&s.files(), not_a_note(&s.cfg()))
}

/// ITEMS (path, title, mtime secs) outside `ROOT/.trash`, newest first, at most LIMIT.
fn recent_by_mtime(items: Vec<(PathBuf, String, u64)>, root: &Path, limit: usize) -> Vec<(PathBuf, String, u64)> {
    let trash = root.join(".trash");
    let mut v: Vec<_> = items.into_iter().filter(|i| !i.0.starts_with(&trash)).collect();
    v.sort_by_key(|i| std::cmp::Reverse(i.2));
    v.truncate(limit);
    v
}

/// Notes (journal and inbox included, `.trash` not) most recently changed first, as (path, title, mtime in epoch seconds).
#[tauri::command(async)]
fn notes_recent(s: State<App>, limit: Option<usize>) -> Vec<(PathBuf, String, u64)> {
    let items = s.files().iter().filter_map(|f| {
        let t = std::fs::metadata(&f.path).and_then(|m| m.modified()).ok()?.duration_since(std::time::UNIX_EPOCH).ok()?;
        Some((f.path.clone(), f.title.clone(), t.as_secs()))
    }).collect();
    recent_by_mtime(items, &s.cfg().notes(), limit.unwrap_or(50))
}

/// ITEMS (path, title, mtime secs) untouched for at least DAYS before NOW, skipping those SKIP accepts or archive files, oldest first, at most LIMIT.
fn stale_by_mtime(items: Vec<(PathBuf, String, u64)>, skip: impl Fn(&Path) -> bool, now: u64, days: u64, limit: usize) -> Vec<(PathBuf, String, u64)> {
    let cutoff = now.saturating_sub(days * 86400);
    let mut v: Vec<_> = items.into_iter().filter(|i| i.2 <= cutoff && !skip(&i.0) && i.0.extension().is_none_or(|e| e != "org_archive")).collect();
    v.sort_by_key(|i| i.2);
    v.truncate(limit);
    v
}

/// Notes (journal, inbox, trash and archives excluded) unchanged for MIN_AGE_DAYS, oldest first, as (path, title, mtime in epoch seconds).
#[tauri::command(async)]
fn notes_stale(s: State<App>, min_age_days: u64, limit: Option<usize>) -> Vec<(PathBuf, String, u64)> {
    let items = s.files().iter().filter_map(|f| {
        let t = std::fs::metadata(&f.path).and_then(|m| m.modified()).ok()?.duration_since(std::time::UNIX_EPOCH).ok()?;
        Some((f.path.clone(), f.title.clone(), t.as_secs()))
    }).collect();
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    stale_by_mtime(items, not_a_note(&s.cfg()), now, min_age_days, limit.unwrap_or(usize::MAX))
}

/// The git commits that touched the note at PATH, newest first (Space n V, #768).
#[tauri::command(async)]
fn note_history(s: State<App>, path: PathBuf) -> R<Vec<backup::Rev>> {
    s.check(&path)?;
    backup::file_log(&path)
}

/// The note at PATH as of commit SHA, where it lived at repo-relative OLD (from `note_history`).
#[tauri::command(async)]
fn note_version(s: State<App>, path: PathBuf, sha: String, old: String) -> R<String> {
    s.check(&path)?;
    backup::file_at(&path, &sha, &old)
}

/// What `#+TRANSCLUDE: [[LINK]] PROPS` in note FROM shows (#784); Err says why it can't (missing, outside the notes).
#[tauri::command(async)]
fn transclude_text(s: State<App>, link: String, props: Option<String>, from: PathBuf) -> R<notes::Transclusion> {
    s.check(&from)?;
    notes::transclude(&s.files(), &from, &link, props.as_deref().unwrap_or(""), &|p| s.check(p))
}

/// Tags used in notes (file and non-task heading tags) as (tag, count), most used first.
#[tauri::command(async)]
fn notes_tags(s: State<App>) -> Vec<(String, usize)> {
    notes::tags(&s.files(), not_a_note(&s.cfg()))
}

/// Notes and headings tagged TAG (any case) as (path, line, title); line 0 is a file tag.
#[tauri::command(async)]
fn notes_tagged(s: State<App>, tag: String) -> Vec<(PathBuf, usize, String)> {
    notes::tagged(&s.files(), &tag, not_a_note(&s.cfg()))
}

/// What renaming a tag changes (or changed): tagged PLACES (headings and `#+FILETAGS` lines) in NOTES,
/// saved VIEWS, notes SKIPPED (unsaved edits), ERRORS (unwritable notes) and the folder of the COPIES.
#[derive(serde::Serialize, Default)]
struct TagRename {
    places: usize,
    notes: usize,
    views: usize,
    skipped: Vec<PathBuf>,
    errors: Vec<String>,
    copies: Option<PathBuf>,
    /// The notes written, as (path under the notes dir, sha256 of the new text): the undo manifest's files.
    #[serde(skip)]
    written: Vec<(PathBuf, String)>,
}

/// `manifest.json` in a tag rename's copies folder (#816): what `tag_rename_undo` needs to put the notes back.
#[derive(serde::Serialize, serde::Deserialize)]
struct RenameManifest {
    old: String,
    new: String,
    files: Vec<(PathBuf, String)>,
}

fn sha256(text: &str) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(text))
}

/// Create a new folder BASE, or BASE-2, BASE-3… if taken (`create_dir` fails on an existing one, so two
/// renames in one second get their own). Returns the folder made.
fn unique_dir(base: &Path) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(base.parent().unwrap_or(base))?;
    for i in 1.. {
        let d = if i == 1 { base.to_path_buf() } else { PathBuf::from(format!("{}-{i}", base.display())) };
        match std::fs::create_dir(&d) {
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            r => return r.map(|_| d),
        }
    }
    unreachable!()
}

/// Each task query with its `tag:OLD` terms renamed to NEW (config::retag_query), None where it has none (#824).
#[tauri::command]
fn retag_queries(queries: Vec<String>, old: String, new: String) -> Vec<Option<String>> {
    queries.iter().map(|q| config::retag_query(q, &old, &new)).collect()
}

/// Rename tag OLD (any case) to NEW in every note's heading tags (tasks too) and `#+FILETAGS`, and in saved
/// views' `tag:` terms (#785); an existing NEW merges. Only counts unless APPLY; the count uses the same
/// rewrite, so it matches what is written. Notes (or config.toml) in SKIP, open with unsaved edits, are left alone and listed.
/// APPLY backs up, writes the views first (a config error stops before any note), then copies each note it
/// changes to a new `<notes>/.trash/tag-rename-<time>/` before writing it, with a `manifest.json` for the undo.
#[tauri::command(async)]
fn tag_rename(app: AppHandle, s: State<App>, old: String, new: String, skip: Vec<PathBuf>, apply: bool) -> R<TagRename> {
    let (old, new) = (old.trim().trim_matches(':'), new.trim().trim_matches(':'));
    if !notes::tag_ok(new) {
        return Err(format!("“{new}” isn't a tag: use letters, digits, _ @ # or %"));
    }
    if old.is_empty() || old == new {
        return Err("That's already its name.".into());
    }
    let notes_dir = s.cfg().notes();
    let (views, n) = config::retag_views(&std::fs::read_to_string(&s.cfg_path).map_err(|e| e.to_string())?, old, new)?;
    let mut r = TagRename { views: n, ..Default::default() };
    if n > 0 && skip.contains(&s.cfg_path) {
        (r.views, r.skipped) = (0, vec![s.cfg_path.clone()]); // config.toml open with unsaved edits
    }
    if apply {
        s.backup();
        if r.views > 0 {
            config::write_atomic(&s.cfg_path, views).map_err(|e| e.to_string())?;
            reload_config(app, s.clone())?;
        }
    }
    // Only the apply writes, so only it holds the lock that autosaves wait on (#821).
    let _lock = apply.then(note_writes);
    let copies = match apply {
        true => Some(unique_dir(&notes_dir.join(".trash").join(chrono::Local::now().format("tag-rename-%Y%m%d-%H%M%S").to_string())).map_err(|e| e.to_string())?),
        false => None,
    };
    retag_files(&s.files(), &notes_dir, old, new, &skip, copies.as_deref(), &mut r);
    if let Some(copies) = copies {
        write_manifest(&copies, old, new, &mut r);
    }
    Ok(r)
}

/// After the apply: COPIES gets the undo manifest of what R wrote, or is removed if no note was written.
fn write_manifest(copies: &Path, old: &str, new: &str, r: &mut TagRename) {
    if r.written.is_empty() {
        let _ = std::fs::remove_dir_all(copies);
        return;
    }
    let m = RenameManifest { old: old.into(), new: new.into(), files: std::mem::take(&mut r.written) };
    let res = serde_json::to_string_pretty(&m).map_err(std::io::Error::other).and_then(|j| std::fs::write(copies.join("manifest.json"), j));
    if let Err(e) = res {
        r.errors.push(format!("undo record: {e}"));
    }
}

/// tag_rename's per-note pass over FILES: counts, and with COPIES (the apply) copies each changed note there
/// and writes it. A note whose cached text lacks OLD (any case) isn't read from disk; the rest are re-read.
fn retag_files(files: &[Arc<org::OrgFile>], notes_dir: &Path, old: &str, new: &str, skip: &[PathBuf], copies: Option<&Path>, r: &mut TagRename) {
    let old_lc = old.to_lowercase();
    for f in files.iter().filter(|f| f.text.to_lowercase().contains(&old_lc)) {
        let Ok(text) = std::fs::read_to_string(&f.path) else { continue };
        let (out, places) = notes::retag(&text, old, new);
        if places == 0 {
            continue;
        }
        if skip.contains(&f.path) {
            r.skipped.push(f.path.clone());
            continue;
        }
        if let Some(copies) = copies {
            let hash = sha256(&out);
            let res = f.path.strip_prefix(notes_dir).map_err(std::io::Error::other).and_then(|rel| {
                let dst = copies.join(rel);
                std::fs::create_dir_all(dst.parent().unwrap_or(copies))?;
                std::fs::copy(&f.path, &dst)?;
                config::write_atomic(&f.path, out).map(|_| rel.to_path_buf())
            });
            match res {
                Ok(rel) => r.written.push((rel, hash)),
                Err(e) => {
                    r.errors.push(format!("{}: {e}", f.path.display()));
                    continue;
                }
            }
            r.copies = Some(copies.to_path_buf());
        }
        (r.places, r.notes) = (r.places + places, r.notes + 1);
    }
}

/// A tag rename that can be undone: its copies FOLDER, OLD and NEW tag, how many NOTES, and when (unix seconds).
#[derive(serde::Serialize)]
struct RenameUndo {
    folder: PathBuf,
    old: String,
    new: String,
    notes: usize,
    secs: u64,
}

/// The tag renames in `ROOT/.trash` with a manifest not yet undone, newest first.
fn rename_undos(root: &Path) -> Vec<RenameUndo> {
    let mut v: Vec<RenameUndo> = std::fs::read_dir(root.join(".trash")).into_iter().flatten().flatten()
        .filter(|e| e.file_name().to_str().is_some_and(|n| n.starts_with("tag-rename-")))
        .filter_map(|e| {
            let p = e.path().join("manifest.json");
            let m: RenameManifest = serde_json::from_str(&std::fs::read_to_string(&p).ok()?).ok()?;
            let secs = std::fs::metadata(&p).and_then(|m| m.modified()).ok()?.duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
            Some(RenameUndo { folder: e.path(), old: m.old, new: m.new, notes: m.files.len(), secs })
        })
        .collect();
    v.sort_by(|a, b| b.secs.cmp(&a.secs).then_with(|| b.folder.cmp(&a.folder)));
    v
}

/// What undoing a tag rename did: notes RESTORED, CONFLICTS (edited since, gone, or open with unsaved edits:
/// left alone), ERRORS, and the folder (UNDO) that got the renamed versions it replaced.
#[derive(serde::Serialize, Default)]
struct RenameUndone {
    restored: Vec<PathBuf>,
    conflicts: Vec<PathBuf>,
    errors: Vec<String>,
    undo: Option<PathBuf>,
}

/// Put back the notes the tag rename in FOLDER (`ROOT/.trash/tag-rename-*`) changed, if each is still exactly
/// what it wrote (manifest hash); each is first copied to a new `ROOT/.trash/tag-rename-undo-<time>/`.
/// Then marks the manifest undone (`manifest.undone.json`), unless an error left something to retry.
fn undo_retag(root: &Path, folder: &Path, skip: &[PathBuf]) -> R<RenameUndone> {
    let name = folder.strip_prefix(root.join(".trash")).ok().filter(|r| r.components().count() == 1).and_then(|r| r.to_str());
    if !name.is_some_and(|n| n.starts_with("tag-rename-") && !n.starts_with("tag-rename-undo-")) {
        return Err(format!("{} isn't a tag rename in .trash", folder.display()));
    }
    let manifest = folder.join("manifest.json");
    let text = std::fs::read_to_string(&manifest).map_err(|e| format!("{}: {e}", manifest.display()))?;
    let m: RenameManifest = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", manifest.display()))?;
    let mut r = RenameUndone::default();
    for (rel, hash) in m.files {
        let note = root.join(&rel);
        if !rel.components().all(|c| matches!(c, Component::Normal(_))) {
            r.errors.push(format!("{}: not a note path", rel.display()));
            continue;
        }
        if skip.contains(&note) || std::fs::read_to_string(&note).map_or(true, |t| sha256(&t) != hash) {
            r.conflicts.push(note);
            continue;
        }
        let res = (|| -> std::io::Result<()> {
            let original = std::fs::read_to_string(folder.join(&rel))?;
            let undo = match &r.undo {
                Some(d) => d.clone(),
                None => {
                    let base = root.join(".trash").join(chrono::Local::now().format("tag-rename-undo-%Y%m%d-%H%M%S").to_string());
                    r.undo.insert(unique_dir(&base)?).clone()
                }
            };
            let dst = undo.join(&rel);
            std::fs::create_dir_all(dst.parent().unwrap_or(&undo))?;
            std::fs::copy(&note, &dst)?;
            config::write_atomic(&note, original)
        })();
        match res {
            Ok(()) => r.restored.push(note),
            Err(e) => r.errors.push(format!("{}: {e}", note.display())),
        }
    }
    if r.errors.is_empty() {
        std::fs::rename(&manifest, folder.join("manifest.undone.json")).map_err(|e| e.to_string())?;
    }
    Ok(r)
}

/// The tag renames that can be undone, newest first (#816).
#[tauri::command(async)]
fn tag_renames(s: State<App>) -> Vec<RenameUndo> {
    rename_undos(&s.cfg().notes())
}

/// Undo the tag rename whose copies are in FOLDER (after a backup); notes in SKIP (unsaved edits) are left alone.
#[tauri::command(async)]
fn tag_rename_undo(s: State<App>, folder: PathBuf, skip: Vec<PathBuf>) -> R<RenameUndone> {
    s.check(&folder)?;
    s.backup();
    let _lock = note_writes();
    undo_retag(&s.cfg().notes(), &folder, &skip)
}

/// Open top-level inbox entries: level-1 headlines that aren't done or archived (plain notes count).
fn open_entries(f: &org::OrgFile, kw: &org::Kw) -> usize {
    f.headlines.iter().filter(|h| h.level == 1 && !h.archived() && !h.keyword.as_deref().is_some_and(|k| kw.is_done(k))).count()
}

/// Entries waiting in the inbox (for the nav badge). Reads the file directly, so an absolute inbox path outside the notes dir counts too.
#[tauri::command(async)]
fn inbox_count(s: State<App>) -> usize {
    let c = s.cfg();
    let path = c.notes().join(&c.inbox);
    std::fs::read_to_string(&path).map_or(0, |t| open_entries(&org::parse(&path, &t, &s.kw()), &s.kw()))
}

#[tauri::command(async)]
fn notes_unlinked(s: State<App>, path: PathBuf) -> Vec<notes::Mention> {
    notes::unlinked(&s.files(), &path)
}

#[tauri::command]
fn notes_ensure_id(text: String) -> (String, String) {
    notes::ensure_id(&text)
}

/// Heading at LINE with an `:ID:` ensured: (new text, id, title).
#[tauri::command]
fn notes_heading_id(s: State<App>, text: String, line: usize) -> (String, String, String) {
    org::ensure_heading_id(&text, line, &s.kw())
}

/// `:BLOCKER:` entries of the heading at LINE of TEXT (PATH's buffer, which stands in for the indexed copy), with their tasks looked up.
#[tauri::command(async)]
fn org_blockers(s: State<App>, path: PathBuf, text: String, line: usize) -> Vec<org::Dep> {
    let mut files = s.files();
    files.retain(|f| f.path != path);
    files.insert(0, Arc::new(org::parse(&path, &text, &s.kw())));
    org::heading_blockers(&text, line, &files)
}

/// The open task blocking the heading at LINE of TEXT (PATH's unsaved buffer), if any.
#[tauri::command(async)]
fn org_blocked_at(s: State<App>, path: PathBuf, text: String, line: usize) -> Option<org::Blocker> {
    let kw = s.kw();
    let mut files = s.files();
    files.retain(|f| f.path != path);
    files.insert(0, Arc::new(org::parse(&path, &text, &kw)));
    org::blocked_at(&text, line, &files, &kw, &path)
}

#[tauri::command(async)]
fn notes_search(s: State<App>, query: String) -> Vec<notes::Hit> {
    notes::search(&s.files(), &s.cfg().notes(), &query, 300)
}

/// Lines of the buffer TEXT matching QUERY, with heading paths (`Space s o`).
#[tauri::command(async)]
fn notes_occur(s: State<App>, text: String, query: String) -> Result<Vec<notes::Occur>, String> {
    notes::occur(&org::parse(Path::new("occur.org"), &text, &s.kw()), &query)
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
    timeclock::leave_time(timeclock::now(), today_h, &tc.cal, &leave, running, on_break)
}

/// Whether the clock runs and today's hours are worked (timeclock::day_done), for the reminder thread.
pub(crate) fn day_done_now(s: &App) -> bool {
    let tc = s.tc();
    let (cur, on_break, today_h) = tc.status();
    let leave: Vec<_> = tc.leave().into_iter().map(|x| x.1).collect();
    timeclock::day_done(timeclock::now(), today_h, &tc.cal, &leave, cur.is_some(), on_break.is_some())
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
        today: today(), today_hours, sessions: tc.sessions(), projects: tc.projects(), leave: tc.leave(), cal: tc.cal.clone(),
        status: (cur, on_break), profile: s.profile(), profiles: s.cfg().profiles, leave_time, backup_error: backup::last_failure(),
    })
}

/// Each of DATES' expected hours after leave, as the Time view shows them (`dashboard`'s `expected`);
/// 0 on days off. WORKED is what the day's log holds (0 if nothing).
fn expected_after_leave(dates: &[NaiveDate], cal: &timeclock::Calendar, leave: &[timeclock::Leave], worked: impl Fn(NaiveDate) -> f64) -> Vec<f64> {
    dates.iter().map(|&d| { let e = timeclock::expected_hours(d, cal); e - timeclock::leave_credit(leave, d, e, worked(d)) }).collect()
}

/// Expected hours (less leave) of the active profile for each `YYYY-MM-DD` in DATES, for the agenda's planned-vs-expected.
#[tauri::command]
fn expected_hours_for(s: State<App>, dates: Vec<String>) -> Vec<f64> {
    let tc = s.tc();
    let (sessions, leave, t) = (tc.sessions(), tc.leave().into_iter().map(|x| x.1).collect::<Vec<_>>(), today());
    let ds: Vec<NaiveDate> = dates.iter().map(|d| d.parse().unwrap_or(t)).collect();
    let today_h = tc.status().2;
    expected_after_leave(&ds, &tc.cal, &leave, |d| if d == t { today_h } else { sessions.iter().filter(|x| x.date == d).map(|x| x.hours).sum() })
}

/// Journal metrics (config `metrics`, from the daily files' top drawers) over the last DAYS days,
/// today included, with each day's hours clocked and expected (#721). Days with neither a journal
/// file nor clocked time are left out.
#[tauri::command(async)]
fn metric_trend(s: State<App>, days: u32) -> org::MetricTrend {
    let (c, tc, to) = (s.cfg(), s.tc(), today());
    let from = to - Duration::days(i64::from(days.clamp(1, 3660)) - 1);
    let in_range = |d: &NaiveDate| (from..=to).contains(d);
    let mut props: std::collections::BTreeMap<NaiveDate, Vec<(String, String)>> = Default::default();
    for f in s.files().iter().filter(|f| is_daily_path(&c.notes(), &c.daily_dir, &f.path)) {
        let day = f.path.file_stem().and_then(|n| n.to_str()?.get(..10)?.parse::<NaiveDate>().ok());
        if let Some(d) = day.filter(in_range) {
            props.entry(d).or_default().extend(org::file_props(&f.text));
        }
    }
    let mut hours: std::collections::HashMap<NaiveDate, f64> = Default::default();
    for x in tc.sessions().into_iter().filter(|x| in_range(&x.date)) {
        *hours.entry(x.date).or_default() += x.hours;
        props.entry(x.date).or_default();
    }
    let dates: Vec<NaiveDate> = props.keys().copied().collect();
    let leave: Vec<_> = tc.leave().into_iter().map(|x| x.1).collect();
    let worked = |d: NaiveDate| hours.get(&d).copied().unwrap_or(0.0);
    let expected = expected_after_leave(&dates, &tc.cal, &leave, worked);
    let ds: Vec<org::MetricDay> = props.into_iter().zip(expected).map(|((date, props), expected)| org::MetricDay { date, props, hours: worked(date), expected }).collect();
    org::metric_trend(&ds, &c.metrics)
}

/// What `dashboard` is built from, gathered by `tc_dashboard`.
struct DashIn {
    today: NaiveDate,
    today_hours: f64,
    sessions: Vec<timeclock::Session>,
    projects: timeclock::Projects,
    leave: Vec<(usize, timeclock::Leave)>,
    cal: timeclock::Calendar,
    status: (Option<(NaiveDateTime, String, String)>, Option<String>),
    profile: String,
    profiles: Vec<String>,
    leave_time: String,
    backup_error: Option<String>,
}

fn dashboard(inp: DashIn) -> Value {
    let DashIn { today: t, today_hours: today_h, sessions, projects, leave, cal, status: (cur, on_break), .. } = &inp;
    let (t, today_h) = (*t, *today_h);
    let leave_only: Vec<_> = leave.iter().map(|x| x.1.clone()).collect();
    let monday = t - Duration::days(t.weekday().num_days_from_monday() as i64);
    let sunday = monday + Duration::days(6);
    let in_week = |d: NaiveDate| monday <= d && d <= sunday;
    let week: Vec<Value> = (0..7)
        .map(|i| {
            let d = monday + Duration::days(i);
            let h: f64 = if d == t { today_h } else { sessions.iter().filter(|x| x.date == d).map(|x| x.hours).sum() };
            let e = timeclock::expected_hours(d, cal);
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
    // Hours this calendar month; projects worked only earlier this month are listed too, with 0 this week.
    let in_month = |d: NaiveDate| d.year() == t.year() && d.month() == t.month();
    merged.iter().filter(|x| in_month(x.date)).for_each(|x| {
        by.entry(x.project.clone()).or_default();
    });
    let month = |p: &str| merged.iter().filter(|x| x.project == p && in_month(x.date)).map(|x| x.hours).sum::<f64>();
    // Billable hours this month at the project's rate (#801), null without one.
    let earned = |p: &str| projects.get(p).and_then(|x| x.rate()).map(|r| timeclock::earned(rounded.iter().filter(|x| x.project == p && in_month(x.date)).map(|x| x.hours).sum(), r));
    let mut per_project: Vec<Value> = by
        .into_iter()
        .map(|(p, (w, b))| json!({ "project": if p.is_empty() { "Other".to_string() } else { p.clone() }, "code": timeclock::export_code(&p, projects), "worked": w, "billable": b, "month": month(&p),
            "budget_hours": projects.get(&p).and_then(|x| x.budget_hours), "budget_period": projects.get(&p).map_or("month", |x| x.budget_period.as_str()),
            "rate": projects.get(&p).and_then(|x| x.rate()), "currency": projects.get(&p).map_or("", |x| x.currency.trim()), "earned": earned(&p) }))
        .collect();
    per_project.sort_by(|a, b| b["worked"].as_f64().unwrap_or(0.0).total_cmp(&a["worked"].as_f64().unwrap_or(0.0)));
    let (flex_total, flex_week, _) = timeclock::flex(sessions, projects, &leave_only, cal, Some((monday, t)));
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
    timeclock::locked(|| s.tc().add_leave(leave))?;
    s.backup();
    Ok(())
}

#[tauri::command(async)]
fn tc_leave_remove(s: State<App>, line: usize) -> R<()> {
    timeclock::locked(|| s.tc().remove_leave(line))?;
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
    if project.rate.is_some_and(|r| !(r.is_finite() && r >= 0.0)) {
        return Err("The hourly rate must be a positive number".into());
    }
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
    let switched = timeclock::locked(|| s.tc().clock_in(&project, &task, export_code.as_deref()))?;
    if switched {
        s.backup();
    }
    Ok(if project.trim().is_empty() { "⏱ Clocked in".into() } else { format!("⏱ Clocked in on: {project}") })
}

#[tauri::command(async)]
fn tc_out(s: State<App>, note: String) -> R<String> {
    if !timeclock::locked(|| s.tc().clock_out(&note))? {
        return Ok("Not clocked in.".into());
    }
    s.backup();
    Ok(if note.trim().is_empty() { "⏱ Clocked out.".into() } else { format!("⏱ Clocked out. Task: {note}") })
}

/// Idle prompt: clock out at SINCE, and back in at BACK unless stopping.
#[tauri::command(async)]
fn tc_idle(s: State<App>, since: NaiveDateTime, back: Option<NaiveDateTime>, note: String) -> R<String> {
    if !timeclock::locked(|| s.tc().discard_idle(since, back, &note))? {
        return Ok("Not clocked in.".into());
    }
    s.backup();
    Ok(if back.is_some() { "⏱ Idle time discarded.".into() } else { "⏱ Idle time discarded and clocked out.".into() })
}

/// Clock out at AT, earlier than now (forgot to stop).
#[tauri::command(async)]
fn tc_out_at(s: State<App>, at: NaiveDateTime, note: String) -> R<String> {
    if !timeclock::locked(|| s.tc().clock_out_earlier(at, &note))? {
        return Ok("Not clocked in.".into());
    }
    s.backup();
    Ok(format!("⏱ Clocked out at {}.", at.format("%H:%M")))
}

#[tauri::command]
fn tc_break(s: State<App>) -> R<String> {
    Ok(match timeclock::locked(|| s.tc().take_break())? {
        Some(p) => format!("☕ Clock paused! Project '{p}' saved."),
        None => "You are not clocked in right now!".into(),
    })
}

#[tauri::command]
fn tc_resume(s: State<App>) -> R<String> {
    Ok(match timeclock::locked(|| s.tc().resume())? {
        Some(p) => format!("▶ Clock is ticking again on: {p}"),
        None => "No valid break found to resume from.".into(),
    })
}

#[tauri::command]
fn tc_adjust(s: State<App>, minutes: i64) -> R<String> {
    Ok(if timeclock::locked(|| s.tc().adjust_start(minutes))? { format!("⏪ Start time moved back by {minutes} minutes.") } else { "⚠ You must be clocked in to adjust the start time!".into() })
}

#[tauri::command]
fn tc_sessions_on(s: State<App>, date_input: String) -> R<Vec<timeclock::Session>> {
    let d = date(&date_input)?;
    Ok(s.tc().sessions().into_iter().filter(|x| x.date == d && x.line.is_some()).collect())
}

/// When clocked time falls by weekday and hour (`timeclock::rhythm`) over RANGE ending today: "4w", "12w"
/// or "month" (the 1st to today), on PROJECT when given.
#[tauri::command(async)]
fn tc_rhythm(s: State<App>, range: String, project: Option<String>) -> R<timeclock::Rhythm> {
    let to = today();
    let from = match range.as_str() {
        "4w" => to - Duration::days(27),
        "12w" => to - Duration::days(83),
        "month" => to.with_day(1).unwrap(),
        _ => return Err(format!("unknown range: {range}")),
    };
    let ev = s.tc().events();
    Ok(timeclock::rhythm(&timeclock::project_spans(&ev, timeclock::now()), from, to, project.as_deref().filter(|p| !p.is_empty())))
}

#[tauri::command]
fn tc_edit_session(s: State<App>, line: usize, note: String, old_hours: f64, new_hours: f64) -> R<String> {
    timeclock::locked(|| s.tc().edit_session(line, &note, old_hours, new_hours))?;
    Ok("✅ Session updated.".into())
}

/// Sessions clocked on task OLD per project (timeclock `Tc::rename_task`), renamed to NEW when APPLY (after a heading rename).
#[tauri::command(async)]
fn tc_rename_task(s: State<App>, category: String, old: String, new: String, apply: bool) -> R<std::collections::BTreeMap<String, usize>> {
    let by = timeclock::locked(|| s.tc().rename_task(&category, &old, &new, apply))?;
    if apply && !by.is_empty() {
        s.backup();
    }
    Ok(by)
}

/// Log a finished session after the fact (an accepted suggestion).
#[tauri::command(async)]
fn tc_add_session(s: State<App>, start: NaiveDateTime, end: NaiveDateTime, project: String, export_code: Option<String>, note: String) -> R<String> {
    timeclock::locked(|| s.tc().add_session(start, end, &project, export_code.as_deref(), &note))?;
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
    let media = if c.activity_media_signal { media::intervals(&c.data().join("activity").join("media"), d) } else { vec![] };
    let all = activity::suggest(&window, &afk, &tracked, &exclude, activity::Known { projects: &projects, tasks: &tasks, meetings: &meetings, calendar: &calendar, rules: &rules, media: &media }, d);
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

/// Whether `ai_exclude` keeps the note at PATH from the AI.
#[tauri::command]
fn ai_excluded(s: State<App>, path: PathBuf) -> bool {
    let c = s.cfg();
    retrieve::excluded(&path, &c.notes(), &c.ai_exclude)
}

/// What "Use my notes" sends for QUESTION, as switched on in `ai_source_*`: the open tasks, agenda
/// and time log when asked about, then the matching notes, linked notes of the OPEN one included
/// (#304, #468). Notes matching `ai_exclude` are left out of all of it.
#[tauri::command(async)]
fn ai_retrieve(s: State<App>, question: String, open: Option<PathBuf>) -> Vec<retrieve::Source> {
    let (c, kw, day) = (s.cfg(), s.kw(), today());
    let root = c.notes();
    let files: Vec<_> = s.files().into_iter().filter(|f| !retrieve::excluded(&f.path, &root, &c.ai_exclude)).collect();
    let [tasks, agenda, time] = retrieve::wants(&question);
    let tasks = (c.ai_source_tasks && tasks).then(|| org::todos(&files, &kw, false, |_, _| true, day));
    let agenda = (c.ai_source_tasks && agenda).then(|| org::agenda(&files, &kw, day, 7, day));
    let time = (c.ai_source_time && time).then(|| s.tc().sessions());
    let mut out = retrieve::app_sources(tasks.as_deref(), agenda.as_deref(), time.as_deref(), day);
    if c.ai_source_notes {
        let used: usize = out.iter().map(retrieve::cost).sum();
        out.extend(retrieve::retrieve(&files, &question, open.as_deref(), retrieve::BUDGET.saturating_sub(used)));
    }
    out
}

/// The model's answer to the chat MESSAGES, about the open NOTE (name, text; left out when the note
/// at OPEN is excluded) and the SOURCES from ai_retrieve if given; the raw answer streams to
/// ON_PIECE as it comes, and the cleaned one is returned.
#[tauri::command(async)]
fn ai_chat(app: AppHandle, s: State<App>, messages: Vec<ai::Msg>, note: Option<(String, String)>, open: Option<PathBuf>, mut sources: Option<Vec<retrieve::Source>>, on_piece: tauri::ipc::Channel<String>) -> R<String> {
    let c = s.cfg();
    if c.ai_model.trim().is_empty() {
        return Err("Pick a model with AI drafts: choose model… (Space f a) to chat with a local model.".into());
    }
    let root = c.notes();
    let ex = |p: &Path| retrieve::excluded(p, &root, &c.ai_exclude);
    let note = note.filter(|_| !open.as_deref().is_some_and(ex));
    if let Some(v) = &mut sources {
        v.retain(|s| s.kind != "note" || !ex(s.path.as_path()));
    }
    let msgs = ai::chat_messages(note.as_ref().map(|(n, t)| (n.as_str(), t.as_str())), sources.as_deref(), &messages)?;
    ai::chat(c.ai_backend, &c.ai_url, &models_dir(&app)?, c.ai_model.trim(), &msgs, &mut |p| {
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

/// Inclusive range of the week report: the Mon-Sun week containing PICKED, else the last 7 days ending TODAY.
fn week_range(picked: Option<NaiveDate>, today: NaiveDate) -> (NaiveDate, NaiveDate) {
    match picked {
        Some(d) => {
            let mon = d - Duration::days(d.weekday().num_days_from_monday().into());
            (mon, mon + Duration::days(6))
        }
        None => (today - Duration::days(6), today),
    }
}

/// Inclusive range of the month report: the calendar month containing PICKED, else the one containing TODAY.
fn month_range(picked: Option<NaiveDate>, today: NaiveDate) -> (NaiveDate, NaiveDate) {
    let first = picked.unwrap_or(today).with_day(1).unwrap();
    (first, first.checked_add_months(chrono::Months::new(1)).unwrap() - Duration::days(1))
}

/// Org-formatted report text. KIND: daily | weekly | monthly | holidays | flex | doctor | backup.
#[tauri::command]
fn tc_report(s: State<App>, kind: String, date_input: Option<String>) -> R<String> {
    let tc = s.tc();
    let profile = s.profile();
    let leave: Vec<_> = tc.leave().into_iter().map(|x| x.1).collect();
    let codes = tc.leave_codes().unwrap_or_default();
    Ok(match kind.as_str() {
        "daily" => timeclock::daily_report(&tc.sessions(), &tc.projects(), (&leave, &codes), &tc.cal, date(date_input.as_deref().unwrap_or(""))?, &profile),
        "weekly" => {
            let (from, to) = week_range(date_input.as_deref().filter(|d| !d.trim().is_empty()).map(date).transpose()?, today());
            timeclock::weekly_report(&tc.sessions(), &tc.projects(), (&leave, &codes), &tc.cal, from, to, &profile)
        }
        "monthly" => {
            let (from, to) = month_range(date_input.as_deref().filter(|d| !d.trim().is_empty()).map(date).transpose()?, today());
            timeclock::weekly_report(&tc.sessions(), &tc.projects(), (&leave, &codes), &tc.cal, from, to, &profile)
        }
        "holidays" => timeclock::holidays_report(date(date_input.as_deref().unwrap_or(""))?.format("%Y").to_string().parse().unwrap(), &tc.cal),
        "flex" => {
            let t = today();
            let week = week_range(None, t);
            let (total, period, days) = timeclock::flex(&tc.sessions(), &tc.projects(), &leave, &tc.cal, Some(week));
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

/// Options of a `#+BEGIN: clocktable` header after the name: `:block today|thisweek|thismonth|YYYY-MM` (required)
/// and `:project NAME` ("quoted" for spaces). Any other option is an error, so Emacs-only blocks stay untouched.
fn clocktable_opts(args: &str, today: NaiveDate) -> R<(Option<String>, NaiveDate, NaiveDate)> {
    let (mut project, mut range) = (None, None);
    for (key, val) in block_opts("clocktable", args)? {
        let val = val.as_str();
        match key.as_str() {
            ":project" => project = Some(val.to_string()),
            ":block" => {
                range = Some(match val {
                    "today" => (today, today),
                    "thisweek" => week_range(Some(today), today),
                    "thismonth" => month_range(Some(today), today),
                    m => month_range(Some(NaiveDate::parse_from_str(&format!("{m}-01"), "%Y-%m-%d").map_err(|_| format!("clocktable: unknown :block {m}"))?), today),
                })
            }
            o => return Err(format!("clocktable: unsupported option {o}")),
        }
    }
    let (from, to) = range.ok_or("clocktable: needs :block today|thisweek|thismonth|YYYY-MM")?;
    Ok((project, from, to))
}

/// `:key value` pairs of the dynamic block NAME's header ARGS; a value may be "quoted", with `\"` for a quote.
fn block_opts(name: &str, args: &str) -> R<Vec<(String, String)>> {
    let mut words = vec![];
    let mut it = args.trim().chars().peekable();
    while let Some(c) = it.next() {
        if c.is_whitespace() {
            continue;
        }
        let mut w = String::new();
        if c == '"' {
            loop {
                match it.next().ok_or(format!("{name}: unclosed quote"))? {
                    '\\' if it.next_if_eq(&'"').is_some() => w.push('"'),
                    '"' => break,
                    c => w.push(c),
                }
            }
        } else {
            w.push(c);
            while let Some(c) = it.next_if(|c| !c.is_whitespace()) {
                w.push(c);
            }
        }
        words.push(w);
    }
    words.chunks(2).map(|p| Ok((p[0].clone(), p.get(1).ok_or_else(|| format!("{name}: {} needs a value", p[0]))?.clone()))).collect()
}

/// Options of a `#+BEGIN: tasks` header (#756): `:query "…"` (required, the saved views' grammar), `:limit N`
/// (default 20) and `:sort priority|date|title` (default priority, the views' order). Anything else is an error.
fn tasks_opts(args: &str) -> R<(String, usize, String)> {
    let (mut query, mut limit, mut sort) = (None, 20, "priority".to_string());
    for (key, val) in block_opts("tasks", args)? {
        match key.as_str() {
            ":query" => query = Some(val),
            ":limit" => limit = val.parse().map_err(|_| format!("tasks: :limit takes a number, not {val}"))?,
            ":sort" if ["priority", "date", "title"].contains(&val.as_str()) => sort = val,
            ":sort" => return Err(format!("tasks: :sort takes priority, date or title, not {val}")),
            o => return Err(format!("tasks: unsupported option {o}")),
        }
    }
    Ok((query.filter(|q| !q.trim().is_empty()).ok_or("tasks: needs :query \"…\"")?, limit, sort))
}

/// Body of a tasks block in the note in folder DIR: the first LIMIT of ITEMS (by priority, as org::todos returns
/// them, or SORT date|title), one plain list item each with a `file:…::*Title` link relative to DIR and the task's
/// SCHEDULED / DEADLINE (from FILES) and effort. No checkboxes: ticking one wouldn't close the task.
fn tasks_body(mut items: Vec<org::Item>, files: &[Arc<org::OrgFile>], dir: &Path, limit: usize, sort: &str) -> String {
    match sort {
        "date" => items.sort_by_key(|i| i.date),
        "title" => items.sort_by_key(|i| i.title.to_lowercase()),
        _ => {}
    }
    let mut out = String::new();
    for i in items.iter().take(limit) {
        let h = files.iter().find(|f| f.path == i.path).and_then(|f| f.headlines.iter().find(|h| h.line == i.line));
        let mut info = vec![];
        if let Some(h) = h {
            info.extend(h.scheduled.as_ref().map(|t| format!("SCHEDULED {}", t.date)));
            info.extend(h.deadline.as_ref().map(|t| format!("DEADLINE {}", t.date)));
        }
        info.extend(i.effort.map(|m| format!("effort {}:{:02}", m / 60, m % 60)));
        let title = notes::heading_bare(&i.title);
        let link = if title.is_empty() || title.contains(['[', ']']) { title } else { format!("[[file:{}::*{title}][{title}]]", notes::rel_path(dir, &i.path)) };
        let pre: String = [i.keyword.clone(), i.priority.as_ref().map(|p| format!("[#{p}]"))].into_iter().flatten().map(|w| w + " ").collect();
        out += &format!("- {pre}{link}{}\n", if info.is_empty() { String::new() } else { format!(" ({})", info.join(", ")) });
    }
    if out.is_empty() { "(no matches)\n".into() } else { out }
}

/// Body of the tasks block with header options ARGS in note PATH (#756).
#[tauri::command(async)]
fn tasks_block(s: State<App>, path: PathBuf, args: String) -> R<String> {
    s.check(&path)?;
    let (query, limit, sort) = tasks_opts(&args)?;
    let files = s.files();
    let items = query_todos(&s, &files, &query)?;
    Ok(tasks_body(items, &files, path.parent().unwrap_or(&path), limit, &sort))
}

/// Options of a `#+BEGIN: columnview` header (#809): `:query "…"` (required, the saved views' grammar over all headings),
/// `:cols "ITEM STATUS"` (default ITEM), `:sort COL` (`-COL` descending) and `:limit N` (default 50, at most 500).
/// Anything else (Emacs' `:id`, `:hlines` …) is an error, so such blocks stay untouched.
type ColOpts = (String, Vec<String>, Option<(String, bool)>, usize);
fn columnview_opts(args: &str) -> R<ColOpts> {
    let (mut query, mut cols, mut sort, mut limit) = (None, vec!["ITEM".to_string()], None, 50);
    for (key, val) in block_opts("columnview", args)? {
        match key.as_str() {
            ":query" => query = Some(val),
            ":cols" => cols = val.split_whitespace().map(String::from).collect(),
            ":sort" => {
                let (c, desc) = val.strip_prefix('-').map_or((val.as_str(), false), |c| (c, true));
                if c.is_empty() { return Err("columnview: :sort needs a column".into()) }
                sort = Some((c.to_string(), desc));
            }
            ":limit" => limit = val.parse().ok().filter(|n| (1..=500).contains(n)).ok_or_else(|| format!("columnview: :limit takes a number up to 500, not {val}"))?,
            o => return Err(format!("columnview: unsupported option {o}")),
        }
    }
    if cols.is_empty() { return Err("columnview: :cols needs at least one column".into()) }
    Ok((query.filter(|q| !q.trim().is_empty()).ok_or("columnview: needs :query \"…\"")?, cols, sort, limit))
}

/// Body of a columnview block in the note in folder DIR: an aligned org table with a COLS header and one row per heading
/// of FILES for which KEEP holds (any heading, keyword or not; archived ones skipped), sorted by SORT (numbers and
/// durations as numbers before text, text ignoring case, empty cells last), the first LIMIT. ITEM links to the heading;
/// TODO, TAGS, SCHEDULED, DEADLINE, EFFORT and CLOCKSUM (from SPENT) come from it, any other column is its property.
fn columnview_body(files: &[Arc<org::OrgFile>], keep: impl Fn(&org::OrgFile, &org::Headline) -> bool, cols: &[String], sort: Option<&(String, bool)>, limit: usize, dir: &Path, spent: &dyn Fn(&str, &str) -> Option<u32>) -> String {
    let hm = |m: u32| format!("{}:{:02}", m / 60, m % 60);
    let esc = |s: &str| s.replace(['\n', '\r'], " ").replace('|', "\\vert{}");
    let mut rows: Vec<(Vec<String>, String)> = vec![];
    for f in files {
        let lines = std::cell::OnceCell::new();
        for h in f.headlines.iter().filter(|h| !h.archived() && keep(f, h)) {
            let cell = |c: &str| esc(&match c.to_uppercase().as_str() {
                "ITEM" => {
                    let t = notes::heading_bare(&h.title);
                    match &h.id {
                        _ if t.is_empty() || t.contains(['[', ']', '|']) => t,
                        Some(id) => format!("[[id:{id}][{t}]]"),
                        None => format!("[[file:{}::*{t}][{t}]]", notes::rel_path(dir, &f.path)),
                    }
                }
                "TODO" => h.keyword.clone().unwrap_or_default(),
                "TAGS" if h.tags.is_empty() => String::new(),
                "TAGS" => format!(":{}:", h.tags.join(":")),
                "SCHEDULED" => h.scheduled.as_ref().map(|t| t.date.to_string()).unwrap_or_default(),
                "DEADLINE" => h.deadline.as_ref().map(|t| t.date.to_string()).unwrap_or_default(),
                "EFFORT" => h.effort.map(hm).unwrap_or_default(),
                "CLOCKSUM" => spent(&h.category, &h.title).map(hm).unwrap_or_default(),
                _ => org::heading_prop(lines.get_or_init(|| f.text.lines().map(String::from).collect::<Vec<_>>()), h.line, c).unwrap_or_default(),
            });
            rows.push((cols.iter().map(|c| cell(c)).collect(), sort.map(|(c, _)| if c.eq_ignore_ascii_case("ITEM") { notes::heading_bare(&h.title) } else { cell(c) }).unwrap_or_default()));
        }
    }
    if let Some((_, desc)) = sort {
        let num = |s: &str| s.parse::<f64>().ok().filter(|x| x.is_finite()).or_else(|| org::parse_effort(s).map(f64::from));
        rows.sort_by(|(_, a), (_, b)| match (a.is_empty(), b.is_empty()) {
            (false, false) => {
                let o = match (num(a), num(b)) {
                    (Some(x), Some(y)) => x.total_cmp(&y),
                    (x, y) => y.is_some().cmp(&x.is_some()).then_with(|| a.to_lowercase().cmp(&b.to_lowercase())),
                };
                if *desc { o.reverse() } else { o }
            }
            (x, y) => x.cmp(&y),
        });
    }
    let table: Vec<Vec<String>> = std::iter::once(cols.iter().map(|c| esc(c)).collect()).chain(rows.into_iter().take(limit).map(|r| r.0)).collect();
    let w: Vec<usize> = (0..cols.len()).map(|i| table.iter().map(|r| r[i].chars().count()).max().unwrap_or(0)).collect();
    let row = |r: &[String]| format!("| {} |\n", r.iter().zip(&w).map(|(c, w)| format!("{c:<w$}")).collect::<Vec<_>>().join(" | "));
    let rule = format!("|{}|\n", w.iter().map(|w| "-".repeat(w + 2)).collect::<Vec<_>>().join("+"));
    row(&table[0]) + &rule + &table[1..].iter().map(|r| row(r)).collect::<String>()
}

/// Body of the columnview block with header options ARGS in note PATH (#809).
#[tauri::command(async)]
fn columnview_block(s: State<App>, path: PathBuf, args: String) -> R<String> {
    s.check(&path)?;
    let (query, cols, sort, limit) = columnview_opts(&args)?;
    let (files, kw, f) = (s.files(), s.kw(), spent_lookup(&s));
    let (g, bl) = (f.clone(), org::blocked(&files, &kw));
    let (keep, _) = org::query(&query, today(), &kw, move |h| g(&h.category, &h.title), move |o, h| bl.contains_key(&(o.path.clone(), h.line)))?;
    Ok(columnview_body(&files, keep, &cols, sort.as_ref(), limit, path.parent().unwrap_or(&path), &*f))
}

/// Body of a clocktable block with header options ARGS (#693).
#[tauri::command]
fn tc_clocktable(s: State<App>, args: String) -> R<String> {
    let (project, from, to) = clocktable_opts(&args, today())?;
    Ok(timeclock::clocktable(&s.tc().sessions(), project.as_deref(), from, to))
}

#[tauri::command]
fn tc_csv(s: State<App>, start: String, end: String, path: PathBuf) -> R<String> {
    let path = csv_target(path, &config::expand(&s.cfg().export_dir));
    csv_dest(&path, &[s.cfg().data(), s.cfg_path.parent().unwrap().to_path_buf()])?;
    let tc = s.tc();
    let leave: Vec<_> = tc.leave().into_iter().map(|x| x.1).collect();
    let (csv, off) = timeclock::csv(&tc.sessions(), &tc.load_projects()?, (&leave, &tc.leave_codes()?), &tc.cal, date(&start)?, date(&end)?);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?; // the export dir may not exist yet
    }
    config::write_atomic(&path, csv).map_err(|e| e.to_string())?;
    Ok(format!("✅ CSV exported to {} ({off})", path.display()))
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

/// TEXT (the buffer of PATH) with its `#+TRANSCLUDE:` lines expanded (#796).
fn expand(s: &App, path: &Path, text: &str) -> String {
    notes::expand_transclusions(&s.files(), path, text, &|p| s.check(p))
}

/// Export note TEXT (the buffer of PATH) to export_dir. FORMAT: html | md | pdf (HTML that opens the print dialog).
#[tauri::command]
fn export_note(s: State<App>, path: PathBuf, text: String, format: String, overwrite: Option<bool>) -> R<PathBuf> {
    let d = export::parse(&expand(&s, &path, &text), &s.kw());
    let stem = stem(&path);
    let base = path.parent().unwrap_or(Path::new(""));
    match format.as_str() {
        "md" => write_export(&s, &format!("{stem}.md"), export::markdown(&d, &stem, base, &export::Ids::new()), overwrite),
        f => write_export(&s, &format!("{stem}.html"), export::html(&d, &stem, base, &export::Ids::new(), f == "pdf"), overwrite),
    }
}

/// TEXT (the buffer or selection of PATH) as Markdown, for the clipboard; nothing is written.
#[tauri::command]
fn export_markdown(s: State<App>, path: PathBuf, text: String) -> String {
    export::markdown(&export::parse(&expand(&s, &path, &text), &s.kw()), &stem(&path), path.parent().unwrap_or(Path::new("")), &export::Ids::new())
}

/// TEXT (the buffer or selection of PATH) as an HTML fragment, for the rich-text clipboard; nothing is written.
#[tauri::command]
fn export_html_fragment(s: State<App>, path: PathBuf, text: String) -> String {
    export::html_body(&export::parse(&expand(&s, &path, &text), &s.kw()), &stem(&path), path.parent().unwrap_or(Path::new("")), &export::Ids::new())
}

/// Export note TEXT (the buffer of PATH) and the notes it reaches by id: links within DEPTH hops
/// as FORMAT (html | md) pages in `<export_dir>/<stem>/`, id: links between them made relative,
/// pages of an earlier export no longer included moved into `old/`; pdf: one `<stem> (linked).html` print page. Returns PATH's page.
#[tauri::command(async)]
fn export_linked(s: State<App>, path: PathBuf, text: String, depth: usize, format: String, overwrite: Option<bool>) -> R<PathBuf> {
    let kw = s.kw();
    let root = org::parse(&path, &text, &kw);
    let files = s.files();
    let files: Vec<&org::OrgFile> = files.iter().map(|f| &**f).collect();
    let ext = if format == "md" { "md" } else { "html" };
    let (pages, ids) = export::bundle(&files, &root, depth, ext);
    let texts: Vec<String> = pages.iter().map(|(f, _)| expand(&s, &f.path, &f.text)).collect();
    if format == "pdf" {
        return write_export(&s, &format!("{} (linked).html", stem(&path)), export::combined(&pages, &texts, &ids, &kw), overwrite);
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
    for ((f, name), text) in pages.iter().zip(&texts) {
        let (d, title, base) = (export::parse(text, &kw), stem(&f.path), f.path.parent().unwrap_or(Path::new("")));
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
    let org = timeclock::weekly_report(&tc.sessions(), &tc.load_projects()?, (&leave, &tc.leave_codes()?), &tc.cal, start, end, &s.profile());
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
    if timeclock::locked(|| s.tc().clock_out(&format!("Auto-clockout (Switched to {name} profile)")))? {
        s.backup();
    }
    *s.profile.lock().unwrap() = name.clone();
    let _ = std::fs::create_dir_all(s.cfg().data());
    std::fs::write(s.profile_file(), &name).map_err(|e| e.to_string())?;
    Ok(format!("🔄 Profile switched to: {name}"))
}

#[tauri::command]
fn tc_import(s: State<App>, dir: String) -> R<String> {
    let n = timeclock::locked(|| s.tc().import_emacs(&config::expand(&dir), &s.profile()))?;
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
            {
                let c = app.state::<App>().cfg();
                apply_spellcheck(app.handle(), c.spellcheck, &c.spellcheck_languages);
            }
            write_calendar(app.handle());
            tray::setup(app.handle())?;
            #[cfg(target_os = "macos")]
            quit::app_menu(app.handle())?;
            app.handle().plugin(tauri_plugin_notification::init())?;
            remind::start(app.handle());
            let h = app.handle().clone();
            backup::start(move || h.state::<App>().cfg().data());
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
            if let WindowEvent::Focused(true) = ev {
                backup::sync_soon(w.app_handle().state::<App>().cfg().data()); // pick up another machine's time
            }
            if let WindowEvent::CloseRequested { api, .. } = ev {
                let app = w.app_handle();
                if app.state::<App>().cfg().close_to_tray && tray::tracking(app) {
                    api.prevent_close();
                    let _ = w.hide();
                } else if !quit::confirmed() {
                    // A real close: the webview holding unsaved edits goes with it, so ask first (#281).
                    api.prevent_close();
                    quit::ask(app);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            watcher::gnome_extension, watcher::screen_recording,
            config, config_recreate, tutorial, update_check, reload_config, save_view, setup, save_config, list_files, read_file, file_exists, write_file, attach_file, attach_bytes, unused_attachments, trash_attachments,
            agenda, overdue_scheduled, todos, habit_overview, search_todos, org_heading, org_edit, org_sort, org_planning, read_date, date_stamp, read_past_date, org_context,
            org_targets, org_tags, drill_cards, metric_trend, org_refile, org_refile_many, org_archive_many, org_refile_same, org_refile_same_many, org_refile_copy, org_refile_copy_same, org_clone, note_extract, note_duplicate, org_archive, org_archive_done, capture_insert, ledger_add, ledger_month, ledger_projects, ledger_csv, ledger_import_preview, ledger_import, ledger_accept, capture_path, capture_templates, template_prompts, capture_template, task_entry, date_preview, tc_dashboard, expected_hours_for,
            notes_new, note_titles, notes_nodes, notes_backlinks, retitle_candidates, retitle_links, rename_note, move_note, trash_note, trashed_notes, restore_note, notes_search, notes_occur, notes_graph, notes_orphans, notes_broken_links, org_stuck, org_goals, notes_recent, notes_stale, note_history, note_version, transclude_text, notes_tags, notes_tagged, tag_rename, tag_renames, tag_rename_undo, retag_queries, person_view, inbox_count, notes_unlinked, notes_ensure_id, notes_heading_id, org_blockers, org_blocked_at,
            tc_status, tc_leave_time, tc_projects, tc_save_project, tc_suggestions, tc_in, tc_out, tc_idle, tc_out_at, tc_break, tc_resume, tc_adjust,
            tc_sessions_on, tc_rhythm, tc_edit_session, tc_add_session, tc_rename_task, tc_leave_add, tc_leave_remove, tc_leave_codes, tc_save_leave_code,activity_suggestions, activity_dismiss, activity_learn, activity_rules_list, activity_rules_delete, ai_note_prompt, ai_day_prompt, ai_draft, ai_retrieve, ai_excluded, ai_chat, ai_draft_cancel, ai_models, ai_set, ai_download, ai_download_cancel, ai_model_delete, tc_report, tc_clocktable, tasks_block, columnview_block, tc_csv, tc_switch_profile, tc_import, backup_now,
            export_note, export_markdown, export_html_fragment, export_linked, export_report, export_open, quit::quit_reply, weblink::fetch_title, babel::babel_run
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, ev| {
            // Replaces kill-emacs-hook: commit on exit.
            match ev {
                RunEvent::ExitRequested { code, api, .. } if !quit::exit_allowed(code, quit::confirmed()) => {
                    api.prevent_exit();
                    quit::ask(app)
                }
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
    fn retag_queries_maps_each_query() {
        let q = vec!["tag:Old due:<7d".into(), "-tag:old".into(), "tag:other".into()];
        assert_eq!(retag_queries(q, "old".into(), "new".into()), vec![Some("tag:new due:<7d".into()), Some("-tag:new".into()), None]);
    }

    #[test]
    fn person_view_lists_open_tagged_tasks_and_mentions() {
        let kw = org::Kw { todo: vec!["TODO".into()], done: vec!["DONE".into()] };
        let doc = "#+title: W\n* Project :@anna:\n** TODO Inherited\n* TODO Call @Anna\n* TODO Mail bob@anna.se\n* DONE Paid @anna\nnotes for @anna\n";
        let files = [Arc::new(org::parse(Path::new("/n/w.org"), doc, &kw))];
        let (tasks, notes) = person(&files, Path::new("/n"), &kw, "Anna");
        let mut t: Vec<_> = tasks.iter().map(|i| i.title.as_str()).collect();
        t.sort();
        assert_eq!(t, ["Call @Anna", "Inherited"]);
        assert_eq!(notes.iter().map(|m| m.line).collect::<Vec<_>>(), [1, 6]);
        assert_eq!(person(&files, Path::new("/n"), &kw, "@").0.len(), 0);
    }

    #[test]
    fn month_range_covers_calendar_month() {
        let d = |y, m, dd| NaiveDate::from_ymd_opt(y, m, dd).unwrap();
        assert_eq!(month_range(Some(d(2024, 2, 10)), d(2024, 3, 5)), (d(2024, 2, 1), d(2024, 2, 29))); // leap Feb
        assert_eq!(month_range(Some(d(2025, 12, 31)), d(2026, 1, 1)), (d(2025, 12, 1), d(2025, 12, 31))); // Dec rollover
        assert_eq!(month_range(None, d(2026, 10, 21)), (d(2026, 10, 1), d(2026, 10, 31)));
    }

    #[test]
    fn recent_notes_newest_first_limited_without_trash() {
        let root = Path::new("/n");
        let it = |p: &str, t| (PathBuf::from(p), String::new(), t);
        let v = vec![it("/n/a.org", 10), it("/n/.trash/x.org", 99), it("/n/daily/2026-10-20.org", 30), it("/n/b.org", 20)];
        let times = |v: Vec<(PathBuf, String, u64)>| v.into_iter().map(|i| i.2).collect::<Vec<_>>();
        assert_eq!(times(recent_by_mtime(v.clone(), root, 50)), [30, 20, 10]);
        assert_eq!(times(recent_by_mtime(v, root, 2)), [30, 20]);
    }

    #[test]
    fn stale_notes_oldest_first_skipping_young_and_archives() {
        let it = |p: &str, t| (PathBuf::from(p), String::new(), t);
        let day = 86400;
        let v = vec![it("/n/a.org", 10 * day), it("/n/b.org", 5 * day), it("/n/new.org", 99 * day), it("/n/x.org_archive", day), it("/n/skip.org", day), it("/n/c.org", 20 * day)];
        let r = stale_by_mtime(v.clone(), |p| p.ends_with("skip.org"), 100 * day, 30, 50);
        assert_eq!(r.iter().map(|i| i.2 / day).collect::<Vec<_>>(), [5, 10, 20]);
        assert_eq!(stale_by_mtime(v.clone(), |_| false, 100 * day, 30, 2).len(), 2);
        assert_eq!(stale_by_mtime(v, |_| false, 100 * day, 0, 50).len(), 5); // 0 days: everything but archives
    }

    #[test]
    fn clocktable_options() {
        let d = |m, day| NaiveDate::from_ymd_opt(2026, m, day).unwrap();
        let t = d(10, 22); // a Thursday
        assert_eq!(clocktable_opts(":block today", t), Ok((None, t, t)));
        assert_eq!(clocktable_opts(":block thisweek", t), Ok((None, d(10, 19), d(10, 25))));
        assert_eq!(clocktable_opts(" :project \"Big Co\"  :block thismonth ", t), Ok((Some("Big Co".into()), d(10, 1), d(10, 31))));
        assert_eq!(clocktable_opts(":block 2026-02 :project Acme", t), Ok((Some("Acme".into()), d(2, 1), d(2, 28))));
        for bad in [":scope file :block thismonth", ":block lastweek", ":block 2026-13", ":project Acme", ":block", ":project \"Acme :block today", ""] {
            assert!(clocktable_opts(bad, t).is_err(), "{bad}");
        }
    }

    #[test]
    fn tasks_block_options() {
        let o = |q: &str, l, s: &str| Ok((q.to_string(), l, s.to_string()));
        assert_eq!(tasks_opts(":query \"tag:work due:<7d\""), o("tag:work due:<7d", 20, "priority"));
        assert_eq!(tasks_opts(" :limit 5  :query \"tag:\\\"my tag\\\"\" :sort date"), o("tag:\"my tag\"", 5, "date"));
        assert_eq!(tasks_opts(":sort title :query todo:NEXT"), o("todo:NEXT", 20, "title"));
        for bad in [":limit 5", ":query \"\"", ":query x :limit many", ":query x :sort size", ":query x :scope file", ":query \"x", ":query", ""] {
            assert!(tasks_opts(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn tasks_block_body() {
        let k = kw();
        let file = |p: &str, t: &str| Arc::new(org::parse(Path::new(p), t, &k));
        let files = vec![
            file("/n/projects/acme.org", "* TODO [#A] Ship it [1/2] :work:\nDEADLINE: <2026-10-30 Fri>\n:PROPERTIES:\n:EFFORT: 1:30\n:END:\n* TODO Odd [x] title\n"),
            file("/n/inbox.org", "* TODO Call Bo :work:\nSCHEDULED: <2026-10-25 Sun>\n* DONE Old :work:\n"),
        ];
        let items = |q: &str| {
            let (keep, done) = org::query(q, NaiveDate::from_ymd_opt(2026, 10, 24).unwrap(), &k, |_| None, |_, _| false).unwrap();
            org::todos(&files, &k, done, keep, NaiveDate::from_ymd_opt(2026, 10, 24).unwrap())
        };
        let dir = Path::new("/n/daily");
        assert_eq!(
            tasks_body(items("tag:work"), &files, dir, 20, "priority"),
            "- TODO [#A] [[file:../projects/acme.org::*Ship it][Ship it]] (DEADLINE 2026-10-30, effort 1:30)\n- TODO [[file:../inbox.org::*Call Bo][Call Bo]] (SCHEDULED 2026-10-25)\n"
        );
        assert_eq!(tasks_body(items("tag:work"), &files, dir, 1, "date"), "- TODO [[file:../inbox.org::*Call Bo][Call Bo]] (SCHEDULED 2026-10-25)\n");
        assert_eq!(tasks_body(items("odd"), &files, Path::new("/n/projects"), 20, "title"), "- TODO Odd [x] title\n"); // brackets: no link
        assert_eq!(tasks_body(items("tag:nope"), &files, dir, 20, "priority"), "(no matches)\n");
    }

    #[test]
    fn columnview_options() {
        let o = |q: &str, c: &[&str], s: Option<(&str, bool)>, l| Ok((q.to_string(), c.iter().map(|c| c.to_string()).collect(), s.map(|(c, d)| (c.to_string(), d)), l));
        assert_eq!(columnview_opts(":query tag:book"), o("tag:book", &["ITEM"], None, 50));
        assert_eq!(columnview_opts(":query \"tag:book\" :cols \"ITEM STATUS  RATING\" :sort -RATING :limit 5"), o("tag:book", &["ITEM", "STATUS", "RATING"], Some(("RATING", true)), 5));
        assert_eq!(columnview_opts(":sort ITEM :query x"), o("x", &["ITEM"], Some(("ITEM", false)), 50));
        for bad in [":cols ITEM", ":query \"\"", ":query x :cols \"\"", ":query x :limit 0", ":query x :limit 501", ":query x :sort -", ":query x :id local", ":query"] {
            assert!(columnview_opts(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn columnview_table() {
        let k = kw();
        let file = |p: &str, t: &str| Arc::new(org::parse(Path::new(p), t, &k));
        let files = vec![
            file("/n/books.org", "* Books\n** Dune :book:\n:PROPERTIES:\n:STATUS: read\n:RATING: 5\n:ID: b-1\n:END:\n** Emma :book:\n:PROPERTIES:\n:status: reading\n:RATING: 10\n:END:\n\
                                  ** TODO A|B :book:\nDEADLINE: <2026-10-30 Fri>\n:PROPERTIES:\n:EFFORT: 1:30\n:END:\n** Old :book:ARCHIVE:\n"),
            file("/n/misc.org", "* Zed :book:\n* Other\n"),
        ];
        let body = |q: &str, cols: &[&str], sort: Option<(&str, bool)>, limit| {
            let (keep, _) = org::query(q, NaiveDate::from_ymd_opt(2026, 10, 24).unwrap(), &k, |_| None, |_, _| false).unwrap();
            let cols: Vec<String> = cols.iter().map(|c| c.to_string()).collect();
            let spent = |_: &str, t: &str| (t == "Dune").then_some(75);
            columnview_body(&files, keep, &cols, sort.map(|(c, d)| (c.to_string(), d)).as_ref(), limit, Path::new("/n/daily"), &spent)
        };
        // Keyword-less tagged headings with properties match (not just tasks); property names ignore case.
        assert_eq!(
            body("tag:book", &["ITEM", "status", "RATING", "TODO", "TAGS", "DEADLINE", "EFFORT", "CLOCKSUM"], None, 50),
            "| ITEM                               | status  | RATING | TODO | TAGS   | DEADLINE   | EFFORT | CLOCKSUM |\n\
             |------------------------------------+---------+--------+------+--------+------------+--------+----------|\n\
             | [[id:b-1][Dune]]                   | read    | 5      |      | :book: |            |        | 1:15     |\n\
             | [[file:../books.org::*Emma][Emma]] | reading | 10     |      | :book: |            |        |          |\n\
             | A\\vert{}B                          |         |        | TODO | :book: | 2026-10-30 | 1:30   |          |\n\
             | [[file:../misc.org::*Zed][Zed]]    |         |        |      | :book: |            |        |          |\n"
        );
        // Numeric sort (10 after 5), descending, empty cells last; then the limit.
        assert_eq!(body("tag:book", &["ITEM", "RATING"], Some(("RATING", true)), 3).lines().skip(2).map(|l| l.split('|').nth(2).unwrap().trim().to_string()).collect::<Vec<_>>(), ["10", "5", ""]);
        assert_eq!(body("tag:book", &["ITEM"], Some(("ITEM", false)), 50).lines().skip(2).map(|l| l.split('|').nth(1).unwrap().trim().to_string()).collect::<Vec<_>>(), ["A\\vert{}B", "[[id:b-1][Dune]]", "[[file:../books.org::*Emma][Emma]]", "[[file:../misc.org::*Zed][Zed]]"]);
        assert_eq!(body("tag:nope", &["ITEM"], None, 50), "| ITEM |\n|------|\n");
    }

    #[test]
    fn week_range_is_seven_inclusive_days() {
        let d = |m, day| NaiveDate::from_ymd_opt(2026, m, day).unwrap();
        assert_eq!(week_range(None, d(10, 20)), (d(10, 14), d(10, 20))); // 7 days, not 8
        assert_eq!(week_range(Some(d(10, 15)), d(10, 20)), (d(10, 12), d(10, 18))); // Thu -> Mon..Sun
        assert_eq!(week_range(Some(d(10, 18)), d(10, 20)), (d(10, 12), d(10, 18))); // Sunday
        assert_eq!(week_range(Some(d(10, 12)), d(10, 20)), (d(10, 12), d(10, 18))); // Monday
    }

    #[test]
    fn system_spell_langs_from_locale() {
        assert_eq!(system_spell_langs(&["sv_SE.UTF-8", "sv_SE", "sv", "C"]), ["sv_SE"]);
        assert_eq!(system_spell_langs(&["de_DE@euro", "en_US.UTF-8"]), ["de_DE", "en_US"]);
        assert_eq!(system_spell_langs(&["C"]), ["en_US"]);
    }

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

    #[test]
    fn inbox_entries_skip_done_nested_and_archived() {
        let t = "* TODO a\n** TODO nested\n* DONE b\n* plain note\n* TODO old :ARCHIVE:\n* NEXT c\n";
        assert_eq!(open_entries(&org::parse(Path::new("inbox.org"), t, &kw()), &kw()), 3);
        assert_eq!(open_entries(&org::parse(Path::new("inbox.org"), "", &kw()), &kw()), 0);
    }

    fn edit(text: &str, op: &str, v: Option<&str>) -> R<String> {
        let now = "2026-01-08T09:00:00".parse::<NaiveDateTime>().unwrap();
        org_edit_op(text, 0, op, v.map(String::from), &kw(), now, false)
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
        assert_eq!(edit("* TODO [#B] Task :a:\n", "title", Some("New")).unwrap(), "* TODO [#B] New :a:\n");
        assert!(edit("* Task\n", "title", Some(" ")).is_err());
        assert_eq!(edit("* Task\n", "note", Some("hi")).unwrap(), "* Task\n- Note taken on [2026-01-08 Thu 09:00] \\\\\n  hi\n");
        assert!(edit("* Task\n", "note", Some(" ")).is_err());
        assert_eq!(edit("* Task\n", "blocker", Some("x1 Two words")).unwrap(), "* Task\n:PROPERTIES:\n:BLOCKER: [[id:x1][Two words]]\n:END:\n");
        assert_eq!(edit("* Task\n:PROPERTIES:\n:BLOCKER: [[id:x1][Two words]] x2\n:END:\n", "unblocker", Some("x1")).unwrap(), "* Task\n:PROPERTIES:\n:BLOCKER: x2\n:END:\n");
        assert_eq!(edit("* Task\n:PROPERTIES:\n:BLOCKER: x2\n:END:\n", "unblocker", Some("x2")).unwrap(), "* Task\n");
        assert_eq!(edit("* Task\n", "ordered", None).unwrap(), "* Task\n:PROPERTIES:\n:ORDERED: t\n:END:\n");
        let sched = "* Task\nSCHEDULED: <2026-01-08 Thu>\n";
        assert_eq!(edit(sched, "shift", Some("SCHEDULED 3")).unwrap(), "* Task\nSCHEDULED: <2026-01-11 Sun>\n");
        assert_eq!(edit(sched, "shift", Some("SCHEDULED -1")).unwrap(), "* Task\nSCHEDULED: <2026-01-07 Wed>\n");
        assert_eq!(edit(sched, "shift", Some("scheduled 3")).unwrap(), sched); // the kind is case-sensitive
        assert_eq!(edit(sched, "shift", Some("DEADLINE 3")).unwrap(), sched); // no such date: untouched
        // move sets an absolute date (org-read-date input), keeping time and repeater (#599).
        let old = "* Task\nSCHEDULED: <2026-01-02 Fri 09:00 +1w>\n";
        assert_eq!(edit(old, "move", Some("SCHEDULED tomorrow")).unwrap(), "* Task\nSCHEDULED: <2026-01-09 Fri 09:00 +1w>\n");
        assert_eq!(edit(old, "move", Some("SCHEDULED today")).unwrap(), "* Task\nSCHEDULED: <2026-01-08 Thu 09:00 +1w>\n");
        let e = "* Task\n:PROPERTIES:\n:EFFORT: 2:00\n:END:\n";
        assert_eq!(edit("* Task\n", "effort", Some("2h")).unwrap(), e);
        assert_eq!(edit(e, "effort", Some("")).unwrap(), "* Task\n");
        assert_eq!(edit("* T\n", "effort", Some("soon")).unwrap_err(), "Not a duration: soon (try 0:30, 2h or 45min)");
        let d = edit(e, "prop", Some("DRILL_DUE 2026-01-09\nDRILL_EASE  2.5 \n")).unwrap();
        assert_eq!(d, "* Task\n:PROPERTIES:\n:EFFORT: 2:00\n:DRILL_DUE: 2026-01-09\n:DRILL_EASE: 2.5\n:END:\n");
        assert_eq!(edit(&d, "prop", Some("DRILL_DUE\nDRILL_EASE")).unwrap(), e);
        assert!(edit(e, "prop", Some("A:B 1")).is_err());
    }

    #[test]
    fn org_edit_errors() {
        assert_eq!(edit("* T\n", "cycle", Some("x")).unwrap_err(), "bad number: x");
        assert_eq!(edit("* T\n", "priority-cycle", None).unwrap_err(), "bad number: ");
        assert_eq!(edit("* T\n", "shift", Some("scheduled")).unwrap_err(), "shift needs \"KIND DAYS\"");
        assert_eq!(edit("* T\n", "shift", None).unwrap_err(), "shift needs \"KIND DAYS\"");
        assert_eq!(edit("* T\n", "shift", Some("SCHEDULED two")).unwrap_err(), "bad day count");
        assert_eq!(edit("* T\n", "move", Some("SCHEDULED")).unwrap_err(), "move needs \"KIND DATE\"");
        assert!(edit("* T\n", "move", Some("SCHEDULED blah")).is_err());
        assert_eq!(edit("* T\n", "file-prop", Some("mood")).unwrap_err(), "file-prop needs \"KEY=VALUE\"");
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
        // Noise under the notes is dropped; open non-.org files, folders and data files stay.
        let noisy = p(&["/n/images/a.png", "/n/#a.org#", "/n/.#a.org", "/n/images/.trash/x.png", "/n/sub", "/n/notes.txt", "/n/v1.2", "/n/n.org", "/d/.hidden", "/n/.git/y"]);
        assert_eq!(split_changes(noisy, &cfg, &cfg, &roots), (false, p(&["/d/.hidden", "/n/images/a.png", "/n/n.org", "/n/notes.txt", "/n/sub", "/n/v1.2"])));
        // A notes folder under a dot folder still works.
        assert_eq!(split_changes(p(&["/h/.cfg/n/a.org"]), &cfg, &cfg, &[PathBuf::from("/h/.cfg/n")]), (false, p(&["/h/.cfg/n/a.org"])));
    }

    #[test]
    fn only_notes_changes_skip_the_refresh() {
        let root = std::env::temp_dir().join(format!("margin-refresh-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("Projects.old")).unwrap();
        for f in ["img.png", "a.org"] {
            std::fs::write(root.join(f), "").unwrap();
        }
        let r = |s: &str| needs_refresh(&root.join(s), &root);
        assert!(r("a.org") && r("Projects.old") && r("2026.10") && r("a.png.gone"));
        assert!(!r("img.png"));
        assert!(needs_refresh(Path::new("/d/timelog.jsonl"), &root));
        std::fs::remove_dir_all(&root).unwrap();
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

    #[test]
    fn restore_puts_a_trashed_note_back() {
        let root = std::env::temp_dir().join(format!("margin-restore-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let put = |p: &str| {
            let p = root.join(p);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, p.to_str().unwrap()).unwrap();
            p
        };
        let t = |p: &str| root.join(".trash").join(p);
        assert_eq!(restore_target(&root, &t("sub/a.org")).unwrap(), root.join("sub/a.org"));
        assert_eq!(restore_target(&root, &t("sub/a.org.12")).unwrap(), root.join("sub/a.org"));
        assert_eq!(restore_target(&root, &t("v1.2.org")).unwrap(), root.join("v1.2.org"));
        // A tag rename's copy goes back in place (and is refused while the note exists), not to a stray folder.
        assert_eq!(restore_target(&root, &t("tag-rename-20260101-120000/sub/a.org")).unwrap(), root.join("sub/a.org"));
        assert_eq!(restore_target(&root, &t("tag-rename-undo-20260101-120000-2/a.org")).unwrap(), root.join("a.org"));
        assert_eq!(restore_target(&root, &t("tag-rename-x.org")).unwrap(), root.join("tag-rename-x.org"));
        for bad in [root.join("sub/a.org"), t("../a.org"), t("a.txt"), t("a.org.x"), root.join(".trash")] {
            assert!(restore_target(&root, &bad).is_err(), "{}", bad.display());
        }
        // Trash twice, list, restore the numbered copy; restoring over an existing note is refused.
        let a = put("sub/a.org");
        let one = trash_to(&root, &a).unwrap();
        let a = put("sub/a.org");
        let two = trash_to(&root, &a).unwrap();
        put(".trash/readme.txt");
        let listed: Vec<PathBuf> = trashed_in(&root).into_iter().map(|(p, _)| p).collect();
        assert_eq!(listed.len(), 2);
        assert!(listed.contains(&one) && listed.contains(&two));
        assert_eq!(restore_to(&root, &two).unwrap(), root.join("sub/a.org"));
        assert!(!two.exists());
        assert!(restore_to(&root, &one).unwrap_err().contains("already exists"));
        assert!(one.exists());
        std::fs::remove_file(root.join("sub/a.org")).unwrap();
        assert_eq!(restore_to(&root, &one).unwrap(), root.join("sub/a.org"));
        assert!(restore_to(&root, &one).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn retag_files_skips_notes_without_the_tag() {
        let root = std::env::temp_dir().join(format!("margin-retag-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("sub")).unwrap();
        let kw = org::Kw { todo: vec![], done: vec![] };
        // CACHED is what the index holds; DISK what's on disk (the plain note gained the tag after indexing).
        let note = |rel: &str, cached: &str, disk: &str| {
            let p = root.join(rel);
            std::fs::write(&p, disk).unwrap();
            Arc::new(org::parse(&p, cached, &kw))
        };
        let files = [note("sub/a.org", "* A :Work:\n", "* A :Work:\n"), note("b.org", "* B\n", "* B :work:\n")];
        let mut r = TagRename::default();
        retag_files(&files, &root, "work", "job", &[], None, &mut r);
        assert_eq!((r.places, r.notes, r.copies), (1, 1, None));
        assert_eq!(std::fs::read_to_string(root.join("sub/a.org")).unwrap(), "* A :Work:\n");
        let copies = root.join(".trash/tag-rename-x");
        let mut r = TagRename::default();
        retag_files(&files, &root, "work", "job", &[], Some(&copies), &mut r);
        assert_eq!((r.places, r.notes, r.copies.as_deref()), (1, 1, Some(copies.as_path())));
        assert_eq!(std::fs::read_to_string(root.join("sub/a.org")).unwrap(), "* A :job:\n");
        assert_eq!(std::fs::read_to_string(copies.join("sub/a.org")).unwrap(), "* A :Work:\n");
        assert_eq!(std::fs::read_to_string(root.join("b.org")).unwrap(), "* B :work:\n");
        assert!(!copies.join("b.org").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn tag_rename_undo_restores_unchanged_notes_only() {
        let root = std::env::temp_dir().join(format!("margin-retag-undo-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        // Two renames in one second get their own folders.
        let base = root.join(".trash/tag-rename-20260101-120000");
        let (d1, d2) = (unique_dir(&base).unwrap(), unique_dir(&base).unwrap());
        assert_eq!((d1.as_path(), d2.clone()), (base.as_path(), root.join(".trash/tag-rename-20260101-120000-2")));
        std::fs::remove_dir(&d1).unwrap();
        let kw = org::Kw { todo: vec![], done: vec![] };
        let note = |rel: &str, text: &str| {
            let p = root.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, text).unwrap();
            Arc::new(org::parse(&p, text, &kw))
        };
        let files = [note("sub/a.org", "* A :work:\n"), note("b.org", "* B :work:\n"), note("c.org", "* C :work:\n"), note("d.org", "* D\n")];
        let mut r = TagRename::default();
        retag_files(&files, &root, "work", "job", &[], Some(&d1), &mut r);
        write_manifest(&d1, "work", "job", &mut r);
        assert!(r.errors.is_empty());
        let undos = rename_undos(&root);
        assert_eq!(undos.len(), 1);
        assert_eq!((undos[0].folder.as_path(), undos[0].old.as_str(), undos[0].new.as_str(), undos[0].notes), (d1.as_path(), "work", "job", 3));
        // An apply that wrote nothing leaves no folder behind.
        write_manifest(&d2, "x", "y", &mut TagRename::default());
        assert!(!d2.exists());
        // b.org was edited since, c.org has unsaved edits: both are left alone.
        std::fs::write(root.join("b.org"), "* B :job:\nmore\n").unwrap();
        assert!(undo_retag(&root, &root.join(".trash/elsewhere"), &[]).is_err());
        let u = undo_retag(&root, &d1, &[root.join("c.org")]).unwrap();
        assert_eq!((u.restored, u.errors.len()), (vec![root.join("sub/a.org")], 0));
        assert_eq!(u.conflicts, vec![root.join("b.org"), root.join("c.org")]);
        let read = |p: &Path| std::fs::read_to_string(p).unwrap();
        assert_eq!(read(&root.join("sub/a.org")), "* A :work:\n");
        assert_eq!(read(&u.undo.unwrap().join("sub/a.org")), "* A :job:\n"); // the undo is itself undoable
        assert_eq!(read(&root.join("b.org")), "* B :job:\nmore\n");
        assert_eq!(read(&root.join("c.org")), "* C :job:\n");
        // Done: no longer listed, and a second undo finds nothing.
        assert!(d1.join("manifest.undone.json").exists() && rename_undos(&root).is_empty());
        assert!(undo_retag(&root, &d1, &[]).is_err());
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
            cal: timeclock::Calendar::new(8.0, &[]),
            status,
            profile: "Work".into(),
            profiles: vec!["Work".into(), "Home".into()],
            leave_time: "7h 00m left today (not clocked in).".into(),
            backup_error: Some("disk full".into()),
        }
    }

    #[test]
    fn dashboard_projects_month() {
        let mut fx = dash_fixture("2026-01-08", (None, None));
        let s = fx.sessions[1].clone(); // Acme, 2026-01-05, 3.2
        fx.sessions.push(timeclock::Session { date: "2026-01-02".parse().unwrap(), hours: 2.0, ..s.clone() }); // this month, last week
        fx.sessions.push(timeclock::Session { date: "2026-01-02".parse().unwrap(), project: "Gamma".into(), hours: 1.5, ..s.clone() }); // only last week
        fx.sessions.push(timeclock::Session { date: "2025-12-30".parse().unwrap(), project: "Delta".into(), hours: 1.0, ..s }); // only last month
        fx.projects.insert("Acme".into(), Project { rate: Some(100.0), currency: "EUR".into(), rounding: None, ..Project::new("A-1") });
        let v = dashboard(fx);
        let acme = &v["projects"][0];
        assert_eq!(acme["project"], json!("Acme"));
        assert_eq!((acme["rate"].as_f64(), &acme["currency"], acme["earned"].as_f64()), (Some(100.0), &json!("EUR"), Some(720.0))); // this month's billable
        assert!((acme["worked"].as_f64().unwrap() - 5.2).abs() < 1e-9 && (acme["month"].as_f64().unwrap() - 7.2).abs() < 1e-9, "{acme}"); // not December's 9.0
        let gamma = v["projects"].as_array().unwrap().iter().find(|p| p["project"] == "Gamma").expect("Gamma listed for its month");
        assert_eq!((gamma["worked"].as_f64(), gamma["month"].as_f64()), (Some(0.0), Some(1.5)));
        assert!(gamma["earned"].is_null() && gamma["rate"].is_null(), "{gamma}");
        assert!(!v["projects"].as_array().unwrap().iter().any(|p| p["project"] == "Delta"), "last month's project listed");
    }

    #[test]
    fn expected_after_leave_subtracts_leave() {
        let fx = dash_fixture("2026-01-08", (None, None));
        let leave: Vec<_> = fx.leave.iter().map(|x| x.1.clone()).collect();
        let d = |s: &str| s.parse::<NaiveDate>().unwrap();
        // Thu 8h, Fri vacation 0h, Sat/Sun off, Tue 6th red day with Vab leave still 0.
        let ds = [d("2026-01-08"), d("2026-01-09"), d("2026-01-10"), d("2026-01-11"), d("2026-01-06")];
        assert_eq!(expected_after_leave(&ds, &fx.cal, &leave, |_| 0.0), [8.0, 0.0, 0.0, 0.0, 0.0]);
        assert_eq!(expected_after_leave(&ds[..1], &fx.cal, &[], |_| 0.0), [8.0]); // without the leave list nothing changes
    }

    #[test]
    fn dashboard_builds_the_week() {
        let v = dashboard(dash_fixture("2026-01-08", (None, None)));
        // The frontend contract (Time.svelte).
        assert_eq!(keys(&v), ["backup_error", "flex_total", "flex_week", "leave", "leave_time", "on_break", "profile", "profiles", "project", "projects", "since", "task", "today", "today_hours", "week"]);
        assert_eq!(keys(&v["week"][0]), ["date", "expected", "hours", "leave", "leave_hours"]);
        assert_eq!(keys(&v["projects"][0]), ["billable", "budget_hours", "budget_period", "code", "currency", "earned", "month", "project", "rate", "worked"]);
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
        let (total, week, _) = timeclock::flex(&fx.sessions, &fx.projects, &leave, &fx.cal, Some((fx.today - Duration::days(3), fx.today)));
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
        assert_eq!(opt_date_at("+1d 14:00", t), Ok(Some((iso("2026-10-13"), Some("14:00".into()), None, None))));
        assert_eq!(opt_date_at("soon", t), Err("can't read date \"soon\"".into()));
        assert_eq!(date_stamp_at("fri +1w", t).unwrap(), "<2026-10-16 Fri +1w>");
        assert_eq!(date_stamp_at("fri -3d", t).unwrap(), "<2026-10-16 Fri -3d>");
        assert_eq!(date_stamp_at("fri 14:00-15:30", t).unwrap(), "<2026-10-16 Fri 14:00-15:30>");
        assert_eq!(date_stamp_at("", t).unwrap(), "<2026-10-12 Mon>");
        assert!(date_stamp_at("soon", t).is_err());
        assert_eq!(date_preview_at(" ", t, false), "");
        assert_eq!(date_preview_at("fri", t, false), "Friday 16 October 2026");
        assert_eq!(date_preview_at("fri 14:00", t, false), "Friday 16 October 2026, 14:00");
        assert_eq!(date_preview_at("soon", t, false), "✗ can't read this date");
        assert_eq!(date_preview_at("fri 2pm", t, false), "✗ can't read this date");
        assert_eq!(date_preview_at("fri +1w", t, false), "Friday 16 October 2026, every week");
        assert_eq!(date_preview_at("fri 9:00 ++2d", t, false), "Friday 16 October 2026, 09:00, every 2 days, catching up");
        assert_eq!(date_preview_at("today .+1m", t, false), "Monday 12 October 2026, 1 month after done");
        assert_eq!(date_preview_at("fri -3d", t, false), "Friday 16 October 2026, warn 3 days before");
        assert_eq!(date_preview_at("fri -3d", t, true), "Friday 16 October 2026, show 3 days later");
        assert_eq!(date_preview_at("fri 9:00 +1w -1w", t, false), "Friday 16 October 2026, 09:00, every week, warn 1 week before");
    }

    #[test]
    fn task_text_and_planning() {
        let (t, todo) = (mon(), vec!["NEXT".to_string(), "TODO".into()]);
        assert_eq!(task_text(&todo, "  ", None, vec![], "", "", t), Err("the task needs a title".into()));
        assert_eq!(task_text(&todo, "Call", Some(""), vec![], "", "", t), Ok("* NEXT Call\n".into())); // first keyword, empty priority dropped
        assert_eq!(task_text(&[], "Call", Some("A"), vec!["home".into()], "+1d", "fri 09:00", t), Ok("* [#A] Call :home:\nSCHEDULED: <2026-10-13 Tue> DEADLINE: <2026-10-16 Fri 09:00>\n".into()));
        assert_eq!(task_text(&[], "Water", None, vec![], "fri +1w", "", t), Ok("* Water\nSCHEDULED: <2026-10-16 Fri +1w>\n".into()));
        assert_eq!(task_text(&[], "Pay", None, vec![], "", "fri +1m -3d", t), Ok("* Pay\nDEADLINE: <2026-10-16 Fri +1m -3d>\n".into()));
        assert_eq!(task_text(&todo, "Call", None, vec![], "", "soon", t), Err("can't read date \"soon\"".into()));
        let set = "* TODO A\nDEADLINE: <2026-10-13 Tue>\n";
        assert_eq!(planning_at("* TODO A\n", 0, "DEADLINE", "+1d", t), Ok(set.into()));
        assert_eq!(planning_at(set, 0, "DEADLINE", " rm ", t), Ok("* TODO A\n".into()));
        assert_eq!(planning_at(set, 0, "DEADLINE", "soon", t), Err("can't parse date".into()));
        // `norepeat` drops the repeater (#474); alone it keeps date, time and warning cookie.
        let rep = "* TODO A\nSCHEDULED: <2026-10-05 Mon 09:00 .+1w/2w -3d>\n";
        assert_eq!(planning_at(rep, 0, "SCHEDULED", "norepeat", t), Ok("* TODO A\nSCHEDULED: <2026-10-05 Mon 09:00 -3d>\n".into()));
        assert_eq!(planning_at(rep, 0, "SCHEDULED", "fri norepeat", t), Ok("* TODO A\nSCHEDULED: <2026-10-16 Fri -3d>\n".into()));
        assert_eq!(planning_at(set, 0, "DEADLINE", "norepeat", t), Ok(set.into())); // nothing to drop
        assert_eq!(planning_at("* TODO A\n", 0, "DEADLINE", "norepeat", t), Ok("* TODO A\n".into()));
        assert!(org::read_date("norepeat", t).is_none()); // other date prompts still reject it
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
    fn journal_template_fills_new_notes_for_their_day() {
        let dir = std::env::temp_dir().join(format!("margin-jt-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("d.org");
        crate::notes::write_note(&p, "d").unwrap();
        let day = NaiveDate::from_ymd_opt(2026, 3, 4).unwrap();
        append_journal_template(&p, "* Plan %t %^{x}%?", day).unwrap();
        let t = std::fs::read_to_string(&p).unwrap();
        assert!(t.ends_with("* Plan <2026-03-04 Wed> \n"), "{t:?}");
        append_journal_template(&p, "  ", day).unwrap(); // empty template: untouched
        assert_eq!(std::fs::read_to_string(&p).unwrap(), t);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn daily_paths_are_direct_children_of_the_daily_dir() {
        let n = Path::new("/n/notes");
        assert!(is_daily_path(n, "daily", Path::new("/n/notes/daily/2026-03-04.org")));
        assert!(is_daily_path(n, "journal/daily/", Path::new("/n/notes/journal/daily/x.org")));
        assert!(!is_daily_path(n, "daily", Path::new("/n/notes/daily/sub/x.org")));
        assert!(!is_daily_path(n, "daily", Path::new("/n/notes/x.org")));
        assert!(!is_daily_path(n, "", Path::new("/n/notes/x.org")));
        assert!(!is_daily_path(n, "/", Path::new("/n/notes/x.org")));
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

    #[test]
    fn csv_target_resolves_relative_names_into_export_dir() {
        let d = Path::new("/home/u/Desktop");
        assert_eq!(csv_target("r.csv".into(), d), d.join("r.csv"));
        assert_eq!(csv_target("/x/r.csv".into(), d), PathBuf::from("/x/r.csv"));
        assert!(csv_dest(&csv_target("../r.csv".into(), d), &[]).is_err());
    }

    #[test]
    fn csv_dest_allows_any_absolute_csv_outside_data_and_config() {
        let t = std::env::temp_dir();
        let (data, cfg) = (t.join("d"), t.join("c"));
        let forbidden = [data.clone(), cfg.clone()];
        for ok in [t.join("Desktop/time_work.csv"), t.join("new/dir/x.CSV"), t.join("d2/x.csv")] {
            assert_eq!(csv_dest(&ok, &forbidden), Ok(()), "{}", ok.display());
        }
        for bad in [PathBuf::from("time.csv"), t.join("Desktop/../x.csv"), t.join("x.org"), t.join("x"), cfg.join("config.toml"), data.join("x.csv"), cfg.join("x.csv")] {
            assert!(csv_dest(&bad, &forbidden).is_err(), "{}", bad.display());
        }
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
            attach_in(&notes, "att", "", &note, "a.png", taken, w)
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

    #[test]
    fn image_dirs_and_trash_roots() {
        let root = std::env::temp_dir().join(format!("margin-imgdirs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for d in ["images", "work/images", "work/images/.trash", ".hidden/images", "att", "imagesx"] {
            std::fs::create_dir_all(root.join(d)).unwrap();
        }
        let mut dirs = image_dirs(&root, "images");
        dirs.sort();
        assert_eq!(dirs, [root.join("images"), root.join("work/images")]);
        assert!(image_dirs(&root, "").is_empty());
        let att = root.join("att");
        let tr = |f: &str| trash_root(&root, &att, "images", &root.join(f));
        assert_eq!(tr("att/n/a.png").unwrap(), att);
        assert_eq!(tr("work/images/a.png").unwrap(), root.join("work/images"));
        assert_eq!(tr("images/a.png").unwrap(), root.join("images"));
        assert!(tr("work/a.png").is_err());
        assert!(trash_root(&root, &att, "images", Path::new("/images/a.png")).is_err());
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
        let tc = Tc { dir: d1.join("work"), diary: n1.join("dagbok-work.org"), cal: timeclock::Calendar::new(8.0, &[]) };
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
