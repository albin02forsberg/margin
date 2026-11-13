//! Read-only BibTeX reader: the `bibliography_file` setting's entries for citation pickers.
use crate::{config, App};
use serde::Serialize;
use tauri::State;

#[derive(Serialize, Debug, PartialEq)]
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

/// Entries of the `bibliography_file` (re-read each call); empty setting = none.
#[tauri::command]
pub fn bib_entries(s: State<App>) -> Result<Vec<BibEntry>, String> {
    let file = s.cfg().bibliography_file;
    if file.trim().is_empty() {
        return Ok(vec![]);
    }
    let path = config::expand(file.trim());
    std::fs::read_to_string(&path).map(|t| parse(&t)).map_err(|e| format!("{}: {e}", path.display()))
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
    fn labels() {
        assert_eq!(label("Jane Smith", "2020"), "Smith 2020");
        assert_eq!(label("{World Bank}", ""), "World Bank");
        assert_eq!(label("", "2001"), "2001");
    }
}
