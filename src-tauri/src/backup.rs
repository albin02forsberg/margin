// Git backup (port of albin/timeclock-git-backup): commit synchronously,
// push on a background thread, keep a log, surface the last failure.
// The data repo also pulls (rebase) before pushing, so machines sharing a remote see each other's time.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::collections::BTreeMap;
use std::sync::Mutex;

static LOG: Mutex<Vec<String>> = Mutex::new(Vec::new());
/// Each repo's last failure, keyed by its dir: one repo's success never hides another's failure.
static LAST_FAILURE: Mutex<BTreeMap<String, String>> = Mutex::new(BTreeMap::new());
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

fn ok(dir: &Path, msg: String) {
    LAST_FAILURE.lock().unwrap().remove(&dir.display().to_string());
    log(msg);
}

/// Logged once while it repeats (an offline sync retries every few minutes).
fn fail(dir: &Path, msg: String) {
    let mut last = LAST_FAILURE.lock().unwrap();
    if last.insert(dir.display().to_string(), msg.clone()).as_ref() != Some(&msg) {
        log(format!("FAILED: {msg}"));
    }
}

pub fn log_text() -> String {
    LOG.lock().unwrap().join("\n")
}

pub fn last_failure() -> Option<String> {
    let l = LAST_FAILURE.lock().unwrap();
    (!l.is_empty()).then(|| l.values().cloned().collect::<Vec<_>>().join("; "))
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
                let kept: Vec<&str> = all.iter().copied().filter(|p| !under(p)).collect();
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

/// Stage and commit everything in DIR (OWN: after `prepare_repo`). The caller holds `BACKUP`.
/// `Ok(None)`: nothing to commit; `Ok(Some(first line of git's output))` otherwise.
fn commit_all(dir: &Path, own: bool) -> Result<Option<String>, String> {
    if own {
        prepare_repo(dir);
    }
    let (ok_, out) = git(dir, &["add", "-A"]);
    if !ok_ {
        return Err(format!("git add: {out}"));
    }
    if git(dir, &["diff", "--cached", "--quiet"]).0 {
        return Ok(None);
    }
    let msg = format!("⏱ Auto-backup: {}", chrono::Local::now().format("%Y-%m-%d %H:%M"));
    let (ok_, out) = git(dir, &["commit", "-m", &msg]);
    if !ok_ {
        return Err(format!("git commit: {out}"));
    }
    Ok(Some(out.lines().next().unwrap_or("").to_string()))
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
            return fail(dir, format!("{name}: git init: {out}"));
        }
    }
    match commit_all(dir, own) {
        Err(e) => return fail(dir, format!("{name}: {e}")),
        Ok(None) => return ok(dir, format!("{name}: nothing new to back up.")),
        Ok(Some(first)) => ok(dir, format!("{name}: committed: {first}")),
    }
    if git(dir, &["remote"]).1.is_empty() {
        return log(format!("{name}: no remote configured, push skipped."));
    }
    let dir = dir.to_path_buf();
    std::thread::spawn(move || match own {
        true => sync(&dir, true),
        false => match git(&dir, &["push"]) {
            (true, _) => ok(&dir, format!("{}: push succeeded.", dir.display())),
            (false, out) => fail(&dir, format!("{}: git push: {out}", dir.display())),
        },
    });
}

/// Append each of LINES to FILE unless a line is already there (user lines are kept).
fn ensure_lines(file: &Path, lines: &[&str]) {
    let old = std::fs::read_to_string(file).unwrap_or_default();
    let missing: Vec<&str> = lines.iter().copied().filter(|l| !old.lines().any(|o| o.trim() == *l)).collect();
    if missing.is_empty() {
        return;
    }
    let mut text = old;
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(&missing.join("\n"));
    text.push('\n');
    let _ = std::fs::write(file, text);
}

/// Append-only logs (the time log, leave, the per-day activity files) merge by keeping both
/// sides' lines, so two machines' events never conflict (`normalize_logs` restores time order
/// of the time log). Only missing lines are added, and from a backup (committed right away):
/// an untracked copy would block a rebase bringing the other machine's.
fn ensure_attributes(dir: &Path) {
    let lines = ["timelog.jsonl merge=union", "leave.jsonl merge=union", "activity/*.jsonl merge=union"];
    ensure_lines(&dir.join(".gitattributes"), &lines);
}

/// The active profile is per machine: ignored, and dropped from the index if an older build
/// committed it (`sync_repo` keeps the local file when a pull removes it).
fn untrack_local_state(dir: &Path) {
    ensure_lines(&dir.join(".gitignore"), &["active-profile.txt"]);
    git(dir, &["rm", "--cached", "-f", "-q", "--ignore-unmatch", "active-profile.txt"]);
}

/// Everything the data repo needs before a commit.
fn prepare_repo(dir: &Path) {
    ensure_attributes(dir);
    untrack_local_state(dir);
}

/// After a pull: put each profile's (`<data>/<profile>/timelog.jsonl`) unioned log back in
/// time order and drop duplicate time off lines; returns the changed files (relative to DIR).
/// Outside the backup lock, since a backup may run under the time-log lock.
fn normalize_logs(dir: &Path) -> Vec<String> {
    use crate::timeclock::Tc;
    let mut changed = vec![];
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten().filter(|e| e.path().is_dir()) {
        let tc = Tc { dir: e.path(), diary: PathBuf::new(), expected: 0.0 };
        type Fix = fn(&Tc) -> Result<bool, String>;
        let fixes: [(&str, Fix); 2] = [("timelog.jsonl", Tc::normalize_log), ("leave.jsonl", Tc::dedupe_leave)];
        for (file, fix) in fixes.into_iter().filter(|(f, _)| e.path().join(f).is_file()) {
            match crate::timeclock::locked(|| fix(&tc)) {
                Ok(true) => changed.push(format!("{}/{file}", e.file_name().to_string_lossy())),
                Ok(false) => {}
                Err(e) => log(format!("FAILED: {}: tidying {file}: {e}", tc.dir.display())), // the sync's result stays the warning
            }
        }
    }
    changed
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
        Ok(Some(m)) => ok(dir, format!("{}: {m}", dir.display())),
        // Nothing moved: clear this repo's old failure (offline, conflict) only.
        Ok(None) => {
            LAST_FAILURE.lock().unwrap().remove(&dir.display().to_string());
        }
        Err(e) => fail(dir, format!("{}: {e}", dir.display())),
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
        // Time-log lock first (as in `normalize_logs`), so no clock event lands between the commit
        // and the rebase; the fetch above and the push below stay outside both locks.
        crate::timeclock::locked(|| -> Result<(), String> {
            let _lock = BACKUP.lock().unwrap_or_else(|e| e.into_inner());
            // Local changes go into a commit first (no autostash: a failed re-apply would leave
            // conflict markers in the tree), so a clash is an ordinary, abortable rebase conflict.
            commit_all(dir, true).map_err(|e| format!("committing local changes before the pull: {e}"))?;
            // A pull that untracks the file (older builds committed it) deletes the working copy.
            let profile_file = dir.join("active-profile.txt");
            let profile = std::fs::read(&profile_file).ok();
            let (ok_, out) = git(dir, &["rebase", "@{u}"]);
            if let Some(p) = ok_.then_some(profile).flatten().filter(|_| !profile_file.exists()) {
                let _ = std::fs::write(&profile_file, p);
            }
            let files = git(dir, &["diff", "--name-only", "--diff-filter=U"]).1;
            let in_rebase = ["rebase-merge", "rebase-apply"].iter().any(|f| Path::new(&gitdir).join(f).exists());
            if ok_ && files.is_empty() {
                return Ok(());
            }
            if !ok_ && files.is_empty() && !in_rebase {
                // E.g. the activity watcher dirtied the tree: nothing to abort, the next sync retries.
                return Err(format!("rebase failed, will retry: {out}"));
            }
            if in_rebase {
                git(dir, &["rebase", "--abort"]);
            }
            let what = if files.is_empty() { out } else { files.lines().collect::<Vec<_>>().join(", ") };
            Err(format!("sync conflict, local changes not pushed: {what}"))
        })?;
    }
    // Commit the sorted logs right away (pushed below), so the tree isn't left dirty for the next rebase.
    let sorted = if incoming == "0" { vec![] } else { normalize_logs(dir) };
    if !sorted.is_empty() {
        let _lock = BACKUP.lock().unwrap_or_else(|e| e.into_inner());
        let mut args = vec!["commit", "-q", "-m", "⏱ Time log sorted after sync", "--"];
        args.extend(sorted.iter().map(String::as_str));
        if let (false, out) = git(dir, &args) {
            log(format!("FAILED: {}: committing the sorted time log: {out}", dir.display()));
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
    fn failures_are_kept_per_repo() {
        let (a, b) = (Path::new("/margin-test/fail-a"), Path::new("/margin-test/fail-b"));
        let mine = || last_failure().unwrap_or_default().split("; ").filter(|m| m.starts_with("/margin-test/")).collect::<Vec<_>>().join("; ");
        fail(a, "/margin-test/fail-a: x".into());
        ok(b, "b done".into()); // another repo succeeding keeps A's failure
        assert_eq!(mine(), "/margin-test/fail-a: x");
        fail(b, "/margin-test/fail-b: y".into());
        assert_eq!(mine(), "/margin-test/fail-a: x; /margin-test/fail-b: y");
        ok(a, "a done".into());
        assert_eq!(mine(), "/margin-test/fail-b: y");
        ok(b, "b done".into());
        assert_eq!(mine(), "");
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
        let me = dir.display().to_string(); // the map is shared with other tests
        assert!(!last_failure().is_some_and(|f| f.contains(&me)), "{}", log_text());
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
        prepare_repo(&a); // as a real data repo has it: a sync then has nothing of its own to add
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
    fn backup_syncs_the_data_repo() {
        let (root, a, b) = two_clones();
        commit(&a, "a.txt", "from a\n");
        assert!(git(&a, &["push"]).0);
        sync(&b, true);
        assert!(b.join("a.txt").exists());
        assert!(log_text().contains("pulled 1 commit(s)."), "{}", log_text());
        sync_soon(b.clone()); // throttled: one just ran
        std::fs::write(b.join("b.txt"), "from b\n").unwrap();
        backup(&b, true);
        // The commit goes out on a background sync: wait for it.
        let t = std::time::Instant::now();
        while git(&b, &["rev-list", "--count", "@{u}..HEAD"]).1 != "0" {
            assert!(t.elapsed() < std::time::Duration::from_secs(20), "{}", log_text());
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn union_merge_keeps_both_machines_time_in_order() {
        let (root, a, b) = two_clones();
        let base = r#"{"ev":"out","t":"2026-09-30T17:00:00"}"#;
        let (ev_a, ev_b) = (r#"{"ev":"in","t":"2026-10-01T10:00:00","project":"A"}"#, r#"{"ev":"in","t":"2026-10-01T08:00:00","project":"B"}"#);
        prepare_repo(&a);
        std::fs::create_dir_all(a.join("work")).unwrap();
        commit(&a, "work/timelog.jsonl", &format!("{base}\n"));
        assert!(git(&a, &["push"]).0);
        assert_eq!(sync_repo(&b), Ok(Some("pulled 1 commit(s).".into())));
        // Both machines append after the same line: a plain merge would conflict.
        commit(&a, "work/timelog.jsonl", &format!("{base}\n{ev_a}\n"));
        assert!(git(&a, &["push"]).0);
        commit(&b, "work/timelog.jsonl", &format!("{base}\n{ev_b}\n"));
        assert_eq!(sync_repo(&b), Ok(Some("pulled 1 commit(s); push succeeded.".into())));
        let sorted = format!("{base}\n{ev_b}\n{ev_a}\n");
        for d in [&b, &a] {
            assert!(sync_repo(d).is_ok());
            assert_eq!(std::fs::read_to_string(d.join("work/timelog.jsonl")).unwrap(), sorted);
            assert_eq!(git(d, &["status", "--porcelain"]).1, ""); // the sort is committed
        }
        assert_eq!(git(&a, &["rev-parse", "HEAD"]).1, git(&b, &["rev-parse", "HEAD"]).1);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn attributes_are_appended_once() {
        let d = std::env::temp_dir().join(format!("margin-attrs-{}", crate::notes::gen_id()));
        std::fs::create_dir_all(&d).unwrap();
        // An older build's file, without a trailing newline, plus a user line.
        std::fs::write(d.join(".gitattributes"), "timelog.jsonl merge=union\nleave.jsonl merge=union\n*.org text").unwrap();
        ensure_attributes(&d);
        ensure_attributes(&d);
        let t = std::fs::read_to_string(d.join(".gitattributes")).unwrap();
        assert_eq!(t, "timelog.jsonl merge=union\nleave.jsonl merge=union\n*.org text\nactivity/*.jsonl merge=union\n");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn activity_files_union_merge() {
        let (root, a, b) = two_clones();
        prepare_repo(&a);
        std::fs::create_dir_all(a.join("activity")).unwrap();
        std::fs::create_dir_all(b.join("activity")).unwrap();
        commit(&a, "activity/2026-10-01.jsonl", "{\"n\":0}\n");
        assert!(git(&a, &["push"]).0);
        assert!(sync_repo(&b).is_ok());
        commit(&a, "activity/2026-10-01.jsonl", "{\"n\":0}\n{\"n\":1}\n");
        assert!(git(&a, &["push"]).0);
        commit(&b, "activity/2026-10-01.jsonl", "{\"n\":0}\n{\"n\":2}\n");
        assert_eq!(sync_repo(&b), Ok(Some("pulled 1 commit(s); push succeeded.".into())));
        let t = std::fs::read_to_string(b.join("activity/2026-10-01.jsonl")).unwrap();
        assert!(t.contains("\"n\":1") && t.contains("\"n\":2"), "{t}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn active_profile_is_untracked_and_survives_the_pull() {
        let (root, a, b) = two_clones();
        // An older build committed it on both machines.
        std::fs::write(a.join("active-profile.txt"), "work\n").unwrap();
        assert!(git(&a, &["add", "-f", "active-profile.txt"]).0); // it is ignored now
        assert!(git(&a, &["commit", "-m", "old build"]).0);
        assert!(git(&a, &["push"]).0);
        assert!(sync_repo(&b).is_ok());
        assert_eq!(std::fs::read_to_string(b.join("active-profile.txt")).unwrap(), "work\n");
        // A new build untracks it and pushes.
        prepare_repo(&a);
        assert!(git(&a, &["add", "-A"]).0);
        assert!(git(&a, &["commit", "-m", "untrack"]).0);
        assert!(git(&a, &["push"]).0);
        assert_eq!(git(&a, &["ls-files", "active-profile.txt"]).1, "");
        assert!(a.join("active-profile.txt").exists());
        // The pull removes the file from the index but B keeps its working copy.
        assert!(sync_repo(&b).is_ok());
        assert_eq!(std::fs::read_to_string(b.join("active-profile.txt")).unwrap(), "work\n");
        assert_eq!(git(&b, &["ls-files", "active-profile.txt"]).1, "");
        assert_eq!(git(&b, &["status", "--porcelain"]).1, "");
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
    fn dirty_change_clashing_with_the_pull_is_committed_not_marked() {
        let (root, a, b) = two_clones();
        commit(&a, "p.toml", "name = \"a\"\n");
        assert!(git(&a, &["push"]).0);
        std::fs::write(b.join("p.toml"), "name = \"b\"\n").unwrap(); // uncommitted
        let e = sync_repo(&b).unwrap_err();
        assert!(e.contains("conflict") && e.contains("p.toml"), "{e}");
        assert!(!b.join(".git/rebase-merge").exists() && !b.join(".git/rebase-apply").exists());
        assert_eq!(std::fs::read_to_string(b.join("p.toml")).unwrap(), "name = \"b\"\n"); // no markers
        assert_eq!(git(&b, &["status", "--porcelain"]).1, "");
        assert_eq!(git(&b, &["show", "HEAD:p.toml"]).1, "name = \"b\""); // kept in a commit
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn dirty_unrelated_file_syncs() {
        let (root, a, b) = two_clones();
        commit(&a, "a.txt", "from a\n");
        assert!(git(&a, &["push"]).0);
        std::fs::write(b.join("notes.txt"), "local\n").unwrap();
        assert_eq!(sync_repo(&b), Ok(Some("pulled 1 commit(s); push succeeded.".into())));
        assert!(b.join("a.txt").exists());
        assert_eq!(git(&b, &["status", "--porcelain"]).1, "");
        assert_eq!(git(&b, &["show", "HEAD:notes.txt"]).1, "local");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn rebase_waits_for_the_time_log_lock() {
        const EVENT: &str = r#"{"ev":"in","t":"2026-10-01T08:00:00","project":"B"}"#;
        let (root, a, b) = two_clones();
        commit(&a, "a.txt", "from a\n");
        assert!(git(&a, &["push"]).0);
        let before = git(&b, &["rev-parse", "HEAD"]).1;
        std::fs::create_dir_all(b.join("work")).unwrap();
        let res = std::thread::scope(|t| {
            let h = crate::timeclock::locked(|| {
                let h = t.spawn(|| sync_repo(&b));
                std::thread::sleep(std::time::Duration::from_millis(400));
                assert!(!h.is_finished(), "the sync must wait for the time-log lock");
                assert_eq!(git(&b, &["rev-parse", "HEAD"]).1, before);
                // A clock event appended meanwhile ends up in the sync's commit, not lost.
                std::fs::write(b.join("work/timelog.jsonl"), format!("{EVENT}\n")).unwrap();
                h
            });
            h.join().unwrap()
        });
        assert_eq!(res, Ok(Some("pulled 1 commit(s); push succeeded.".into())));
        assert_eq!(git(&b, &["show", "HEAD:work/timelog.jsonl"]).1, EVENT);
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
