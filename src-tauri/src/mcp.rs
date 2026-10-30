// Read-only MCP server over stdio (`margin --mcp`, #929): Claude Desktop / Claude Code start it and
// ask it to search and read notes, tasks and time. JSON-RPC 2.0, one message per line; stdout carries
// only responses, anything else goes to stderr.

use crate::config::Config;
use crate::{check_in, files_of, kw_of, notes, org, range_report, retrieve, saved_profile, tc_of, week_range};
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
            "instructions": "Read-only access to the user's Margin notes (org files), tasks and time log.",
        }),
        "ping" => json!({}),
        "tools/list" => json!({ "tools": tools() }),
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

fn tools() -> Value {
    let obj = |props: Value, req: &[&str]| json!({ "type": "object", "properties": props, "required": req });
    let s = |d: &str| json!({ "type": "string", "description": d });
    let note = obj(json!({ "path": s("Path relative to the notes folder, as listed by the other tools"), "title": s("Note title (when no path is given)") }), &[]);
    json!([
        { "name": "search_notes", "description": "Lines of the notes containing every word of QUERY (any case; -word excludes, in:folder narrows).",
          "inputSchema": obj(json!({ "query": s("Search words"), "limit": { "type": "integer", "description": "Max hits (default 20, max 200)" } }), &["query"]) },
        { "name": "read_note", "description": "Full text of one note (org format), by path or title. Long notes are cut at 100 KB.", "inputSchema": note },
        { "name": "list_notes", "description": "Every note: path, title and tags.", "inputSchema": obj(json!({}), &[]) },
        { "name": "list_tasks", "description": "Open tasks (with STATE: tasks in that state, done ones too), most urgent first.",
          "inputSchema": obj(json!({ "state": s("Task keyword, e.g. TODO, NEXT, DONE"), "project": s("Category, file tag or tag the task must have") }), &[]) },
        { "name": "time_report", "description": "Clocked time per day and project over FROM..TO (default this week), as an org report.",
          "inputSchema": obj(json!({ "from": s("Start date, YYYY-MM-DD"), "to": s("End date, YYYY-MM-DD (default FROM's week end)") }), &[]) },
        { "name": "list_projects", "description": "Time-tracking projects and their settings (active, budget).", "inputSchema": obj(json!({}), &[]) },
        { "name": "neighbors", "description": "Notes linked to or from one note, and the lines that link to it.", "inputSchema": note },
    ])
}

/// Run tool NAME with ARGS; the text result or the error message.
fn call(ctx: &Ctx, name: &str, args: &Value) -> Result<String, String> {
    let c = Config::load(&ctx.cfg_path).unwrap_or_else(|e| {
        eprintln!("margin --mcp: {e}; using defaults");
        Config::default()
    });
    let root = c.notes();
    let arg = |k: &str| args.get(k).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty());
    let rel = |p: &Path| p.strip_prefix(&root).unwrap_or(p).to_string_lossy().replace('\\', "/");
    let notes_tool = matches!(name, "search_notes" | "read_note" | "list_notes" | "list_tasks" | "neighbors");
    if notes_tool && !root.is_dir() {
        return Err(format!("the notes folder {} doesn't exist", root.display()));
    }
    let all = if notes_tool { files_of(&c, &ctx.cache) } else { vec![] };
    let excluded = |p: &Path| retrieve::excluded(p, &root, &c.ai_exclude);
    let files: Vec<Arc<org::OrgFile>> = all.iter().filter(|f| !excluded(&f.path)).cloned().collect();
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
            let mut text = f.text.clone();
            if text.len() > MAX_NOTE {
                let cut = (0..=MAX_NOTE).rev().find(|&i| text.is_char_boundary(i)).unwrap_or(0);
                let len = text.len();
                text.truncate(cut);
                text.push_str(&format!("\n\n[truncated: first {cut} of {len} bytes]"));
            }
            return Ok(format!("# {} ({})\n\n{text}", f.title, rel(&f.path)));
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
            return Ok(range_report(&tc_of(&c, &saved_profile(&c)), &saved_profile(&c), from, to));
        }
        "list_projects" => json!(tc_of(&c, &saved_profile(&c)).load_projects()?),
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
        assert_eq!(names, ["search_notes", "read_note", "list_notes", "list_tasks", "time_report", "list_projects", "neighbors"]);
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
        std::fs::remove_dir_all(d.join("notes")).unwrap();
        let (e, err) = tool(&ctx, "list_notes", json!({}));
        assert!(err && e.contains("doesn't exist"), "{e}");
    }
}
