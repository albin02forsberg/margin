// Drafts of diary and journal notes from a small local model, through Ollama's HTTP API
// (https://ollama.com). Nothing is sent unless `ai_model` is set, and only to localhost.

use crate::activity::{request, Suggestion};
use crate::timeclock::Session;
use chrono::NaiveDate;
use regex::Regex;

/// Prompts stay under this many bytes; later lines are dropped.
pub const MAX_PROMPT: usize = 4000;
/// Seconds to wait for a draft; a 3B model on a laptop CPU takes a few.
const WAIT: u64 = 60;

/// HEAD followed by LINES, as many as fit in MAX_PROMPT.
fn capped(head: &str, lines: impl IntoIterator<Item = String>) -> String {
    let mut p = head.to_string();
    for l in lines {
        if p.len() + l.len() + 1 > MAX_PROMPT {
            break;
        }
        p.push('\n');
        p.push_str(&l);
    }
    p
}

fn kept<'a>(xs: &'a [String], exclude: &'a [Regex]) -> impl Iterator<Item = &'a String> {
    xs.iter().filter(|x| !x.trim().is_empty() && !exclude.iter().any(|r| r.is_match(x)))
}

/// Prompt for a one-line diary note about a block of work: its PROJECT, APPS and TITLES (minus EXCLUDEd ones).
pub fn note_prompt(project: Option<&str>, apps: &[String], titles: &[String], exclude: &[Regex]) -> String {
    let head = "Write one short line (under 15 words, past tense, no quotes) for a work diary saying what I worked on. Use only these facts.";
    let apps: Vec<_> = kept(apps, exclude).map(String::as_str).collect();
    let facts = project.filter(|p| !p.trim().is_empty()).map(|p| format!("Project: {p}")).into_iter().chain([format!("Apps: {}", apps.join(", ")), "Window titles:".into()]);
    capped(head, facts.chain(kept(titles, exclude).map(|t| format!("- {t}"))))
}

/// Prompt for a short summary of DAY: its logged SESSIONS, then the top titles of each activity BLOCK.
pub fn day_prompt(day: NaiveDate, sessions: &[Session], blocks: &[Suggestion], exclude: &[Regex]) -> String {
    let head = format!("Summarize my workday {day} in 3 to 5 short bullet points for my journal. Use only these facts; don't invent anything.\nLogged time:");
    let logged = sessions.iter().map(|s| {
        let what = [s.project.trim(), s.desc.trim()].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join(": ");
        format!("- {}–{} {} ({:.1} h)", s.start.format("%H:%M"), s.end.format("%H:%M"), if what.is_empty() { "Other" } else { &what }, s.hours)
    });
    let seen = blocks.iter().map(|b| format!("- {}–{}: {}", b.start.format("%H:%M"), b.end.format("%H:%M"), kept(&b.titles, exclude).map(String::as_str).collect::<Vec<_>>().join("; ")));
    let none = sessions.is_empty().then(|| "- nothing".to_string());
    capped(&head, logged.chain(none).chain((!blocks.is_empty()).then(|| "Window titles on screen:".to_string())).chain(seen))
}

/// The JSON for `POST /api/generate`.
pub fn body(model: &str, prompt: &str) -> String {
    serde_json::json!({ "model": model, "prompt": prompt, "stream": false }).to_string()
}

/// Ollama's answer (HTTP STATUS, BODY) → the draft, or a friendly error.
pub fn parse(status: u16, body: &str, model: &str) -> Result<String, String> {
    let v: serde_json::Value = serde_json::from_str(body).unwrap_or_default();
    let err = v["error"].as_str().unwrap_or("").to_string();
    match status {
        200 => v["response"].as_str().map(String::from).ok_or_else(|| "Ollama sent no draft".into()),
        _ if status == 404 || err.contains("not found") => Err(format!("Model {model} isn't installed. Run `ollama pull {model}`.")),
        _ => Err(format!("Ollama: {}", if err.is_empty() { format!("HTTP {status}") } else { err })),
    }
}

/// A model's raw TEXT → org text: no reasoning, `*` bullets as `-` so they don't become
/// headings; ONE_LINE keeps the first line without bullet or quotes.
pub fn clean(text: &str, one_line: bool) -> String {
    let text = Regex::new(r"(?s)<think>.*?</think>").unwrap().replace_all(text, "");
    if one_line {
        let l = text.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
        return l.trim_start_matches(['-', '*', ' ']).trim_matches(['"', '“', '”', ' ']).to_string();
    }
    let bullet = Regex::new(r"(?m)^\s*\*+ ").unwrap();
    bullet.replace_all(text.trim(), "- ").into_owned()
}

/// A draft for PROMPT from MODEL at URL.
pub fn generate(url: &str, model: &str, prompt: &str, one_line: bool) -> Result<String, String> {
    let (status, resp) = request("Ollama", "ai_url", url, "POST", "/api/generate", Some(&body(model, prompt)), WAIT).map_err(|e| {
        if e.contains("isn't reachable") { format!("Ollama isn't running at {url}. Start it, or see ollama.com.") } else { e }
    })?;
    Ok(clean(&parse(status, &resp, model)?, one_line))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDateTime, NaiveTime};
    use std::io::{Read, Write};

    fn t(s: &str) -> NaiveTime {
        NaiveTime::parse_from_str(s, "%H:%M").unwrap()
    }
    fn dt(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(&format!("2026-10-03 {s}"), "%Y-%m-%d %H:%M").unwrap()
    }
    fn v(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn prompts() {
        let ex = vec![Regex::new("(?i)keepass").unwrap()];
        let p = note_prompt(Some("Margin"), &v(&["Code", "KeePassXC"]), &v(&["lib.rs - margin", "bank.kdbx - KeePassXC", ""]), &ex);
        assert!(p.ends_with("\nProject: Margin\nApps: Code\nWindow titles:\n- lib.rs - margin"), "{p}");
        let day = NaiveDate::from_ymd_opt(2026, 10, 3).unwrap();
        let s = Session { date: day, project: "Margin".into(), desc: "AI drafts".into(), hours: 2.5, start: t("09:00"), end: t("11:30"), line: Some(3) };
        let b = Suggestion { start: dt("13:00"), end: dt("14:00"), apps: v(&["Firefox"]), titles: v(&["Ollama docs", "KeePassXC"]), project: None };
        let p = day_prompt(day, std::slice::from_ref(&s), &[b], &ex);
        assert!(p.contains("workday 2026-10-03") && p.ends_with("Logged time:\n- 09:00–11:30 Margin: AI drafts (2.5 h)\nWindow titles on screen:\n- 13:00–14:00: Ollama docs"), "{p}");
        assert!(day_prompt(day, &[], &[], &ex).ends_with("Logged time:\n- nothing"));
        let many = vec![s; 500];
        let p = day_prompt(day, &many, &[], &ex);
        assert!(p.len() <= MAX_PROMPT && p.len() > MAX_PROMPT - 100 && p.ends_with("h)"));
    }

    #[test]
    fn responses() {
        assert_eq!(parse(200, r#"{"model":"m","response":"Fixed the parser.","done":true}"#, "m").unwrap(), "Fixed the parser.");
        assert!(parse(404, r#"{"error":"model \"llama3.2:3b\" not found, try pulling it first"}"#, "llama3.2:3b").unwrap_err().contains("ollama pull llama3.2:3b"));
        assert_eq!(parse(500, r#"{"error":"out of memory"}"#, "m").unwrap_err(), "Ollama: out of memory");
        assert_eq!(parse(502, "", "m").unwrap_err(), "Ollama: HTTP 502");
        assert_eq!(clean("<think>hmm\nok</think>\n\n- \"Wired up Ollama drafts.\"\nMore", true), "Wired up Ollama drafts.");
        assert_eq!(clean("  * Fixed a bug\n** Bold claim\n**Bold** stays\n", false), "- Fixed a bug\n- Bold claim\n**Bold** stays");
    }

    #[test]
    fn http() {
        assert!(generate("http://example.com:11434", "m", "p", true).unwrap_err().contains("only localhost"));
        // Nothing listens on a port we just freed.
        let free = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap();
        assert!(generate(&format!("http://{free}"), "m", "p", true).unwrap_err().contains("isn't running"));
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", l.local_addr().unwrap());
        let srv = std::thread::spawn(move || {
            let (mut c, _) = l.accept().unwrap();
            let (mut req, mut buf) = (vec![], [0; 512]);
            while !req.ends_with(b"}") {
                let n = c.read(&mut buf).unwrap();
                req.extend_from_slice(&buf[..n]);
            }
            c.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"response\":\" Drafted it.\\n\",\"done\":true}").unwrap();
            String::from_utf8_lossy(&req).into_owned()
        });
        assert_eq!(generate(&base, "llama3.2:3b", "Say hi", true).unwrap(), "Drafted it.");
        let req = srv.join().unwrap();
        assert!(req.starts_with("POST /api/generate HTTP/1.0\r\n"), "{req}");
        assert!(req.ends_with(r#"{"model":"llama3.2:3b","prompt":"Say hi","stream":false}"#), "{req}");
    }
}
