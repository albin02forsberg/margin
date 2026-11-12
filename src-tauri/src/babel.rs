//! Run a `#+begin_src sh` block on demand (`Ctrl+Enter` / `Space i r`, like org-babel).
//! Opt-in: only languages listed in `babel_languages` run, in the notes folder, for at most 10 s.

use std::path::Path;
use std::time::Duration;

/// Bytes of output kept.
#[cfg(unix)]
const MAX: usize = 64 * 1024;
/// How long a block may run before it and everything it started is killed.
pub const TIMEOUT: Duration = Duration::from_secs(10);
/// Languages this slice can run, all through `sh -c`.
const SHELLS: [&str; 2] = ["sh", "shell"];

/// What a run printed (stdout, then stderr, plus marker lines for a cut, a timeout or a failing exit) and its exit code.
#[derive(serde::Serialize, Debug)]
pub struct Out {
    pub output: String,
    pub exit: Option<i32>,
}

/// Run CODE as LANG in CWD, if ALLOWED (the `babel_languages` setting) lists LANG.
pub fn run(allowed: &[String], lang: &str, code: &str, cwd: &Path, timeout: Duration) -> Result<Out, String> {
    let l = lang.to_ascii_lowercase();
    if !allowed.iter().any(|a| a.trim().eq_ignore_ascii_case(&l)) {
        return Err(format!("{lang} blocks don't run: add {l} to “Run code blocks” (babel_languages) in Settings"));
    }
    if !SHELLS.contains(&l.as_str()) {
        return Err(format!("only sh blocks can run, not {lang}"));
    }
    sh(code, cwd, timeout)
}

/// `Ctrl+Enter` / `Space i r`: run a block. The language is checked here again and the folder is
/// always the notes folder, whatever the webview sends.
#[tauri::command(async)]
pub fn babel_run(s: tauri::State<crate::App>, lang: String, code: String) -> Result<Out, String> {
    let c = s.cfg();
    run(&c.babel_languages, &lang, &code, &c.notes(), TIMEOUT)
}

/// One file to tangle (`tangleFiles` in babel.ts): PATH relative to the note's folder, its CONTENT,
/// `0755` if EXEC (`:shebang`), missing folders created if MKDIRP.
#[derive(serde::Deserialize, Debug)]
pub struct Tangle {
    pub path: String,
    pub content: String,
    pub exec: bool,
    pub mkdirp: bool,
}

/// Where REL lands under DIR, if it stays inside NOTES, through symlinks too: only plain relative parts, and the
/// nearest existing ancestor (the file itself if it exists) must resolve inside NOTES.
fn target(notes: &Path, dir: &Path, rel: &str) -> Result<std::path::PathBuf, String> {
    use std::path::Component;
    let r = Path::new(rel);
    if rel.trim().is_empty() || !r.components().all(|c| matches!(c, Component::Normal(_) | Component::CurDir)) {
        return Err(format!("{rel}: a tangle path must be relative to the note's folder, without .."));
    }
    let p = dir.join(r);
    let notes = notes.canonicalize().map_err(|e| format!("notes folder: {e}"))?;
    let mut a = p.as_path();
    while !a.exists() {
        a = a.parent().ok_or_else(|| format!("{rel}: no existing folder"))?;
    }
    if !a.canonicalize().is_ok_and(|a| a.starts_with(&notes)) {
        return Err(format!("{rel} is outside the notes folder"));
    }
    Ok(p)
}

/// Org-babel-tangle FILES of NOTE (in NOTES): every path is checked first, then, if WRITE, each file whose
/// content changes is copied to `<file>.bak` and replaced atomically. Returns the paths whose content changes.
pub fn tangle(notes: &Path, note: &Path, files: &[Tangle], write: bool) -> Result<Vec<String>, String> {
    let dir = note.parent().ok_or("the note has no folder")?;
    let mut plan = Vec::new();
    for f in files {
        let p = target(notes, dir, &f.path)?;
        if p.is_dir() {
            return Err(format!("{} is a folder", f.path));
        }
        if !f.mkdirp && !p.parent().is_some_and(Path::is_dir) {
            return Err(format!("{}: its folder doesn't exist (add :mkdirp yes)", f.path));
        }
        let changes = std::fs::read(&p).is_ok_and(|old| old != f.content.as_bytes());
        plan.push((p, f, changes));
    }
    if write {
        for (p, f, changes) in &plan {
            let err = |e: std::io::Error| format!("{}: {e}", f.path);
            if let Some(d) = p.parent().filter(|_| f.mkdirp) {
                std::fs::create_dir_all(d).map_err(err)?;
            }
            if *changes {
                let mut bak = p.clone().into_os_string();
                bak.push(".bak");
                let _ = std::fs::remove_file(&bak); // a link there must not redirect the copy
                std::fs::copy(p, &bak).map_err(err)?;
            }
            crate::config::write_atomic(p, &f.content).map_err(err)?;
            #[cfg(unix)]
            if f.exec {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o755)).map_err(err)?;
            }
        }
    }
    Ok(plan.into_iter().filter(|x| x.2).map(|x| x.1.path.clone()).collect())
}

/// "Notes: tangle code blocks to files": WRITE false only lists the files that would change (for the confirm).
/// NOTE and every target are checked here again, whatever the webview sends.
#[tauri::command(async)]
pub fn babel_tangle(s: tauri::State<crate::App>, note: std::path::PathBuf, files: Vec<Tangle>, write: bool) -> Result<Vec<String>, String> {
    s.check(&note)?;
    let notes = s.cfg().notes();
    crate::in_notes(&notes, &note)?;
    tangle(&notes, &note, &files, write)
}

#[cfg(not(unix))]
fn sh(_: &str, _: &Path, _: Duration) -> Result<Out, String> {
    Err("sh not available on this system".into())
}

#[cfg(unix)]
fn sh(code: &str, cwd: &Path, timeout: Duration) -> Result<Out, String> {
    use std::io::Read;
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};
    use std::sync::mpsc::{channel, Receiver};
    use std::time::Instant;

    type Pipe = Option<(Vec<u8>, bool)>;
    /// Collect the reader threads' results until UNTIL; a pipe still held open (by a background job) stays None.
    fn collect(rx: &Receiver<(usize, Vec<u8>, bool)>, outs: &mut [Pipe; 2], until: Instant) {
        while outs.iter().any(Option::is_none) {
            match rx.recv_timeout(until.saturating_duration_since(Instant::now())) {
                Ok((i, b, cut)) => outs[i] = Some((b, cut)),
                Err(_) => break,
            }
        }
    }

    // Its own process group, so a timeout kills `sleep 60 | cat` and `cmd &` too, not just sh.
    let mut child = Command::new("sh").arg("-c").arg(code).current_dir(cwd).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).process_group(0)
        .spawn().map_err(|e| format!("can't run sh: {e}"))?;
    let (tx, rx) = channel();
    let pipes: [Box<dyn Read + Send>; 2] = [Box::new(child.stdout.take().unwrap()), Box::new(child.stderr.take().unwrap())];
    for (i, mut p) in pipes.into_iter().enumerate() {
        let tx = tx.clone();
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = (&mut p).take(MAX as u64 + 1).read_to_end(&mut buf);
            let cut = buf.len() > MAX;
            if cut {
                let _ = std::io::copy(&mut p, &mut std::io::sink()); // keep draining so the command isn't blocked on a full pipe
            }
            let _ = tx.send((i, buf, cut));
        });
    }
    drop(tx);
    let deadline = Instant::now() + timeout;
    let mut status = None;
    while status.is_none() && Instant::now() < deadline {
        status = child.try_wait().map_err(|e| e.to_string())?;
        if status.is_none() {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    let mut outs: [Pipe; 2] = [None, None];
    collect(&rx, &mut outs, deadline);
    let timed_out = status.is_none() || outs.iter().any(Option::is_none);
    if timed_out {
        // The group id is sh's pid; it can't have been reused while sh is unreaped or a member still holds a pipe.
        let _ = Command::new("kill").args(["-KILL", "--", &format!("-{}", child.id())]).stderr(Stdio::null()).status();
        let _ = child.kill();
        status = child.wait().ok();
        collect(&rx, &mut outs, Instant::now() + Duration::from_secs(1));
    }
    let mut cut = false;
    let mut text = String::new();
    for (b, c) in outs.into_iter().flatten() {
        text.push_str(&String::from_utf8_lossy(&b));
        cut |= c;
    }
    if text.len() > MAX {
        let mut n = MAX;
        while !text.is_char_boundary(n) {
            n -= 1;
        }
        text.truncate(n);
        cut = true;
    }
    let exit = status.and_then(|s| s.code());
    let mut mark = |m: String| {
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(&m);
    };
    if cut {
        mark(format!("[output cut at {} KB]", MAX / 1024));
    }
    if timed_out {
        mark(format!("[stopped after {} s]", timeout.as_secs_f64()));
    } else if exit != Some(0) {
        mark(exit.map_or("[killed by a signal]".into(), |c| format!("[exit {c}]")));
    }
    Ok(Out { output: text, exit })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::time::Instant;

    fn sh_run(code: &str, timeout: Duration) -> Out {
        run(&["sh".into()], "sh", code, &std::env::temp_dir(), timeout).unwrap()
    }

    #[test]
    fn only_listed_languages() {
        let tmp = std::env::temp_dir();
        assert!(run(&[], "sh", "echo hi", &tmp, TIMEOUT).unwrap_err().contains("babel_languages"));
        assert!(run(&["python".into()], "sh", "echo hi", &tmp, TIMEOUT).is_err());
        assert!(run(&["python".into()], "python", "print(1)", &tmp, TIMEOUT).unwrap_err().contains("only sh"));
        assert_eq!(run(&["SH".into()], "sh", "echo hi", &tmp, TIMEOUT).unwrap().output, "hi\n");
        assert!(run(&["sh".into()], "shell", "echo hi", &tmp, TIMEOUT).is_err(), "each name must be listed");
    }

    #[test]
    fn output_and_exit() {
        let o = sh_run("echo out; echo err >&2; exit 3", TIMEOUT);
        assert_eq!((o.output.as_str(), o.exit), ("out\nerr\n[exit 3]", Some(3)));
        let o = sh_run("printf ok", TIMEOUT);
        assert_eq!((o.output.as_str(), o.exit), ("ok", Some(0)));
        let o = sh_run("", TIMEOUT);
        assert_eq!((o.output.as_str(), o.exit), ("", Some(0)));
        assert!(sh_run("cat", TIMEOUT).exit == Some(0), "stdin is empty, not the app's");
    }

    #[test]
    fn runs_in_cwd() {
        let d = std::env::temp_dir().join(format!("margin-babel-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("marker.txt"), "").unwrap();
        let o = run(&["sh".into()], "sh", "ls", &d, TIMEOUT).unwrap();
        std::fs::remove_dir_all(&d).unwrap();
        assert_eq!(o.output, "marker.txt\n");
    }

    #[test]
    fn output_is_cut() {
        let o = sh_run("yes x | head -c 200000", TIMEOUT);
        assert!(o.output.ends_with("\n[output cut at 64 KB]"), "{}", &o.output[o.output.len() - 40..]);
        assert!(o.output.len() < MAX + 40);
        // A command printing far more than the limit still finishes instead of blocking on a full pipe.
        let t = Instant::now();
        assert_eq!(sh_run("yes | head -c 5000000; echo done >&2", TIMEOUT).exit, Some(0));
        assert!(t.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn timeout_kills_the_group() {
        let t = Instant::now();
        let o = sh_run("echo started; sleep 30 | cat", Duration::from_secs(1));
        assert!(t.elapsed() < Duration::from_secs(5), "took {:?}", t.elapsed());
        assert_eq!(o.output, "started\n[stopped after 1 s]");
        // A background job holding the pipe after sh exits is stopped too.
        let t = Instant::now();
        let o = sh_run("sleep 30 & echo bg", Duration::from_secs(1));
        assert!(t.elapsed() < Duration::from_secs(5), "took {:?}", t.elapsed());
        assert!(o.output.starts_with("bg\n[stopped after"), "{}", o.output);
        // Nothing in the pipeline outlives the kill.
        let d = std::env::temp_dir().join(format!("margin-babel-kill-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        run(&["sh".into()], "sh", "(sleep 2; touch late) | cat", &d, Duration::from_secs(1)).unwrap();
        std::thread::sleep(Duration::from_millis(2500));
        let late = d.join("late").exists();
        std::fs::remove_dir_all(&d).unwrap();
        assert!(!late, "the subshell kept running after the timeout");
    }

    #[test]
    fn tangle_writes_inside_the_notes_folder_only() {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!("margin-tangle-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (notes, outside) = (root.join("notes"), root.join("outside"));
        std::fs::create_dir_all(notes.join("sub")).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, notes.join("sub/out")).unwrap();
        let note = notes.join("sub/n.org");
        let f = |path: &str, content: &str, exec: bool, mkdirp: bool| Tangle { path: path.into(), content: content.into(), exec, mkdirp };
        let err = |path: &str, mkdirp: bool| tangle(&notes, &note, &[f(path, "x", false, mkdirp)], true).unwrap_err();

        assert!(err("../../outside/x", true).contains("without .."));
        assert!(err(outside.join("x").to_str().unwrap(), true).contains("without .."));
        assert!(err("out/x", true).contains("outside the notes"), "a symlink out is refused");
        assert!(err("out/new/x", true).contains("outside the notes"));
        assert!(err("new/x", false).contains(":mkdirp"));
        assert!(err("", false).contains("relative"));
        assert_eq!(std::fs::read_dir(&outside).unwrap().count(), 0);

        // New files: nothing would change, so no confirm; :mkdirp creates folders, :shebang makes it executable.
        let files = [f("bin/run.sh", "#!/bin/sh\necho\n", true, true), f("a.conf", "a\n", false, false)];
        assert_eq!(tangle(&notes, &note, &files, false).unwrap(), Vec::<String>::new());
        assert!(!notes.join("sub/bin").exists(), "a dry run writes nothing");
        tangle(&notes, &note, &files, true).unwrap();
        assert_eq!(std::fs::read_to_string(notes.join("sub/bin/run.sh")).unwrap(), "#!/bin/sh\necho\n");
        assert_eq!(std::fs::metadata(notes.join("sub/bin/run.sh")).unwrap().permissions().mode() & 0o777, 0o755);
        // Same content again: nothing to confirm or back up. Changed content: listed, and the old one kept as .bak.
        assert!(tangle(&notes, &note, &files, true).unwrap().is_empty());
        assert!(!notes.join("sub/a.conf.bak").exists());
        let changed = [f("a.conf", "b\n", false, false)];
        assert_eq!(tangle(&notes, &note, &changed, false).unwrap(), ["a.conf"]);
        assert_eq!(std::fs::read_to_string(notes.join("sub/a.conf")).unwrap(), "a\n");
        tangle(&notes, &note, &changed, true).unwrap();
        assert_eq!(std::fs::read_to_string(notes.join("sub/a.conf")).unwrap(), "b\n");
        assert_eq!(std::fs::read_to_string(notes.join("sub/a.conf.bak")).unwrap(), "a\n");
        std::fs::remove_dir_all(&root).unwrap();
    }
}
