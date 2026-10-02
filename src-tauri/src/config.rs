use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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
    /// Dropped and pasted files go to <attachments_dir>/<note name>/, relative to notes_dir.
    pub attachments_dir: String,
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
            attachments_dir: "attachments".into(),
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

    /// Load PATH, writing the defaults there on first run.
    pub fn load(path: &Path) -> Result<Config, String> {
        match std::fs::read_to_string(path) {
            Ok(s) => toml::from_str(&s).map_err(|e| format!("{}: {e}", path.display())),
            Err(_) => {
                let c = Config::default();
                if let Some(d) = path.parent() {
                    let _ = std::fs::create_dir_all(d);
                }
                let _ = std::fs::write(path, toml::to_string(&c).unwrap());
                Ok(c)
            }
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
