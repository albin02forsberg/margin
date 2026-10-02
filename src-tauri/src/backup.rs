// Git backup (port of albin/timeclock-git-backup): commit synchronously,
// push on a background thread, keep a log, surface the last failure.

use std::path::Path;
use std::process::Command;
use std::sync::Mutex;

static LOG: Mutex<Vec<String>> = Mutex::new(Vec::new());
static LAST_FAILURE: Mutex<Option<String>> = Mutex::new(None);

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

fn git(dir: &Path, args: &[&str]) -> (bool, String) {
    let mut c = Command::new("git");
    c.args(args).current_dir(dir);
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
