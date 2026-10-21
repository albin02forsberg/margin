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

/// Create `<dir>/YYYYMMDDHHMMSS-slug.org` (`-2`, `-3`… if taken) holding BODY
/// under the header, returning (path, id).
pub fn new_note(dir: &Path, title: &str, body: &str) -> std::io::Result<(PathBuf, String)> {
    let name = format!("{}-{}", chrono::Local::now().format("%Y%m%d%H%M%S"), slug(title));
    let path = (1..).map(|n| dir.join(if n == 1 { format!("{name}.org") } else { format!("{name}-{n}.org") })).find(|p| !p.exists()).unwrap();
    let id = write_note(&path, title)?;
    if !body.trim().is_empty() {
        use std::io::Write;
        std::fs::OpenOptions::new().append(true).open(&path)?.write_all(format!("{}\n", body.trim_end()).as_bytes())?;
    }
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

/// Path from folder DIR to TO (both normalised), with "/" separators, as org `file:` links expect.
fn rel_path(dir: &Path, to: &Path) -> String {
    let (a, b): (Vec<_>, Vec<_>) = (dir.components().collect(), to.components().collect());
    let i = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let up = std::iter::repeat_n("..".to_string(), a.len() - i);
    up.chain(b[i..].iter().map(|c| c.as_os_str().to_string_lossy().into_owned())).collect::<Vec<_>>().join("/")
}

/// TEXT, the note at NOTE that will live at AT, with every one-line `[[file:…]]` link that resolves to
/// OLD pointed at NEW and, when the note changes folder, its other relative links re-based so they
/// still resolve. Keeps each link's style (relative, `~/`, absolute), `::search` and description;
/// a same-folder rename only swaps the file name. None if nothing changed.
pub fn relink(text: &str, note: &Path, at: &Path, old: &Path, new: &Path) -> Option<String> {
    let (old_name, new_name) = (old.file_name()?.to_str()?, new.file_name()?.to_str()?);
    let (dir, moved) = (at.parent()?, note.parent() != at.parent());
    let mut changed = false;
    let out = crate::org::LINK.replace_all(text, |c: &regex::Captures| {
        let (all, t) = (c.get(0).unwrap(), c.get(2).unwrap());
        let keep = || all.as_str().to_string();
        let p = t.as_str().split("::").next().unwrap_or("");
        let target = resolve(note, t.as_str());
        let hits = target == old;
        if &c[1] != "file" || all.as_str().contains('\n') || !(hits || moved) {
            return keep();
        }
        let target = if hits { new.to_path_buf() } else { target };
        let cut = p.rfind(['/', '\\']).map_or(0, |i| i + 1);
        let path = if !moved && old.parent() == new.parent() && &p[cut..] == old_name {
            format!("{}{new_name}", &p[..cut])
        } else if p.starts_with(['~', '/', '\\']) || Path::new(p).is_absolute() {
            // Absolute styles don't depend on where the note is.
            if !hits {
                return keep();
            }
            match std::env::home_dir().filter(|_| p.starts_with("~/")).and_then(|h| target.strip_prefix(h).ok().map(|r| rel_path(Path::new(""), r))) {
                Some(r) => format!("~/{r}"),
                None => target.display().to_string(),
            }
        } else {
            let r = rel_path(dir, &target);
            if p.starts_with("./") && !r.starts_with("..") {
                format!("./{r}")
            } else {
                r
            }
        };
        if path == p {
            return keep();
        }
        changed = true;
        let (a, b) = (t.start() - all.start(), t.start() - all.start() + p.len());
        format!("{}{path}{}", &all.as_str()[..a], &all.as_str()[b..])
    });
    changed.then(|| out.into_owned())
}

/// Do A and B name the same existing file (e.g. differing only in case on a case-insensitive disk)?
fn same_file(a: &Path, b: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        matches!((std::fs::metadata(a), std::fs::metadata(b)), (Ok(x), Ok(y)) if (x.dev(), x.ino()) == (y.dev(), y.ino()))
    }
    #[cfg(not(unix))]
    {
        matches!((std::fs::canonicalize(a), std::fs::canonicalize(b)), (Ok(x), Ok(y)) if x == y)
    }
}

/// Rename note FROM to NAME in the same folder (see `move_to`).
pub fn rename(files: &[Arc<OrgFile>], from: &Path, name: &str) -> Result<(PathBuf, Vec<PathBuf>), String> {
    let name = name.trim();
    if name.is_empty() || name.starts_with('.') || name.contains(['/', '\\', '[', ']', ':']) {
        return Err(format!("“{name}” isn't a plain file name"));
    }
    let to = from.with_file_name(name);
    if !from.is_file() {
        return Err(format!("{} doesn't exist", from.display()));
    }
    if to == from {
        return Err("That's already its name.".into());
    }
    if to.exists() && !same_file(from, &to) {
        return Err(format!("{name} already exists"));
    }
    move_to(files, from, &to).map_err(|e| format!("Nothing renamed: {e}"))
}

/// Can notes move into DIR? Only under NOTES, and not into a dot folder (unscanned, like `.trash`), the attachments folder
/// ATTACH or an IMAGES folder.
pub fn folder_ok(notes: &Path, attach: &str, images: &str, dir: &Path) -> Result<(), String> {
    let r = dir.strip_prefix(notes).map_err(|_| format!("{} isn't in the notes folder", dir.display()))?;
    let hidden = r.components().any(|c| !matches!(c, Component::Normal(n) if !n.to_string_lossy().starts_with('.')));
    let in_images = !images.is_empty() && r.ancestors().any(|a| a.ends_with(images));
    if hidden || (!attach.is_empty() && r.starts_with(attach)) || in_images {
        return Err(format!("Notes can't go in {}", dir.display()));
    }
    Ok(())
}

/// Move note FROM into folder DIR (created if missing), keeping links to and from it working (see `move_to`).
pub fn move_note(files: &[Arc<OrgFile>], from: &Path, dir: &Path) -> Result<(PathBuf, Vec<PathBuf>), String> {
    let to = dir.join(from.file_name().ok_or("not a note")?);
    if !from.is_file() {
        return Err(format!("{} doesn't exist", from.display()));
    }
    if from.parent() == Some(dir) {
        return Err("It's already in that folder.".into());
    }
    if to.exists() {
        return Err(format!("{} already exists", to.display()));
    }
    let made = !dir.exists();
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    move_to(files, from, &to).map_err(|e| {
        if made {
            let _ = std::fs::remove_dir(dir);
        }
        format!("Nothing moved: {e}")
    })
}

/// Move note FROM to TO, first pointing the `[[file:]]` links in FILES at it and re-basing its own.
/// All or nothing: if a write or the move fails, the notes already rewritten are put back.
/// Returns the new path and the notes whose links changed.
fn move_to(files: &[Arc<OrgFile>], from: &Path, to: &Path) -> Result<(PathBuf, Vec<PathBuf>), String> {
    let links_here = |f: &OrgFile| f.links.iter().any(|l| l.file && resolve(&f.path, &l.target) == from);
    let mut notes: Vec<&Path> = files.iter().filter(|f| f.path == from || links_here(f)).map(|f| f.path.as_path()).collect();
    if !notes.contains(&from) {
        notes.push(from);
    }
    let mut edits = vec![];
    for p in notes {
        let text = std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
        // One edit for the moved note: its links to itself and to others, written as seen from TO.
        if let Some(new) = relink(&text, p, if p == from { to } else { p }, from, to) {
            edits.push((p.to_path_buf(), text, new));
        }
    }
    let mut done = 0;
    let res = edits
        .iter()
        .try_for_each(|(p, _, new)| {
            crate::config::write_atomic(p, new).map_err(|e| format!("{}: {e}", p.display()))?;
            done += 1;
            Ok::<_, String>(())
        })
        .and_then(|_| std::fs::rename(from, to).map_err(|e| e.to_string()));
    if let Err(e) = res {
        for (p, old, _) in &edits[..done] {
            let _ = crate::config::write_atomic(p, old);
        }
        return Err(e);
    }
    // The moved note's own links now live at TO.
    let changed = edits.into_iter().map(|(p, ..)| if p == from { to.to_path_buf() } else { p }).collect();
    Ok((to.to_path_buf(), changed))
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

/// For each of FILES, the indexes of the files it links to or is linked from (itself excluded).
pub fn adjacency(files: &[Arc<OrgFile>]) -> Vec<BTreeSet<usize>> {
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
    adj
}

/// The files that no other file links to and that link to none, except those SKIP says to leave out.
pub fn orphans(files: &[Arc<OrgFile>], skip: impl Fn(&Path) -> bool) -> Vec<Arc<OrgFile>> {
    let adj = adjacency(files);
    files.iter().zip(adj).filter(|(f, a)| a.is_empty() && !skip(&f.path)).map(|(f, _)| f.clone()).collect()
}

/// (file, line, title, tags) of one tagged place.
type TagSite<'a> = (&'a OrgFile, usize, &'a str, Vec<&'a str>);

/// Tagged places in notes: the file's `#+FILETAGS` (line 0) and each non-task heading's own tags
/// (not inherited ones). `ARCHIVE` isn't a browsable tag.
fn tag_sites<'a>(files: &'a [Arc<OrgFile>], skip: &'a impl Fn(&Path) -> bool) -> impl Iterator<Item = TagSite<'a>> {
    let own = |t: &'a [String]| t.iter().map(String::as_str).filter(|t| *t != "ARCHIVE").collect::<Vec<_>>();
    files.iter().filter(move |f| !skip(&f.path)).flat_map(move |f| {
        let f: &'a OrgFile = f;
        std::iter::once((f, 0, f.title.as_str(), own(f.tags.as_slice())))
            .chain(f.headlines.iter().filter(|h| h.keyword.is_none()).map(move |h| (f, h.line, h.title.as_str(), own(h.tags.as_slice()))))
            .filter(|s| !s.3.is_empty())
    })
}

/// Tags used in notes with how many places carry each, matched case-insensitively (first spelling shown),
/// most used first, then by name.
pub fn tags(files: &[Arc<OrgFile>], skip: impl Fn(&Path) -> bool) -> Vec<(String, usize)> {
    let mut m: HashMap<String, (String, usize)> = HashMap::new();
    for (_, _, _, ts) in tag_sites(files, &skip) {
        let mut seen = BTreeSet::new();
        for t in ts {
            if seen.insert(t.to_lowercase()) {
                m.entry(t.to_lowercase()).or_insert_with(|| (t.to_string(), 0)).1 += 1;
            }
        }
    }
    let mut v: Vec<_> = m.into_values().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase())));
    v
}

/// Places tagged TAG (any case) as (path, line, title); line 0 is a file tag.
pub fn tagged(files: &[Arc<OrgFile>], tag: &str, skip: impl Fn(&Path) -> bool) -> Vec<(PathBuf, usize, String)> {
    let tag = tag.to_lowercase();
    tag_sites(files, &skip).filter(|s| s.3.iter().any(|t| t.to_lowercase() == tag)).map(|(f, l, t, _)| (f.path.clone(), l, t.to_string())).collect()
}

pub fn graph(files: &[Arc<OrgFile>], path: &Path, depth: usize) -> Graph {
    let Some(start) = files.iter().position(|f| f.path == path) else { return Graph::default() };
    let adj = adjacency(files);
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
/// A parsed search query: words/phrases that must match, `-word` ones that must not, `in:folder` filters.
#[derive(Default)]
struct Query {
    include: Vec<String>,
    exclude: Vec<String>,
    folders: Vec<String>,
}

/// Lowercased query: whitespace-separated words, a `"quoted phrase"` kept whole (an unbalanced quote
/// takes the rest), `-word` / `-"phrase"` to exclude, `in:folder` / `in:"my folder"` to restrict.
/// A lone `-` is an ordinary word.
fn parse_query(query: &str) -> Query {
    let mut q = Query::default();
    let low = query.to_lowercase();
    let (mut chars, mut word) = (low.chars().peekable(), String::new());
    // Files a finished token under its prefix: "-" exclude, "in:" folder, else include.
    let mut flush = |w: &mut String, quoted: bool| {
        let t = std::mem::take(w);
        if let Some(r) = t.strip_prefix('-').filter(|r| !r.is_empty() || quoted) {
            q.exclude.push(r.to_string());
        } else if let Some(r) = t.strip_prefix("in:") {
            q.folders.push(r.to_string());
        } else {
            q.include.push(t);
        }
    };
    while let Some(c) = chars.next() {
        if c == '"' {
            // Quoted segment: whole phrase, glued to any "-" / "in:" typed right before it.
            let mut ph = String::new();
            for c in chars.by_ref() {
                if c == '"' {
                    break;
                }
                ph.push(c);
            }
            word.push_str(&ph);
            flush(&mut word, true);
        } else if c.is_whitespace() {
            if !word.is_empty() {
                flush(&mut word, false);
            }
        } else {
            word.push(c);
        }
    }
    if !word.is_empty() {
        flush(&mut word, false);
    }
    for v in [&mut q.include, &mut q.exclude, &mut q.folders] {
        v.retain(|t| !t.trim().is_empty());
    }
    q
}

/// Lines containing every term of `query` (case-insensitive, any order) and no `-excluded` one, only in
/// notes under a folder named by `in:` (if any; under `root`); none without a positive term.
pub fn search(files: &[Arc<OrgFile>], root: &Path, query: &str, limit: usize) -> Vec<Hit> {
    let q = parse_query(query);
    let mut out = vec![];
    if q.include.is_empty() {
        return out;
    }
    for f in files {
        if !q.folders.is_empty() {
            let Some(dirs) = f.path.strip_prefix(root).ok().and_then(Path::parent) else { continue };
            let names: Vec<String> = dirs.components().map(|c| c.as_os_str().to_string_lossy().to_lowercase()).collect();
            if !q.folders.iter().any(|d| names.iter().any(|n| n.starts_with(d.as_str()))) {
                continue;
            }
        }
        for (i, l) in f.text.lines().enumerate() {
            let low = l.to_lowercase();
            if q.include.iter().all(|t| low.contains(t.as_str())) && !q.exclude.iter().any(|t| low.contains(t.as_str())) {
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
    fn search_matches_every_term() {
        let kw = Kw { todo: vec![], done: vec![] };
        let f = Arc::new(parse(Path::new("/n/a.org"), "#+title: A\nMeeting about Alpha\ndraft of the report\nreport draft\nalpha only\n", &kw));
        let files = [f];
        let lines = |q: &str, limit| search(&files, Path::new("/n"), q, limit).iter().map(|h| h.line).collect::<Vec<_>>();
        assert_eq!(lines("alpha meeting", 10), vec![1]);
        assert_eq!(lines("ALPHA Meeting", 10), vec![1]);
        assert_eq!(lines("report draft", 10), vec![2, 3]);
        assert_eq!(lines("\"report draft\"", 10), vec![3]);
        assert_eq!(lines("alpha budget", 10), Vec::<usize>::new());
        assert_eq!(lines("only \"alpha ", 10), vec![4]);
        assert_eq!(lines("  \t ", 10), Vec::<usize>::new());
        assert_eq!(lines("\"\"", 10), Vec::<usize>::new());
        assert_eq!(lines("report", 1), vec![2]);
        assert_eq!(lines("report -alpha", 10), vec![2, 3]);
        assert_eq!(lines("report -draft", 10), Vec::<usize>::new());
        assert_eq!(lines("report -\"report draft\"", 10), vec![2]);
        assert_eq!(lines("-alpha", 10), Vec::<usize>::new());
        assert_eq!(lines("in:x", 10), Vec::<usize>::new());
    }

    #[test]
    fn search_filters_by_folder() {
        let kw = Kw { todo: vec![], done: vec![] };
        let mk = |p: &str| Arc::new(parse(Path::new(p), "meeting about stand up\nmeeting alone\n- a - b\n", &kw));
        let files = [mk("/n/projects.org"), mk("/n/Projects/alpha/x.org"), mk("/n/journal/j.org"), mk("/n/my folder/m.org"), mk("/other/o.org")];
        let hits = |q: &str| search(&files, Path::new("/n"), q, 100).iter().map(|h| h.path.to_string_lossy().into_owned()).collect::<std::collections::BTreeSet<_>>().into_iter().collect::<Vec<_>>();
        assert_eq!(hits("meeting in:proj"), ["/n/Projects/alpha/x.org"]);
        assert_eq!(hits("meeting in:ALPHA"), ["/n/Projects/alpha/x.org"]);
        assert_eq!(hits("meeting in:journal in:alpha"), ["/n/Projects/alpha/x.org", "/n/journal/j.org"]);
        assert_eq!(hits("meeting in:\"my folder\""), ["/n/my folder/m.org"]);
        assert_eq!(hits("meeting in:other"), Vec::<String>::new());
        assert_eq!(hits("meeting").len(), 5);
        let lines = |q: &str| search(&files[..1], Path::new("/n"), q, 100).iter().map(|h| h.line).collect::<Vec<_>>();
        assert_eq!(lines("meeting -\"stand up\""), vec![1]);
        assert_eq!(lines("meeting -"), Vec::<usize>::new());
        assert_eq!(lines("a - b"), vec![2]);
    }

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
    fn orphans_have_no_links_in_or_out() {
        let fs = files(&[
            ("/n/a.org", ":PROPERTIES:\n:ID: a\n:END:\n[[file:b.org]]\n"),
            ("/n/b.org", ""),
            ("/n/lone.org", "nothing\n"),
            ("/n/self.org", ":PROPERTIES:\n:ID: s\n:END:\n[[id:s]] [[file:self.org]]\n"),
            ("/n/dead.org", "[[file:gone.org]] [[id:nope]]\n"),
            ("/n/daily/d.org", ""),
        ]);
        let o = orphans(&fs, |p| p.starts_with("/n/daily"));
        assert_eq!(o.iter().map(|f| f.path.to_str().unwrap()).collect::<Vec<_>>(), ["/n/lone.org", "/n/self.org", "/n/dead.org"]);
    }

    #[test]
    fn notes_tags_and_tagged() {
        let kw = Kw { todo: vec!["TODO".into()], done: vec!["DONE".into()] };
        let fs: Vec<_> = [
            ("/n/acme.org", "#+title: Acme\n#+FILETAGS: :client:meeting:\n* Call :Client:\n* TODO Invoice :money:\n* Plain\n"),
            ("/n/bare.org", "#+filetags: :CLIENT:\nno headings\n"),
            ("/n/old.org", "* Gone :ARCHIVE:idea:\n"),
            ("/n/daily/d.org", "#+filetags: :client:\n"),
        ]
        .iter()
        .map(|(p, t)| Arc::new(parse(Path::new(p), t, &kw)))
        .collect();
        let skip = |p: &Path| p.starts_with("/n/daily");
        assert_eq!(fs[1].tags, ["CLIENT"]); // heading-less note keeps its file tags
        // Own tags only: "Plain" inherits client/meeting but isn't counted; TODO headings and ARCHIVE are left out.
        assert_eq!(tags(&fs, skip), [("client".to_string(), 3), ("idea".into(), 1), ("meeting".into(), 1)]);
        let t = tagged(&fs, "CLIENT", skip);
        assert_eq!(t.iter().map(|(p, l, n)| (p.to_str().unwrap(), *l, n.as_str())).collect::<Vec<_>>(), [("/n/acme.org", 0, "Acme"), ("/n/acme.org", 2, "Call"), ("/n/bare.org", 0, "bare")]);
        assert!(tagged(&fs, "money", skip).is_empty());
        assert!(tagged(&fs, "archive", skip).is_empty());
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
    fn relinks_file_links() {
        let old = Path::new("/n/sub/c.org");
        let new = Path::new("/n/sub/d.org");
        let r = |note: &str, text: &str| relink(text, Path::new(note), Path::new(note), old, new);
        assert_eq!(r("/n/sub/x.org", "see [[file:c.org]] and [[file:c.org::*H][C]]!").as_deref(), Some("see [[file:d.org]] and [[file:d.org::*H][C]]!"));
        assert_eq!(r("/n/a.org", "[[file:sub/c.org][c.org]] [[file:./sub/../sub/c.org::12]]").as_deref(), Some("[[file:sub/d.org][c.org]] [[file:./sub/../sub/d.org::12]]"));
        assert_eq!(r("/n/sub/deep/y.org", "[[file:../c.org]]").as_deref(), Some("[[file:../d.org]]"));
        assert_eq!(r("/z.org", "[[file:/n/sub/c.org]]").as_deref(), Some("[[file:/n/sub/d.org]]"));
        // Other targets, id links, look-alikes and links split over lines stay as they are.
        assert_eq!(r("/n/a.org", "[[file:c.org]] [[id:c.org]] [[file:sub/c.orgx]] [[file:sub/xc.org]] file:sub/c.org [[file:sub/\nc.org]]"), None);
        if let Some(home) = std::env::home_dir() {
            let old = home.join("n/c.org");
            let a = Path::new("/x/a.org");
            assert_eq!(relink("[[file:~/n/c.org]]", a, a, &old, &home.join("n/d.org")).as_deref(), Some("[[file:~/n/d.org]]"));
            // Moving the target keeps `~/`; moving the note leaves `~/` links to others alone.
            let to = home.join("n/sub/c.org");
            assert_eq!(relink("[[file:~/n/c.org][C]]", a, a, &old, &to).as_deref(), Some("[[file:~/n/sub/c.org][C]]"));
            assert_eq!(relink("[[file:~/x.org]]", &old, &to, &old, &to), None);
        }
    }

    #[test]
    fn relinks_moves() {
        let (old, new) = (Path::new("/n/c.org"), Path::new("/n/projects/c.org"));
        let r = |note: &str, text: &str| relink(text, Path::new(note), Path::new(note), old, new);
        // Links to the moved note, from the root, a sibling folder and below.
        assert_eq!(r("/n/a.org", "[[file:c.org::*H][C]] [[file:./c.org]]").as_deref(), Some("[[file:projects/c.org::*H][C]] [[file:./projects/c.org]]"));
        assert_eq!(r("/n/daily/d.org", "[[file:../c.org]]").as_deref(), Some("[[file:../projects/c.org]]"));
        assert_eq!(r("/n/projects/p.org", "[[file:../c.org]]").as_deref(), Some("[[file:c.org]]"));
        assert_eq!(r("/z.org", "[[file:/n/c.org]] [[id:c.org]] [[file:x.org]]").as_deref(), Some("[[file:/n/projects/c.org]] [[id:c.org]] [[file:x.org]]"));
        assert_eq!(r("/n/a.org", "[[file:x.org]] [[file:/n/x.org]]"), None);

        // The moved note's own links: relative ones (attachments too) re-based, absolute and id ones untouched.
        let own = "[[file:a.org]] [[file:attachments/c/x.png]] [[file:daily/d.org::12][D]] [[file:/n/a.org]] [[id:abc]] [[file:c.org::*Self]]";
        let got = relink(own, old, new, old, new).unwrap();
        assert_eq!(got, "[[file:../a.org]] [[file:../attachments/c/x.png]] [[file:../daily/d.org::12][D]] [[file:/n/a.org]] [[id:abc]] [[file:c.org::*Self]]");
        // ...and back out of the folder, or across to a sibling.
        assert_eq!(relink(&got, new, old, new, old).as_deref(), Some(own));
        let side = Path::new("/n/archive/c.org");
        assert_eq!(relink("[[file:../a.org]] [[file:p.org]]", new, side, new, side).as_deref(), Some("[[file:../a.org]] [[file:../projects/p.org]]"));
    }

    #[test]
    fn note_folders() {
        let n = Path::new("/n");
        assert!(folder_ok(n, "attachments", "images", Path::new("/n")).is_ok() && folder_ok(n, "attachments", "images", Path::new("/n/projects/x")).is_ok());
        for bad in ["/n/.trash", "/n/a/.hidden", "/n/attachments", "/n/attachments/x", "/elsewhere", "/n/../etc", "/n/images", "/n/work/images/x"] {
            assert!(folder_ok(n, "attachments", "images", Path::new(bad)).is_err(), "{bad}");
        }
        assert!(folder_ok(n, "files/att", "", Path::new("/n/files/att/x")).is_err() && folder_ok(n, "files/att", "", Path::new("/n/files")).is_ok());
        assert!(folder_ok(n, "attachments", "", Path::new("/n/images")).is_ok());
    }

    #[test]
    fn new_note_names_are_unique() {
        let dir = std::env::temp_dir().join(format!("margin-new-{}", gen_id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (a, _) = new_note(&dir, "Same", "").unwrap();
        let (b, id) = new_note(&dir, "Same", "* Child\n").unwrap();
        assert_ne!(a, b);
        assert_eq!(std::fs::read_to_string(&b).unwrap(), format!(":PROPERTIES:\n:ID:       {id}\n:END:\n#+title: Same\n\n* Child\n"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn renames_notes() {
        let dir = std::env::temp_dir().join(format!("margin-rename-{}", gen_id()));
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        let (a, c, x, y) = (dir.join("a.org"), dir.join("sub/c.org"), dir.join("sub/x.org"), dir.join("y.org"));
        let docs = [(&a, "[[file:sub/c.org][C]]\n"), (&c, "self [[file:c.org::*H]]\n"), (&x, "[[file:c.org]] [[file:a.org]]\n"), (&y, "[[file:a.org]]\n")];
        for (p, t) in docs {
            std::fs::write(p, t).unwrap();
        }
        let kw = Kw { todo: vec![], done: vec![] };
        let fs: Vec<_> = docs.iter().map(|(p, t)| Arc::new(parse(p, t, &kw))).collect();
        assert!(rename(&fs, &c, "x.org").unwrap_err().contains("already exists"));
        assert!(rename(&fs, &c, "../d.org").is_err() && rename(&fs, &c, "a]b.org").is_err() && rename(&fs, &c, "c.org").is_err());
        assert!(rename(&fs, &dir.join("none.org"), "e.org").is_err());

        #[cfg(unix)]
        {
            // A write that fails puts back the notes already rewritten and renames nothing.
            use std::os::unix::fs::PermissionsExt;
            let ro = dir.join("ro");
            std::fs::create_dir(&ro).unwrap();
            let z = ro.join("z.org");
            std::fs::write(&z, "[[file:../sub/c.org]]\n").unwrap();
            std::fs::set_permissions(&ro, std::fs::Permissions::from_mode(0o555)).unwrap();
            let mut fs2 = fs.clone();
            fs2.push(Arc::new(parse(&z, "[[file:../sub/c.org]]\n", &kw)));
            let e = rename(&fs2, &c, "d.org");
            std::fs::set_permissions(&ro, std::fs::Permissions::from_mode(0o755)).unwrap();
            assert!(e.unwrap_err().starts_with("Nothing renamed"));
            assert!(c.exists() && !dir.join("sub/d.org").exists());
            for (p, t) in docs {
                assert_eq!(std::fs::read_to_string(p).unwrap(), t);
            }
        }

        let (to, changed) = rename(&fs, &c, " d.org ").unwrap();
        let d = dir.join("sub/d.org");
        assert_eq!((to, changed), (d.clone(), vec![a.clone(), d.clone(), x.clone()]));
        assert!(!c.exists());
        assert_eq!(std::fs::read_to_string(&a).unwrap(), "[[file:sub/d.org][C]]\n");
        assert_eq!(std::fs::read_to_string(&d).unwrap(), "self [[file:d.org::*H]]\n");
        assert_eq!(std::fs::read_to_string(&x).unwrap(), "[[file:d.org]] [[file:a.org]]\n");
        assert_eq!(std::fs::read_to_string(&y).unwrap(), "[[file:a.org]]\n");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn moves_notes() {
        let dir = std::env::temp_dir().join(format!("margin-move-{}", gen_id()));
        std::fs::create_dir_all(dir.join("daily")).unwrap();
        let (a, c, d) = (dir.join("a.org"), dir.join("c.org"), dir.join("daily/d.org"));
        let docs = [(&a, "[[file:c.org][C]]\n"), (&c, "[[file:a.org]] [[file:attachments/c/x.png]] [[file:c.org::*H]]\n"), (&d, "[[file:../a.org]]\n")];
        for (p, t) in docs {
            std::fs::write(p, t).unwrap();
        }
        let kw = Kw { todo: vec![], done: vec![] };
        let fs: Vec<_> = docs.iter().map(|(p, t)| Arc::new(parse(p, t, &kw))).collect();
        assert!(move_note(&fs, &c, &dir).unwrap_err().contains("already in"));
        std::fs::write(dir.join("daily/c.org"), "").unwrap();
        assert!(move_note(&fs, &c, &dir.join("daily")).unwrap_err().contains("already exists"));
        assert!(move_note(&fs, &dir.join("none.org"), &dir.join("daily")).is_err());

        let p = dir.join("projects/2026");
        let (to, changed) = move_note(&fs, &c, &p).unwrap();
        let n = p.join("c.org");
        assert_eq!((to, changed), (n.clone(), vec![a.clone(), n.clone()]));
        assert!(!c.exists());
        assert_eq!(std::fs::read_to_string(&a).unwrap(), "[[file:projects/2026/c.org][C]]\n");
        assert_eq!(std::fs::read_to_string(&n).unwrap(), "[[file:../../a.org]] [[file:../../attachments/c/x.png]] [[file:c.org::*H]]\n");
        assert_eq!(std::fs::read_to_string(&d).unwrap(), "[[file:../a.org]]\n");
        std::fs::remove_dir_all(&dir).unwrap();
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
