// One-shot commands for scripts, launchers and status bars (#1354): `margin --add`, `--journal`,
// `--clock`, `--clock-out` and `--status` load config.toml like `--mcp` (no window, no running app
// needed), print one line and exit; errors go to stderr with exit code 1. Writes take the same locks
// and content checks as the MCP server, and a running app picks them up through its file watcher.

use crate::config::Config;
use crate::{kw_of, mcp, org, read_or_empty, saved_profile, tc_of, timeclock, write_checked};
use std::path::Path;

#[derive(Debug, PartialEq)]
pub enum Cmd {
    Add(String),
    Journal(String),
    Clock(String, String),
    ClockOut(String),
    Status,
}

const USAGE: &str = "usage: margin --add <task> | --journal <text> | --clock <project> [task] | --clock-out [note] | --status";

/// The command in ARGS (without the program name), None when there's none; Err for a missing argument.
/// Text arguments are the rest of ARGS joined by spaces, so quoting is optional.
pub fn parse(args: &[String]) -> Option<Result<Cmd, String>> {
    let i = args.iter().position(|a| matches!(a.as_str(), "--add" | "--journal" | "--clock" | "--clock-out" | "--status"))?;
    let rest = |from: usize| args.get(from..).unwrap_or_default().join(" ").trim().to_string();
    let need = |s: String| if s.is_empty() { Err(USAGE.to_string()) } else { Ok(s) };
    Some(match args[i].as_str() {
        "--add" => need(rest(i + 1)).map(Cmd::Add),
        "--journal" => need(rest(i + 1)).map(Cmd::Journal),
        "--clock" => need(args.get(i + 1).map(|p| p.trim().to_string()).unwrap_or_default()).map(|p| Cmd::Clock(p, rest(i + 2))),
        "--clock-out" => Ok(Cmd::ClockOut(rest(i + 1))),
        _ => Ok(Cmd::Status),
    })
}

/// HOURS as `1h12m`, or `12m` under an hour.
fn hm(hours: f64) -> String {
    let m = (hours * 60.0).round().max(0.0) as i64;
    if m < 60 { format!("{m}m") } else { format!("{}h{:02}m", m / 60, m % 60) }
}

/// The status-bar line: `Acme: Fix login 1h12m (today 3h40m)`, `Acme: on break (today …)`, or "" when not clocked in.
fn status_line(cur: Option<(chrono::NaiveDateTime, String, String)>, on_break: Option<String>, today: f64, now: chrono::NaiveDateTime) -> String {
    match (cur, on_break) {
        (Some((since, project, task)), _) => {
            let task = if task.is_empty() { String::new() } else { format!(": {task}") };
            format!("{project}{task} {} (today {})", hm((now - since).num_seconds() as f64 / 3600.0), hm(today))
        }
        (None, Some(project)) => format!("{project}: on break (today {})", hm(today)),
        (None, None) => String::new(),
    }
}

/// Run CMD against config C; the line to print.
pub fn exec(c: &Config, cmd: Cmd) -> Result<String, String> {
    if !matches!(cmd, Cmd::Status) && !c.cli_write {
        return Err("writing from the command line is off (cli_write = false in config.toml)".into());
    }
    let root = c.notes();
    let busy = |e: String| if e == "changed-on-disk" { "the file changed while writing: try again".into() } else { e };
    match cmd {
        Cmd::Status => {
            let (cur, on_break, today) = tc_of(c, &saved_profile(c)).status();
            Ok(status_line(cur, on_break, today, timeclock::now()))
        }
        Cmd::Clock(project, task) => {
            let projects = tc_of(c, &saved_profile(c)).load_projects()?;
            let Some(name) = projects.keys().find(|k| k.eq_ignore_ascii_case(&project)) else {
                let known = if projects.is_empty() { "none yet: add one in Margin".into() } else { format!("known: {}", projects.keys().cloned().collect::<Vec<_>>().join(", ")) };
                return Err(format!("unknown project {project} ({known})"));
            };
            mcp::clock_in(c, name, &task)
        }
        Cmd::ClockOut(note) => mcp::clock_out(c, &note),
        Cmd::Journal(text) => {
            let (p, day) = mcp::journal_path(c, &root, None)?;
            mcp::append_day(c, &p, day, None, &text).map_err(busy)?;
            Ok(format!("Added to {}", p.display()))
        }
        Cmd::Add(task) => {
            let p = root.join(&c.inbox);
            let kw = kw_of(c);
            let read = read_or_empty(&p)?.replace("\r\n", "\n").replace('\r', "\n");
            let line = format!("* {} {task}", kw.todo.first().map_or("TODO", |s| s));
            write_checked(&p, org::capture_insert(&read, None, &line, &kw), Some(&read)).map_err(busy)?;
            Ok(format!("Added \"{line}\" to {}", p.display()))
        }
    }
}

/// config.toml at PATH; the defaults when it doesn't exist yet (the app hasn't run), an error when unreadable.
fn load(path: &Path) -> Result<Config, String> {
    match path.try_exists() {
        Ok(false) => Ok(Config::default()),
        _ => Config::load(path),
    }
}

/// Run the one-shot command in ARGS (the process's, program name first), printing its line; the exit
/// code, or None when ARGS hold none (start the app).
pub fn cli(args: &[String]) -> Option<i32> {
    let cmd = parse(args.get(1..).unwrap_or_default())?;
    let run = || exec(&load(&mcp::config_path().ok_or("can't find the config folder")?)?, cmd?);
    Some(match run() {
        Ok(line) => {
            if !line.is_empty() {
                println!("{line}");
            }
            0
        }
        Err(e) => {
            eprintln!("margin: {e}");
            1
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &str) -> Vec<String> {
        s.split(' ').filter(|a| !a.is_empty()).map(String::from).collect()
    }

    #[test]
    fn parses() {
        assert_eq!(parse(&args("")), None);
        assert_eq!(parse(&args("--capture")), None);
        assert_eq!(parse(&args("--add Buy milk")), Some(Ok(Cmd::Add("Buy milk".into()))));
        assert_eq!(parse(&["--journal".into(), " Wrote code. ".into()]), Some(Ok(Cmd::Journal("Wrote code.".into()))));
        assert_eq!(parse(&args("--clock Acme Fix login")), Some(Ok(Cmd::Clock("Acme".into(), "Fix login".into()))));
        assert_eq!(parse(&args("--clock Acme")), Some(Ok(Cmd::Clock("Acme".into(), "".into()))));
        assert_eq!(parse(&args("--clock-out")), Some(Ok(Cmd::ClockOut("".into()))));
        assert_eq!(parse(&args("--status")), Some(Ok(Cmd::Status)));
        for a in ["--add", "--journal  ", "--clock"] {
            assert_eq!(parse(&args(a)), Some(Err(USAGE.into())), "{a}");
        }
    }

    #[test]
    fn status_lines() {
        let t = |s: &str| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap();
        let now = t("2026-11-14 10:12");
        assert_eq!(status_line(Some((t("2026-11-14 09:00"), "Acme".into(), "Fix login".into())), None, 3.0 + 40.0 / 60.0, now), "Acme: Fix login 1h12m (today 3h40m)");
        assert_eq!(status_line(Some((t("2026-11-14 10:07"), "Acme".into(), "".into())), None, 0.083, now), "Acme 5m (today 5m)");
        assert_eq!(status_line(None, Some("Acme".into()), 2.0, now), "Acme: on break (today 2h00m)");
        assert_eq!(status_line(None, None, 2.0, now), "");
    }

    #[test]
    fn runs_commands() {
        let dir = std::env::temp_dir().join(format!("margin-cli-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut c = Config { notes_dir: dir.join("notes").to_string_lossy().into(), data_dir: dir.join("data").to_string_lossy().into(), ..Default::default() };
        std::fs::create_dir_all(c.notes()).unwrap();
        // The inbox is created on the first add, appended to after.
        assert!(exec(&c, Cmd::Add("Buy milk".into())).unwrap().starts_with("Added \"* TODO Buy milk\""));
        exec(&c, Cmd::Add("Call Bob".into())).unwrap();
        let inbox = std::fs::read_to_string(c.notes().join(&c.inbox)).unwrap();
        assert!(inbox.contains("* TODO Buy milk\n") && inbox.contains("* TODO Call Bob"), "{inbox}");
        exec(&c, Cmd::Journal("Coffee.".into())).unwrap();
        let (day, _) = mcp::journal_path(&c, &c.notes(), None).unwrap();
        assert!(std::fs::read_to_string(day).unwrap().contains("Coffee."));
        // Clocking needs a known project, matched in any case.
        assert_eq!(exec(&c, Cmd::Status), Ok(String::new()));
        assert!(exec(&c, Cmd::Clock("Nope".into(), "".into())).unwrap_err().contains("unknown project Nope"));
        mcp::clock_in(&c, "Acme", "").unwrap();
        mcp::clock_out(&c, "").unwrap();
        assert_eq!(exec(&c, Cmd::Clock("acme".into(), "Fix login".into())), Ok("⏱ Clocked in on: Acme".into()));
        assert!(exec(&c, Cmd::Status).unwrap().starts_with("Acme: Fix login 0m (today "));
        assert_eq!(exec(&c, Cmd::ClockOut("Done".into())), Ok("⏱ Clocked out. Task: Done".into()));
        // cli_write off: writes refused, status still works.
        c.cli_write = false;
        assert!(exec(&c, Cmd::Add("x".into())).unwrap_err().contains("cli_write"));
        assert!(exec(&c, Cmd::ClockOut("".into())).is_err());
        assert_eq!(exec(&c, Cmd::Status), Ok(String::new()));
        // An unreadable (non-UTF-8) inbox is an error, never wiped.
        c.cli_write = true;
        std::fs::write(c.notes().join("bad.org"), [0xE5, 0x0A]).unwrap();
        c.inbox = "bad.org".into();
        assert!(exec(&c, Cmd::Add("x".into())).is_err());
        assert_eq!(std::fs::read(c.notes().join("bad.org")).unwrap(), [0xE5, 0x0A]);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
