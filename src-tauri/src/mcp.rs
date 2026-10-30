// MCP server over stdio (`margin --mcp`, #929): Claude Desktop / Claude Code start it and ask it to
// search and read notes, tasks and time, and with `mcp_write` on to create and edit notes and tasks
// (#930). JSON-RPC 2.0, one message per line; stdout carries only responses, anything else goes to stderr.

use crate::config::Config;
use crate::{capture_file, check_in, files_of, journal_file, kw_of, new_day_file, notes, org, org_edit_op, range_report, retrieve, saved_profile, tc_of, timeclock, week_range, write_checked};
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Longest note text read_note returns, in bytes.
const MAX_NOTE: usize = 100_000;

pub struct Ctx {
    pub cfg_path: PathBuf,
    cache: org::Cache,
}

impl Ctx {
    pub fn new(cfg_path: PathBuf) -> Self {
        Ctx { cfg_path, cache: Default::default() }
    }
}

/// The tools that change notes or the time log; only listed and run with `mcp_write` on.
const WRITE_TOOLS: [&str; 7] = ["create_note", "append_to_note", "add_task", "set_task_state", "append_to_journal", "clock_in", "clock_out"];

/// config.toml, or the defaults when it doesn't load.
fn load(ctx: &Ctx) -> Config {
    Config::load(&ctx.cfg_path).unwrap_or_else(|e| {
        eprintln!("margin --mcp: {e}; using defaults");
        Config::default()
    })
}

/// The OS config dir Tauri's `app_config_dir` uses, plus the app identifier (no Tauri app here).
pub fn config_path() -> Option<PathBuf> {
    let env = |k| std::env::var_os(k).map(PathBuf::from).filter(|p| p.is_absolute());
    let base = if cfg!(target_os = "macos") {
        std::env::home_dir()?.join("Library/Application Support")
    } else if cfg!(windows) {
        env("APPDATA")?
    } else {
        env("XDG_CONFIG_HOME").or_else(|| std::env::home_dir().map(|h| h.join(".config")))?
    };
    Some(base.join("dev.albin.margin").join("config.toml"))
}

/// Serve CTX on stdin/stdout until stdin closes.
pub fn serve(ctx: &Ctx) {
    let (stdin, mut out) = (std::io::stdin(), std::io::stdout());
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if let Some(resp) = handle(ctx, &line) {
            if writeln!(out, "{resp}").and_then(|_| out.flush()).is_err() {
                break;
            }
        }
    }
}

/// The response to one JSON-RPC LINE; None for notifications and blank lines.
pub fn handle(ctx: &Ctx, line: &str) -> Option<String> {
    if line.trim().is_empty() {
        return None;
    }
    let msg: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => return Some(error(Value::Null, -32700, &format!("parse error: {e}"))),
    };
    let id = msg.get("id").cloned()?; // notifications (no id) get no answer
    let params = msg.get("params").cloned().unwrap_or(json!({}));
    let result = match msg["method"].as_str().unwrap_or("") {
        "initialize" => json!({
            "protocolVersion": params["protocolVersion"].as_str().unwrap_or("2025-06-18"),
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "margin", "version": env!("CARGO_PKG_VERSION") },
            "instructions": if load(ctx).mcp_write {
                "Access to the user's Margin notes (org files), tasks and time log: read them, create notes, append to notes, add tasks, change task states, add to the journal and clock in and out."
            } else {
                "Read-only access to the user's Margin notes (org files), tasks and time log."
            },
        }),
        "ping" => json!({}),
        "tools/list" => json!({ "tools": tools(load(ctx).mcp_write) }),
        "tools/call" => {
            let (text, err) = match call(ctx, params["name"].as_str().unwrap_or(""), &params["arguments"]) {
                Ok(t) => (t, false),
                Err(e) => (e, true),
            };
            json!({ "content": [{ "type": "text", "text": text }], "isError": err })
        }
        m => return Some(error(id, -32601, &format!("method not found: {m}"))),
    };
    Some(json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string())
}

fn error(id: Value, code: i64, message: &str) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } }).to_string()
}

/// The tool list; WRITE adds WRITE_TOOLS.
fn tools(write: bool) -> Value {
    let obj = |props: Value, req: &[&str]| json!({ "type": "object", "properties": props, "required": req });
    let s = |d: &str| json!({ "type": "string", "description": d });
    let note = obj(json!({ "path": s("Path relative to the notes folder, as listed by the other tools"), "title": s("Note title (when no path is given)") }), &[]);
    let day = s("The day: YYYY-MM-DD, or relative like -1 for yesterday (default today)");
    // A write tool's arguments: the target note, EXPECT, and PROPS.
    let target = |props: Value, req: &[&str]| {
        let mut p = json!({ "path": s("Path relative to the notes folder"), "title": s("Note title (when no path is given)"),
            "expect": s("The note's text as you last read it (without read_note's title line): the edit is refused if the file changed since. Default: the text read just before the edit") });
        p.as_object_mut().unwrap().extend(props.as_object().unwrap().clone());
        obj(p, req)
    };
    let mut v = json!([
        { "name": "search_notes", "description": "Lines of the notes containing every word of QUERY (any case; -word excludes, in:folder narrows).",
          "inputSchema": obj(json!({ "query": s("Search words"), "limit": { "type": "integer", "description": "Max hits (default 20, max 200)" } }), &["query"]) },
        { "name": "read_note", "description": "Full text of one note (org format), by path or title. Long notes are cut at 100 KB.", "inputSchema": note },
        { "name": "list_notes", "description": "Every note: path, title and tags.", "inputSchema": obj(json!({}), &[]) },
        { "name": "list_tasks", "description": "Open tasks (with STATE: tasks in that state, done ones too), most urgent first.",
          "inputSchema": obj(json!({ "state": s("Task keyword, e.g. TODO, NEXT, DONE"), "project": s("Category, file tag or tag the task must have") }), &[]) },
        { "name": "time_report", "description": "Clocked time per day and project over FROM..TO (default this week), as an org report.",
          "inputSchema": obj(json!({ "from": s("Start date, YYYY-MM-DD"), "to": s("End date, YYYY-MM-DD (default FROM's week end)") }), &[]) },
        { "name": "list_projects", "description": "Time-tracking projects and their settings (active, budget).", "inputSchema": obj(json!({}), &[]) },
        { "name": "clock_status", "description": "The running clock: whether you're clocked in, since when, on which project and task, any break, and today's hours.", "inputSchema": obj(json!({}), &[]) },
        { "name": "neighbors", "description": "Notes linked to or from one note, and the lines that link to it.", "inputSchema": note },
        { "name": "read_journal", "description": "One day's journal entry (the daily folder's day file), by default today's.", "inputSchema": obj(json!({ "date": day.clone() }), &[]) },
    ]);
    if write {
        let w = v.as_array_mut().unwrap();
        w.push(json!({ "name": "create_note", "description": "Create a new note (an org-roam file with an ID) titled TITLE holding BODY (org text).",
            "inputSchema": obj(json!({ "title": s("Note title"), "body": s("Org text below the title"), "folder": s("Subfolder of the notes folder (default: the notes folder)") }), &["title"]) }));
        w.push(json!({ "name": "append_to_note", "description": "Append TEXT (org) to a note: at its end, or as the last child of the first heading titled HEADING (created if missing).",
            "inputSchema": target(json!({ "text": s("Org text to append"), "heading": s("Heading title to append under") }), &["text"]) }));
        w.push(json!({ "name": "add_task", "description": "Add a task heading to a note: at its end, or under the first heading titled HEADING (created if missing).",
            "inputSchema": target(json!({ "task": s("The task's title"), "state": s("Task keyword (default the first, e.g. TODO)"), "heading": s("Heading title to add it under") }), &["task"]) }));
        w.push(json!({ "name": "set_task_state", "description": "Change one heading's task keyword, e.g. to DONE, by its title or its line (as list_tasks gives it); stamps CLOSED like the app.",
            "inputSchema": target(json!({ "state": s("New task keyword, e.g. DONE"), "heading": s("The heading's title (must match exactly one, any case)"), "line": { "type": "integer", "description": "1-based line of the heading (instead of heading)" } }), &["state"]) }));
        w.push(json!({ "name": "append_to_journal", "description": "Append TEXT (org) to a day's journal entry (created with the journal template if missing): at its end, or under the first heading titled HEADING (created if missing).",
            "inputSchema": obj(json!({ "text": s("Org text to append"), "date": day, "heading": s("Heading title to append under") }), &["text"]) }));
        w.push(json!({ "name": "clock_in", "description": "Clock in on PROJECT (clocking out of a running clock first), in the active profile's time log.",
            "inputSchema": obj(json!({ "project": s("Project name (new ones are added)"), "task": s("What you're working on") }), &["project"]) }));
        w.push(json!({ "name": "clock_out", "description": "Clock out of the running clock, with an optional NOTE on what was done.",
            "inputSchema": obj(json!({ "note": s("What was done") }), &[]) }));
    }
    v
}

/// Run tool NAME with ARGS; the text result or the error message.
fn call(ctx: &Ctx, name: &str, args: &Value) -> Result<String, String> {
    let c = load(ctx);
    let write = WRITE_TOOLS.contains(&name);
    if write && !c.mcp_write {
        return Err("editing is off: turn on “Let Claude (MCP) edit notes” in Margin's settings (mcp_write = true)".into());
    }
    let arg = |k: &str| args.get(k).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty());
    // The app's tc_in / tc_out under the same locks, minus its backup (the app's next one takes it).
    let tc = || tc_of(&c, &saved_profile(&c));
    match name {
        "clock_in" => {
            let project = arg("project").ok_or("project is required")?;
            tc().locked(|tc| tc.clock_in(project, arg("task").unwrap_or(""), None))?;
            return Ok(format!("⏱ Clocked in on: {project}"));
        }
        "clock_out" => {
            let note = arg("note").unwrap_or("");
            return Ok(match tc().locked(|tc| tc.clock_out(note))? {
                false => "Not clocked in.".into(),
                true if note.is_empty() => "⏱ Clocked out.".into(),
                true => format!("⏱ Clocked out. Task: {note}"),
            });
        }
        _ => {}
    }
    let root = c.notes();
    let rel = |p: &Path| p.strip_prefix(&root).unwrap_or(p).to_string_lossy().replace('\\', "/");
    let notes_tool = write || matches!(name, "search_notes" | "read_note" | "list_notes" | "list_tasks" | "neighbors" | "read_journal");
    if notes_tool && !root.is_dir() {
        return Err(format!("the notes folder {} doesn't exist", root.display()));
    }
    let all = if notes_tool { files_of(&c, &ctx.cache) } else { vec![] };
    let excluded = |p: &Path| retrieve::excluded(p, &root, &c.ai_exclude);
    let files: Vec<Arc<org::OrgFile>> = all.iter().filter(|f| !excluded(&f.path)).cloned().collect();
    if write {
        return edit(&c, &root, &all, name, args).map_err(|e| if e == "changed-on-disk" { "the note changed since it was read: read it again and retry".into() } else { e });
    }
    let out = match name {
        "search_notes" => {
            let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(20).clamp(1, 200) as usize;
            let hits = notes::search(&files, &root, arg("query").ok_or("query is required")?, limit);
            json!(hits.iter().map(|h| json!({ "path": rel(&h.path), "title": h.title, "line": h.line + 1, "text": h.text })).collect::<Vec<_>>())
        }
        "read_note" => {
            let f = find(&all, &root, arg("path"), arg("title"))?;
            if excluded(&f.path) {
                return Err(format!("{} is excluded from the AI (ai_exclude)", rel(&f.path)));
            }
            return Ok(format!("# {} ({})\n\n{}", f.title, rel(&f.path), cut(f.text.clone())));
        }
        "read_journal" => {
            let (p, day) = journal_path(&c, &root, arg("date"))?;
            if excluded(&p) {
                return Err(format!("{} is excluded from the AI (ai_exclude)", rel(&p)));
            }
            return match std::fs::read_to_string(&p) {
                Ok(text) => Ok(format!("# Journal {day} ({})\n\n{}", rel(&p), cut(text))),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(format!("No journal entry for {day} ({} doesn't exist)", rel(&p))),
                Err(e) => Err(e.to_string()),
            };
        }
        "list_notes" => json!(files.iter().map(|f| json!({ "path": rel(&f.path), "title": f.title, "tags": f.tags })).collect::<Vec<_>>()),
        "list_tasks" => {
            let kw = kw_of(&c);
            let state = arg("state").map(str::to_uppercase);
            let project = arg("project").map(str::to_lowercase);
            let done = state.as_deref().is_some_and(|s| kw.is_done(s));
            let items = org::todos(&files, &kw, done, |_, h| state.as_ref().is_none_or(|s| h.keyword.as_deref().is_some_and(|k| k.eq_ignore_ascii_case(s))), crate::today());
            let has = |i: &org::Item| project.as_ref().is_none_or(|p| std::iter::once(&i.category).chain(&i.tags).chain(&i.inherited).any(|t| t.to_lowercase() == *p));
            json!(items
                .iter()
                .filter(|i| has(i))
                .map(|i| json!({ "state": i.keyword, "priority": i.priority, "title": i.title, "tags": i.tags, "category": i.category, "date": (i.date != chrono::NaiveDate::MAX).then_some(i.date), "path": rel(&i.path), "line": i.line + 1 }))
                .collect::<Vec<_>>())
        }
        "time_report" => {
            let from = arg("from").map(crate::date).transpose()?;
            let (from, to) = match (from, arg("to").map(crate::date).transpose()?) {
                (Some(f), Some(t)) => (f, t),
                (f, _) => week_range(f, crate::today()),
            };
            if to < from {
                return Err(format!("to ({to}) is before from ({from})"));
            }
            return Ok(range_report(&tc(), &saved_profile(&c), from, to));
        }
        "list_projects" => json!(tc().load_projects()?),
        "clock_status" => {
            let profile = saved_profile(&c);
            let (cur, on_break, today) = tc_of(&c, &profile).status();
            json!({ "profile": profile, "clocked_in": cur.is_some(), "since": cur.as_ref().map(|c| c.0), "project": cur.as_ref().map(|c| &c.1),
                "task": cur.as_ref().map(|c| &c.2).filter(|t| !t.is_empty()), "on_break": on_break, "today_hours": (today * 100.0).round() / 100.0 })
        }
        "neighbors" => {
            let f = find(&files, &root, arg("path"), arg("title"))?;
            let linked: Vec<_> = notes::graph(&files, &f.path, 1).nodes.into_iter().filter(|n| n.hops == 1).map(|n| json!({ "path": rel(&n.path), "title": n.title })).collect();
            let back: Vec<_> = notes::backlinks(&files, &f.path).into_iter().map(|h| json!({ "path": rel(&h.path), "title": h.title, "line": h.line + 1, "text": h.text })).collect();
            json!({ "linked": linked, "backlinks": back })
        }
        n => return Err(format!("unknown tool {n}")),
    };
    Ok(serde_json::to_string_pretty(&out).unwrap())
}

/// Run write tool NAME (one of WRITE_TOOLS) with ARGS on the notes ALL under ROOT; what changed.
fn edit(c: &Config, root: &Path, all: &[Arc<org::OrgFile>], name: &str, args: &Value) -> Result<String, String> {
    let arg = |k: &str| args.get(k).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty());
    let rel = |p: &Path| p.strip_prefix(root).unwrap_or(p).to_string_lossy().replace('\\', "/");
    let excluded = |p: &Path| retrieve::excluded(p, root, &c.ai_exclude);
    let kw = kw_of(c);
    if name == "create_note" {
        let title = arg("title").ok_or("title is required")?;
        let dir = root.join(arg("folder").unwrap_or(""));
        check_in(&[root.to_path_buf()], &dir).map_err(|_| format!("{} isn't in the notes folder", dir.display()))?;
        if excluded(&dir.join("_")) || c.templates_path().is_some_and(|t| dir.starts_with(t)) {
            return Err(format!("{} is excluded from the AI (ai_exclude) or the templates folder", rel(&dir)));
        }
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let (p, id) = notes::new_note(&dir, title, args.get("body").and_then(Value::as_str).unwrap_or("")).map_err(|e| e.to_string())?;
        return Ok(format!("Created {} (:ID: {id})", rel(&p)));
    }
    if name == "append_to_journal" {
        let text = args.get("text").and_then(Value::as_str).filter(|t| !t.trim().is_empty()).ok_or("text is required")?;
        let (p, day) = journal_path(c, root, arg("date"))?;
        if excluded(&p) {
            return Err(format!("{} is excluded from the AI (ai_exclude)", rel(&p)));
        }
        // A new day file isn't in the scan cache yet, so this works on the path itself, not find().
        let created = !p.exists();
        if created {
            new_day_file(&p, &c.journal_template, day)?;
        }
        let read = std::fs::read_to_string(&p).map_err(|e| e.to_string())?.replace("\r\n", "\n").replace('\r', "\n");
        write_checked(&p, org::capture_insert(&read, arg("heading"), text, &kw), Some(&read))?;
        return Ok(format!("{} {}", if created { "Created and appended to" } else { "Appended to" }, rel(&p)));
    }
    let f = find(all, root, arg("path"), arg("title"))?;
    if excluded(&f.path) {
        return Err(format!("{} is excluded from the AI (ai_exclude)", rel(&f.path)));
    }
    // Edit the file as it is on disk now, not the scan cache's copy.
    let read = std::fs::read_to_string(&f.path).map_err(|e| e.to_string())?.replace("\r\n", "\n").replace('\r', "\n");
    let keywords = || kw.todo.iter().chain(&kw.done);
    let state = || -> Result<Option<String>, String> {
        let Some(s) = arg("state").map(str::to_uppercase) else { return Ok(None) };
        match keywords().any(|k| *k == s) {
            true => Ok(Some(s)),
            false => Err(format!("{s} isn't a task keyword ({})", keywords().cloned().collect::<Vec<_>>().join(", "))),
        }
    };
    let (new, done) = match name {
        "append_to_note" => {
            let text = args.get("text").and_then(Value::as_str).filter(|t| !t.trim().is_empty()).ok_or("text is required")?;
            (org::capture_insert(&read, arg("heading"), text, &kw), format!("Appended to {}", rel(&f.path)))
        }
        "add_task" => {
            let task = arg("task").ok_or("task is required")?;
            let st = state()?.or_else(|| kw.todo.first().cloned()).unwrap_or_else(|| "TODO".into());
            (org::capture_insert(&read, arg("heading"), &format!("* {st} {task}"), &kw), format!("Added \"{st} {task}\" to {}", rel(&f.path)))
        }
        "set_task_state" => {
            let st = state()?.ok_or("state is required")?;
            let line = heading_line(&read, &f.path, &kw, arg("heading"), args.get("line").and_then(Value::as_u64))?;
            let new = org_edit_op(&read, line, "keyword", Some(st), &kw, timeclock::now(), c.log_done)?;
            let now = new.lines().nth(line).unwrap_or("").to_string();
            (new, format!("{} line {}: {now}", rel(&f.path), line + 1))
        }
        n => return Err(format!("unknown tool {n}")),
    };
    // NOTE_WRITES only serialises writers in one process, so between this server and a running app the
    // content check is the guard: the save fails if the file no longer holds what the caller read (and
    // the app's autosave in turn sees this write as changed-on-disk). What's left is the microseconds
    // between write_checked's re-read and write_atomic's rename, where the last rename wins.
    write_checked(&f.path, new, Some(args.get("expect").and_then(Value::as_str).unwrap_or(&read)))?;
    Ok(done)
}

/// The journal's day file under ROOT for DATE (default today), checked to lie inside ROOT, and its day.
fn journal_path(c: &Config, root: &Path, date: Option<&str>) -> Result<(PathBuf, chrono::NaiveDate), String> {
    let now = timeclock::now();
    let day = date.map(|d| crate::date_at(d, now.date())).transpose()?.unwrap_or(now.date());
    let p = capture_file(root, &journal_file(&c.daily_dir), None, day.and_time(now.time()))?;
    check_in(&[root.to_path_buf()], &p).map_err(|_| format!("{} isn't in the notes folder", p.display()))?;
    Ok((p, day))
}

/// TEXT cut at MAX_NOTE bytes, with a note saying so.
fn cut(mut text: String) -> String {
    if text.len() > MAX_NOTE {
        let cut = (0..=MAX_NOTE).rev().find(|&i| text.is_char_boundary(i)).unwrap_or(0);
        let len = text.len();
        text.truncate(cut);
        text.push_str(&format!("\n\n[truncated: first {cut} of {len} bytes]"));
    }
    text
}

/// The 0-based line of the heading in TEXT at LINE (1-based), else of the one titled HEADING (any
/// case); an error listing the candidates when several match.
fn heading_line(text: &str, path: &Path, kw: &org::Kw, heading: Option<&str>, line: Option<u64>) -> Result<usize, String> {
    let f = org::parse(path, text, kw);
    if let Some(n) = line {
        return f.headlines.iter().find(|h| h.line as u64 + 1 == n).map(|h| h.line).ok_or_else(|| format!("line {n} isn't a heading"));
    }
    let t = heading.ok_or("give a heading or a line")?;
    let hits: Vec<_> = f.headlines.iter().filter(|h| h.title.eq_ignore_ascii_case(t)).collect();
    match hits[..] {
        [h] => Ok(h.line),
        [] => Err(format!("no heading titled {t}")),
        _ => Err(format!("{} headings are titled {t}, at lines {}: pass line instead", hits.len(), hits.iter().map(|h| (h.line + 1).to_string()).collect::<Vec<_>>().join(", "))),
    }
}

/// The note at PATH (relative to ROOT, or absolute inside it), else the one titled TITLE (or with that alias).
fn find<'a>(files: &'a [Arc<org::OrgFile>], root: &Path, path: Option<&str>, title: Option<&str>) -> Result<&'a Arc<org::OrgFile>, String> {
    if let Some(p) = path {
        let p = root.join(p);
        check_in(&[root.to_path_buf()], &p).map_err(|_| format!("{} isn't in the notes folder", p.display()))?;
        return files.iter().find(|f| f.path == p).ok_or_else(|| format!("no note at {}", p.display()));
    }
    let t = title.ok_or("give a path or a title")?.to_lowercase();
    let named = |f: &&Arc<org::OrgFile>| f.title.to_lowercase() == t || f.aliases.iter().any(|a| a.to_lowercase() == t);
    files.iter().find(named).ok_or_else(|| format!("no note titled {t}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (PathBuf, Ctx) {
        let d = std::env::temp_dir().join(format!("margin-mcp-{}", notes::gen_id()));
        let n = d.join("notes");
        std::fs::create_dir_all(n.join("private")).unwrap();
        std::fs::write(n.join("a.org"), ":PROPERTIES:\n:ID: aaa\n:END:\n#+title: Alpha\n\nApples and pears.\nSee [[id:bbb][Beta]].\n* TODO Buy apples :shop:\n* DONE Eat pears\n").unwrap();
        std::fs::write(n.join("b.org"), ":PROPERTIES:\n:ID: bbb\n:END:\n#+title: Beta\n\nMore apples here.\n").unwrap();
        std::fs::write(n.join("private/s.org"), "#+title: Secret\n\nApples in secret.\n* TODO Hidden task\n").unwrap();
        std::fs::write(d.join("outside.org"), "#+title: Out\n").unwrap();
        let cfg = d.join("config.toml");
        let c = Config { notes_dir: n.to_string_lossy().into(), data_dir: d.join("data").to_string_lossy().into(), ai_exclude: vec!["/private/".into()], ..Config::default() };
        std::fs::write(&cfg, toml::to_string(&c).unwrap()).unwrap();
        (d, Ctx::new(cfg))
    }

    fn rpc(ctx: &Ctx, method: &str, params: Value) -> Value {
        serde_json::from_str(&handle(ctx, &json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params }).to_string()).unwrap()).unwrap()
    }

    /// (text, isError) of tools/call NAME.
    fn tool(ctx: &Ctx, name: &str, args: Value) -> (String, bool) {
        let r = rpc(ctx, "tools/call", json!({ "name": name, "arguments": args }));
        (r["result"]["content"][0]["text"].as_str().unwrap().to_string(), r["result"]["isError"].as_bool().unwrap())
    }

    #[test]
    fn protocol() {
        let (_d, ctx) = setup();
        let r = rpc(&ctx, "initialize", json!({ "protocolVersion": "2025-03-26" }));
        assert_eq!(r["result"]["protocolVersion"], "2025-03-26");
        assert_eq!(r["result"]["serverInfo"]["name"], "margin");
        let names: Vec<_> = rpc(&ctx, "tools/list", json!({}))["result"]["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap().to_string()).collect();
        assert_eq!(names, ["search_notes", "read_note", "list_notes", "list_tasks", "time_report", "list_projects", "clock_status", "neighbors", "read_journal"]);
        assert_eq!(handle(&ctx, r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#), None);
        assert_eq!(rpc(&ctx, "nope", json!({}))["error"]["code"], -32601);
        let bad: Value = serde_json::from_str(&handle(&ctx, "{oops").unwrap()).unwrap();
        assert_eq!(bad["error"]["code"], -32700);
        assert!(tool(&ctx, "nope", json!({})).1);
    }

    #[test]
    fn notes_tools() {
        let (_d, ctx) = setup();
        let (s, err) = tool(&ctx, "search_notes", json!({ "query": "apples" }));
        assert!(!err && s.contains("a.org") && s.contains("b.org") && !s.contains("Secret"), "{s}");
        let (l, _) = tool(&ctx, "list_notes", json!({}));
        assert!(l.contains("Alpha") && !l.contains("Secret"), "{l}");
        let (r, err) = tool(&ctx, "read_note", json!({ "title": "beta" }));
        assert!(!err && r.contains("More apples here."), "{r}");
        assert!(tool(&ctx, "read_note", json!({ "path": "a.org" })).0.contains("Apples and pears."));
        // Path traversal and the excluded note are refused.
        let (t, err) = tool(&ctx, "read_note", json!({ "path": "../outside.org" }));
        assert!(err && t.contains("isn't in the notes folder"), "{t}");
        let (x, err) = tool(&ctx, "read_note", json!({ "path": "private/s.org" }));
        assert!(err && x.contains("ai_exclude"), "{x}");
        let (n, _) = tool(&ctx, "neighbors", json!({ "title": "Beta" }));
        let n: Value = serde_json::from_str(&n).unwrap();
        assert_eq!(n["linked"][0]["title"], "Alpha");
        assert_eq!(n["backlinks"][0]["path"], "a.org");
    }

    #[test]
    fn write_tools() {
        let (d, ctx) = setup();
        let n = d.join("notes");
        let listed = |ctx: &Ctx| rpc(ctx, "tools/list", json!({}))["result"]["tools"].as_array().unwrap().iter().any(|t| t["name"] == "create_note");
        // Off by default: hidden, refused, and the instructions say read-only.
        assert!(!listed(&ctx));
        let (e, err) = tool(&ctx, "create_note", json!({ "title": "New" }));
        assert!(err && e.contains("mcp_write"), "{e}");
        assert!(rpc(&ctx, "initialize", json!({}))["result"]["instructions"].as_str().unwrap().starts_with("Read-only"));
        let mut c = Config::load(&ctx.cfg_path).unwrap();
        c.mcp_write = true;
        c.log_done = true;
        std::fs::write(&ctx.cfg_path, toml::to_string(&c).unwrap()).unwrap();
        assert!(listed(&ctx));
        assert!(!rpc(&ctx, "initialize", json!({}))["result"]["instructions"].as_str().unwrap().starts_with("Read-only"));

        // create_note, inside the notes folder only, never into an excluded one.
        let (r, err) = tool(&ctx, "create_note", json!({ "title": "Fresh idea", "body": "Body text.", "folder": "ideas" }));
        assert!(!err && r.starts_with("Created ideas/") && r.contains("fresh_idea"), "{r}");
        let made = std::fs::read_dir(n.join("ideas")).unwrap().next().unwrap().unwrap().path();
        let t = std::fs::read_to_string(made).unwrap();
        assert!(t.contains("#+title: Fresh idea") && t.ends_with("Body text.\n"), "{t}");
        assert!(tool(&ctx, "create_note", json!({ "title": "X", "folder": "../out" })).0.contains("isn't in the notes folder"));
        assert!(tool(&ctx, "create_note", json!({ "title": "X", "folder": "private" })).0.contains("ai_exclude"));

        // append_to_note and add_task, under a heading (created if missing).
        let (r, err) = tool(&ctx, "append_to_note", json!({ "path": "b.org", "text": "A new line.", "heading": "Log" }));
        assert!(!err && r == "Appended to b.org", "{r}");
        let (r, err) = tool(&ctx, "add_task", json!({ "title": "Beta", "task": "Call Bob", "heading": "Log" }));
        assert!(!err && r.contains("TODO Call Bob"), "{r}");
        assert_eq!(std::fs::read_to_string(n.join("b.org")).unwrap(), ":PROPERTIES:\n:ID: bbb\n:END:\n#+title: Beta\n\nMore apples here.\n* Log\nA new line.\n** TODO Call Bob\n");
        assert!(tool(&ctx, "add_task", json!({ "path": "b.org", "task": "x", "state": "MAYBE" })).0.contains("isn't a task keyword"));

        // Traversal and excluded notes are refused, and nothing is written.
        assert!(tool(&ctx, "append_to_note", json!({ "path": "../outside.org", "text": "x" })).0.contains("isn't in the notes folder"));
        assert!(tool(&ctx, "append_to_note", json!({ "path": "private/s.org", "text": "x" })).0.contains("ai_exclude"));
        assert!(tool(&ctx, "set_task_state", json!({ "path": "private/s.org", "heading": "Hidden task", "state": "DONE" })).1);
        assert_eq!(std::fs::read_to_string(d.join("outside.org")).unwrap(), "#+title: Out\n");
        assert!(!std::fs::read_to_string(n.join("private/s.org")).unwrap().contains("DONE"));

        // set_task_state goes through org_edit_op (CLOSED with log_done); an ambiguous title lists the lines.
        let (r, err) = tool(&ctx, "set_task_state", json!({ "path": "a.org", "heading": "buy apples", "state": "done" }));
        assert!(!err && r == "a.org line 8: * DONE Buy apples :shop:", "{r}");
        assert!(std::fs::read_to_string(n.join("a.org")).unwrap().contains("* DONE Buy apples :shop:\nCLOSED: ["));
        std::fs::write(n.join("c.org"), "#+title: Gamma\n* TODO Dup\n* TODO Dup\n").unwrap();
        let (e, err) = tool(&ctx, "set_task_state", json!({ "path": "c.org", "heading": "Dup", "state": "DONE" }));
        assert!(err && e.contains("at lines 2, 3"), "{e}");
        assert!(!tool(&ctx, "set_task_state", json!({ "path": "c.org", "line": 3, "state": "DONE" })).1);
        assert_eq!(std::fs::read_to_string(n.join("c.org")).unwrap().lines().nth(1), Some("* TODO Dup"));

        // A stale EXPECT (the file changed since the caller read it) is refused.
        let (e, err) = tool(&ctx, "append_to_note", json!({ "path": "c.org", "text": "late", "expect": "#+title: Gamma\n" }));
        assert!(err && e.contains("changed since"), "{e}");
        assert!(!std::fs::read_to_string(n.join("c.org")).unwrap().contains("late"));
    }

    #[test]
    fn journal_tools() {
        let (d, ctx) = setup();
        let n = d.join("notes");
        let mut c = Config::load(&ctx.cfg_path).unwrap();
        c.journal_template = "* Morning\n".into();
        std::fs::write(&ctx.cfg_path, toml::to_string(&c).unwrap()).unwrap();
        let today = timeclock::now().date().format("%Y-%m-%d").to_string();
        let day = n.join(format!("daily/{today}.org"));
        // Read-only: a missing day is no error; appending needs mcp_write.
        let (r, err) = tool(&ctx, "read_journal", json!({}));
        assert!(!err && r.starts_with(&format!("No journal entry for {today}")), "{r}");
        let (e, err) = tool(&ctx, "append_to_journal", json!({ "text": "x" }));
        assert!(err && e.contains("mcp_write") && !day.exists(), "{e}");
        c.mcp_write = true;
        std::fs::write(&ctx.cfg_path, toml::to_string(&c).unwrap()).unwrap();
        assert!(rpc(&ctx, "tools/list", json!({}))["result"]["tools"].as_array().unwrap().iter().any(|t| t["name"] == "append_to_journal"));

        // The day file is created with the template, then appended to under a heading.
        let (r, err) = tool(&ctx, "append_to_journal", json!({ "text": "Coffee.", "heading": "Morning" }));
        assert!(!err && r == format!("Created and appended to daily/{today}.org"), "{r}");
        let (r, err) = tool(&ctx, "append_to_journal", json!({ "text": "Wrote code." }));
        assert!(!err && r == format!("Appended to daily/{today}.org"), "{r}");
        let t = std::fs::read_to_string(&day).unwrap();
        assert!(t.contains(&format!("#+title: {today}\n")) && t.ends_with("* Morning\nCoffee.\nWrote code.\n"), "{t}");
        let (r, err) = tool(&ctx, "read_journal", json!({ "date": today }));
        assert!(!err && r.starts_with(&format!("# Journal {today} (daily/{today}.org)")) && r.contains("Coffee."), "{r}");
        assert!(tool(&ctx, "read_journal", json!({ "date": "-1" })).0.starts_with("No journal entry"));
        assert!(tool(&ctx, "read_journal", json!({ "date": "not a date" })).1);

        // An excluded daily folder is refused, and no file is created; an empty daily_dir stays in the notes folder.
        c.ai_exclude = vec!["/daily/".into()];
        std::fs::write(&ctx.cfg_path, toml::to_string(&c).unwrap()).unwrap();
        assert!(tool(&ctx, "read_journal", json!({})).0.contains("ai_exclude"));
        let (e, err) = tool(&ctx, "append_to_journal", json!({ "text": "x", "date": "-1" }));
        assert!(err && e.contains("ai_exclude") && std::fs::read_dir(n.join("daily")).unwrap().count() == 1, "{e}");
        c.daily_dir = "/".into();
        std::fs::write(&ctx.cfg_path, toml::to_string(&c).unwrap()).unwrap();
        assert!(!tool(&ctx, "append_to_journal", json!({ "text": "x" })).1);
        assert!(n.join(format!("{today}.org")).exists());
    }

    #[test]
    fn clock_tools() {
        let (d, ctx) = setup();
        let log = d.join("data/work/timelog.jsonl");
        // Off by default: hidden and refused, nothing written.
        assert!(!rpc(&ctx, "tools/list", json!({}))["result"]["tools"].as_array().unwrap().iter().any(|t| t["name"] == "clock_in"));
        let (e, err) = tool(&ctx, "clock_in", json!({ "project": "Margin" }));
        assert!(err && e.contains("mcp_write") && !log.exists(), "{e}");
        let mut c = Config::load(&ctx.cfg_path).unwrap();
        c.mcp_write = true;
        std::fs::write(&ctx.cfg_path, toml::to_string(&c).unwrap()).unwrap();
        // Works without a notes folder, like the time tools.
        std::fs::remove_dir_all(d.join("notes")).unwrap();
        assert_eq!(tool(&ctx, "clock_out", json!({})), ("Not clocked in.".to_string(), false));
        assert!(tool(&ctx, "clock_in", json!({})).1);
        assert_eq!(tool(&ctx, "clock_in", json!({ "project": "Margin", "task": "MCP" })), ("⏱ Clocked in on: Margin".to_string(), false));
        assert_eq!(tc_of(&load(&ctx), "Work").current().map(|c| (c.1, c.2)), Some(("Margin".into(), "MCP".into())));
        assert_eq!(tool(&ctx, "clock_out", json!({ "note": "Wrote tools" })), ("⏱ Clocked out. Task: Wrote tools".to_string(), false));
        assert_eq!(tc_of(&load(&ctx), "Work").current(), None);
        assert!(d.join("data/work/.timelog.lock").exists());
    }

    #[test]
    fn tasks_time_and_errors() {
        let (d, ctx) = setup();
        let (t, _) = tool(&ctx, "list_tasks", json!({}));
        assert!(t.contains("Buy apples") && !t.contains("Eat pears") && !t.contains("Hidden task"), "{t}");
        assert!(tool(&ctx, "list_tasks", json!({ "state": "done" })).0.contains("Eat pears"));
        assert!(tool(&ctx, "list_tasks", json!({ "project": "SHOP" })).0.contains("Buy apples"));
        assert_eq!(tool(&ctx, "list_tasks", json!({ "project": "none" })).0, "[]");
        let (r, err) = tool(&ctx, "time_report", json!({ "from": "2026-01-05", "to": "2026-01-11" }));
        assert!(!err && r.contains("2026-01-05"), "{r}");
        assert!(tool(&ctx, "time_report", json!({ "from": "2026-01-05", "to": "2026-01-01" })).1);
        assert_eq!(tool(&ctx, "list_projects", json!({})), ("{}".to_string(), false));
        let status = |ctx: &Ctx| serde_json::from_str::<Value>(&tool(ctx, "clock_status", json!({})).0).unwrap();
        let s = status(&ctx);
        assert_eq!((&s["profile"], &s["clocked_in"], &s["project"], &s["today_hours"]), (&json!("Work"), &json!(false), &Value::Null, &json!(0.0)));
        tc_of(&load(&ctx), "Work").clock_in("Margin", "MCP", None).unwrap();
        let s = status(&ctx);
        assert_eq!((&s["clocked_in"], &s["project"], &s["task"]), (&json!(true), &json!("Margin"), &json!("MCP")), "{s}");
        assert!(s["since"].is_string(), "{s}");
        std::fs::remove_dir_all(d.join("notes")).unwrap();
        let (e, err) = tool(&ctx, "list_notes", json!({}));
        assert!(err && e.contains("doesn't exist"), "{e}");
    }
}
