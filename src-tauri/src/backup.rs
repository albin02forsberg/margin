// Git backup (port of albin/timeclock-git-backup): commit synchronously,
// push on a background thread, keep a log, surface the last failure.

use std::path::Path;
use std::process::Command;
use std::sync::Mutex;

static LOG: Mutex<Vec<String>> = Mutex::new(Vec::new());
static LAST_FAILURE: Mutex<Option<String>> = Mutex::new(None);
/// Held for a whole backup: commands run on a thread pool, and two `git add` in one repo clash on `index.lock`.
static BACKUP: Mutex<()> = Mutex::new(());

fn log(msg: String) {
    let mut l = LOG.lock().unwrap();
    l.push(format!("[{}] {msg}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")));
    let n = l.len();
    if n > 500 {
        l.drain(..n - 500);
    }
}

fn ok(msg: String) {
    *LAST_FAILURE.lock().unwrap() = None;
    log(msg);
}

fn fail(msg: String) {
    log(format!("FAILED: {msg}"));
    *LAST_FAILURE.lock().unwrap() = Some(msg);
}

pub fn log_text() -> String {
    LOG.lock().unwrap().join("\n")
}

pub fn last_failure() -> Option<String> {
    LAST_FAILURE.lock().unwrap().clone()
}

/// Env changes that keep a child process off the AppImage's bundled libs:
/// `(key, Some(new))` sets, `(key, None)` removes. Only entries under APPDIR go.
fn clean_env(vars: &[(&str, String)], appdir: &str) -> Vec<(String, Option<String>)> {
    let under = |p: &str| !appdir.is_empty() && Path::new(p).starts_with(appdir);
    let mut out = Vec::new();
    for (k, v) in vars {
        match *k {
            "LD_LIBRARY_PATH" => {
                let all: Vec<&str> = v.split(':').filter(|p| !p.is_empty()).collect();
                let kept: Vec<&str> = all.iter().copied().filter(|p| !under(*p)).collect();
                if kept.len() != all.len() {
                    out.push((k.to_string(), (!kept.is_empty()).then(|| kept.join(":"))));
                }
            }
            "GIT_EXEC_PATH" | "GIT_SSL_CAINFO" | "GIT_TEMPLATE_DIR" | "PERLLIB" if under(v.as_str()) => out.push((k.to_string(), None)),
            _ => {}
        }
    }
    out
}

fn git(dir: &Path, args: &[&str]) -> (bool, String) {
    let mut c = Command::new("git");
    c.args(args).current_dir(dir);
    if let Ok(appdir) = std::env::var("APPDIR") {
        let vars: Vec<(&str, String)> = ["LD_LIBRARY_PATH", "GIT_EXEC_PATH", "GIT_SSL_CAINFO", "GIT_TEMPLATE_DIR", "PERLLIB"]
            .into_iter()
            .filter_map(|k| std::env::var(k).ok().map(|v| (k, v)))
            .collect();
        for (k, v) in clean_env(&vars, &appdir) {
            match v {
                Some(v) => c.env(k, v),
                None => c.env_remove(k),
            };
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW: no console flash
    }
    match c.output() {
        Ok(o) => (o.status.success(), format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr)).trim().to_string()),
        Err(e) => (false, format!("git: {e}")),
    }
}

/// Commit everything in DIR; push in the background if a remote exists.
/// INIT creates the repo if missing (only for dirs this app owns).
pub fn backup(dir: &Path, init: bool) {
    let _lock = BACKUP.lock().unwrap_or_else(|e| e.into_inner());
    let name = dir.display();
    if !dir.join(".git").exists() {
        if !init {
            return;
        }
        let _ = std::fs::create_dir_all(dir);
        let (ok_, out) = git(dir, &["init"]);
        if !ok_ {
            return fail(format!("{name}: git init: {out}"));
        }
    }
    let (ok_, out) = git(dir, &["add", "-A"]);
    if !ok_ {
        return fail(format!("{name}: git add: {out}"));
    }
    if git(dir, &["diff", "--cached", "--quiet"]).0 {
        return ok(format!("{name}: nothing new to back up."));
    }
    let msg = format!("⏱ Auto-backup: {}", chrono::Local::now().format("%Y-%m-%d %H:%M"));
    let (ok_, out) = git(dir, &["commit", "-m", &msg]);
    if !ok_ {
        return fail(format!("{name}: git commit: {out}"));
    }
    ok(format!("{name}: committed: {}", out.lines().next().unwrap_or("")));
    if git(dir, &["remote"]).1.is_empty() {
        return log(format!("{name}: no remote configured, push skipped."));
    }
    let dir = dir.to_path_buf();
    std::thread::spawn(move || match git(&dir, &["push"]) {
        (true, _) => ok(format!("{}: push succeeded.", dir.display())),
        (false, out) => fail(format!("{}: git push: {out}", dir.display())),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(k: &'static str, s: &str) -> (&'static str, String) {
        (k, s.to_string())
    }

    #[test]
    fn concurrent_backups_both_succeed() {
        let dir = std::env::temp_dir().join(format!("margin-backup-{}", crate::notes::gen_id()));
        std::fs::create_dir_all(&dir).unwrap();
        for args in [&["init"][..], &["config", "user.email", "t@example.com"], &["config", "user.name", "t"], &["config", "commit.gpgsign", "false"]] {
            assert!(git(&dir, args).0);
        }
        for n in 0..50 {
            std::fs::write(dir.join(format!("{n}.org")), "x\n").unwrap();
        }
        std::thread::scope(|t| {
            for _ in 0..2 {
                t.spawn(|| backup(&dir, false));
            }
        });
        assert_eq!(last_failure(), None, "{}", log_text());
        assert_eq!(git(&dir, &["status", "--porcelain"]).1, "");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn strips_appdir_lib_entries() {
        let r = clean_env(&[v("LD_LIBRARY_PATH", "/tmp/.mount_X/usr/lib:/opt/lib::/tmp/.mount_X/lib64")], "/tmp/.mount_X");
        assert_eq!(r, vec![("LD_LIBRARY_PATH".to_string(), Some("/opt/lib".to_string()))]);
    }

    #[test]
    fn removes_var_when_only_appdir_entries() {
        let r = clean_env(&[v("LD_LIBRARY_PATH", "/tmp/.mount_X/usr/lib")], "/tmp/.mount_X");
        assert_eq!(r, vec![("LD_LIBRARY_PATH".to_string(), None)]);
    }

    #[test]
    fn leaves_foreign_values_and_prefix_lookalikes() {
        let r = clean_env(&[v("LD_LIBRARY_PATH", "/opt/lib:/tmp/.mount_XY/lib"), v("GIT_EXEC_PATH", "/usr/lib/git-core")], "/tmp/.mount_X");
        assert!(r.is_empty());
    }

    #[test]
    fn removes_git_vars_under_appdir() {
        let r = clean_env(&[v("GIT_EXEC_PATH", "/tmp/.mount_X/usr/libexec/git-core"), v("PERLLIB", "/usr/share/perl")], "/tmp/.mount_X");
        assert_eq!(r, vec![("GIT_EXEC_PATH".to_string(), None)]);
    }
}
