//! Read-only BibTeX / CSL-JSON (`.json`) reader: a note's `#+BIBLIOGRAPHY:` files, else the `bibliography_file` setting, for citations and export.
use crate::{config, App};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::State;

#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct BibEntry {
    pub key: String,
    pub author: String,
    pub year: String,
    pub title: String,
    /// Journal, else booktitle, else publisher.
    pub container: String,
    /// Author-year label: "Smith 2020", "Smith & Lee 2020", "Smith et al. 2020".
    pub label: String,
}

/// Strip one level of outer braces/quotes and surrounding space; LaTeX escapes stay as written.
fn unwrap(v: &str) -> String {
    let v = v.trim();
    let b = v.as_bytes();
    if b.len() >= 2 && ((b[0] == b'{' && b[b.len() - 1] == b'}') || (b[0] == b'"' && b[b.len() - 1] == b'"')) {
        return v[1..v.len() - 1].trim().to_string();
    }
    v.to_string()
}

/// Index just past the `}` closing the `{` at `open` (or the end when unbalanced).
fn close(s: &str, open: usize) -> usize {
    let mut depth = 0;
    for (i, c) in s[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return open + i + 1;
                }
            }
            _ => {}
        }
    }
    s.len()
}

/// Split an entry body at top-level commas (outside braces and quotes).
fn split_fields(body: &str) -> Vec<&str> {
    let (mut out, mut depth, mut quoted, mut start) = (vec![], 0, false, 0);
    for (i, c) in body.char_indices() {
        match c {
            '{' if !quoted => depth += 1,
            '}' if !quoted => depth -= 1,
            '"' if depth == 0 => quoted = !quoted,
            ',' if depth == 0 && !quoted => {
                out.push(&body[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&body[start..]);
    out
}

fn surname(a: &str) -> String {
    let a = a.trim();
    let n = match a.split_once(',') {
        Some((l, _)) => l.trim(),
        None if a.starts_with('{') => a,
        None => a.rsplit(' ').next().unwrap_or(a),
    };
    n.trim_matches(['{', '}']).to_string()
}

fn label(author: &str, year: &str) -> String {
    let names: Vec<String> = author.split(" and ").filter(|a| !a.trim().is_empty()).map(surname).collect();
    let who = match names.as_slice() {
        [] => String::new(),
        [a] => a.clone(),
        [a, b] => format!("{a} & {b}"),
        [a, ..] => format!("{a} et al."),
    };
    format!("{who} {year}").trim().to_string()
}

pub fn parse(text: &str) -> Vec<BibEntry> {
    let mut out = vec![];
    let mut at = 0;
    while let Some(p) = text[at..].find('@') {
        let s = at + p;
        let Some(o) = text[s..].find(['{', '(']).map(|i| s + i) else { break };
        let kind = text[s + 1..o].trim().to_lowercase();
        let end = if text.as_bytes()[o] == b'{' { close(text, o) } else { text[o..].find(')').map_or(text.len(), |i| o + i + 1) };
        at = end.max(s + 1);
        if matches!(kind.as_str(), "comment" | "preamble" | "string") || kind.is_empty() || kind.contains(char::is_whitespace) {
            continue;
        }
        let body = &text[o + 1..end.saturating_sub(1).max(o + 1)];
        let mut parts = split_fields(body).into_iter();
        let key = parts.next().unwrap_or("").trim().to_string();
        if key.is_empty() {
            continue;
        }
        let mut f = std::collections::HashMap::new();
        for p in parts {
            if let Some((k, v)) = p.split_once('=') {
                f.insert(k.trim().to_lowercase(), unwrap(v));
            }
        }
        let get = |k: &str| f.get(k).cloned().unwrap_or_default();
        let container = ["journal", "booktitle", "publisher"].iter().map(|k| get(k)).find(|v| !v.is_empty()).unwrap_or_default();
        let (author, year) = (get("author"), get("year"));
        out.push(BibEntry { label: label(&author, &year), key, author, year, title: get("title"), container });
    }
    out
}

/// CSL-JSON (Zotero/Pandoc export): an array of items. Authors become BibTeX-style "Family, Given" (a bare family or a
/// `literal` name is braced) so `label` works as for `.bib`. Items without an `id` are skipped.
pub fn parse_csl(text: &str) -> Result<Vec<BibEntry>, String> {
    let v: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let items = v.as_array().ok_or("CSL-JSON must be an array of items")?;
    let str_of = |v: &serde_json::Value| match v {
        serde_json::Value::String(s) => s.trim().to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        _ => String::new(),
    };
    Ok(items
        .iter()
        .filter_map(|it| {
            let get = |k: &str| it.get(k).map(str_of).unwrap_or_default();
            let key = get("id");
            if key.is_empty() {
                return None;
            }
            let author = it.get("author").and_then(|a| a.as_array()).map_or(String::new(), |a| {
                let names: Vec<String> = a
                    .iter()
                    .filter_map(|p| {
                        let f = |k: &str| p.get(k).map(str_of).unwrap_or_default();
                        match (f("family"), f("given"), f("literal")) {
                            (fam, giv, _) if !fam.is_empty() && !giv.is_empty() => Some(format!("{fam}, {giv}")),
                            (fam, _, _) if !fam.is_empty() => Some(format!("{{{fam}}}")),
                            (_, _, l) if !l.is_empty() => Some(format!("{{{l}}}")),
                            _ => None,
                        }
                    })
                    .collect();
                names.join(" and ")
            });
            let issued = it.get("issued");
            let year = issued
                .and_then(|i| i.get("date-parts"))
                .and_then(|d| d.get(0))
                .and_then(|d| d.get(0))
                .map(str_of)
                .filter(|y| !y.is_empty())
                .or_else(|| {
                    // "literal"/"raw" dates: the first 4-digit run.
                    let raw = issued.and_then(|i| i.get("literal").or(i.get("raw"))).map(str_of).unwrap_or_default();
                    raw.as_bytes().windows(4).position(|w| w.iter().all(u8::is_ascii_digit)).map(|i| raw[i..i + 4].to_string())
                })
                .unwrap_or_default();
            let container = ["container-title", "publisher"].iter().map(|k| get(k)).find(|v| !v.is_empty()).unwrap_or_default();
            Some(BibEntry { label: label(&author, &year), key, author, year, title: get("title"), container })
        })
        .collect())
}

type Cached = HashMap<PathBuf, ((std::time::SystemTime, u64), Arc<Vec<BibEntry>>)>;
/// Parsed entries per file, keyed on its (mtime, len); cleared past 16 files (paths come from notes).
static CACHE: Mutex<Option<Cached>> = Mutex::new(None);

/// The parsed entries of PATH, re-parsed only when its (mtime, len) changed. Errors are never cached.
fn load(path: &Path) -> std::io::Result<Arc<Vec<BibEntry>>> {
    let m = std::fs::metadata(path)?;
    let stamp = (m.modified()?, m.len());
    let hit = |c: &Option<Cached>| c.as_ref().and_then(|c| c.get(path)).filter(|(s, _)| *s == stamp).map(|(_, e)| e.clone());
    if let Some(e) = hit(&CACHE.lock().unwrap_or_else(|e| e.into_inner())) {
        return Ok(e);
    }
    let text = std::fs::read_to_string(path)?;
    let entries = Arc::new(if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("json")) {
        parse_csl(&text).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?
    } else {
        parse(&text)
    }); // parsed without the lock
    let mut g = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let c = g.get_or_insert_with(HashMap::new);
    if c.len() >= 16 {
        c.clear();
    }
    c.insert(path.to_path_buf(), (stamp, entries.clone()));
    Ok(entries)
}

/// Entries of the BibTeX (or CSL-JSON, `.json`) FILE (a `bibliography_file` setting, `~` expanded); empty setting = none.
pub fn entries_from(file: &str) -> Result<Vec<BibEntry>, String> {
    if file.trim().is_empty() {
        return Ok(vec![]);
    }
    let path = config::expand(file.trim());
    load(&path).map(|e| (*e).clone()).map_err(|e| format!("{}: {e}", path.display()))
}

/// The files named by TEXT's `#+BIBLIOGRAPHY:` lines (org-cite: repeatable, one file each, quotes stripped),
/// `~` expanded, relative ones against DIR (the note's folder; none = skipped).
pub fn note_bib_paths(text: &str, dir: Option<&Path>) -> Vec<PathBuf> {
    text.lines()
        .filter_map(|l| {
            let l = l.trim_start();
            let v = l.get(..15).filter(|k| k.eq_ignore_ascii_case("#+bibliography:")).map(|_| l[15..].trim().trim_matches('"'))?;
            let p = config::expand(v);
            match (v.is_empty(), p.is_absolute(), dir) {
                (true, ..) => None,
                (_, true, _) => Some(p),
                (_, _, Some(d)) => Some(d.join(p)),
                _ => None,
            }
        })
        .collect()
}

/// Entries for note PATH with buffer TEXT (#1358): its `#+BIBLIOGRAPHY:` files merged in order (first file wins per key,
/// unreadable ones skipped), else FALLBACK (the `bibliography_file` setting). Note-declared files may lie outside the
/// notes/data/config roots and deliberately don't go through `App::check`: like the global setting they are only read
/// and parsed as BibTeX or CSL-JSON, never written.
pub fn entries_for_note(text: &str, path: Option<&Path>, fallback: &str) -> Result<Vec<BibEntry>, String> {
    let files = note_bib_paths(text, path.and_then(Path::parent));
    if files.is_empty() {
        return entries_from(fallback);
    }
    let mut seen = std::collections::HashSet::new();
    Ok(files.iter().filter_map(|f| load(f).ok()).flat_map(|e| (*e).clone()).filter(|e| seen.insert(e.key.clone())).collect())
}

/// Entries for note PATH (TEXT: its buffer, or just its `#+BIBLIOGRAPHY:` lines), re-read each call; see entries_for_note.
#[tauri::command]
pub fn bib_entries(s: State<App>, path: Option<PathBuf>, text: Option<String>) -> Result<Vec<BibEntry>, String> {
    entries_for_note(&text.unwrap_or_default(), path.as_deref(), &s.cfg().bibliography_file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_entries() {
        let t = r#"@comment{x}
@string{j = "J"}
@Article{smith20,
  Author = {Smith, Jane and Lee, Bo},
  title = {A {Nested} \"{o} title},
  year = 2020,
  JOURNAL = "Nature",
}
@book(b1, author={Ann Doe and B One and C Two}, year={1999}, publisher={MIT})
@preamble{"x"}"#;
        let e = parse(t);
        assert_eq!(e.len(), 2);
        assert_eq!((e[0].key.as_str(), e[0].year.as_str(), e[0].container.as_str()), ("smith20", "2020", "Nature"));
        assert_eq!(e[0].title, "A {Nested} \\\"{o} title");
        assert_eq!(e[0].label, "Smith & Lee 2020");
        assert_eq!((e[1].label.as_str(), e[1].container.as_str()), ("Doe et al. 1999", "MIT"));
    }

    #[test]
    fn load_cached_until_file_changes() {
        let f = std::env::temp_dir().join(format!("margin-bibcache-{}.bib", std::process::id()));
        std::fs::write(&f, "@book{a, author={A One}, year={2001}}").unwrap();
        let (x, y) = (load(&f).unwrap(), load(&f).unwrap());
        assert!(Arc::ptr_eq(&x, &y), "unchanged file is served from the cache");
        std::fs::write(&f, "@book{a, author={A One}, year={2001}}\n@book{b, author={B Two}, year={2002}}").unwrap();
        assert_eq!(load(&f).unwrap().len(), 2);
        std::fs::remove_file(&f).unwrap();
        assert!(load(&f).is_err(), "deleted file is an error, not a stale hit");
    }

    #[test]
    fn csl_json() {
        let t = r#"[{"id":"smith20","type":"article-journal","title":"A title","author":[{"family":"Smith","given":"Jane"},{"family":"Lee","given":"Bo"}],
            "issued":{"date-parts":[[2020,5]]},"container-title":"Nature"},
          {"id":"who","author":[{"literal":"World Bank"}],"issued":{"raw":"circa 1999-01"},"publisher":"WB"},
          {"id":"nodate","author":[{"family":"Doe"}],"issued":{"date-parts":[[null]]}},{"title":"no id"}]"#;
        let e = parse_csl(t).unwrap();
        assert_eq!(e.len(), 3);
        assert_eq!((e[0].label.as_str(), e[0].container.as_str(), e[0].title.as_str()), ("Smith & Lee 2020", "Nature", "A title"));
        assert_eq!((e[1].label.as_str(), e[1].container.as_str()), ("World Bank 1999", "WB"));
        assert_eq!(e[2].label, "Doe");
        assert!(parse_csl("{}").is_err() && parse_csl("nope").is_err());
        let f = std::env::temp_dir().join(format!("margin-csl-{}.JSON", std::process::id()));
        std::fs::write(&f, t).unwrap();
        assert_eq!(load(&f).unwrap().len(), 3);
        std::fs::write(&f, "oops").unwrap();
        assert_eq!(load(&f).unwrap_err().kind(), std::io::ErrorKind::InvalidData);
        std::fs::remove_file(&f).unwrap();
    }

    #[test]
    fn note_bibliography() {
        let d = std::env::temp_dir().join(format!("margin-bib-{}", std::process::id()));
        std::fs::create_dir_all(d.join("refs")).unwrap();
        std::fs::write(d.join("refs/a.bib"), "@book{x, author={A One}, year={2001}}\n@book{y, author={Y Why}, year={2002}}").unwrap();
        std::fs::write(d.join("b.bib"), "@book{x, author={B Two}, year={1999}}\n@book{z, author={Z Zed}, year={2003}}").unwrap();
        let global = d.join("b.bib").to_string_lossy().to_string();
        let note = d.join("note.org");
        let text = format!("#+title: N\n#+BIBLIOGRAPHY: refs/a.bib\n  #+bibliography: \"{global}\"\n#+bibliography: missing.bib\n");
        assert_eq!(note_bib_paths(&text, Some(&d)), vec![d.join("refs/a.bib"), d.join("b.bib"), d.join("missing.bib")]);
        let labels = |e: Vec<BibEntry>| e.into_iter().map(|e| e.label).collect::<Vec<_>>();
        // First file wins on x; the missing file is skipped.
        assert_eq!(labels(entries_for_note(&text, Some(&note), "").unwrap()), ["One 2001", "Why 2002", "Zed 2003"]);
        // No keyword: the global file. A relative keyword without a folder is skipped.
        assert_eq!(labels(entries_for_note("* x", Some(&note), &global).unwrap()), ["Two 1999", "Zed 2003"]);
        assert!(note_bib_paths("#+bibliography: refs/a.bib", None).is_empty());
        assert!(entries_for_note("", None, "").unwrap().is_empty());
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn labels() {
        assert_eq!(label("Jane Smith", "2020"), "Smith 2020");
        assert_eq!(label("{World Bank}", ""), "World Bank");
        assert_eq!(label("", "2001"), "2001");
    }
}
