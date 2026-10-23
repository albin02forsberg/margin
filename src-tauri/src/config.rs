use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Config {
    pub notes_dir: String,
    pub data_dir: String,
    pub export_dir: String,
    pub expected_daily_hours: f64,
    /// Expected hours Monday to Sunday (7 values), e.g. a 4-day week; empty = expected_daily_hours Mon–Fri.
    #[serde(deserialize_with = "weekday_hours")]
    pub weekday_hours: Vec<f64>,
    /// Public holidays that count as 0 expected hours: "se" (Swedish red days) or "none".
    #[serde(deserialize_with = "public_holidays")]
    pub public_holidays: String,
    /// Extra days off (ISO dates, e.g. "2026-12-24"), 0 expected hours like a holiday.
    #[serde(deserialize_with = "extra_days_off")]
    pub extra_days_off: Vec<String>,
    pub profiles: Vec<String>,
    pub todo_keywords: Vec<String>,
    pub done_keywords: Vec<String>,
    /// Keywords that count as a next action for stuck projects (`Space x S`).
    pub stuck_next_keywords: Vec<String>,
    /// Daily journal folder, relative to notes_dir.
    pub daily_dir: String,
    /// Org text put in new journal files after the title; `%t`/`%T`/`%u`/`%U` expand for that day,
    /// prompts and `%?` don't. Empty = nothing.
    pub journal_template: String,
    /// Daily metrics offered by `Space n j` (stored upper-cased in the journal day's top drawer).
    pub metrics: Vec<String>,
    /// Where new tasks go by default, relative to notes_dir.
    pub inbox: String,
    /// Stamp `CLOSED: [date time]` on tasks when they are marked done (org-log-done 'time).
    pub log_done: bool,
    /// Closing the window while a timer runs hides it to the tray instead of quitting.
    pub close_to_tray: bool,
    /// System-wide shortcut for quick capture, e.g. "Super+Shift+N"; empty to disable.
    pub capture_shortcut: String,
    /// Ask what to do with the time after this long away from a running timer; 0 to disable.
    pub idle_threshold_minutes: i64,
    /// Nudge a break after this many minutes of one running session (since its clock-in or resume); 0 = off.
    pub focus_minutes: i64,
    /// The break length that nudge suggests.
    pub focus_break_minutes: i64,
    /// Desktop notifications for timed entries and upcoming deadlines.
    pub reminders: bool,
    pub remind_before_minutes: i64,
    /// Remind about date-only deadlines this many days ahead (0 = on the day).
    pub deadline_warning_days: i64,
    /// Saved task searches shown in the sidebar: `[[views]] name = "…" query = "…"`.
    pub views: Vec<View>,
    /// Where to keep a calendar (.ics) of scheduled tasks and deadlines; empty for none.
    pub calendar_file: String,
    /// Capture templates (`Space c`): `[[templates]] key name file heading body`.
    pub templates: Vec<Template>,
    /// More templates, one per .org file, relative to notes_dir; empty to disable.
    pub templates_dir: String,
    /// Editor snippets: `<key` + Tab inserts text (`[[snippets]] key text`, %t/%U dates, %? cursor).
    pub snippets: Vec<Snippet>,
    /// Dropped and pasted files go to <attachments_dir>/<note name>/, relative to notes_dir.
    pub attachments_dir: String,
    /// Dropped and pasted images go to <images_dir>/ in the note's own folder, shared by the
    /// notes there (Emacs org style); empty = attachments_dir like other files.
    pub images_dir: String,
    /// ActivityWatch server to suggest sessions from, e.g. "http://localhost:5600"; empty = off.
    pub activitywatch_url: String,
    /// Regexes (case-insensitive) over app names and window titles that suggestions ignore.
    pub activity_exclude: Vec<String>,
    /// Regexes (case-insensitive) over app names, window titles and URLs of meetings.
    pub activity_meetings: Vec<String>,
    /// A local .ics file whose events name meeting suggestions; empty = off.
    pub activity_calendar: String,
    /// Record the active app and window title to <data>/activity/ (opt-in; see watcher.rs).
    pub activity_watcher: bool,
    /// Days of that log to keep; 0 = all.
    pub activity_retention_days: i64,
    /// Note when a mic or camera is in use (yes/no only) to <data>/activity/media/, so calls are
    /// marked as meetings (opt-in; see media.rs).
    pub activity_media_signal: bool,
    /// Where drafts come from: "ollama", or "embedded" (a model Margin downloads and runs).
    pub ai_backend: crate::ai::Backend,
    /// Model that drafts diary and journal notes: an Ollama model, e.g. "llama3.2:3b", or a
    /// built-in model id (ai::MODELS); empty = off.
    pub ai_model: String,
    /// The Ollama server; only local addresses are allowed.
    pub ai_url: String,
    /// What the chat's "Use my notes" sends: matching notes, open tasks and the agenda, the time log.
    pub ai_source_notes: bool,
    pub ai_source_tasks: bool,
    pub ai_source_time: bool,
    /// Notes the AI never sees (not retrieved, linked, sent as the open note, or read for tasks):
    /// case-insensitive substrings of their path below notes_dir.
    pub ai_exclude: Vec<String>,
    /// Reopen the last session's tabs on startup.
    pub restore_tabs: bool,
    /// Underline misspelled words in org notes (the webview's own spell checker).
    pub spellcheck: bool,
    /// Spell-check languages (Linux/WebKitGTK only), e.g. `sv_SE`; empty = the system locale.
    pub spellcheck_languages: Vec<String>,
    /// Update from the daily builds of main (the `nightly` prerelease) instead of stable releases.
    pub nightly_updates: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct View {
    pub name: String,
    pub query: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Snippet {
    pub key: String,
    pub text: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Template {
    pub key: String,
    pub name: String,
    /// strftime pattern relative to notes_dir; empty means the inbox.
    #[serde(default)]
    pub file: String,
    /// File under this heading (created if missing) instead of at the end.
    pub heading: Option<String>,
    pub body: String,
}

impl Template {
    /// A template file named STEM: leading `#+key:` `#+title:` `#+file:` `#+heading:` lines (other
    /// keywords, blank lines and an ID drawer skipped), then the body. Key defaults to STEM's first letter.
    pub fn from_file(stem: &str, text: &str) -> Template {
        let key = stem.chars().next().map(String::from).unwrap_or_default();
        let mut t = Template { key, name: stem.into(), file: String::new(), heading: None, body: String::new() };
        let (mut off, mut drawer) = (0, false);
        for l in text.split_inclusive('\n') {
            let s = l.trim();
            if off == 0 && s.eq_ignore_ascii_case(":PROPERTIES:") {
                drawer = true;
            } else if drawer {
                drawer = !s.eq_ignore_ascii_case(":END:");
            } else if let Some((k, v)) = s.strip_prefix("#+").and_then(|kv| kv.split_once(':')).filter(|(k, _)| !k.contains(char::is_whitespace)) {
                let v = v.trim().to_string();
                match k.to_ascii_lowercase().as_str() {
                    "key" if !v.is_empty() => t.key = v,
                    "title" => t.name = v,
                    "file" => t.file = v,
                    "heading" => t.heading = Some(v).filter(|h| !h.is_empty()),
                    _ => {}
                }
            } else if !s.is_empty() {
                break;
            }
            off += l.len();
        }
        t.body = text[off..].into();
        t
    }
}

/// `weekday_hours`: empty, or 7 hours from 0 to 24.
fn weekday_hours<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<f64>, D::Error> {
    let v = Vec::<f64>::deserialize(d)?;
    if !v.is_empty() && (v.len() != 7 || v.iter().any(|h| !(0.0..=24.0).contains(h))) {
        return Err(serde::de::Error::custom("weekday_hours needs 7 numbers from 0 to 24, Monday first, or none"));
    }
    Ok(v)
}

/// `public_holidays`: "se" or "none".
fn public_holidays<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    let v = String::deserialize(d)?.trim().to_lowercase();
    if v != "se" && v != "none" {
        return Err(serde::de::Error::custom("public_holidays must be \"se\" or \"none\""));
    }
    Ok(v)
}

/// `extra_days_off`: ISO dates (YYYY-MM-DD).
fn extra_days_off<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    let v = Vec::<String>::deserialize(d)?;
    match v.iter().find(|s| s.parse::<chrono::NaiveDate>().is_err()) {
        Some(bad) => Err(serde::de::Error::custom(format!("extra_days_off: {bad:?} is not a date like 2026-12-24"))),
        None => Ok(v),
    }
}

impl Default for Config {
    fn default() -> Self {
        let v = |a: &[&str]| a.iter().map(|s| s.to_string()).collect();
        Config {
            notes_dir: "~/notes".into(),
            data_dir: "~/timeclock".into(),
            export_dir: "~/Desktop".into(),
            expected_daily_hours: 8.0,
            weekday_hours: vec![],
            public_holidays: "se".into(),
            extra_days_off: vec![],
            profiles: v(&["Work", "Personal"]),
            todo_keywords: v(&["TODO", "NEXT", "WAIT"]),
            done_keywords: v(&["DONE", "CANCELLED"]),
            stuck_next_keywords: v(&["NEXT", "DOING"]),
            daily_dir: "daily".into(),
            journal_template: String::new(),
            metrics: v(&["mood", "energy", "sleep"]),
            inbox: "inbox.org".into(),
            log_done: true,
            close_to_tray: true,
            capture_shortcut: "Super+Shift+N".into(),
            idle_threshold_minutes: 10,
            focus_minutes: 0,
            focus_break_minutes: 5,
            reminders: true,
            remind_before_minutes: 10,
            deadline_warning_days: 1,
            views: vec![],
            calendar_file: String::new(),
            templates: vec![
                Template { key: "n".into(), name: "Note".into(), file: String::new(), heading: None, body: "* %^{Title}\n%U\n%i%?".into() },
                Template { key: "m".into(), name: "Meeting notes".into(), file: "meetings.org".into(), heading: None, body: "* %^{Meeting} :meeting:\n%U\n- %?".into() },
            ],
            templates_dir: "templates".into(),
            snippets: vec![],
            attachments_dir: "attachments".into(),
            images_dir: "images".into(),
            activitywatch_url: String::new(),
            activity_exclude: v(&["KeePass", "1Password", "Bitwarden", "Private Browsing", "Incognito", "InPrivate"]),
            activity_calendar: String::new(),
            activity_watcher: false,
            activity_retention_days: 30,
            activity_media_signal: false,
            activity_meetings: v(&["^zoom", "Microsoft Teams", r"meet\.google\.com", "Slack.*huddle", "Webex"]),
            ai_backend: Default::default(),
            ai_model: String::new(),
            ai_url: "http://localhost:11434".into(),
            ai_source_notes: true,
            ai_source_tasks: true,
            ai_source_time: true,
            ai_exclude: vec![],
            restore_tabs: true,
            spellcheck: false,
            spellcheck_languages: vec![],
            nightly_updates: false,
        }
    }
}

/// Replace PATH's contents all at once: write a hidden temp file beside it, sync, rename.
/// A crash or full disk mid-write leaves the old file instead of a truncated one.
/// Symlinked files stay links (the target is replaced).
pub fn write_atomic(path: &Path, contents: impl AsRef<[u8]>) -> std::io::Result<()> {
    use std::io::Write;
    let path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let name = path.file_name().ok_or_else(|| std::io::Error::other("no file name"))?.to_string_lossy();
    let tmp = path.with_file_name(format!(".{name}.margin-tmp"));
    let mut f = std::fs::File::create(&tmp)?;
    if let Ok(m) = std::fs::metadata(&path) {
        let _ = f.set_permissions(m.permissions());
    }
    let done = f.write_all(contents.as_ref()).and_then(|_| f.sync_all()).and_then(|_| std::fs::rename(&tmp, &path));
    if done.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    done
}

pub fn expand(p: &str) -> PathBuf {
    match (p.strip_prefix("~"), std::env::home_dir()) {
        (Some(rest), Some(home)) => home.join(rest.trim_start_matches(['/', '\\'])),
        _ => PathBuf::from(p),
    }
}

impl Config {
    pub fn notes(&self) -> PathBuf {
        expand(&self.notes_dir)
    }
    pub fn data(&self) -> PathBuf {
        expand(&self.data_dir)
    }
    /// <notes>/<templates_dir>, if that's a folder strictly inside the notes ("", ".", ".." or "/"
    /// would otherwise turn every note into a template and hide it from the agenda).
    pub fn templates_path(&self) -> Option<PathBuf> {
        let (n, t) = (self.notes(), self.notes().join(&self.templates_dir));
        (t.starts_with(&n) && t != n && !t.components().any(|c| c == Component::ParentDir)).then_some(t)
    }

    /// Load PATH; a missing or unreadable file is an error (nothing is written).
    pub fn load(path: &Path) -> Result<Config, String> {
        let s = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        toml::from_str(&s).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// Startup: like `load`, but writes the defaults on first run (only when PATH doesn't exist:
    /// an unreadable file is an error, never overwritten).
    pub fn load_or_create(path: &Path) -> Result<Config, String> {
        if !matches!(path.try_exists(), Ok(false)) {
            return Self::load(path);
        }
        let c = Config::default();
        let _ = recreate(path, &c);
        Ok(c)
    }
}

/// Write C to PATH, but only while PATH doesn't exist (a file restored meanwhile is never overwritten).
pub fn recreate(path: &Path, c: &Config) -> Result<(), String> {
    if !matches!(path.try_exists(), Ok(false)) {
        return Err(format!("{} exists again; reload it instead", path.display()));
    }
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
    }
    write_atomic(path, toml::to_string(c).map_err(|e| e.to_string())?).map_err(|e| format!("{}: {e}", path.display()))
}

/// TEXT (a config.toml) with a `[[views]]` entry appended; errors if the result won't
/// load or anything but the new view would change.
pub fn add_view(text: &str, name: &str, query: &str) -> Result<String, String> {
    let err = |e: toml::de::Error| format!("can't add the view to config.toml: {e}");
    // The defaults written on first run say `views = []`, which a `[[views]]` table would redefine.
    let mut out: String = text.split_inclusive('\n').filter(|l| l.trim() != "views = []").collect();
    let q = |s: &str| toml::Value::String(s.into()).to_string();
    out += &format!("\n[[views]]\nname = {}\nquery = {}\n", q(name), q(query));
    toml::from_str::<Config>(&out).map_err(err)?;
    let (mut old, mut new): (toml::Table, toml::Table) = (text.parse().map_err(err)?, out.parse().map_err(err)?);
    if let Some(toml::Value::Array(v)) = new.get_mut("views") {
        v.pop();
    }
    for t in [&mut old, &mut new] {
        if t.get("views").and_then(|v| v.as_array()).is_some_and(|v| v.is_empty()) {
            t.remove("views");
        }
    }
    if old != new {
        return Err("can't add the view to config.toml without changing other settings; add it by hand".into());
    }
    Ok(out)
}

/// TEXT with the `name = …` line of the first `[[views]]` entry whose query is QUERY
/// rewritten to NAME; errors if anything but that name would change.
pub fn rename_view(text: &str, query: &str, name: &str) -> Result<String, String> {
    let err = |e: toml::de::Error| format!("can't rename the view in config.toml: {e}");
    let mut want: toml::Table = text.parse().map_err(err)?;
    let views = want.get_mut("views").and_then(|v| v.as_array_mut());
    let Some((i, v)) = views.and_then(|v| v.iter_mut().enumerate().find(|(_, v)| v.get("query").and_then(|q| q.as_str()) == Some(query))) else {
        return Err("no saved view has that query".into());
    };
    v.as_table_mut().ok_or("bad [[views]] entry")?.insert("name".into(), name.into());
    // Count `[[views]]` headers to reach entry I, then swap its first `name =` line.
    let (mut seen, mut done) = (0, false);
    let out: String = text
        .split_inclusive('\n')
        .map(|l| {
            let t = l.trim_start();
            if t.starts_with('[') {
                seen += usize::from(t.split('#').next().unwrap_or("").trim_end() == "[[views]]");
            } else if !done && seen == i + 1 && t.split_once('=').is_some_and(|(k, _)| k.trim() == "name") {
                done = true;
                let eol = &l[l.trim_end_matches(['\r', '\n']).len()..];
                return format!("{}name = {}{}{eol}", &l[..l.len() - t.len()], toml::Value::String(name.into()), trailing_comment(l));
            }
            l.to_string()
        })
        .collect();
    if !done || out.parse::<toml::Table>().map_err(err)? != want {
        return Err("can't rename the view in config.toml without changing other settings; edit it by hand".into());
    }
    toml::from_str::<Config>(&out).map_err(err)?;
    Ok(out)
}

/// TEXT (a config.toml) with the setup answers: notes and data folders and comma-separated
/// profiles (trimmed, case-insensitive duplicates dropped); an empty answer keeps the current value.
/// Only those top-level lines change, so comments survive; errors if anything else would.
pub fn setup(text: &str, notes_dir: &str, data_dir: &str, profiles: &str) -> Result<(String, Config), String> {
    let mut ps: Vec<String> = vec![];
    for p in profiles.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        if !ps.iter().any(|q| q.to_lowercase() == p.to_lowercase()) {
            ps.push(p.into());
        }
    }
    let mut out = text.to_string();
    let sets = [("notes_dir", notes_dir.trim().into()), ("data_dir", data_dir.trim().into()), ("profiles", toml::Value::from(ps.clone()))];
    for (key, value) in sets.into_iter().filter(|(_, v)| v.as_str().map_or(!ps.is_empty(), |s| !s.is_empty())) {
        out = set_key(&out, key, value)?;
    }
    let c = toml::from_str::<Config>(&out).map_err(|e| format!("config.toml: {e}"))?;
    Ok((out, c))
}

/// TEXT with the settings page's VALUES (top-level keys) set via set_key and the RESET keys
/// removed (back to their defaults); errors on a key that isn't a setting (or is
/// one of TABLES, which stay in the file) or a value Config won't load.
pub fn save(text: &str, values: toml::Table, reset: &[String]) -> Result<(String, Config), String> {
    let known = toml::Table::try_from(Config::default()).map_err(|e| e.to_string())?;
    if let Some(key) = values.keys().chain(reset).find(|k| !known.contains_key(*k) || TABLES.contains(&k.as_str())) {
        return Err(format!("{key} can't be set here; edit config.toml"));
    }
    let mut out = text.to_string();
    for (key, value) in values {
        out = set_key(&out, &key, value)?;
    }
    for key in reset {
        out = remove_key(&out, key)?;
    }
    let c = toml::from_str::<Config>(&out).map_err(|e| format!("config.toml: {e}"))?;
    if c.profiles.is_empty() {
        return Err("keep at least one profile".into());
    }
    Ok((out, c))
}

/// The lists of tables the settings page edits whole (set_tables), not via save.
pub const TABLES: [&str; 3] = ["views", "templates", "snippets"];

/// TEXT with the `[[KEY]]` tables (one of TABLES) replaced by ITEMS, appended at the end;
/// other lines stay as written, comments inside the old tables are dropped. An empty list is
/// written `KEY = []` (a missing `templates` means the built-in ones). Errors if anything else would change.
pub fn set_tables(text: &str, key: &str, items: Vec<toml::Table>) -> Result<(String, Config), String> {
    if !TABLES.contains(&key) {
        return Err(format!("{key} isn't a list of tables"));
    }
    let err = |e: toml::de::Error| format!("can't update config.toml: {e}");
    let hand = || format!("can't save {key} without changing other settings; edit config.toml by hand (Space f C)");
    let mut want: toml::Table = text.parse().map_err(err)?;
    want.insert(key.into(), toml::Value::Array(items.iter().cloned().map(toml::Value::Table).collect()));
    let header = format!("[[{key}]]");
    let mut inside = false;
    let out: String = text
        .split_inclusive('\n')
        .filter(|l| {
            let t = l.trim_start();
            if t.starts_with('[') {
                inside = t.split('#').next().unwrap_or("").trim_end() == header;
            }
            !inside
        })
        .collect();
    // A top-level `key = […]` (the first-run `views = []`, or the inline form) goes too.
    let out = remove_key(&out, key).map_err(|_| hand())?;
    let out = if items.is_empty() {
        set_key(&out, key, toml::Value::Array(vec![]))?
    } else {
        let blocks = toml::to_string(&toml::Table::from_iter([(key.to_string(), want[key].clone())])).map_err(|e| e.to_string())?;
        let head = out.trim_end();
        format!("{head}{}{blocks}", if head.is_empty() { "" } else { "\n\n" })
    };
    if out.parse::<toml::Table>().map_err(|_| hand())? != want {
        return Err(hand());
    }
    let c = toml::from_str::<Config>(&out).map_err(|e| format!("config.toml: {e}"))?;
    Ok((out, c))
}

/// LINE's trailing ` # comment` (with the spaces before it, without the line ending), or "":
/// it starts at the first `#` whose prefix parses on its own, so a `#` inside a string isn't one.
fn trailing_comment(line: &str) -> &str {
    let l = line.trim_end_matches(['\r', '\n']);
    l.match_indices('#').map(|(i, _)| &l[..i]).find(|s| s.parse::<toml::Table>().is_ok_and(|t| t.len() == 1)).map_or("", |s| &l[s.trim_end().len()..])
}

/// TEXT with top-level KEY's one-line `key = …` set to VALUE (added at the top if missing),
/// keeping its trailing comment; errors if anything else would change, e.g. a multi-line value.
pub fn set_key(text: &str, key: &str, value: toml::Value) -> Result<String, String> {
    let err = |e: toml::de::Error| format!("can't update config.toml: {e}");
    let mut want: toml::Table = text.parse().map_err(err)?;
    want.insert(key.into(), value.clone());
    let (mut table, mut done) = (false, false);
    let out: String = text
        .split_inclusive('\n')
        .map(|l| {
            let t = l.trim_start();
            table |= t.starts_with('[');
            if !table && !done && t.split_once('=').is_some_and(|(k, _)| k.trim() == key) {
                done = true;
                return format!("{key} = {value}{}{}", trailing_comment(l), &l[l.trim_end_matches(['\r', '\n']).len()..]);
            }
            l.to_string()
        })
        .collect();
    let out = if done { out } else { format!("{key} = {value}\n{out}") };
    if out.parse::<toml::Table>().map_err(err)? != want {
        return Err("can't update config.toml without changing other settings; edit it by hand (Space f C)".into());
    }
    Ok(out)
}

/// TEXT without top-level KEY's one-line `key = …` (unchanged if it isn't there);
/// errors if anything else would change, e.g. a multi-line value.
pub fn remove_key(text: &str, key: &str) -> Result<String, String> {
    let err = |e: toml::de::Error| format!("can't update config.toml: {e}");
    let mut want: toml::Table = text.parse().map_err(err)?;
    if want.remove(key).is_none() {
        return Ok(text.into());
    }
    let mut table = false;
    let out: String = text
        .split_inclusive('\n')
        .filter(|l| {
            let t = l.trim_start();
            table |= t.starts_with('[');
            table || t.split_once('=').is_none_or(|(k, _)| k.trim() != key)
        })
        .collect();
    if out.parse::<toml::Table>().map_err(err)? != want {
        return Err("can't reset that setting without changing others; edit config.toml by hand (Space f C)".into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spellcheck_languages_parse() {
        assert!(Config::default().spellcheck_languages.is_empty());
        let c = toml::from_str::<Config>("spellcheck_languages = [\"sv_SE\", \"en_US\"]").unwrap();
        assert_eq!(c.spellcheck_languages, ["sv_SE", "en_US"]);
    }

    #[test]
    fn spellcheck_off_by_default() {
        assert!(!Config::default().spellcheck);
        assert!(toml::from_str::<Config>("spellcheck = true").unwrap().spellcheck);
    }

    #[test]
    fn setup_answers() {
        let first = toml::to_string(&Config::default()).unwrap();
        let (s, c) = setup(&first, " /n ", "", " Job, job ,, Home ").unwrap();
        assert_eq!((c.notes_dir.as_str(), c.data_dir.as_str(), c.profiles.join("|")), ("/n", "~/timeclock", "Job|Home".into()));
        assert_eq!(s.lines().count(), first.lines().count()); // replaced in place
        assert_eq!(setup(&s, "", "", " , ").unwrap().0, s); // empty keeps current
        let mine = "# mine\r\nprofiles = [\"A\"] # here\r\n\r\n[[views]]\r\nname = \"x\"\r\nquery = \"y\"\r\n";
        let (s, c) = setup(mine, "~/org", "", "").unwrap();
        assert_eq!(s, format!("notes_dir = \"~/org\"\n{mine}")); // missing key added, comments and CRLF kept
        assert_eq!(c.views.len(), 1);
        assert!(setup("profiles = [\n  \"A\",\n]\n", "", "", "B").is_err()); // multi-line value: refused
        assert!(setup("bad", "x", "", "").is_err() && setup("profiles = 1", "", "", "").is_err());
    }

    #[test]
    fn save_settings() {
        let t = |s: &str| s.parse::<toml::Table>().unwrap();
        let mine = "# mine\nexpected_daily_hours = 8.0 # here\n\n[[views]]\nname = \"A\"\nquery = \"a\"\n";
        let (s, c) = save(mine, t("expected_daily_hours = 7.5\nreminders = false\nprofiles = [\"A\", \"B\"]"), &[]).unwrap();
        assert!(s.ends_with(&mine.replace("8.0 # here", "7.5 # here")), "{s}"); // in place, comment and other lines kept
        assert!(s.contains("\nreminders = false\n") || s.starts_with("reminders = false\n"), "{s}");
        assert_eq!((c.expected_daily_hours, c.reminders, c.views.len()), (7.5, false, 1));
        assert_eq!(save(mine, t("idle_threshold_minutes = 5"), &[]).unwrap().1.idle_threshold_minutes, 5);
        assert!(save(mine, t("idle_threshold_minutes = 1.5"), &[]).is_err()); // wrong type
        assert!(save(mine, t("nope = 1"), &[]).is_err() && save(mine, t("views = []"), &[]).is_err());
        assert!(save(mine, t("profiles = []"), &[]).is_err()); // keep at least one profile
        assert_eq!(save(mine, t("weekday_hours = [8, 8, 8, 8.5, 0, 0, 0]"), &[]).unwrap().1.weekday_hours, [8.0, 8.0, 8.0, 8.5, 0.0, 0.0, 0.0]);
        for bad in ["[8, 8, 8, 8, 8]", "[8, 8, 8, 8, 8, 0, -1]", "[8, 8, 8, 8, 8, 0, 25]"] {
            assert!(save(mine, t(&format!("weekday_hours = {bad}")), &[]).unwrap_err().contains("7 numbers"), "{bad}");
        }
        assert!(toml::from_str::<Config>("weekday_hours = [1]").is_err()); // on load too
        assert_eq!(Config::default().public_holidays, "se");
        let c = save(mine, t("public_holidays = \"none\"\nextra_days_off = [\"2026-12-24\"]"), &[]).unwrap().1;
        assert_eq!((c.public_holidays.as_str(), c.extra_days_off), ("none", vec!["2026-12-24".to_string()]));
        assert!(save(mine, t("public_holidays = \"us\""), &[]).unwrap_err().contains("\"se\" or \"none\""));
        assert!(save(mine, t("extra_days_off = [\"24/12\"]"), &[]).unwrap_err().contains("not a date"));
        assert!(save("profiles = [\n  \"A\",\n]\n", t("profiles = [\"B\"]"), &[]).is_err()); // multi-line: by hand
    }

    #[test]
    fn set_key_keeps_comments() {
        let set = |text: &str, v: toml::Value| set_key(text, "a", v).unwrap();
        assert_eq!(set("a = 1 # note\n", 2.into()), "a = 2 # note\n");
        assert_eq!(set("a = \"x#y\"   # c # d\r\nb = 1\r\n", "z".into()), "a = \"z\"   # c # d\r\nb = 1\r\n");
        assert_eq!(set("a = ['#', \"]\"]# arr\n", toml::Value::from(vec!["q"])), "a = [\"q\"]# arr\n");
        assert_eq!(set("a = \"x#y\"\n", "z".into()), "a = \"z\"\n"); // no comment
        assert_eq!(set("a = 1 #\n", 2.into()), "a = 2 #\n");
    }

    #[test]
    fn reset_settings() {
        let t = |s: &str| s.parse::<toml::Table>().unwrap();
        let mine = "# mine\r\ninbox = \"x.org\" # here\r\nreminders = false\r\n\r\n[[views]]\r\nname = \"A\"\r\nquery = \"a\"\r\n";
        let (s, c) = save(mine, t("reminders = true"), &["inbox".into()]).unwrap();
        assert_eq!(s, "# mine\r\nreminders = true\r\n\r\n[[views]]\r\nname = \"A\"\r\nquery = \"a\"\r\n");
        assert_eq!((c.inbox.as_str(), c.reminders, c.views.len()), ("inbox.org", true, 1));
        assert_eq!(remove_key(mine, "daily_dir").unwrap(), mine); // not set: nothing to do
        assert!(remove_key("profiles = [\n  \"A\",\n]\n", "profiles").is_err()); // multi-line: by hand
        assert!(save(mine, t(""), &["views".into()]).is_err() && save(mine, t(""), &["nope".into()]).is_err());
    }

    #[test]
    fn add_views() {
        let first = toml::to_string(&Config::default()).unwrap();
        let s = add_view(&first, "Work \"now\"", r"tag:work due:<=+7d \x").unwrap();
        let c: Config = toml::from_str(&s).unwrap();
        assert_eq!((c.views[0].name.as_str(), c.views[0].query.as_str()), ("Work \"now\"", r"tag:work due:<=+7d \x"));
        let s = add_view(&s, "Next", "todo:NEXT").unwrap();
        assert_eq!(toml::from_str::<Config>(&s).unwrap().views.len(), 2);
        let mine = "# my notes\nnotes_dir = \"~/org\" # here\n\n[[views]]\nname = \"A\"\nquery = \"a\"\n";
        assert!(add_view(mine, "B", "b").unwrap().starts_with(mine)); // comments kept
        assert!(add_view("views = [{ name = \"A\", query = \"a\" }]\n", "B", "b").is_err());
        let crlf = "notes_dir = \"~/org\"\r\nviews = []\r\n";
        assert!(add_view(crlf, "B", "b").unwrap().starts_with("notes_dir = \"~/org\"\r\n")); // line endings kept
        // A `views = []` line that isn't the top-level key is the user's text, not ours to drop.
        let body = "[[templates]]\nkey = \"v\"\nname = \"V\"\nbody = \"\"\"\nviews = []\n\"\"\"\n";
        assert!(add_view(body, "B", "b").is_err_and(|e| e.contains("changing")));
    }

    #[test]
    fn rename_views() {
        let text = "# mine\r\nnotes_dir = \"~/org\"\r\n\r\n[[views]]\r\nname = \"A\"\r\nquery = \"a\"\r\n\r\n[[views]] # second\r\n  name = \"B\" # old\r\nquery = 'tag:\"x y\"'\r\n";
        let s = rename_view(text, "tag:\"x y\"", "New \"b\"").unwrap();
        assert_eq!(s, text.replace("  name = \"B\" # old", "  name = 'New \"b\"' # old")); // only that line; CRLF, comments kept
        let c: Config = toml::from_str(&rename_view(&s, "a", "Z").unwrap()).unwrap();
        assert_eq!((c.views[0].name.as_str(), c.views[1].name.as_str()), ("Z", "New \"b\""));
        assert!(rename_view(text, "nope", "X").is_err());
        assert!(rename_view("views = [{ name = \"A\", query = \"a\" }]\n", "a", "X").is_err()); // inline: by hand
        // A `[[views]]` lookalike inside a multi-line string is refused, not miswritten.
        let body = "[[templates]]\nkey = \"v\"\nname = \"V\"\nbody = \"\"\"\n[[views]]\nname = x\n\"\"\"\n\n[[views]]\nname = \"A\"\nquery = \"a\"\n";
        assert!(rename_view(body, "a", "X").is_err_and(|e| e.contains("changing")));
    }

    #[test]
    fn set_views_and_templates() {
        let t = |s: &str| s.parse::<toml::Table>().unwrap();
        let views = |s: &str| t(s)["views"].as_array().unwrap().iter().map(|v| v.as_table().unwrap().clone()).collect::<Vec<_>>();
        let two = views("views = [{ name = 'B \"x\"', query = 'tag:b' }, { name = 'A', query = 'todo:NEXT' }]");
        let mine = "# mine\nnotes_dir = \"~/org\" # here\n\n[[views]] # old\nname = \"A\"\nquery = \"a\"\n\n[[templates]]\nkey = \"t\"\nname = \"T\"\nbody = \"x\"\n\n[[views]]\nname = \"C\"\nquery = \"c\"\n";
        let (s, c) = set_tables(mine, "views", two.clone()).unwrap();
        assert!(s.starts_with("# mine\nnotes_dir = \"~/org\" # here\n\n[[templates]]\nkey = \"t\"\nname = \"T\"\nbody = \"x\"\n\n[[views]]\n"), "{s}");
        assert_eq!(c.views.iter().map(|v| v.name.as_str()).collect::<Vec<_>>(), ["B \"x\"", "A"]);
        assert_eq!(c.templates.len(), 1);
        let (s, c) = set_tables(&s, "views", vec![]).unwrap(); // empty: all gone
        assert!(c.views.is_empty() && s.starts_with("views = []\n# mine\n") && !s.contains("[[views]]"), "{s}");
        // First run: `views = []` and the default templates' tables are replaced.
        let first = toml::to_string(&Config::default()).unwrap();
        let (s, c) = set_tables(&first, "views", two.clone()).unwrap();
        assert!(!s.contains("views = []") && c.views.len() == 2 && c.templates.len() == 2, "{s}");
        let (s, c) = set_tables(&s, "templates", vec![]).unwrap();
        assert!(c.templates.is_empty() && c.views.len() == 2 && s.contains("templates = []\n"), "{s}");
        let tpl = views("views = [{ key = 'j', name = 'J', body = \"* %?\\n[[file:a.org]]\\n\", heading = 'Log' }]");
        let (s, c) = set_tables(&s, "templates", tpl.clone()).unwrap();
        assert!(!s.contains("templates = []") && c.templates[0].heading.as_deref() == Some("Log") && c.templates[0].body == "* %?\n[[file:a.org]]\n", "{s}");
        // Inline arrays: one line is replaced, multi-line is refused.
        assert_eq!(set_tables("views = [{ name = 'A', query = 'a' }]\n", "views", two.clone()).unwrap().1.views.len(), 2);
        assert!(set_tables("views = [\n  { name = 'A', query = 'a' },\n]\n", "views", two.clone()).is_err_and(|e| e.contains("by hand")));
        // A body line that looks like a table header is refused, not miswritten.
        let body = "[[templates]]\nkey = \"j\"\nname = \"J\"\nbody = \"\"\"\n[[file:a.org]]\nx = 1\n\"\"\"\n";
        assert!(set_tables(body, "templates", tpl).is_err_and(|e| e.contains("by hand")));
        assert!(set_tables(mine, "views", views("views = [{ name = 1, query = 'a' }]")).is_err()); // bad type
        assert!(set_tables(mine, "profiles", vec![]).is_err());
        // Snippets: old configs load with none; a multi-line text round-trips; `save` won't touch them.
        assert!(toml::from_str::<Config>(mine).unwrap().snippets.is_empty());
        let sn = views("views = [{ key = 'mtg', text = \"* Meeting %u\\n- %?\" }]");
        let (s, c) = set_tables(mine, "snippets", sn).unwrap();
        assert!(c.snippets[0].key == "mtg" && c.snippets[0].text == "* Meeting %u\n- %?" && c.views.len() == 2, "{s}");
        assert_eq!(toml::from_str::<Config>(&s).unwrap().snippets[0].text, c.snippets[0].text);
        assert!(save(&s, toml::Table::new(), &["snippets".into()]).is_err());
    }

    #[test]
    fn defaults_round_trip() {
        let c: Config = toml::from_str(&toml::to_string(&Config::default()).unwrap()).unwrap();
        assert_eq!(c.templates.len(), 2);
        assert!(toml::from_str::<Config>("templates = []").unwrap().templates.is_empty());
        // Configs from before ai_backend keep using Ollama; ai_set's edit round-trips.
        assert_eq!(toml::from_str::<Config>("ai_model = \"m\"").unwrap().ai_backend, crate::ai::Backend::Ollama);
        let s = set_key("ai_model = \"m\"\n", "ai_backend", toml::Value::try_from(crate::ai::Backend::Embedded).unwrap()).unwrap();
        assert_eq!(s, "ai_backend = \"embedded\"\nai_model = \"m\"\n");
        assert_eq!(toml::from_str::<Config>(&s).unwrap().ai_backend, crate::ai::Backend::Embedded);
        assert!(toml::from_str::<Config>("ai_backend = \"cloud\"").is_err());
    }

    #[test]
    fn atomic_writes() {
        let dir = std::env::temp_dir().join(format!("margin-atomic-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("a.org");
        write_atomic(&f, "one").unwrap();
        write_atomic(&f, "two").unwrap();
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "two");
        #[cfg(unix)]
        {
            let link = dir.join("link.org");
            std::os::unix::fs::symlink(&f, &link).unwrap();
            write_atomic(&link, "three").unwrap();
            assert!(std::fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
            assert_eq!(std::fs::read_to_string(&f).unwrap(), "three");
        }
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), if cfg!(unix) { 2 } else { 1 }, "no temp files left");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn template_files() {
        let t = Template::from_file("meeting", ":PROPERTIES:\n:ID: x\n:END:\n#+title: Meeting notes\n#+FILE: meetings/%Y.org\n#+heading: Log\n#+filetags: :t:\n\n* %^{Who}\n#+begin_quote\n%i\n");
        assert_eq!((t.key.as_str(), t.name.as_str(), t.file.as_str(), t.heading.as_deref()), ("m", "Meeting notes", "meetings/%Y.org", Some("Log")));
        assert_eq!(t.body, "* %^{Who}\n#+begin_quote\n%i\n");
        // A block line with a colon later on is body, not a header keyword.
        assert_eq!(Template::from_file("s", "#+begin_src sh :results output\nls\n").body, "#+begin_src sh :results output\nls\n");
        let t = Template::from_file("journal", "#+key: J\n- %?");
        assert_eq!((t.key.as_str(), t.name.as_str(), t.file.as_str(), t.heading, t.body.as_str()), ("J", "journal", "", None, "- %?"));
    }

    #[test]
    fn templates_path() {
        let c = |d: &str| Config { notes_dir: "/n".into(), templates_dir: d.into(), ..Config::default() }.templates_path();
        assert_eq!(c("templates"), Some(PathBuf::from("/n/templates")));
        assert_eq!(c("/n/t"), Some(PathBuf::from("/n/t")));
        for d in ["", ".", "./", "..", "t/..", "/", "/other"] {
            assert_eq!(c(d), None, "{d:?}");
        }
    }

    #[test]
    fn load_keeps_unreadable_config() {
        let dir = std::env::temp_dir().join(format!("margin-load-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let p = dir.join("config.toml");
        assert!(Config::load(&p).is_err() && !p.exists()); // strict load never creates
        assert!(Config::load_or_create(&p).is_ok() && toml::from_str::<Config>(&std::fs::read_to_string(&p).unwrap()).is_ok()); // first run
        std::fs::write(&p, b"notes_dir = \"\xff\"\n").unwrap();
        assert!(Config::load(&p).is_err() && Config::load_or_create(&p).is_err());
        assert_eq!(std::fs::read(&p).unwrap(), b"notes_dir = \"\xff\"\n");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn recreate_only_when_missing() {
        let dir = std::env::temp_dir().join(format!("margin-recreate-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let p = dir.join("sub/config.toml");
        let c = Config { notes_dir: "/n".into(), profiles: vec!["A".into()], expected_daily_hours: 6.5, ..Config::default() };
        recreate(&p, &c).unwrap();
        let back = Config::load(&p).unwrap();
        assert_eq!(toml::Table::try_from(back).unwrap(), toml::Table::try_from(&c).unwrap());
        std::fs::write(&p, "# mine\n").unwrap();
        assert!(recreate(&p, &Config::default()).is_err());
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "# mine\n");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
