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
