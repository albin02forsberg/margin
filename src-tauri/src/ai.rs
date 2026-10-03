// Drafts of diary and journal notes from a small local model: through Ollama's HTTP API
// (https://ollama.com), or a pinned model run inside Margin (feature `embedded-ai`). Nothing is sent unless
// `ai_model` is set, and only to localhost.

use crate::activity::{request, Suggestion};
use crate::timeclock::Session;
use chrono::NaiveDate;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Seek, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// Where drafts come from (`ai_backend`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Backend {
    #[default]
    Ollama,
    /// A model from MODELS, downloaded and run by Margin itself.
    Embedded,
}

/// A model Margin can download: a GGUF file pinned to one Hugging Face commit and checksum.
#[derive(Serialize, Debug)]
pub struct Model {
    pub id: &'static str,
    pub name: &'static str,
    pub url: &'static str,
    pub size: u64,
    pub sha256: &'static str,
    /// Roughly how much free memory it needs to run, in GB.
    pub ram_gb: u8,
    pub licence: &'static str,
}

/// The models on offer; the first is the default.
pub const MODELS: &[Model] = &[
    Model {
        id: "qwen2.5-3b-instruct-q4",
        name: "Qwen2.5 3B Instruct (Q4)",
        url: "https://huggingface.co/Qwen/Qwen2.5-3B-Instruct-GGUF/resolve/7dabda4d13d513e3e842b20f0d435c732f172cbe/qwen2.5-3b-instruct-q4_k_m.gguf",
        size: 2_104_932_768,
        sha256: "626b4a6678b86442240e33df819e00132d3ba7dddfe1cdc4fbb18e0a9615c62d",
        ram_gb: 4,
        licence: "Qwen Research licence: non-commercial use only",
    },
    Model {
        id: "qwen2.5-1.5b-instruct-q4",
        name: "Qwen2.5 1.5B Instruct (Q4)",
        url: "https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/91cad51170dc346986eccefdc2dd33a9da36ead9/qwen2.5-1.5b-instruct-q4_k_m.gguf",
        size: 1_117_320_736,
        sha256: "6a1a2eb6d15622bf3c96857206351ba97e1af16c30d7a74ee38970e434e9407e",
        ram_gb: 2,
        licence: "Apache-2.0",
    },
];

impl Model {
    /// Where the model lives in DIR once downloaded and checked.
    pub fn path(&self, dir: &Path) -> PathBuf {
        dir.join(self.url.rsplit('/').next().unwrap_or(self.id))
    }
}

/// FILE with ".part" added: a download in progress.
pub fn part(file: &Path) -> PathBuf {
    let mut p = file.as_os_str().to_owned();
    p.push(".part");
    p.into()
}

fn sha256_file(p: &Path) -> std::io::Result<String> {
    let mut h = Sha256::new();
    std::io::copy(&mut std::fs::File::open(p)?, &mut h)?;
    Ok(format!("{:x}", h.finalize()))
}

/// How long a download may go without receiving a byte before it fails (resumable).
const READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(if cfg!(test) { 1 } else { 60 });

/// Download URL (SIZE bytes, checksum SHA256) to DEST through DEST.part, resuming what's there
/// with a Range request. PROGRESS gets the bytes so far; CANCEL stops it, keeping the .part.
/// DEST only appears once length and checksum match; a bad file is deleted.
pub async fn download(url: &str, dest: &Path, size: u64, sha256: &str, cancel: &AtomicBool, mut progress: impl FnMut(u64)) -> Result<(), String> {
    let tmp = part(dest);
    let io = |e: std::io::Error| format!("{}: {e}", tmp.display());
    let mut have = std::fs::metadata(&tmp).map(|m| m.len()).unwrap_or(0);
    if have > size {
        have = 0;
    }
    if have < size {
        if rustls::crypto::CryptoProvider::get_default().is_none() {
            let _ = rustls::crypto::ring::default_provider().install_default();
        }
        let client = reqwest::Client::builder().user_agent("Margin").connect_timeout(std::time::Duration::from_secs(30)).read_timeout(READ_TIMEOUT).build().map_err(|e| e.to_string())?;
        let mut req = client.get(url);
        if have > 0 {
            req = req.header(reqwest::header::RANGE, format!("bytes={have}-"));
        }
        let mut resp = req.send().await.map_err(|e| format!("Download failed: {e}"))?;
        let resumed = resp.headers().get(reqwest::header::CONTENT_RANGE).and_then(|v| v.to_str().ok()).is_some_and(|v| v.starts_with(&format!("bytes {have}-")));
        match resp.status().as_u16() {
            206 if resumed => {}
            200 => have = 0, // the server ignored Range: start over
            s => return Err(format!("Download failed: HTTP {s}")),
        }
        std::fs::create_dir_all(dest.parent().unwrap_or(dest)).map_err(io)?;
        let mut f = std::fs::OpenOptions::new().create(true).write(true).truncate(false).open(&tmp).map_err(io)?;
        f.set_len(have).map_err(io)?;
        f.seek(std::io::SeekFrom::End(0)).map_err(io)?;
        progress(have);
        while let Some(chunk) = resp.chunk().await.map_err(|e| format!("Download interrupted ({e}); try again to resume."))? {
            if cancel.load(Ordering::Relaxed) {
                return Err("Download cancelled.".into());
            }
            f.write_all(&chunk).map_err(io)?;
            have += chunk.len() as u64;
            progress(have);
            if have > size {
                break;
            }
        }
        f.sync_all().map_err(io)?;
    }
    if have < size {
        return Err("Download interrupted; try again to resume.".into());
    }
    let t = tmp.clone();
    let sum = tauri::async_runtime::spawn_blocking(move || sha256_file(&t)).await.map_err(|e| e.to_string())?.map_err(io)?;
    if have > size || sum != sha256 {
        let _ = std::fs::remove_file(&tmp);
        return Err("The download didn't match its checksum and was deleted. Try again.".into());
    }
    std::fs::rename(&tmp, dest).map_err(io)
}

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

/// Prompt for a one-line diary note about a block of work: its PROJECT, TASK, APPS and TITLES (minus EXCLUDEd ones).
pub fn note_prompt(project: Option<&str>, task: Option<&str>, apps: &[String], titles: &[String], exclude: &[Regex]) -> String {
    let head = "Write one short line (under 15 words, past tense, no quotes) for a work diary saying what I worked on. Use only these facts.";
    let apps: Vec<_> = kept(apps, exclude).map(String::as_str).collect();
    let some = |label: &str, x: Option<&str>| x.filter(|x| !x.trim().is_empty()).map(|x| format!("{label}: {x}"));
    let facts = some("Project", project).into_iter().chain(some("Task", task)).chain([format!("Apps: {}", apps.join(", ")), "Window titles:".into()]);
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

/// A draft for PROMPT from MODEL, through BACKEND (Ollama at URL, or a built-in model in DIR).
pub fn generate(backend: Backend, url: &str, dir: &Path, model: &str, prompt: &str, one_line: bool) -> Result<String, String> {
    match backend {
        Backend::Ollama => ollama(url, model, prompt, one_line),
        Backend::Embedded => embedded(dir, model, prompt, one_line),
    }
}

/// Whether this build can run built-in models (the `embedded-ai` cargo feature).
pub const RUNS: bool = cfg!(feature = "embedded-ai");

/// A draft for PROMPT from built-in MODEL, downloaded into DIR.
fn embedded(dir: &Path, model: &str, prompt: &str, one_line: bool) -> Result<String, String> {
    let m = MODELS.iter().find(|m| m.id == model).ok_or_else(|| format!("Unknown built-in model {model}. Pick one with AI drafts: choose model…"))?;
    #[cfg(not(feature = "embedded-ai"))]
    {
        let _ = (dir, prompt, one_line);
        Err(format!("This build of Margin can't run {} itself. Use Ollama (AI drafts: choose model…).", m.name))
    }
    #[cfg(feature = "embedded-ai")]
    {
        let path = m.path(dir);
        if !path.is_file() {
            return Err(format!("{} isn't downloaded. Download it with AI drafts: choose model…", m.name));
        }
        Ok(clean(&llama::generate(m, &path, prompt, if one_line { 48 } else { 320 })?, one_line))
    }
}

/// GB of memory the OS can hand out, from /proc/meminfo's MemAvailable.
#[cfg(any(test, all(feature = "embedded-ai", target_os = "linux")))]
fn available_gb(meminfo: &str) -> Option<f64> {
    let kb: f64 = meminfo.lines().find_map(|l| l.strip_prefix("MemAvailable:"))?.trim().trim_end_matches("kB").trim().parse().ok()?;
    Some(kb / 1024.0 / 1024.0)
}

/// GB macOS can hand out, from `vm_stat`: free, inactive and speculative pages.
#[cfg(any(test, all(feature = "embedded-ai", target_os = "macos")))]
fn vm_stat_gb(out: &str) -> Option<f64> {
    let page: f64 = out.split("page size of ").nth(1)?.split(' ').next()?.parse().ok()?;
    let pages = |k: &str| out.lines().find_map(|l| l.strip_prefix(k)).and_then(|v| v.trim().trim_end_matches('.').parse::<f64>().ok());
    Some((pages("Pages free:")? + pages("Pages inactive:")? + pages("Pages speculative:").unwrap_or(0.0)) * page / 1024.0 / 1024.0 / 1024.0)
}

/// Built-in models run by llama.cpp on the CPU (GPU through Metal on Apple Silicon). The model
/// loads on the first draft and stays loaded until IDLE passes without one.
#[cfg(feature = "embedded-ai")]
mod llama {
    use super::{Model, MAX_PROMPT};
    use llama_cpp_2::context::params::LlamaContextParams;
    use llama_cpp_2::llama_backend::LlamaBackend;
    use llama_cpp_2::llama_batch::LlamaBatch;
    use llama_cpp_2::model::params::LlamaModelParams;
    use llama_cpp_2::model::{LlamaChatMessage, LlamaModel};
    use llama_cpp_2::sampling::LlamaSampler;
    use std::num::NonZeroU32;
    use std::path::Path;
    use std::sync::{Mutex, OnceLock};
    use std::time::{Duration, Instant};

    const IDLE: Duration = Duration::from_secs(300);
    /// Tokens of context: MAX_PROMPT bytes plus the template and the answer fit easily.
    const CTX: u32 = 2048;
    static BACKEND: OnceLock<Result<LlamaBackend, String>> = OnceLock::new();
    /// The loaded model (by id) and when it was last used. Drafts take turns on this lock.
    static LOADED: Mutex<Option<(&str, Instant, LlamaModel)>> = Mutex::new(None);

    /// GB of memory the OS can hand out, or None when it can't tell (then the check is skipped).
    #[allow(unreachable_code)]
    fn free_gb() -> Option<f64> {
        #[cfg(target_os = "linux")]
        return std::fs::read_to_string("/proc/meminfo").ok().as_deref().and_then(super::available_gb);
        #[cfg(target_os = "macos")]
        return std::process::Command::new("vm_stat").output().ok().and_then(|o| super::vm_stat_gb(&String::from_utf8_lossy(&o.stdout)));
        #[cfg(windows)]
        {
            use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
            let mut s = MEMORYSTATUSEX { dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32, ..Default::default() };
            // SAFETY: s is a MEMORYSTATUSEX with dwLength set, as the API requires.
            return (unsafe { GlobalMemoryStatusEx(&mut s) } != 0).then(|| s.ullAvailPhys as f64 / 1024.0 / 1024.0 / 1024.0);
        }
        None
    }

    fn load(backend: &LlamaBackend, m: &Model, path: &Path) -> Result<LlamaModel, String> {
        if let Some(free) = free_gb() {
            if free < f64::from(m.ram_gb) {
                return Err(format!("{} needs ~{} GB of free memory and only {free:.1} GB is free. Close some apps, or pick a smaller model.", m.name, m.ram_gb));
            }
        }
        LlamaModel::load_from_file(backend, path, &LlamaModelParams::default()).map_err(|e| format!("Couldn't load {}: {e}. Delete it and download it again (AI drafts: choose model…).", m.name))
    }

    /// The model's answer to PROMPT, at most MAX_TOKENS long.
    pub fn generate(m: &'static Model, path: &Path, prompt: &str, max_tokens: i32) -> Result<String, String> {
        // Release builds compile llama.cpp for AVX2 (release.yml); without it, it would crash.
        #[cfg(target_arch = "x86_64")]
        if !std::is_x86_feature_detected!("avx2") {
            return Err("Built-in models need a processor with AVX2 (most from 2013 on). Use Ollama instead.".into());
        }
        let backend = BACKEND.get_or_init(|| LlamaBackend::init().map(|mut b| { b.void_logs(); b }).map_err(|e| e.to_string())).as_ref()?;
        let mut slot = LOADED.lock().unwrap_or_else(|e| e.into_inner());
        if slot.as_ref().is_none_or(|(id, ..)| *id != m.id) {
            *slot = None; // free the old model before loading another
            *slot = Some((m.id, Instant::now(), load(backend, m, path)?));
        }
        let (_, used, model) = slot.as_mut().expect("loaded above");
        let text = run(backend, model, prompt, max_tokens);
        *used = Instant::now();
        drop(slot);
        std::thread::spawn(|| {
            std::thread::sleep(IDLE);
            let mut slot = LOADED.lock().unwrap_or_else(|e| e.into_inner());
            if slot.as_ref().is_some_and(|(_, used, _)| used.elapsed() >= IDLE) {
                *slot = None;
            }
        });
        text
    }

    fn run(backend: &LlamaBackend, model: &LlamaModel, prompt: &str, max_tokens: i32) -> Result<String, String> {
        let e = |e: &dyn std::fmt::Display| format!("The built-in model failed: {e}");
        let chat = [LlamaChatMessage::new("user".into(), prompt.chars().take(MAX_PROMPT + 200).collect()).map_err(|x| e(&x))?];
        let tmpl = model.chat_template(None).map_err(|x| e(&x))?;
        let text = model.apply_chat_template(&tmpl, &chat, true).map_err(|x| e(&x))?;
        let vocab = model.vocab();
        let tokens = vocab.tokenize(text.as_bytes(), false, true);
        if tokens.len() as i32 + max_tokens > CTX as i32 {
            return Err("That prompt is too long for the built-in model.".into());
        }
        let threads = std::thread::available_parallelism().map_or(4, |n| n.get().min(8)) as i32;
        let params = LlamaContextParams::default().with_n_ctx(NonZeroU32::new(CTX)).with_n_batch(CTX).with_n_threads(threads).with_n_threads_batch(threads);
        let mut ctx = model.new_context(backend, params).map_err(|x| e(&x))?;
        let mut batch = LlamaBatch::new(CTX as usize, 1);
        let last = tokens.len() as i32 - 1;
        for (i, t) in (0..).zip(&tokens) {
            batch.add(*t, i, &[0], i == last).map_err(|x| e(&x))?;
        }
        ctx.decode(&mut batch).map_err(|x| e(&x))?;
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.subsec_nanos());
        let mut sampler = LlamaSampler::chain_simple([LlamaSampler::top_k(40), LlamaSampler::top_p(0.9, 1), LlamaSampler::temp(0.7), LlamaSampler::dist(seed)]);
        let (mut out, mut pos) = (Vec::new(), last + 1);
        for _ in 0..max_tokens {
            let t = sampler.sample(&ctx, batch.n_tokens() - 1); // also accepts it
            if vocab.is_eog(t) {
                break;
            }
            out.extend(vocab.token_to_piece(t, false, None));
            batch.clear();
            batch.add(t, pos, &[0], true).map_err(|x| e(&x))?;
            pos += 1;
            ctx.decode(&mut batch).map_err(|x| e(&x))?;
        }
        Ok(String::from_utf8_lossy(&out).into_owned())
    }
}

/// A draft for PROMPT from MODEL in Ollama at URL.
fn ollama(url: &str, model: &str, prompt: &str, one_line: bool) -> Result<String, String> {
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
        let p = note_prompt(Some("Margin"), Some("Task matching"), &v(&["Code", "KeePassXC"]), &v(&["lib.rs - margin", "bank.kdbx - KeePassXC", ""]), &ex);
        assert!(p.ends_with("\nProject: Margin\nTask: Task matching\nApps: Code\nWindow titles:\n- lib.rs - margin"), "{p}");
        assert!(note_prompt(None, None, &[], &[], &ex).ends_with("facts.\nApps: \nWindow titles:"));
        let day = NaiveDate::from_ymd_opt(2026, 10, 3).unwrap();
        let s = Session { date: day, project: "Margin".into(), desc: "AI drafts".into(), hours: 2.5, start: t("09:00"), end: t("11:30"), line: Some(3) };
        let b = Suggestion { start: dt("13:00"), end: dt("14:00"), apps: v(&["Firefox"]), titles: v(&["Ollama docs", "KeePassXC"]), project: None, task: None, meeting: None };
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
    fn models_and_dispatch() {
        assert_eq!(serde_json::to_string(&Backend::default()).unwrap(), "\"ollama\"");
        for m in MODELS {
            assert!(m.url.starts_with("https://") && m.url.ends_with(".gguf") && m.sha256.len() == 64 && m.size > 0, "{}", m.id);
        }
        assert!(MODELS[0].id.starts_with("qwen2.5-3b"), "default model");
        let dir = Path::new("/nonexistent");
        let err = generate(Backend::Embedded, "", dir, MODELS[0].id, "p", true).unwrap_err();
        assert!(err.contains(if RUNS { "isn't downloaded" } else { "can't run" }), "{err}");
        // Embedded never talks to ai_url (which would be refused as not local).
        assert!(generate(Backend::Embedded, "http://example.com", dir, "nope", "p", true).unwrap_err().contains("Unknown built-in model nope"));
        assert_eq!(available_gb("MemTotal:       16000000 kB\nMemAvailable:    3145728 kB\n"), Some(3.0));
        assert_eq!(available_gb("MemTotal: 1 kB\n"), None);
        let vm = "Mach Virtual Memory Statistics: (page size of 16384 bytes)\nPages free:                               65536.\nPages active:                            999999.\nPages inactive:                           131072.\nPages speculative:                         65536.\n";
        assert_eq!(vm_stat_gb(vm), Some(4.0));
        assert_eq!(vm_stat_gb("Pages free: 1.\n"), None);
    }

    /// A real draft: `MARGIN_MODELS=<dir holding the 1.5B model> cargo test --features embedded-ai -- --ignored`.
    #[cfg(feature = "embedded-ai")]
    #[test]
    #[ignore]
    fn embedded_drafts() {
        let dir = PathBuf::from(std::env::var("MARGIN_MODELS").expect("MARGIN_MODELS"));
        let p = note_prompt(Some("Margin"), Some("AI drafts"), &v(&["Code"]), &v(&["ai.rs - margin", "llama.cpp docs"]), &[]);
        for one_line in [true, false] {
            let d = generate(Backend::Embedded, "", &dir, MODELS[1].id, &p, one_line).unwrap();
            println!("{d}");
            assert!(!d.trim().is_empty() && (!one_line || !d.contains('\n')), "{d:?}");
        }
    }

    /// Answers one connection per reply in turn; returns the base URL and the request heads.
    fn serve(replies: Vec<Vec<u8>>) -> (String, std::thread::JoinHandle<Vec<String>>) {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}/m.gguf", l.local_addr().unwrap());
        let srv = std::thread::spawn(move || {
            replies.into_iter().map(|r| {
                let (mut c, _) = l.accept().unwrap();
                let (mut req, mut buf) = (vec![], [0; 512]);
                while !req.ends_with(b"\r\n\r\n") {
                    let n = c.read(&mut buf).unwrap();
                    req.extend_from_slice(&buf[..n]);
                }
                c.write_all(&r).unwrap();
                String::from_utf8_lossy(&req).to_lowercase()
            }).collect()
        });
        (base, srv)
    }

    #[test]
    fn download_stall_times_out() {
        let dest = std::env::temp_dir().join(format!("margin-stall-{}", std::process::id())).join("m.gguf");
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/m.gguf", l.local_addr().unwrap());
        let srv = std::thread::spawn(move || {
            let (mut c, _) = l.accept().unwrap();
            let _ = c.read(&mut [0; 512]);
            c.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\n\r\nhello").unwrap();
            std::thread::sleep(std::time::Duration::from_secs(3)); // hold the socket open: a stall
        });
        let t = std::time::Instant::now();
        let err = tauri::async_runtime::block_on(download(&url, &dest, 11, "", &AtomicBool::new(false), |_| {})).unwrap_err();
        assert!(err.contains("interrupted") && t.elapsed().as_secs() < 3, "{err} after {:?}", t.elapsed()); // the timeout, not the socket closing
        assert_eq!(std::fs::read(part(&dest)).unwrap(), b"hello");
        srv.join().unwrap();
        std::fs::remove_dir_all(dest.parent().unwrap()).unwrap();
    }

    #[test]
    fn download_resumes_and_checks() {
        let dir = std::env::temp_dir().join(format!("margin-dl-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let dest = dir.join("m.gguf");
        let body = b"hello world";
        let sum = format!("{:x}", Sha256::digest(body));
        let no = AtomicBool::new(false);
        let get = |url: &str, sum: &str, cancel: &AtomicBool| tauri::async_runtime::block_on(download(url, &dest, 11, sum, cancel, |_| {}));
        let ok = |b: &str| format!("HTTP/1.1 200 OK\r\nContent-Length: 11\r\n\r\n{b}").into_bytes();
        let (url, srv) = serve(vec![
            b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\n\r\nhello".to_vec(), // cut short
            b"HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 5-10/11\r\nContent-Length: 6\r\n\r\n world".to_vec(),
            ok("hello wurld"),
            ok("hello world"),
            ok("hello world"),
        ]);
        assert!(get(&url, &sum, &no).unwrap_err().contains("interrupted"));
        assert_eq!(std::fs::read(part(&dest)).unwrap(), b"hello");
        get(&url, &sum, &no).unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), body);
        assert!(!part(&dest).exists());
        std::fs::remove_file(&dest).unwrap();
        // A corrupt file is deleted, not kept.
        assert!(get(&url, &sum, &no).unwrap_err().contains("checksum"));
        assert!(!dest.exists() && !part(&dest).exists());
        // A server that ignores Range sends it all again.
        std::fs::write(part(&dest), "hel").unwrap();
        get(&url, &sum, &no).unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), body);
        std::fs::remove_file(&dest).unwrap();
        assert!(get(&url, &sum, &AtomicBool::new(true)).unwrap_err().contains("cancelled"));
        assert!(!dest.exists());
        let reqs = srv.join().unwrap();
        assert!(!reqs[0].contains("range:") && reqs[1].contains("range: bytes=5-") && reqs[3].contains("range: bytes=3-"), "{reqs:?}");
        // A finished .part (say the app quit while checking it) needs no request.
        std::fs::write(part(&dest), body).unwrap();
        get("http://127.0.0.1:1/never", &sum, &no).unwrap();
        assert_eq!(MODELS[0].path(&dir), dir.join("qwen2.5-3b-instruct-q4_k_m.gguf"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn http() {
        let generate = |url: &str, m: &str, p: &str, one: bool| generate(Backend::Ollama, url, Path::new(""), m, p, one);
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
