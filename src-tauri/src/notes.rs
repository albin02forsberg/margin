// Zettelkasten on top of the org scan: org-roam compatible notes
// (`:ID:` property + `#+title:`), [[id:]] links, backlinks and search.

use crate::org::OrgFile;
use serde::Serialize;
use std::hash::{BuildHasher, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Node {
    pub id: String,
    pub title: String,
    pub path: PathBuf,
    pub line: usize,
}

#[derive(Serialize, Clone, Debug)]
pub struct Hit {
    pub path: PathBuf,
    pub title: String,
    pub line: usize,
    pub text: String,
}

/// Random UUIDv4-shaped id, from std's randomly keyed hasher (no rand crate).
pub fn gen_id() -> String {
    let r = || std::collections::hash_map::RandomState::new().build_hasher().finish();
    let (a, b) = (r(), r());
    format!(
        "{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
        a >> 32,
        (a >> 16) & 0xffff,
        a & 0xfff,
        (b >> 48) & 0x3fff | 0x8000,
        b & 0xffff_ffff_ffff
    )
}

pub fn slug(title: &str) -> String {
    let s: String = title.to_lowercase().chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect();
    s.split('_').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("_")
}

/// Create `<dir>/YYYYMMDDHHMMSS-slug.org`, returning (path, id).
pub fn new_note(dir: &Path, title: &str) -> std::io::Result<(PathBuf, String)> {
    let id = gen_id();
    let stamp = chrono::Local::now().format("%Y%m%d%H%M%S");
    let path = dir.join(format!("{stamp}-{}.org", slug(title)));
    std::fs::create_dir_all(dir)?;
    std::fs::write(&path, format!(":PROPERTIES:\n:ID:       {id}\n:END:\n#+title: {}\n\n", title.trim()))?;
    Ok((path, id))
}

/// Every linkable node: files with an ID, and headings with an ID.
pub fn nodes(files: &[Arc<OrgFile>]) -> Vec<Node> {
    let mut out = vec![];
    for f in files {
        if let Some(id) = &f.id {
            out.push(Node { id: id.clone(), title: f.title.clone(), path: f.path.clone(), line: 0 });
        }
        for h in &f.headlines {
            if let Some(id) = &h.id {
                out.push(Node { id: id.clone(), title: h.title.clone(), path: f.path.clone(), line: h.line });
            }
        }
    }
    out
}

/// Links pointing at any node defined in PATH.
pub fn backlinks(files: &[Arc<OrgFile>], path: &Path) -> Vec<Hit> {
    let Some(f) = files.iter().find(|f| f.path == path) else { return vec![] };
    let ids: Vec<&String> = f.id.iter().chain(f.headlines.iter().filter_map(|h| h.id.as_ref())).collect();
    files
        .iter()
        .flat_map(|o| o.links.iter().filter(|l| ids.contains(&&l.target)).map(|l| Hit { path: o.path.clone(), title: o.title.clone(), line: l.line, text: l.text.clone() }))
        .collect()
}

/// Case-insensitive full-text search.
// ponytail: linear scan of every file per query; add an inverted index if it gets slow past ~10k notes.
pub fn search(files: &[Arc<OrgFile>], query: &str, limit: usize) -> Vec<Hit> {
    let q = query.to_lowercase();
    let mut out = vec![];
    for f in files {
        let Ok(text) = std::fs::read_to_string(&f.path) else { continue };
        for (i, l) in text.lines().enumerate() {
            if l.to_lowercase().contains(&q) {
                out.push(Hit { path: f.path.clone(), title: f.title.clone(), line: i, text: l.trim().to_string() });
                if out.len() >= limit {
                    return out;
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::org::{parse, Kw};

    #[test]
    fn links_and_backlinks() {
        let kw = Kw { todo: vec![], done: vec![] };
        let a = Arc::new(parse(Path::new("/n/a.org"), ":PROPERTIES:\n:ID: aaa\n:END:\n#+title: A\n* Sub\n:PROPERTIES:\n:ID: sub\n:END:\n", &kw));
        let b = Arc::new(parse(Path::new("/n/b.org"), "#+title: B\nsee [[id:aaa][A]] and [[id:sub]]\n", &kw));
        let files = [a, b];
        assert_eq!(nodes(&files).iter().map(|n| n.id.as_str()).collect::<Vec<_>>(), vec!["aaa", "sub"]);
        let bl = backlinks(&files, Path::new("/n/a.org"));
        assert_eq!(bl.len(), 2);
        assert_eq!((bl[0].title.as_str(), bl[0].line), ("B", 1));
        assert_eq!(slug("Hej på dig: Zettel!"), "hej_på_dig_zettel");
        assert_ne!(gen_id(), gen_id());
        assert_eq!(gen_id().len(), 36);
    }
}
