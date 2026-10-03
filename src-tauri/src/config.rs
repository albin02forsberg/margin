use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Config {
    pub notes_dir: String,
    pub data_dir: String,
    pub export_dir: String,
    pub expected_daily_hours: f64,
    pub profiles: Vec<String>,
    pub todo_keywords: Vec<String>,
    pub done_keywords: Vec<String>,
    /// Daily journal folder, relative to notes_dir.
    pub daily_dir: String,
    /// Where new tasks go by default, relative to notes_dir.
    pub inbox: String,
    /// Closing the window while a timer runs hides it to the tray instead of quitting.
    pub close_to_tray: bool,
    /// System-wide shortcut for quick capture, e.g. "Super+Shift+N"; empty to disable.
    pub capture_shortcut: String,
    /// Ask what to do with the time after this long away from a running timer; 0 to disable.
    pub idle_threshold_minutes: i64,
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
    /// Dropped and pasted files go to <attachments_dir>/<note name>/, relative to notes_dir.
    pub attachments_dir: String,
    /// ActivityWatch server to suggest sessions from, e.g. "http://localhost:5600"; empty = off.
    pub activitywatch_url: String,
    /// Regexes (case-insensitive) over app names and window titles that suggestions ignore.
    pub activity_exclude: Vec<String>,
    /// Ollama model that drafts diary and journal notes, e.g. "llama3.2:3b"; empty = off.
    pub ai_model: String,
    /// The Ollama server; only local addresses are allowed.
    pub ai_url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct View {
    pub name: String,
    pub query: String,
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

impl Default for Config {
    fn default() -> Self {
        let v = |a: &[&str]| a.iter().map(|s| s.to_string()).collect();
        Config {
            notes_dir: "~/notes".into(),
            data_dir: "~/timeclock".into(),
            export_dir: "~/Desktop".into(),
            expected_daily_hours: 8.0,
            profiles: v(&["Work", "Personal"]),
            todo_keywords: v(&["TODO", "NEXT", "WAIT"]),
            done_keywords: v(&["DONE", "CANCELLED"]),
            daily_dir: "daily".into(),
            inbox: "inbox.org".into(),
            close_to_tray: true,
            capture_shortcut: "Super+Shift+N".into(),
            idle_threshold_minutes: 10,
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
            attachments_dir: "attachments".into(),
            activitywatch_url: String::new(),
            activity_exclude: v(&["KeePass", "1Password", "Bitwarden", "Private Browsing", "Incognito", "InPrivate"]),
            ai_model: String::new(),
            ai_url: "http://localhost:11434".into(),
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

    /// Load PATH, writing the defaults there on first run (only when it doesn't exist:
    /// an unreadable file is an error, never overwritten).
    pub fn load(path: &Path) -> Result<Config, String> {
        match std::fs::read_to_string(path) {
            Ok(s) => toml::from_str(&s).map_err(|e| format!("{}: {e}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let c = Config::default();
                if let Some(d) = path.parent() {
                    let _ = std::fs::create_dir_all(d);
                }
                let _ = write_atomic(path, toml::to_string(&c).unwrap());
                Ok(c)
            }
            Err(e) => Err(format!("{}: {e}", path.display())),
        }
    }
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
                return format!("{}name = {}{eol}", &l[..l.len() - t.len()], toml::Value::String(name.into()));
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

/// TEXT with top-level KEY's one-line `key = …` set to VALUE (added at the top if missing);
/// errors if anything else would change, e.g. a multi-line value.
fn set_key(text: &str, key: &str, value: toml::Value) -> Result<String, String> {
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
                return format!("{key} = {value}{}", &l[l.trim_end_matches(['\r', '\n']).len()..]);
            }
            l.to_string()
        })
        .collect();
    let out = if done { out } else { format!("{key} = {value}\n{out}") };
    if out.parse::<toml::Table>().map_err(err)? != want {
        return Err("can't update config.toml without changing other settings; edit it by hand (Ctrl+,)".into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(s, text.replace("  name = \"B\" # old", "  name = 'New \"b\"'")); // only that line; CRLF, comments kept
        let c: Config = toml::from_str(&rename_view(&s, "a", "Z").unwrap()).unwrap();
        assert_eq!((c.views[0].name.as_str(), c.views[1].name.as_str()), ("Z", "New \"b\""));
        assert!(rename_view(text, "nope", "X").is_err());
        assert!(rename_view("views = [{ name = \"A\", query = \"a\" }]\n", "a", "X").is_err()); // inline: by hand
        // A `[[views]]` lookalike inside a multi-line string is refused, not miswritten.
        let body = "[[templates]]\nkey = \"v\"\nname = \"V\"\nbody = \"\"\"\n[[views]]\nname = x\n\"\"\"\n\n[[views]]\nname = \"A\"\nquery = \"a\"\n";
        assert!(rename_view(body, "a", "X").is_err_and(|e| e.contains("changing")));
    }

    #[test]
    fn defaults_round_trip() {
        let c: Config = toml::from_str(&toml::to_string(&Config::default()).unwrap()).unwrap();
        assert_eq!(c.templates.len(), 2);
        assert!(toml::from_str::<Config>("templates = []").unwrap().templates.is_empty());
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
        assert!(Config::load(&p).is_ok() && toml::from_str::<Config>(&std::fs::read_to_string(&p).unwrap()).is_ok()); // first run
        std::fs::write(&p, b"notes_dir = \"\xff\"\n").unwrap();
        assert!(Config::load(&p).is_err());
        assert_eq!(std::fs::read(&p).unwrap(), b"notes_dir = \"\xff\"\n");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
