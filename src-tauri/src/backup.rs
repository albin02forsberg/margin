// Git backup (port of albin/timeclock-git-backup): commit synchronously,
// push on a background thread, keep a log, surface the last failure.
// The data repo also pulls (rebase) before pushing, so machines sharing a remote see each other's time.

use std::path::{Path, PathBuf};
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

/// Logged once while it repeats (an offline sync retries every few minutes).
fn fail(msg: String) {
    let mut last = LAST_FAILURE.lock().unwrap();
    if last.as_deref() != Some(&msg) {
        log(format!("FAILED: {msg}"));
    }
    *last = Some(msg);
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
/// OWN (the app's data repo): create the repo if missing and `sync` instead of a bare push.
pub fn backup(dir: &Path, own: bool) {
    let _lock = BACKUP.lock().unwrap_or_else(|e| e.into_inner());
    let name = dir.display();
    if !dir.join(".git").exists() {
        if !own {
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
    std::thread::spawn(move || match own {
        true => sync(&dir, true),
        false => match git(&dir, &["push"]) {
            (true, _) => ok(format!("{}: push succeeded.", dir.display())),
            (false, out) => fail(format!("{}: git push: {out}", dir.display())),
        },
    });
}

/// One sync at a time: `sync_soon` and the timer skip while one runs, a backup's sync waits.
static SYNC: Mutex<()> = Mutex::new(());
static LAST_SYNC: Mutex<Option<std::time::Instant>> = Mutex::new(None);

/// Pull (fetch + rebase onto upstream), then push if ahead; logs only when something moved.
/// WAIT: queue behind a running sync instead of skipping it (a backup's commit must go out).
pub fn sync(dir: &Path, wait: bool) {
    let _sync = match SYNC.try_lock() {
        Ok(g) => g,
        Err(std::sync::TryLockError::Poisoned(e)) => e.into_inner(),
        Err(_) if wait => SYNC.lock().unwrap_or_else(|e| e.into_inner()),
        Err(_) => return,
    };
    *LAST_SYNC.lock().unwrap() = Some(std::time::Instant::now());
    match sync_repo(dir) {
        Ok(Some(m)) => ok(format!("{}: {m}", dir.display())),
        // Nothing moved: clear this repo's old failure (offline, conflict) only.
        Ok(None) => {
            let mut last = LAST_FAILURE.lock().unwrap();
            if last.as_ref().is_some_and(|f| f.starts_with(&format!("{}: ", dir.display()))) {
                *last = None;
            }
        }
        Err(e) => fail(format!("{}: {e}", dir.display())),
    }
}

/// A conflict is aborted and reported with the file names, so the repo is never left
/// mid-rebase. No remote: nothing; no upstream: a plain push.
fn sync_repo(dir: &Path) -> Result<Option<String>, String> {
    if !dir.join(".git").exists() || git(dir, &["remote"]).1.is_empty() {
        return Ok(None);
    }
    let gitdir = git(dir, &["rev-parse", "--absolute-git-dir"]).1;
    if ["rebase-merge", "rebase-apply", "MERGE_HEAD"].iter().any(|f| Path::new(&gitdir).join(f).exists()) {
        return Err("a rebase or merge is in progress; finish or abort it to resume syncing.".into());
    }
    let pushed = |pulled: &str| match git(dir, &["push"]) {
        (true, _) => Ok(Some(format!("{pulled}push succeeded."))),
        (false, out) => Err(format!("git push: {out}")),
    };
    if !git(dir, &["rev-parse", "--abbrev-ref", "@{u}"]).0 {
        return pushed("");
    }
    // Network outside the backup lock, so a slow remote never holds up a commit.
    let (ok_, out) = git(dir, &["fetch", "--quiet"]);
    if !ok_ {
        return Err(format!("git fetch (offline?): {out}"));
    }
    let incoming = git(dir, &["rev-list", "--count", "HEAD..@{u}"]).1;
    if incoming != "0" {
        let _lock = BACKUP.lock().unwrap_or_else(|e| e.into_inner());
        let (ok_, out) = git(dir, &["-c", "rebase.autoStash=true", "rebase", "@{u}"]);
        if !ok_ {
            let files = git(dir, &["diff", "--name-only", "--diff-filter=U"]).1;
            git(dir, &["rebase", "--abort"]);
            let what = if files.is_empty() { out } else { files.lines().collect::<Vec<_>>().join(", ") };
            return Err(format!("sync conflict, local changes not pushed: {what}"));
        }
    }
    let pulled = if incoming == "0" { String::new() } else { format!("pulled {incoming} commit(s); ") };
    if git(dir, &["rev-list", "--count", "@{u}..HEAD"]).1 == "0" {
        return Ok((!pulled.is_empty()).then(|| pulled.trim_end_matches("; ").to_string() + "."));
    }
    pushed(&pulled)
}

/// Sync in the background unless one started in the last minute (window focus).
pub fn sync_soon(dir: PathBuf) {
    if LAST_SYNC.lock().unwrap().is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(60)) {
        return;
    }
    std::thread::spawn(move || sync(&dir, false));
}

/// Sync DIR() every 5 minutes, so another machine's time shows up without a local change.
pub fn start(dir: impl Fn() -> PathBuf + Send + 'static) {
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(300));
        sync(&dir(), false);
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

    /// A bare remote with two clones A and B (branch main, one commit), under a fresh temp dir.
    fn two_clones() -> (PathBuf, PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!("margin-sync-{}", crate::notes::gen_id()));
        let (remote, a, b) = (root.join("remote.git"), root.join("a"), root.join("b"));
        std::fs::create_dir_all(&a).unwrap();
        fn cfg(d: &Path) {
            for args in [&["config", "user.email", "t@example.com"][..], &["config", "user.name", "t"], &["config", "commit.gpgsign", "false"]] {
                assert!(git(d, args).0);
            }
        }
        assert!(git(&root, &["init", "--bare", "-b", "main", "remote.git"]).0);
        assert!(git(&a, &["init", "-b", "main"]).0);
        cfg(&a);
        commit(&a, "p.toml", "name = \"x\"\n");
        assert!(git(&a, &["remote", "add", "origin", remote.to_str().unwrap()]).0);
        assert!(git(&a, &["push", "-u", "origin", "main"]).0);
        assert!(git(&root, &["clone", "remote.git", "b"]).0);
        cfg(&b);
        (root, a, b)
    }

    fn commit(dir: &Path, file: &str, text: &str) {
        std::fs::write(dir.join(file), text).unwrap();
        assert!(git(dir, &["add", "-A"]).0);
        assert!(git(dir, &["commit", "-m", file]).0);
    }

    #[test]
    fn sync_pulls_before_pushing() {
        let (root, a, b) = two_clones();
        commit(&a, "a.txt", "from a\n");
        assert!(git(&a, &["push"]).0);
        commit(&b, "b.txt", "from b\n");
        assert_eq!(sync_repo(&b), Ok(Some("pulled 1 commit(s); push succeeded.".into())));
        assert!(b.join("a.txt").exists());
        assert_eq!(sync_repo(&a), Ok(Some("pulled 1 commit(s).".into())));
        assert_eq!(std::fs::read_to_string(a.join("b.txt")).unwrap(), "from b\n");
        assert_eq!(sync_repo(&a), Ok(None)); // nothing moved: nothing logged
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn sync_conflict_is_aborted_and_named() {
        let (root, a, b) = two_clones();
        commit(&a, "p.toml", "name = \"a\"\n");
        assert!(git(&a, &["push"]).0);
        commit(&b, "p.toml", "name = \"b\"\n");
        let e = sync_repo(&b).unwrap_err();
        assert!(e.contains("conflict") && e.contains("p.toml"), "{e}");
        assert!(!b.join(".git/rebase-merge").exists() && !b.join(".git/rebase-apply").exists());
        assert_eq!(git(&b, &["status", "--porcelain"]).1, "");
        assert_eq!(std::fs::read_to_string(b.join("p.toml")).unwrap(), "name = \"b\"\n"); // local kept
        // A leftover rebase is refused, not stepped on.
        std::fs::create_dir(b.join(".git/rebase-merge")).unwrap();
        assert!(sync_repo(&b).unwrap_err().contains("in progress"));
        let _ = std::fs::remove_dir_all(&root);
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
