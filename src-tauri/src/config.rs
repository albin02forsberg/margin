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

/// TEXT (a config.toml) with a `[[views]]` entry appended; errors if the result won't load.
pub fn add_view(text: &str, name: &str, query: &str) -> Result<String, String> {
    // The defaults written on first run say `views = []`, which a `[[views]]` table would redefine.
    let mut out: String = text.lines().filter(|l| l.trim() != "views = []").map(|l| format!("{l}\n")).collect();
    let q = |s: &str| toml::Value::String(s.into()).to_string();
    out += &format!("\n[[views]]\nname = {}\nquery = {}\n", q(name), q(query));
    toml::from_str::<Config>(&out).map_err(|e| format!("can't add the view to config.toml: {e}"))?;
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
    }

    #[test]
    fn defaults_round_trip() {
        let c: Config = toml::from_str(&toml::to_string(&Config::default()).unwrap()).unwrap();
        assert_eq!(c.templates.len(), 2);
        assert!(toml::from_str::<Config>("templates = []").unwrap().templates.is_empty());
    }
}
