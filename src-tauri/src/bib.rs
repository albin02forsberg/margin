//! Read-only BibTeX reader: a note's `#+BIBLIOGRAPHY:` files, else the `bibliography_file` setting, for citations and export.
use crate::{config, App};
use serde::Serialize;
use std::path::{Path, PathBuf};
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

/// Entries of the BibTeX FILE (a `bibliography_file` setting, `~` expanded); empty setting = none.
pub fn entries_from(file: &str) -> Result<Vec<BibEntry>, String> {
    if file.trim().is_empty() {
        return Ok(vec![]);
    }
    let path = config::expand(file.trim());
    std::fs::read_to_string(&path).map(|t| parse(&t)).map_err(|e| format!("{}: {e}", path.display()))
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
/// and parsed as BibTeX, never written.
pub fn entries_for_note(text: &str, path: Option<&Path>, fallback: &str) -> Result<Vec<BibEntry>, String> {
    let files = note_bib_paths(text, path.and_then(Path::parent));
    if files.is_empty() {
        return entries_from(fallback);
    }
    let mut seen = std::collections::HashSet::new();
    Ok(files.iter().filter_map(|f| std::fs::read_to_string(f).ok()).flat_map(|t| parse(&t)).filter(|e| seen.insert(e.key.clone())).collect())
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
