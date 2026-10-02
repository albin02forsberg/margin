// Zettelkasten on top of the org scan: org-roam compatible notes
// (`:ID:` property + `#+title:`), [[id:]] links, backlinks and search.

use crate::org::{Link, OrgFile};
use regex::Regex;
use serde::Serialize;
use std::collections::{BTreeSet, HashMap, VecDeque};
use std::hash::{BuildHasher, Hasher};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, LazyLock};

static ANY_LINK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[\[[^\]]*\](?:\[[^\]]*\])?\]").unwrap());

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

/// Write a new org-roam style note at PATH, returning its ID.
pub fn write_note(path: &Path, title: &str) -> std::io::Result<String> {
    let id = gen_id();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, format!(":PROPERTIES:\n:ID:       {id}\n:END:\n#+title: {}\n\n", title.trim()))?;
    Ok(id)
}

/// Create `<dir>/YYYYMMDDHHMMSS-slug.org`, returning (path, id).
pub fn new_note(dir: &Path, title: &str) -> std::io::Result<(PathBuf, String)> {
    let stamp = chrono::Local::now().format("%Y%m%d%H%M%S");
    let path = dir.join(format!("{stamp}-{}.org", slug(title)));
    let id = write_note(&path, title)?;
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
        .flat_map(|o| o.links.iter().filter(|l| !l.file && ids.contains(&&l.target)).map(|l| Hit { path: o.path.clone(), title: o.title.clone(), line: l.line, text: l.text.clone() }))
        .collect()
}

/// Where a `file:` link written in FROM points, normalised lexically.
fn resolve(from: &Path, target: &str) -> PathBuf {
    let t = target.split("::").next().unwrap_or(target);
    let mut out = PathBuf::new();
    for c in from.parent().unwrap_or(Path::new("")).join(crate::config::expand(t)).components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            c => out.push(c),
        }
    }
    out
}

/// Does link L, written in file FROM, point at file TO?
fn points_to(from: &OrgFile, l: &Link, to: &OrgFile) -> bool {
    if l.file {
        resolve(&from.path, &l.target) == to.path
    } else {
        to.id.as_ref() == Some(&l.target) || to.headlines.iter().any(|h| h.id.as_ref() == Some(&l.target))
    }
}

#[derive(Serialize, Debug, PartialEq)]
pub struct GraphNode {
    pub path: PathBuf,
    pub title: String,
    pub hops: usize,
}

/// Notes within DEPTH links (either direction) of PATH; edges index into `nodes`, which starts with PATH.
#[derive(Serialize, Debug, Default, PartialEq)]
pub struct Graph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<(usize, usize)>,
    /// Notes left out to keep the picture readable (and the layout fast).
    pub hidden: usize,
}

/// At most this many nodes; nearer notes are kept first.
const MAX_NODES: usize = 150;

pub fn graph(files: &[Arc<OrgFile>], path: &Path, depth: usize) -> Graph {
    let Some(start) = files.iter().position(|f| f.path == path) else { return Graph::default() };
    let by_path: HashMap<&Path, usize> = files.iter().enumerate().map(|(i, f)| (f.path.as_path(), i)).collect();
    let mut by_id: HashMap<&str, usize> = HashMap::new();
    for (i, f) in files.iter().enumerate() {
        for id in f.id.iter().chain(f.headlines.iter().filter_map(|h| h.id.as_ref())) {
            by_id.insert(id, i);
        }
    }
    let mut adj = vec![BTreeSet::new(); files.len()];
    for (i, f) in files.iter().enumerate() {
        for l in &f.links {
            let j = if l.file { by_path.get(resolve(&f.path, &l.target).as_path()) } else { by_id.get(l.target.as_str()) };
            if let Some(&j) = j.filter(|&&j| j != i) {
                adj[i].insert(j);
                adj[j].insert(i);
            }
        }
    }
    let mut hops = HashMap::from([(start, 0)]);
    let mut order = vec![start];
    let mut queue = VecDeque::from([start]);
    while let Some(i) = queue.pop_front() {
        if hops[&i] == depth {
            continue;
        }
        for &j in &adj[i] {
            if !hops.contains_key(&j) {
                hops.insert(j, hops[&i] + 1);
                if order.len() < MAX_NODES {
                    order.push(j);
                    queue.push_back(j);
                }
            }
        }
    }
    let idx: HashMap<usize, usize> = order.iter().enumerate().map(|(k, &i)| (i, k)).collect();
    let mut edges = vec![];
    for (k, &i) in order.iter().enumerate() {
        edges.extend(adj[i].iter().filter_map(|j| idx.get(j)).filter(|&&m| m > k).map(|&m| (k, m)));
    }
    let nodes = order.iter().map(|&i| GraphNode { path: files[i].path.clone(), title: files[i].title.clone(), hops: hops[&i] }).collect();
    Graph { nodes, edges, hidden: hops.len() - order.len() }
}

/// A plain-text occurrence of a note's title; COL/LEN in UTF-16 units, for the editor.
#[derive(Serialize, Debug, PartialEq)]
pub struct Mention {
    pub path: PathBuf,
    pub title: String,
    pub line: usize,
    pub col: usize,
    pub len: usize,
    pub text: String,
}

/// Places in other files where PATH's title appears (case-insensitive, whole word)
/// outside any link, on lines that don't already link to it. READ gives a file's text.
// ponytail: reads every file per call, like `search`; cache texts if it gets slow.
pub fn unlinked(files: &[Arc<OrgFile>], path: &Path) -> Vec<Mention> {
    let Some(f) = files.iter().find(|f| f.path == path) else { return vec![] };
    // The title and aliases, longest first so "Zettel box" wins over "Zettel".
    let mut names: Vec<&str> = std::iter::once(f.title.trim()).chain(f.aliases.iter().map(|a| a.trim())).filter(|n| !n.is_empty()).collect();
    names.sort_by_key(|n| std::cmp::Reverse(n.len()));
    if names.is_empty() {
        return vec![];
    }
    let word = |c: Option<char>| if c.is_some_and(|c| c.is_alphanumeric() || c == '_') { r"\b" } else { "" };
    let alts: Vec<String> = names.iter().map(|n| format!("{}{}{}", word(n.chars().next()), regex::escape(n), word(n.chars().last()))).collect();
    let Ok(re) = Regex::new(&format!("(?i){}", alts.join("|"))) else { return vec![] };
    let mut out = vec![];
    for o in files.iter().filter(|o| o.path != path) {
        for (i, l) in o.text.lines().enumerate() {
            let t = l.trim_start();
            if t.starts_with("#+") || t.starts_with(':') || o.links.iter().any(|k| k.line == i && points_to(o, k, f)) {
                continue;
            }
            let links: Vec<_> = ANY_LINK.find_iter(l).map(|m| m.range()).collect();
            for m in re.find_iter(l).filter(|m| !links.iter().any(|r| r.contains(&m.start()))) {
                let (col, len) = (l[..m.start()].encode_utf16().count(), m.as_str().encode_utf16().count());
                out.push(Mention { path: o.path.clone(), title: o.title.clone(), line: i, col, len, text: t.trim_end().to_string() });
            }
        }
    }
    out
}

/// TEXT with a file-level `:ID:` (reusing one in a top property drawer), and that ID.
pub fn ensure_id(text: &str) -> (String, String) {
    let lines: Vec<&str> = text.lines().collect();
    if lines.first().is_some_and(|l| l.trim().eq_ignore_ascii_case(":PROPERTIES:")) {
        for l in lines.iter().skip(1).take_while(|l| !l.trim().eq_ignore_ascii_case(":END:")) {
            if let Some(id) = l.trim().strip_prefix(":ID:") {
                return (text.to_string(), id.trim().to_string());
            }
        }
        let id = gen_id();
        let (head, rest) = text.split_once('\n').unwrap_or((text, ""));
        return (format!("{head}\n:ID:       {id}\n{rest}"), id);
    }
    let id = gen_id();
    (format!(":PROPERTIES:\n:ID:       {id}\n:END:\n{text}"), id)
}

/// Case-insensitive full-text search.
// ponytail: linear scan of every file per query; add an inverted index if it gets slow past ~10k notes.
pub fn search(files: &[Arc<OrgFile>], query: &str, limit: usize) -> Vec<Hit> {
    let q = query.to_lowercase();
    let mut out = vec![];
    for f in files {
        for (i, l) in f.text.lines().enumerate() {
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

    fn files(docs: &[(&str, &str)]) -> Vec<Arc<OrgFile>> {
        let kw = Kw { todo: vec![], done: vec![] };
        docs.iter().map(|(p, t)| Arc::new(parse(Path::new(p), t, &kw))).collect()
    }

    #[test]
    fn graph_neighbourhood() {
        // a -id-> b -file-> sub/c -id(heading)-> d -> e ; x links to a; y is unrelated.
        let fs = files(&[
            ("/n/a.org", ":PROPERTIES:\n:ID: a\n:END:\n#+title: A\n[[id:b][B]] [[id:b]] [[id:a]]\n"),
            ("/n/b.org", ":PROPERTIES:\n:ID: b\n:END:\n[[file:./sub/../sub/c.org::*X][C]]\n"),
            ("/n/sub/c.org", "[[id:dh]]\n"),
            ("/n/d.org", "* H\n:PROPERTIES:\n:ID: dh\n:END:\n[[file:e.org]]\n"),
            ("/n/e.org", ""),
            ("/n/x.org", "[[file:a.org]]\n"),
            ("/n/y.org", "[[id:nope]]\n"),
        ]);
        let g = graph(&fs, Path::new("/n/b.org"), 2);
        let names: Vec<_> = g.nodes.iter().map(|n| (n.title.as_str(), n.hops)).collect();
        assert_eq!(names, vec![("b", 0), ("A", 1), ("c", 1), ("x", 2), ("d", 2)]);
        assert_eq!(g.edges, vec![(0, 1), (0, 2), (1, 3), (2, 4)]);
        assert_eq!(graph(&fs, Path::new("/n/y.org"), 2).nodes.len(), 1);
        assert_eq!(graph(&fs, Path::new("/n/none.org"), 2), Graph::default());
        // A hub with more neighbours than fit: nearest kept, the rest counted.
        let docs: Vec<(String, String)> = (0..200).map(|i| (format!("/n/{i}.org"), if i == 0 { String::new() } else { "[[file:0.org]]\n".into() })).collect();
        let docs: Vec<(&str, &str)> = docs.iter().map(|(p, t)| (p.as_str(), t.as_str())).collect();
        let g = graph(&files(&docs), Path::new("/n/0.org"), 2);
        assert_eq!((g.nodes.len(), g.hidden, g.edges.len()), (MAX_NODES, 200 - MAX_NODES, MAX_NODES - 1));
        assert_eq!(resolve(Path::new("/n/sub/c.org"), "../d.org::foo"), PathBuf::from("/n/d.org"));
    }

    #[test]
    fn unlinked_mentions() {
        let docs = [
            ("/n/zk.org", ":PROPERTIES:\n:ID: zk\n:END:\n#+title: Zettel Box\nzettel box itself\n"),
            ("/n/m.org", "#+title: zettel box notes\nI keep a ZETTEL BOX. ö Zettel box!\nzettel boxes and myzettel box\nsee [[id:zk][Zettel Box]], also zettel box\n[[file:other.org][zettel box]] zettel box\n"),
        ];
        let got: Vec<_> = unlinked(&files(&docs), Path::new("/n/zk.org")).into_iter().map(|m| (m.line, m.col, m.len)).collect();
        // line 1 twice (ö is one UTF-16 unit), line 2 has no whole-word match, line 3 already links, line 4 only outside the link.
        assert_eq!(got, vec![(1, 9, 10), (1, 23, 10), (4, 31, 10)]);

        // Aliases match too, the longest name first.
        let docs = [
            ("/n/zk.org", ":PROPERTIES:\n:ID: zk\n:ROAM_ALIASES: \"slip box\" ZK\n:END:\n#+title: Zettelkasten\n"),
            ("/n/m.org", "My slip box, aka zk, not zkx. Zettelkasten!\n"),
        ];
        let f = files(&docs);
        assert_eq!(f[0].aliases, ["slip box", "ZK"]);
        let got: Vec<_> = unlinked(&f, Path::new("/n/zk.org")).into_iter().map(|m| (m.col, m.len)).collect();
        assert_eq!(got, vec![(3, 8), (17, 2), (30, 12)]);
    }

    #[test]
    fn ensures_ids() {
        let (t, id) = ensure_id(":PROPERTIES:\n:ID:  keep\n:END:\nx\n");
        assert_eq!((t.as_str(), id.as_str()), (":PROPERTIES:\n:ID:  keep\n:END:\nx\n", "keep"));
        let (t, id) = ensure_id(":PROPERTIES:\n:CATEGORY: c\n:END:\nx\n");
        assert_eq!(t, format!(":PROPERTIES:\n:ID:       {id}\n:CATEGORY: c\n:END:\nx\n"));
        let (t, id) = ensure_id("#+title: T\n");
        assert_eq!(t, format!(":PROPERTIES:\n:ID:       {id}\n:END:\n#+title: T\n"));
    }
}
