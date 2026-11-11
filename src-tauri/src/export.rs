// Org → HTML / Markdown for exporting notes and reports. A small block parser
// (headings, paragraphs, lists, tables, blocks) plus inline markup; drawers,
// planning lines, comments and #+ keywords other than title/subtitle are dropped.

use crate::org::{self, Kw, OrgFile};
use regex::Regex;
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::LazyLock;

static LINK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\[\[([^\]]+)\](?:\[([^\]]*)\])?\]").unwrap());
static ITEM: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\s*)([-+*]|\d+[.)])\s+(?:\[([ Xx-])\]\s+)?(.*)$").unwrap());
static BLOCK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^\s*#\+begin_(\w+)\s*(\S*)").unwrap());
static DRAWER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*:[A-Za-z_]+:\s*$").unwrap());
/// Script, event-handler attribute or javascript: URL: an SVG with one is never embedded.
static UNSAFE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)<script|javascript:|<[^>]*\son[a-z]+\s*=").unwrap());
/// A URL scheme (two letters or more, so `C:\x` stays a path).
static SCHEME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[A-Za-z][A-Za-z0-9+.-]+:").unwrap());
static CUSTOM_ID: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^\s*:CUSTOM_ID:\s*(\S+)\s*$").unwrap());
/// `<<target>>` (a `<<<radio>>>` is checked for by the caller).
static TARGET: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^<<([^<>\n]+)>>").unwrap());
/// A bare link target that is a file, not a same-page fuzzy target: scheme, path separator, extension or `::search`.
static FILE_LIKE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^[a-z][\w+.-]*:|[/\\]|\.\w+$|::").unwrap());

#[derive(Debug, PartialEq)]
pub struct Heading {
    level: usize,
    kw: Option<(String, bool)>, // (keyword, done)
    prio: Option<String>,
    title: String,
    tags: Vec<String>,
    /// `:CUSTOM_ID:` of the property drawer right under it.
    custom_id: Option<String>,
    /// Anchor id: the custom id, else a slug of the title (unique in the page; set by `link_anchors`).
    id: String,
}

#[derive(Debug, PartialEq)]
pub struct Item {
    check: Option<char>,
    /// `- term :: text` (description list).
    term: Option<String>,
    text: String,
    children: Vec<Block>,
}

#[derive(Debug, PartialEq)]
pub enum Block {
    Heading(Heading),
    Para(String),
    List(bool, Vec<Item>), // (ordered, items)
    /// Row groups split by separator lines; with more than one, the first is the header.
    Table(Vec<Vec<Vec<String>>>),
    /// (kind, language, raw body) for src/example/verse/… blocks.
    Pre(String, String, String),
    Quote(Vec<Block>),
    Rule,
    /// `#+NAME: name`: an anchor for links to it.
    Anchor(String),
}

/// Where an export's links go. IDS: id → file name of the exported page holding it (or its `#p-N` section in a
/// combined page). SITE (a published site, #1037): local file (`norm`ed) → its page or copied asset; there any
/// other local link is plain text and images aren't embedded, so no `file:///` path leaks.
#[derive(Default)]
pub struct Ids {
    pub ids: HashMap<String, String>,
    pub site: Option<HashMap<PathBuf, String>>,
}

impl Ids {
    pub fn new() -> Ids {
        Ids::default()
    }
}

pub struct Doc {
    pub title: Option<String>,
    subtitle: Option<String>,
    blocks: Vec<Block>,
    /// Rendered mermaid SVGs by `diagram_key` (#942), embedded by the HTML export; empty from `parse`.
    diagrams: HashMap<String, String>,
    /// Same-page link keys (`*title`, `#custom-id`, bare fuzzy target) → anchor id (see `link_anchors`).
    anchors: HashMap<String, String>,
    /// Prepended to every anchor id (`p-N-` for a note in a combined page).
    prefix: String,
}

impl Doc {
    pub fn with_diagrams(self, diagrams: HashMap<String, String>) -> Doc {
        Doc { diagrams, ..self }
    }
}

/// A diagram's code as its SVG's key, the same as `diagramKey` in babel.ts: each line trimmed and its
/// `,*` / `,#+` escape undone, blank lines dropped.
pub fn diagram_key(code: &str) -> String {
    code.lines().map(|l| l.trim()).map(|l| if l.starts_with(",*") || l.starts_with(",#+") { &l[1..] } else { l }).filter(|l| !l.is_empty()).collect::<Vec<_>>().join("\n")
}

fn indent(l: &str) -> usize {
    l.len() - l.trim_start().len()
}

/// Expands `#+LINK: name template` abbreviations in `[[name:tag]]` links (first definition wins; `file`, `id`, `http`, `https`, `mailto` can't be redefined). Mirrors `expandLink` in links.ts.
fn expand_links(text: &str) -> String {
    static DEF: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?im)^[ \t]*#\+link:[ \t]+([^\s:\[\]]+)[ \t]+(\S.*?)\s*$").unwrap());
    static REF: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[\[([^\s:\[\]]+):([^\]]*)\]").unwrap());
    // Lines inside src/example blocks (an unterminated one runs to the end) are code: no definitions, no rewriting; same rule as radio.ts.
    let mut in_code = false;
    let lines: Vec<(&str, bool)> = text
        .split_inclusive('\n')
        .map(|l| {
            let t = l.trim_start().to_lowercase();
            let code = in_code || t.starts_with("#+begin_src") || t.starts_with("#+begin_example");
            if code {
                in_code = !(in_code && (t.starts_with("#+end_src") || t.starts_with("#+end_example")));
            }
            (l, code)
        })
        .collect();
    let mut ab: HashMap<String, String> = HashMap::new();
    for (l, _) in lines.iter().filter(|(_, code)| !code) {
        if let Some(c) = DEF.captures(l) {
            if !["file", "id", "http", "https", "mailto"].contains(&&c[1]) {
                ab.entry(c[1].to_string()).or_insert_with(|| c[2].to_string());
            }
        }
    }
    if ab.is_empty() {
        return text.to_string();
    }
    let sub = |l: &str| REF.replace_all(l, |c: &regex::Captures| match ab.get(&c[1]) {
        Some(t) if t.contains("%s") || t.contains("%h") => {
            let enc: String = c[2].bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect();
            format!("[[{}]", t.replace("%h", &enc).replace("%s", &c[2]))
        }
        Some(t) => format!("[[{t}{}]", &c[2]),
        None => c[0].to_string(),
    }).into_owned();
    lines.iter().map(|&(l, code)| if code { l.to_string() } else { sub(l) }).collect()
}

pub fn parse(text: &str, kw: &Kw) -> Doc {
    let text = expand_links(&crate::crypt::strip(text));
    let lines: Vec<&str> = text.lines().collect();
    let mut d = Doc { title: None, subtitle: None, blocks: vec![], diagrams: HashMap::new(), anchors: HashMap::new(), prefix: String::new() };
    let b = blocks(&lines, kw, &mut d);
    d.blocks = b;
    link_anchors(&mut d);
    d
}

/// Kebab-case slug of S (letters and digits, lowercased) for anchor ids; `section` when it has none.
fn slug(s: &str) -> String {
    let s = s.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect::<Vec<_>>().join("-").to_lowercase();
    if s.is_empty() { "section".into() } else { s }
}

fn target_id(t: &str) -> String {
    format!("target-{}", slug(t))
}

fn name_id(n: &str) -> String {
    format!("name-{}", slug(n))
}

/// BS's headings, nested ones (quotes, list items) included, in document order.
fn headings(bs: &mut [Block]) -> Vec<&mut Heading> {
    let mut o = vec![];
    for b in bs {
        match b {
            Block::Heading(h) => o.push(h),
            Block::Quote(inner) => o.extend(headings(inner)),
            Block::List(_, items) => items.iter_mut().for_each(|it| o.extend(headings(&mut it.children))),
            _ => {}
        }
    }
    o
}

/// Gives D's headings their ids and fills `anchors`: `*title` and `#custom-id` reach headings; a bare target
/// reaches the first `<<target>>`, else `#+NAME`, else heading of that name (`fuzzyTarget` in links.ts).
/// Built from what is exported, so noexport subtrees and src blocks never resolve.
fn link_anchors(d: &mut Doc) {
    let norm = crate::notes::heading_norm;
    let (mut targets, mut names) = (vec![], vec![]);
    let mut add = |s: &str| each_span(&spans(s), &mut |sp| if let Span::Target(t) = sp { targets.push(t.clone()) });
    d.title.iter().chain(&d.subtitle).for_each(|t| add(t));
    each_text(&d.blocks, &mut add);
    each_block(&d.blocks, &mut |b| if let Block::Anchor(n) = b { names.push(n.clone()) });
    let mut keys: HashMap<String, String> = HashMap::new();
    for (t, id) in targets.iter().map(|t| (t, target_id(t))).chain(names.iter().map(|n| (n, name_id(n)))) {
        keys.entry(norm(t)).or_insert(id);
    }
    let mut used: std::collections::HashSet<String> = targets.iter().map(|t| target_id(t)).chain(names.iter().map(|n| name_id(n))).collect();
    for h in headings(&mut d.blocks) {
        h.id = match &h.custom_id {
            Some(c) => c.clone(),
            None => {
                let base = slug(&crate::notes::heading_bare(&plain(&spans(&h.title))));
                std::iter::once(base.clone()).chain((2..).map(|n| format!("{base}-{n}"))).find(|s| !used.contains(s)).unwrap()
            }
        };
        used.insert(h.id.clone());
        let t = norm(&h.title);
        keys.entry(format!("*{t}")).or_insert(h.id.clone());
        keys.entry(t).or_insert(h.id.clone());
        if let Some(c) = &h.custom_id {
            keys.entry(format!("#{c}")).or_insert(h.id.clone());
        }
    }
    d.anchors = keys;
}

fn blocks(lines: &[&str], kw: &Kw, doc: &mut Doc) -> Vec<Block> {
    let mut out = vec![];
    let mut para: Vec<&str> = vec![];
    let mut i = 0;
    let flush = |para: &mut Vec<&str>, out: &mut Vec<Block>| {
        if !para.is_empty() {
            out.push(Block::Para(para.join(" ")));
            para.clear();
        }
    };
    while i < lines.len() {
        let l = lines[i];
        let t = l.trim();
        let lower = t.to_lowercase();
        if let Some((level, k, prio, title, tags)) = org::split_headline(l, kw) {
            flush(&mut para, &mut out);
            // `:noexport:` / `COMMENT` subtrees stay out (org's default exclusion); `#+begin_*` ranges are jumped so a `* ` line in a src block doesn't end the skip.
            if tags.iter().any(|t| t.eq_ignore_ascii_case("noexport")) || title == "COMMENT" || title.starts_with("COMMENT ") {
                i += 1;
                while i < lines.len() {
                    if let Some(c) = BLOCK.captures(lines[i]) {
                        let end = format!("#+end_{}", c[1].to_lowercase());
                        while i + 1 < lines.len() && lines[i + 1].trim().to_lowercase() != end {
                            i += 1;
                        }
                        i += 2;
                        continue;
                    }
                    if org::split_headline(lines[i], kw).is_some_and(|h| h.0 <= level) {
                        break;
                    }
                    i += 1;
                }
                continue;
            }
            out.push(Block::Heading(Heading { level, kw: k.map(|k| (k.clone(), kw.is_done(&k))), prio, title, tags, custom_id: None, id: String::new() }));
        } else if t.is_empty() || org::is_planning(l) || t == "#" || t.starts_with("# ") {
            flush(&mut para, &mut out);
        } else if DRAWER.is_match(l) && lower != ":end:" {
            flush(&mut para, &mut out);
            let start = i;
            while i < lines.len() && lines[i].trim().to_lowercase() != ":end:" {
                i += 1;
            }
            if let (":properties:", Some(Block::Heading(h))) = (lower.as_str(), out.last_mut()) {
                h.custom_id = lines[start..i].iter().find_map(|l| CUSTOM_ID.captures(l)).map(|c| c[1].to_string());
            }
        } else if let Some(c) = BLOCK.captures(l) {
            flush(&mut para, &mut out);
            let kind = c[1].to_lowercase();
            let end = format!("#+end_{kind}");
            let start = i + 1;
            while i + 1 < lines.len() && lines[i + 1].trim().to_lowercase() != end {
                i += 1;
            }
            let body = &lines[start..(i + 1).max(start)];
            i += 1; // the #+end line
            if kind == "quote" {
                out.push(Block::Quote(blocks(body, kw, doc)));
            } else {
                let cut = body.iter().filter(|l| !l.trim().is_empty()).map(|l| indent(l)).min().unwrap_or(0);
                let raw: Vec<&str> = body.iter().map(|l| l.get(cut..).unwrap_or("")).collect();
                out.push(Block::Pre(kind, c[2].to_string(), raw.join("\n")));
            }
        } else if lower.starts_with("#+") {
            flush(&mut para, &mut out);
            if let Some(v) = lower.strip_prefix("#+title:").map(|_| t[8..].trim()) {
                doc.title = Some(v.to_string());
            } else if let Some(v) = lower.strip_prefix("#+subtitle:").map(|_| t[11..].trim()) {
                doc.subtitle = Some(v.to_string());
            } else if let Some(v) = lower.strip_prefix("#+name:").map(|_| t[7..].trim()).filter(|v| !v.is_empty()) {
                out.push(Block::Anchor(v.to_string()));
            }
        } else if t.starts_with('|') {
            flush(&mut para, &mut out);
            let mut groups = vec![vec![]];
            while i < lines.len() && lines[i].trim().starts_with('|') {
                let r = lines[i].trim();
                if r.starts_with("|-") {
                    groups.push(vec![]);
                } else {
                    let r = r.trim_start_matches('|');
                    groups.last_mut().unwrap().push(r.strip_suffix('|').unwrap_or(r).split('|').map(|c| c.trim().to_string()).collect());
                }
                i += 1;
            }
            groups.retain(|g| !g.is_empty());
            out.push(Block::Table(groups));
            continue;
        } else if t.len() >= 5 && t.chars().all(|c| c == '-') {
            flush(&mut para, &mut out);
            out.push(Block::Rule);
        } else if let Some(c) = ITEM.captures(l).filter(|c| !(c[1].is_empty() && &c[2] == "*")) {
            flush(&mut para, &mut out);
            let ordered = c[2].starts_with(|c: char| c.is_ascii_digit());
            let mut items = vec![];
            let base = indent(l);
            let item_at = |i: usize| lines.get(i).and_then(|l| ITEM.captures(l)).filter(|c| c[1].len() == base && c[2].starts_with(|c: char| c.is_ascii_digit()) == ordered);
            // Items at BASE indent; deeper or blank-then-deeper lines belong to the item.
            loop {
                if lines.get(i).is_some_and(|l| l.trim().is_empty()) && item_at(i + 1).is_some() {
                    i += 1; // a blank line between items keeps the list going
                }
                let Some(c) = item_at(i) else { break };
                let mut j = i + 1;
                while j < lines.len() && (indent(lines[j]) > base && !lines[j].trim().is_empty() || lines[j].trim().is_empty() && lines.get(j + 1).is_some_and(|n| indent(n) > base && !n.trim().is_empty())) {
                    j += 1;
                }
                let mut text = c[4].to_string();
                let mut children = blocks(&lines[i + 1..j], kw, doc);
                if let Some(Block::Para(_)) = children.first() {
                    let Block::Para(p) = children.remove(0) else { unreachable!() };
                    text = format!("{text} {p}");
                }
                let (term, text) = match text.split_once(" :: ") {
                    Some((t, d)) if !ordered => (Some(t.trim().to_string()), d.trim().to_string()),
                    _ => (None, text),
                };
                items.push(Item { check: c.get(3).and_then(|m| m.as_str().chars().next()), term, text, children });
                i = j;
            }
            out.push(Block::List(ordered, items));
            continue;
        } else {
            para.push(t);
        }
        i += 1;
    }
    flush(&mut para, &mut out);
    out
}

// ---------------------------------------------------------------- inline

#[derive(Debug, PartialEq)]
enum Span {
    Text(String),
    /// Emphasis marker (* / _ + = ~) and its contents; = and ~ hold one raw Text.
    Mark(char, Vec<Span>),
    /// (target, description)
    Link(String, Option<String>),
    /// `<<target>>`: an invisible anchor.
    Target(String),
}

const PRE: &str = " \t-({'\"";
const POST: &str = " \t-.,:!?;'\")}[";

fn spans(s: &str) -> Vec<Span> {
    let mut out = vec![];
    let mut text = String::new();
    let mut i = 0;
    while i < s.len() {
        let rest = &s[i..];
        let c = rest.chars().next().unwrap();
        if let Some(m) = LINK.captures(rest) {
            out.extend((!text.is_empty()).then(|| Span::Text(std::mem::take(&mut text))));
            out.push(Span::Link(m[1].to_string(), m.get(2).map(|d| d.as_str().to_string())));
            i += m[0].len();
            continue;
        }
        if let Some(m) = TARGET.captures(rest).filter(|m| !s[..i].ends_with('<') && !rest[m[0].len()..].starts_with('>')) {
            out.extend((!text.is_empty()).then(|| Span::Text(std::mem::take(&mut text))));
            out.push(Span::Target(m[1].to_string()));
            i += m[0].len();
            continue;
        }
        let prev_ok = s[..i].chars().last().is_none_or(|p| PRE.contains(p));
        if prev_ok && (rest.starts_with("https://") || rest.starts_with("http://")) {
            // A bare URL, minus trailing punctuation that ends the sentence.
            let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
            let url = rest[..end].trim_end_matches(['.', ',', ';', ':', '!', '?', ')', '\'', '"']);
            out.extend((!text.is_empty()).then(|| Span::Text(std::mem::take(&mut text))));
            out.push(Span::Link(url.to_string(), None));
            i += url.len();
            continue;
        }
        if "*/_+=~".contains(c) && prev_ok && rest[1..].starts_with(|n: char| !n.is_whitespace()) {
            // First closing marker that follows a non-space and precedes a boundary.
            let close = rest.char_indices().skip(2).find(|&(j, ch)| {
                ch == c && !rest[..j].ends_with(char::is_whitespace) && rest[j + 1..].chars().next().is_none_or(|n| POST.contains(n))
            });
            if let Some((j, _)) = close {
                out.extend((!text.is_empty()).then(|| Span::Text(std::mem::take(&mut text))));
                let inner = &rest[1..j];
                out.push(Span::Mark(c, if "=~".contains(c) { vec![Span::Text(inner.into())] } else { spans(inner) }));
                i += j + 1;
                continue;
            }
        }
        text.push(c);
        i += c.len_utf8();
    }
    out.extend((!text.is_empty()).then_some(Span::Text(text)));
    out
}

/// The visible text of S (a link's description, else its target; `<<target>>`s are invisible).
fn plain(s: &[Span]) -> String {
    s.iter()
        .map(|sp| match sp {
            Span::Text(t) => t.clone(),
            Span::Mark(_, inner) => plain(inner),
            Span::Link(t, d) => d.clone().unwrap_or_else(|| t.clone()),
            Span::Target(_) => String::new(),
        })
        .collect()
}

/// Every span of S, inside emphasis too.
fn each_span(s: &[Span], f: &mut impl FnMut(&Span)) {
    for sp in s {
        f(sp);
        if let Span::Mark(_, inner) = sp {
            each_span(inner, f);
        }
    }
}

/// Every inline text of BS (heading titles, paragraphs, list items, cells, verse lines; not src/example bodies).
fn each_text(bs: &[Block], f: &mut impl FnMut(&str)) {
    for b in bs {
        match b {
            Block::Heading(h) => f(&h.title),
            Block::Para(p) => f(p),
            Block::List(_, items) => {
                for it in items {
                    it.term.iter().for_each(|t| f(t));
                    f(&it.text);
                    each_text(&it.children, f);
                }
            }
            Block::Table(g) => g.iter().flatten().flatten().for_each(|c| f(c)),
            Block::Pre(kind, _, body) if kind == "verse" => body.lines().for_each(&mut *f),
            Block::Quote(inner) => each_text(inner, f),
            _ => {}
        }
    }
}

/// Every block of BS, nested ones (quotes, list items) after their parent.
fn each_block(bs: &[Block], f: &mut impl FnMut(&Block)) {
    for b in bs {
        f(b);
        match b {
            Block::Quote(inner) => each_block(inner, f),
            Block::List(_, items) => items.iter().for_each(|it| each_block(&it.children, f)),
            _ => {}
        }
    }
}

/// A local image as a data: URI, so the exported page works on its own.
fn embed(t: &str, base: &Path) -> Option<String> {
    if t.starts_with("http://") || t.starts_with("https://") {
        return None;
    }
    let p = t.strip_prefix("file:").unwrap_or(t);
    let p = base.join(p.split("::").next().unwrap_or(p));
    let ext = p.extension()?.to_string_lossy().to_lowercase();
    let mime = match ext.as_str() { "jpg" | "jpeg" => "jpeg".into(), "svg" => "svg+xml".into(), e => e.to_string() };
    Some(format!("data:image/{mime};base64,{}", b64(&std::fs::read(&p).ok()?)))
}

fn b64(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn is_image(t: &str) -> bool {
    let t = t.to_lowercase();
    [".png", ".jpg", ".jpeg", ".gif", ".svg", ".webp"].iter().any(|e| t.ends_with(e))
}

/// T's local file path (a `file:` link, or a target with no URL scheme), minus any `::search`.
fn local(t: &str) -> Option<&str> {
    let p = t.strip_prefix("file:").or_else(|| (!SCHEME.is_match(t)).then_some(t))?;
    Some(p.split("::").next().unwrap_or(p))
}

/// P with `.` and `..` folded away, lexically (the disk isn't touched).
pub fn norm(p: &Path) -> PathBuf {
    let mut o = PathBuf::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => _ = o.pop(),
            c => o.push(c),
        }
    }
    o
}

/// What rendering a page needs: BASE (the note's folder) for file: links, IDS for id: and site links, DOC for its
/// anchors and diagrams.
struct Cx<'a> {
    base: &'a Path,
    ids: &'a Ids,
    doc: &'a Doc,
}

impl Cx<'_> {
    /// An invisible `<a id>` (HTML, and inline HTML in Markdown) for anchor ID of the page.
    fn anchor(&self, id: &str) -> String {
        format!("<a id=\"{}{}\"></a>", esc(&self.doc.prefix), esc(id))
    }
}

/// Resolve a link target: Ok(url) or Err(plain text) for id: links not in IDS (id → exported page) and for
/// same-page links (`*Heading`, `#custom-id`, a bare target that isn't a file) to nothing on the page. file: paths
/// become absolute file:// URLs (relative to BASE, the note's folder); in a site, the local file's page or asset,
/// else plain text (the description, or just the file name).
fn target(t: &str, desc: Option<&str>, cx: &Cx) -> Result<String, String> {
    let (base, ids) = (cx.base, cx.ids);
    if let Some(id) = t.strip_prefix("id:") {
        if let Some(page) = ids.ids.get(id) {
            return Ok(page.clone());
        }
        return Err(desc.unwrap_or(id).to_string());
    }
    let hn = crate::notes::heading_norm;
    let key = match t.as_bytes().first() {
        Some(b'*') => Some(format!("*{}", hn(&t[1..]))),
        Some(b'#') => Some(format!("#{}", t[1..].trim())),
        _ => (!FILE_LIKE.is_match(t)).then(|| hn(t)),
    };
    if let Some(k) = key {
        return cx.doc.anchors.get(&k).map(|a| format!("#{}{a}", cx.doc.prefix)).ok_or_else(|| desc.unwrap_or(t.strip_prefix('*').unwrap_or(t)).to_string());
    }
    if let Some(site) = &ids.site {
        let Some(p) = local(t) else { return Ok(t.to_string()) };
        let text = || desc.map_or_else(|| Path::new(p).file_name().map_or(p.into(), |n| n.to_string_lossy().to_string()), String::from);
        return site.get(&norm(&base.join(p))).cloned().ok_or_else(text);
    }
    match t.strip_prefix("file:") {
        Some(p) => {
            let p = base.join(p.split("::").next().unwrap_or(p));
            Ok(format!("file:///{}", p.to_string_lossy().replace('\\', "/").trim_start_matches('/')))
        }
        None => Ok(t.to_string()),
    }
}

/// NAME, or the first free `stem (2).ext`, `stem (3).ext`… when TAKEN.
pub fn free_name(name: &str, taken: impl Fn(&str) -> bool) -> String {
    let (stem, ext) = name.rfind('.').filter(|&i| i > 0).map_or((name, ""), |i| name.split_at(i));
    std::iter::once(name.to_string()).chain((2..).map(|n| format!("{stem} ({n}){ext}"))).find(|n| !taken(n)).unwrap()
}

fn ids_of(f: &OrgFile) -> impl Iterator<Item = &String> {
    f.id.iter().chain(f.headlines.iter().filter_map(|h| h.id.as_ref()))
}

/// PAGES, each with a unique `<stem>.<EXT>` name (never RESERVED), and the ids those pages hold.
fn name_pages<'a>(pages: Vec<&'a OrgFile>, ext: &str, reserved: &str) -> (Vec<(&'a OrgFile, String)>, Ids) {
    let mut out: Vec<(&OrgFile, String)> = vec![];
    let mut ids = Ids::new();
    for f in pages {
        let stem = f.path.file_stem().map_or("note".into(), |s| s.to_string_lossy().to_string());
        let name = free_name(&format!("{stem}.{ext}"), |n| n.eq_ignore_ascii_case(reserved) || out.iter().any(|(_, o)| o.eq_ignore_ascii_case(n)));
        ids.ids.extend(ids_of(f).map(|id| (id.clone(), name.clone())));
        out.push((f, name));
    }
    (out, ids)
}

/// ROOT plus the notes it reaches by id: links within DEPTH hops (from FILES, where ROOT's
/// own entry is ignored), each with a unique `<stem>.<EXT>` name, and the ids those pages hold.
pub fn bundle<'a>(files: &[&'a OrgFile], root: &'a OrgFile, depth: usize, ext: &str) -> (Vec<(&'a OrgFile, String)>, Ids) {
    let all: Vec<&OrgFile> = std::iter::once(root).chain(files.iter().copied().filter(|f| f.path != root.path)).collect();
    let by_id: HashMap<&str, &OrgFile> = all.iter().flat_map(|&f| ids_of(f).map(move |id| (id.as_str(), f))).collect();
    let mut pages = vec![root];
    let mut frontier = vec![root];
    for _ in 0..depth {
        let mut next = vec![];
        for l in frontier.iter().flat_map(|f| &f.links).filter(|l| !l.file) {
            if let Some(&g) = by_id.get(l.target.as_str()).filter(|g| !pages.iter().any(|p| p.path == g.path)) {
                pages.push(g);
                next.push(g);
            }
        }
        frontier = next;
    }
    name_pages(pages, ext, "")
}

/// Names in EXISTING ending in `.EXT` that aren't in KEEP: pages left over from an earlier export.
pub fn stale(existing: &[String], keep: &[String], ext: &str) -> Vec<String> {
    existing.iter().filter(|n| n.ends_with(&format!(".{ext}")) && !keep.contains(n)).cloned().collect()
}

// ---------------------------------------------------------------- HTML

pub fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#39;")
}

fn html_spans(s: &[Span], cx: &Cx) -> String {
    s.iter()
        .map(|sp| match sp {
            Span::Text(t) => esc(t),
            Span::Mark(m, inner) => {
                let tag = match m { '*' => "b", '/' => "i", '_' => "u", '+' => "del", _ => "code" };
                format!("<{tag}>{}</{tag}>", html_spans(inner, cx))
            }
            Span::Link(t, d) => match target(t, d.as_deref(), cx) {
                Err(text) => html_spans(&spans(&text), cx),
                Ok(u) if d.is_none() && is_image(t) => format!("<img src=\"{}\" alt=\"\">", esc(&cx.ids.site.is_none().then(|| embed(t, cx.base)).flatten().unwrap_or(u))),
                Ok(u) => format!("<a href=\"{}\">{}</a>", esc(&u), d.as_deref().map_or_else(|| esc(t.strip_prefix('*').unwrap_or(t)), |d| html_spans(&spans(d), cx))),
            },
            Span::Target(t) => cx.anchor(&target_id(t)),
        })
        .collect()
}

fn html_blocks(bs: &[Block], cx: &Cx) -> String {
    let inl = |s: &str| html_spans(&spans(s), cx);
    let dg = &cx.doc.diagrams;
    let mut o = String::new();
    for b in bs {
        match b {
            Block::Heading(h) => {
                let n = (h.level + 1).min(6);
                o += &format!("<h{n} id=\"{}{}\">", esc(&cx.doc.prefix), esc(&h.id));
                if let Some((k, done)) = &h.kw {
                    o += &format!("<span class=\"kw {}\">{}</span> ", if *done { "done" } else { "todo" }, esc(k));
                }
                if let Some(p) = &h.prio {
                    o += &format!("<span class=\"prio\">[#{}]</span> ", esc(p));
                }
                o += &inl(&h.title);
                if !h.tags.is_empty() {
                    o += &format!(" <span class=\"tags\">{}</span>", h.tags.iter().map(|t| format!("<span>{}</span>", esc(t))).collect::<String>());
                }
                o += &format!("</h{n}>\n");
            }
            Block::Para(p) => o += &format!("<p>{}</p>\n", inl(p)),
            Block::List(_, items) if items.iter().all(|it| it.term.is_some()) => {
                o += "<dl>\n";
                for it in items {
                    o += &format!("<dt>{}</dt><dd>{}{}</dd>\n", inl(it.term.as_deref().unwrap_or("")), inl(&it.text), html_blocks(&it.children, cx));
                }
                o += "</dl>\n";
            }
            Block::List(ordered, items) => {
                let tag = if *ordered { "ol" } else { "ul" };
                o += &format!("<{tag}>\n");
                for it in items {
                    let cb = it.check.map_or(String::new(), |c| format!("<input type=\"checkbox\" disabled{}> ", if c == 'X' || c == 'x' { " checked" } else { "" }));
                    o += &format!("<li>{cb}{}{}</li>\n", inl(&it.text), html_blocks(&it.children, cx));
                }
                o += &format!("</{tag}>\n");
            }
            Block::Table(groups) => {
                o += "<table>\n";
                for (gi, g) in groups.iter().enumerate() {
                    let head = gi == 0 && groups.len() > 1;
                    let (sec, cell) = if head { ("thead", "th") } else { ("tbody", "td") };
                    o += &format!("<{sec}>\n");
                    for r in g {
                        o += &format!("<tr>{}</tr>\n", r.iter().map(|c| format!("<{cell}>{}</{cell}>", inl(c))).collect::<String>());
                    }
                    o += &format!("</{sec}>\n");
                }
                o += "</table>\n";
            }
            Block::Pre(kind, _, body) if kind == "verse" => o += &format!("<p class=\"verse\">{}</p>\n", body.lines().map(inl).collect::<Vec<_>>().join("<br>\n")),
            Block::Pre(kind, lang, body) if kind == "src" && lang.eq_ignore_ascii_case("mermaid") && dg.get(&diagram_key(body)).is_some_and(|s| !UNSAFE.is_match(s)) => {
                o += &format!("<figure class=\"diagram\">{}</figure>\n", dg[&diagram_key(body)]);
            }
            Block::Pre(kind, lang, body) => {
                let lang = if lang.is_empty() { String::new() } else { format!(" data-lang=\"{}\"", esc(lang)) };
                o += &format!("<pre class=\"{}\"{lang}><code>{}</code></pre>\n", esc(kind), esc(body));
            }
            Block::Quote(inner) => o += &format!("<blockquote>\n{}</blockquote>\n", html_blocks(inner, cx)),
            Block::Rule => o += "<hr>\n",
            Block::Anchor(n) => o += &(cx.anchor(&name_id(n)) + "\n"),
        }
    }
    o
}

const CSS: &str = "
:root { color-scheme: light dark; --fg: #1f2328; --bg: #fff; --dim: #6e7781; --line: #d0d7de; --code: #f3f4f6; --todo: #cf222e; --done: #1a7f37; --link: #0969da; }
@media (prefers-color-scheme: dark) { :root { --fg: #e6edf3; --bg: #0d1117; --dim: #8d96a0; --line: #30363d; --code: #161b22; --todo: #ff7b72; --done: #3fb950; --link: #4493f8; } }
body { font: 16px/1.6 system-ui, sans-serif; color: var(--fg); background: var(--bg); max-width: 46rem; margin: 2rem auto; padding: 0 1rem; }
h1, h2, h3, h4, h5, h6 { line-height: 1.25; margin: 1.6em 0 .5em; }
.subtitle { color: var(--dim); margin-top: -.5em; }
a { color: var(--link); }
code, pre { font: 14px ui-monospace, monospace; background: var(--code); border-radius: 4px; }
code { padding: 0 .25em; }
pre { padding: .8em 1em; overflow-x: auto; }
pre code { padding: 0; background: none; }
dl dt { font-weight: 600; } dl dd { margin: 0 0 .5em 1.5em; }
.verse { white-space: pre-wrap; font-style: italic; }
blockquote { margin: 1em 0; padding: 0 1em; border-left: 3px solid var(--line); color: var(--dim); }
table { border-collapse: collapse; margin: 1em 0; }
th, td { border: 1px solid var(--line); padding: .3em .7em; text-align: left; }
tbody + tbody { border-top: 2px solid var(--dim); }
img { max-width: 100%; }
figure.diagram { margin: 1em 0; padding: .5em; text-align: center; overflow-x: auto; background: #fff; border-radius: 4px; }
figure.diagram svg { max-width: 100%; height: auto; }
hr { border: 0; border-top: 1px solid var(--line); }
.kw { font: 600 .75em ui-monospace, monospace; } .kw.todo { color: var(--todo); } .kw.done { color: var(--done); }
.prio { color: var(--dim); font-weight: 400; font-size: .8em; }
.tags span { font-size: .6em; font-weight: 400; color: var(--dim); border: 1px solid var(--line); border-radius: 1em; padding: 0 .5em; margin-left: .3em; vertical-align: middle; }
.index .date { color: var(--dim); font-size: .85em; margin-left: .5em; }
li:has(> input[type=checkbox]) { list-style: none; margin-left: -1.3em; }
section + section { border-top: 1px solid var(--line); break-before: page; }
@media print { body { margin: 0; max-width: none; } a { color: inherit; } section + section { border: 0; } }
";

/// Self-contained HTML page. BASE resolves relative file: links, IDS maps id: links to sibling pages; PRINT opens the print dialog on load.
pub fn html(d: &Doc, fallback_title: &str, base: &Path, ids: &Ids, print: bool) -> String {
    page(d.title.as_deref().unwrap_or(fallback_title), &html_body(d, fallback_title, base, ids), print)
}

/// A note's title, subtitle and blocks as an HTML fragment (no page, no CSS).
pub fn html_body(d: &Doc, fallback_title: &str, base: &Path, ids: &Ids) -> String {
    let cx = Cx { base, ids, doc: d };
    let title = d.title.as_deref().unwrap_or(fallback_title);
    let mut o = format!("<h1>{}</h1>\n", html_spans(&spans(title), &cx));
    if let Some(s) = &d.subtitle {
        o += &format!("<p class=\"subtitle\">{}</p>\n", html_spans(&spans(s), &cx));
    }
    o + &html_blocks(&d.blocks, &cx)
}

fn page(title: &str, body: &str, print: bool) -> String {
    let mut o = format!("<!doctype html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>{}</title>\n<style>{CSS}</style>\n</head>\n<body>\n{body}", esc(title));
    if print {
        o += "<script>addEventListener(\"load\", () => print())</script>\n";
    }
    o + "</body>\n</html>\n"
}

/// PAGES (from `bundle`) as one print page: a `<section id="p-N">` per note, each on a new sheet (its anchors
/// prefixed `p-N-`), with IDS pointing at the sections instead of the page files and DIAGRAMS embedded in every note.
pub fn combined(pages: &[(&OrgFile, String)], texts: &[String], ids: &Ids, kw: &Kw, diagrams: &HashMap<String, String>) -> String {
    let ids = Ids { ids: ids.ids.iter().filter_map(|(id, name)| Some((id.clone(), format!("#p-{}", pages.iter().position(|(_, n)| n == name)?)))).collect(), site: None };
    let docs: Vec<(Doc, String)> = pages.iter().zip(texts).enumerate().map(|(i, ((f, _), t))| (Doc { prefix: format!("p-{i}-"), ..parse(t, kw).with_diagrams(diagrams.clone()) }, f.path.file_stem().map_or("note".into(), |s| s.to_string_lossy().to_string()))).collect();
    let body: String = docs.iter().zip(pages).enumerate().map(|(i, ((d, stem), (f, _)))| format!("<section id=\"p-{i}\">\n{}</section>\n", html_body(d, stem, f.path.parent().unwrap_or(Path::new("")), &ids))).collect();
    page(docs[0].0.title.as_deref().unwrap_or(&docs[0].1), &body, true)
}

// ---------------------------------------------------------------- site (#1037)

/// The site's list of generated files, so a later run only ever deletes what it wrote.
pub const MANIFEST: &str = ".margin-publish";

/// Every link target D renders (title, subtitle and blocks, not inside src/example blocks).
fn doc_links(d: &Doc) -> Vec<String> {
    let mut o = vec![];
    let mut text = |s: &str| each_span(&spans(s), &mut |sp| if let Span::Link(t, _) = sp { o.push(t.clone()) });
    d.title.iter().chain(&d.subtitle).for_each(|t| text(t));
    each_text(&d.blocks, &mut text);
    o
}

/// The site's index page: PAGES by title, each with its modified date and tags.
fn site_index(pages: &[(&OrgFile, String)], title: &str) -> String {
    let mut rows: Vec<&(&OrgFile, String)> = pages.iter().collect();
    rows.sort_by_key(|(f, _)| f.title.to_lowercase());
    let li: String = rows
        .iter()
        .map(|(f, n)| {
            let date = f.mtime.map_or(String::new(), |t| format!(" <span class=\"date\">{}</span>", chrono::DateTime::<chrono::Local>::from(t).format("%Y-%m-%d")));
            let tags = if f.tags.is_empty() { String::new() } else { format!(" <span class=\"tags\">{}</span>", f.tags.iter().map(|t| format!("<span>{}</span>", esc(t))).collect::<String>()) };
            format!("<li><a href=\"{}\">{}</a>{date}{tags}</li>\n", esc(n), esc(&f.title))
        })
        .collect();
    page(title, &format!("<h1>{}</h1>\n<ul class=\"index\">\n{li}</ul>\n", esc(title)), false)
}

/// The names in an earlier run's MANIFEST (one per line) that KEEP no longer has. Only a bare file name, or one
/// under `assets/`, counts: a hand-edited manifest can't reach outside the site folder. Names compare ignoring
/// case, like `name_pages` and a case-insensitive filesystem (a case-only rename must not delete the new file).
/// Trade-off: on a case-sensitive filesystem the old-cased file stays behind as an orphan (no longer listed; a
/// rename back then fails as "in the way"), and a foreign `a.html` next to a manifest entry `A.html` is overwritten.
fn manifest_stale(manifest: &str, keep: &[String]) -> Vec<String> {
    let bare = |n: &str| !n.is_empty() && n != "." && n != ".." && !n.contains(['/', '\\', ':']);
    manifest.lines().map(str::trim).filter(|n| bare(n.strip_prefix("assets/").unwrap_or(n)) && !keep.iter().any(|k| k.eq_ignore_ascii_case(n))).map(String::from).collect()
}

/// Publish the notes in FILES tagged TAG (file tags) to DIR as a static site: a page per note (its text from
/// TEXT_OF), links between them relative, `index.html`, and the other local files they link to under NOTES
/// copied to `assets/`. Files of an earlier run (DIR's MANIFEST) no longer generated are deleted; a file in
/// the way that no earlier run wrote is an error. Returns the number of pages.
pub fn publish(dir: &Path, notes: &Path, files: &[&OrgFile], tag: &str, kw: &Kw, text_of: impl Fn(&OrgFile) -> String) -> Result<usize, String> {
    let io = |e: std::io::Error| e.to_string();
    let picked: Vec<&OrgFile> = files.iter().copied().filter(|f| f.tags.iter().any(|t| t.eq_ignore_ascii_case(tag))).collect();
    if picked.is_empty() {
        return Err(format!("No notes tagged :{tag}: to publish"));
    }
    let (pages, mut ids) = name_pages(picked, "html", "index.html");
    let mut site: HashMap<PathBuf, String> = pages.iter().map(|(f, n)| (norm(&f.path), n.clone())).collect();
    let docs: Vec<Doc> = pages.iter().map(|(f, _)| parse(&text_of(f), kw)).collect();
    let (notes, mut assets) = (norm(notes), vec![]);
    for ((f, _), d) in pages.iter().zip(&docs) {
        let base = f.path.parent().unwrap_or(Path::new(""));
        for p in doc_links(d).iter().filter_map(|t| local(t)).map(|p| norm(&base.join(p))) {
            if site.contains_key(&p) || !p.starts_with(&notes) || !p.is_file() || p.extension().is_some_and(|e| e.eq_ignore_ascii_case("org")) {
                continue;
            }
            let file = p.file_name().map_or("file".into(), |n| n.to_string_lossy().to_string());
            let name = format!("assets/{}", free_name(&file, |n| site.values().any(|a| a.eq_ignore_ascii_case(&format!("assets/{n}")))));
            site.insert(p.clone(), name.clone());
            assets.push((p, name));
        }
    }
    ids.site = Some(site);
    let names: Vec<String> = pages.iter().map(|(_, n)| n).chain(assets.iter().map(|(_, n)| n)).cloned().chain(["index.html".to_string()]).collect();
    let old = std::fs::read_to_string(dir.join(MANIFEST)).unwrap_or_default();
    if let Some(n) = names.iter().find(|n| dir.join(n).exists() && !old.lines().any(|o| o.trim().eq_ignore_ascii_case(n))) {
        return Err(format!("{} is in the way: an earlier publish didn't write it. Move it, or pick another folder.", dir.join(n).display()));
    }
    std::fs::create_dir_all(dir.join(if assets.is_empty() { "" } else { "assets" })).map_err(io)?;
    // Old and new names first, so an interrupted run still owns everything it wrote.
    let manifest = |names: &[String]| crate::config::write_atomic(&dir.join(MANIFEST), names.iter().map(|n| format!("{n}\n")).collect::<String>()).map_err(io);
    manifest(&[names.clone(), manifest_stale(&old, &names)].concat())?;
    for ((f, name), d) in pages.iter().zip(&docs) {
        crate::config::write_atomic(&dir.join(name), html(d, &f.title, f.path.parent().unwrap_or(Path::new("")), &ids, false)).map_err(io)?;
    }
    for (src, name) in &assets {
        crate::config::write_atomic(&dir.join(name), std::fs::read(src).map_err(io)?).map_err(io)?;
    }
    let title = dir.file_name().map_or("Notes".into(), |n| n.to_string_lossy().to_string());
    crate::config::write_atomic(&dir.join("index.html"), site_index(&pages, &title)).map_err(io)?;
    let root = std::fs::canonicalize(dir).map_err(io)?;
    for n in manifest_stale(&old, &names) {
        let p = dir.join(&n);
        if p.is_file() && std::fs::canonicalize(&p).is_ok_and(|c| c.starts_with(&root)) {
            std::fs::remove_file(&p).map_err(io)?;
        }
    }
    manifest(&names)?;
    Ok(pages.len())
}

// ---------------------------------------------------------------- Markdown

fn md_esc(s: &str) -> String {
    s.chars().fold(String::new(), |mut o, c| {
        if "\\`*_[]<>|".contains(c) {
            o.push('\\');
        }
        o.push(c);
        o
    })
}

fn md_spans(s: &[Span], cx: &Cx) -> String {
    s.iter()
        .map(|sp| match sp {
            Span::Text(t) => md_esc(t),
            Span::Mark('=' | '~', inner) => {
                let Some(Span::Text(t)) = inner.first() else { return String::new() };
                let tick = if t.contains('`') { "``" } else { "`" };
                let pad = if t.starts_with('`') || t.ends_with('`') { " " } else { "" };
                format!("{tick}{pad}{t}{pad}{tick}")
            }
            Span::Mark(m, inner) => {
                let (a, b) = match m { '*' => ("**", "**"), '/' => ("*", "*"), '+' => ("~~", "~~"), _ => ("<u>", "</u>") };
                format!("{a}{}{b}", md_spans(inner, cx))
            }
            Span::Link(t, d) => match target(t, d.as_deref(), cx) {
                Err(text) => md_spans(&spans(&text), cx),
                Ok(u) => {
                    let u = if u.contains([' ', '(', ')', '<', '>']) { format!("<{}>", u.replace('<', "%3C").replace('>', "%3E")) } else { u };
                    let img = if d.is_none() && is_image(t) { "!" } else { "" };
                    let text = if img.is_empty() { d.as_deref().map_or_else(|| md_esc(t.strip_prefix('*').unwrap_or(t)), |d| md_spans(&spans(d), cx)) } else { String::new() };
                    format!("{img}[{text}]({u})")
                }
            },
            Span::Target(t) => cx.anchor(&target_id(t)),
        })
        .collect()
}

fn md_blocks(bs: &[Block], cx: &Cx) -> String {
    let inl = |s: &str| md_spans(&spans(s), cx);
    let mut parts: Vec<String> = vec![];
    for b in bs {
        parts.push(match b {
            Block::Heading(h) => {
                // Our own anchor: each renderer slugs headings its own way.
                let mut o = format!("{} {}", "#".repeat((h.level + 1).min(6)), cx.anchor(&h.id));
                if let Some((k, _)) = &h.kw {
                    o += &format!("**{k}** ");
                }
                if let Some(p) = &h.prio {
                    o += &format!("\\[#{p}\\] ");
                }
                o += &inl(&h.title);
                if !h.tags.is_empty() {
                    o += &format!(" {}", h.tags.iter().map(|t| format!("`{t}`")).collect::<Vec<_>>().join(" "));
                }
                o
            }
            Block::Para(p) => inl(p),
            Block::List(ordered, items) => items
                .iter()
                .enumerate()
                .map(|(n, it)| {
                    let marker = if *ordered { format!("{}. ", n + 1) } else { "- ".into() };
                    let cb = it.check.map_or("", |c| if c == 'X' || c == 'x' { "[x] " } else { "[ ] " });
                    let cb = it.term.as_deref().map_or(cb.to_string(), |t| format!("{cb}**{}**: ", inl(t)));
                    let pad = " ".repeat(marker.len());
                    let kids = md_blocks(&it.children, cx);
                    let kids: String = kids.lines().map(|l| if l.is_empty() { "\n".into() } else { format!("\n{pad}{l}") }).collect();
                    format!("{marker}{cb}{}{kids}", inl(&it.text))
                })
                .collect::<Vec<_>>()
                .join("\n"),
            Block::Table(groups) => {
                // Markdown tables need exactly one header row: the org header, or the first row.
                let rows: Vec<&Vec<String>> = groups.iter().flatten().collect();
                let n = rows.iter().map(|r| r.len()).max().unwrap_or(0);
                let line = |r: &Vec<String>| format!("| {} |", (0..n).map(|i| r.get(i).map_or(String::new(), |c| inl(c))).collect::<Vec<_>>().join(" | "));
                let mut o: Vec<String> = rows.iter().map(|r| line(r)).collect();
                if !o.is_empty() {
                    o.insert(1, format!("|{}|", vec![" --- "; n].join("|")));
                }
                o.join("\n")
            }
            Block::Pre(kind, _, body) if kind == "verse" => body.lines().map(inl).collect::<Vec<_>>().join("  \n"),
            Block::Pre(kind, lang, body) => {
                let mut fence = "```".to_string();
                while body.contains(&fence) {
                    fence.push('`');
                }
                format!("{fence}{}\n{body}\n{fence}", if kind == "src" { lang.as_str() } else { "" })
            }
            Block::Quote(inner) => md_blocks(inner, cx).lines().map(|l| if l.is_empty() { ">".into() } else { format!("> {l}") }).collect::<Vec<_>>().join("\n"),
            Block::Rule => "---".into(),
            Block::Anchor(n) => cx.anchor(&name_id(n)),
        });
    }
    parts.join("\n\n")
}

/// Markdown page; IDS maps id: links to sibling pages like `html`.
pub fn markdown(d: &Doc, fallback_title: &str, base: &Path, ids: &Ids) -> String {
    let cx = Cx { base, ids, doc: d };
    let title = d.title.as_deref().unwrap_or(fallback_title);
    let mut o = format!("# {}\n\n", md_spans(&spans(title), &cx));
    if let Some(s) = &d.subtitle {
        o += &format!("*{}*\n\n", md_spans(&spans(s), &cx));
    }
    o + &md_blocks(&d.blocks, &cx) + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kw() -> Kw {
        Kw { todo: vec!["TODO".into()], done: vec!["DONE".into()] }
    }

    const NOTE: &str = ":PROPERTIES:\n:ID: abc\n:END:\n#+title: Plans <&>\n#+filetags: :x:\n\
* TODO [#A] Ship *it* :work:urgent:\nSCHEDULED: <2026-10-01 Thu>\n:PROPERTIES:\n:ID: h1\n:END:\n\
Some *bold* /italic/ _under_ =a<b= ~code~ +gone+ text, a*b*c and 3 + 4.\nSecond line.\n\n\
# a comment\n- one\n  - nested [[https://x.org/?a=1&b=2][site]]\n- [X] done item\n- [ ] open item\n\n\
1. first\n2) second\n\n| Name | Qty |\n|------+-----|\n| <a> | 1 |\n|------+-----|\n| Sum | 1 |\n#+TBLFM: $2=vsum(@I..@II)\n\n\
#+begin_src rust\nfn x() -> &'a str { \"<\" }\n#+end_src\n#+begin_quote\nQuoted /line/.\n#+end_quote\n#+BEGIN_EXAMPLE\n  <raw>\n#+END_EXAMPLE\n\
** DONE See [[id:abc][Other note]] and [[file:pic.png]] and [[file:doc.org][doc]]\n-----\n";

    #[test]
    fn export_skips_encrypted_blocks() {
        let t = "* Vault :crypt:\n-----BEGIN MARGIN ENCRYPTED-----\nv=1 zebra\nzebra==\n-----END MARGIN ENCRYPTED-----\nafter\n";
        let h = html(&parse(t, &kw()), "x", Path::new("/n"), &Ids::new(), false);
        assert!(!h.contains("zebra") && !h.contains("ENCRYPTED") && h.contains("after"));
        assert!(!markdown(&parse(t, &kw()), "x", Path::new("/n"), &Ids::new()).contains("zebra"));
    }

    #[test]
    fn parses_blocks() {
        let d = parse(NOTE, &kw());
        assert_eq!(d.title.as_deref(), Some("Plans <&>"));
        let kinds: Vec<&str> = d.blocks.iter().map(|b| match b {
            Block::Heading(_) => "h", Block::Para(_) => "p", Block::List(..) => "l", Block::Table(_) => "t", Block::Pre(..) => "pre", Block::Quote(_) => "q", Block::Rule => "hr", Block::Anchor(_) => "a",
        }).collect();
        assert_eq!(kinds, ["h", "p", "l", "l", "t", "pre", "q", "pre", "h", "hr"]);
        let Block::List(false, items) = &d.blocks[2] else { panic!() };
        assert_eq!((items.len(), items[1].check, items[2].check), (3, Some('X'), Some(' ')));
        assert!(matches!(&items[0].children[..], [Block::List(false, _)]));
        let Block::Table(g) = &d.blocks[4] else { panic!() };
        assert_eq!((g.len(), g[1][0][0].as_str()), (3, "<a>"));
    }

    #[test]
    fn link_abbreviations() {
        let t = "#+LINK: gh https://github.com/a/b/issues/%s\n#+LINK: s https://x/?q=%h\n#+LINK: p https://p/\n#+LINK: gh nope\n#+LINK: id https://evil/%s\n[[gh:12][i]] [[s:a b&c]] [[p:x]] [[zz:1]] [[id:q]]\n";
        let e = expand_links(t);
        assert!(e.contains("[[https://github.com/a/b/issues/12][i]]") && e.contains("[[https://x/?q=a%20b%26c]]") && e.contains("[[https://p/x]]"));
        assert!(e.contains("[[zz:1]] [[id:q]]"));
    }

    #[test]
    fn link_abbreviations_skip_code() {
        let t = "#+LINK: gh https://g/%s\n#+BEGIN_SRC org\n#+LINK: x https://x/%s\n[[gh:1]] [[x:2]]\n#+END_SRC\n[[gh:3]] [[x:4]]\n";
        let e = expand_links(t);
        assert!(e.contains("#+BEGIN_SRC org\n#+LINK: x https://x/%s\n[[gh:1]] [[x:2]]\n#+END_SRC\n[[https://g/3]] [[x:4]]\n"));
        // Unterminated block: code to the end.
        assert_eq!(expand_links("#+LINK: gh https://g/%s\n#+begin_example\n[[gh:1]]\n"), "#+LINK: gh https://g/%s\n#+begin_example\n[[gh:1]]\n");
    }

    #[test]
    fn inline_markup() {
        assert_eq!(spans("a*b*c"), [Span::Text("a*b*c".into())]);
        assert_eq!(spans("3 + 4 + 5"), [Span::Text("3 + 4 + 5".into())]);
        assert_eq!(spans("(*x /y/*)"), [Span::Text("(".into()), Span::Mark('*', vec![Span::Text("x ".into()), Span::Mark('/', vec![Span::Text("y".into())])]), Span::Text(")".into())]);
        assert_eq!(spans("=*no*="), [Span::Mark('=', vec![Span::Text("*no*".into())])]);
    }

    #[test]
    fn html_fragment() {
        let h = html_body(&parse(NOTE, &kw()), "x", Path::new("/notes"), &Ids::new());
        assert!(h.starts_with("<h1>Plans &lt;&amp;&gt;</h1>") && h.contains("<a href=\"https://x.org/?a=1&amp;b=2\">site</a>"));
        assert!(!h.contains("<html") && !h.contains("<style") && !h.contains("@media"));
    }

    #[test]
    fn mermaid_diagrams() {
        // Same fixture as babel.test.ts's diagramKey test, so both sides key a block alike.
        let src = "  #+begin_src mermaid\n    graph TD  \n\n      A --> B\n    ,* x\n  #+end_src\n";
        let Block::Pre(_, _, body) = &parse(src, &kw()).blocks[0] else { panic!() };
        assert_eq!(diagram_key(body), "graph TD\nA --> B\n* x");
        let text = format!("{src}- item\n  #+begin_src mermaid\n  graph LR\n  #+end_src\n#+begin_src mermaid\nbroken\n#+end_src\n");
        let dg = |svg: &str| HashMap::from([("graph TD\nA --> B\n* x".to_string(), "<svg id=\"a\">A</svg>".to_string()), ("graph LR".to_string(), svg.to_string())]);
        let h = html_body(&Doc { diagrams: dg("<svg>LR</svg>"), ..parse(&text, &kw()) }, "x", Path::new(""), &Ids::new());
        assert!(h.contains("<figure class=\"diagram\"><svg id=\"a\">A</svg></figure>") && h.contains("<li>item<figure class=\"diagram\"><svg>LR</svg></figure>"));
        assert!(h.contains("<pre class=\"src\" data-lang=\"mermaid\"><code>broken</code></pre>"), "unmatched keeps its source");
        for bad in ["<svg><script>x()</script></svg>", "<svg onload=\"x()\"></svg>", "<svg><a href=\"javascript:x()\">l</a></svg>"] {
            let h = html_body(&Doc { diagrams: dg(bad), ..parse(&text, &kw()) }, "x", Path::new(""), &Ids::new());
            assert!(h.contains("<code>graph LR</code>") && !h.contains(bad), "{bad}");
        }
        let ok = html_body(&Doc { diagrams: dg("<svg><text>one = two</text></svg>"), ..parse(&text, &kw()) }, "x", Path::new(""), &Ids::new());
        assert!(ok.contains("<figure class=\"diagram\"><svg><text>one = two</text>"));
        assert!(!markdown(&Doc { diagrams: dg("<svg/>"), ..parse(&text, &kw()) }, "x", Path::new(""), &Ids::new()).contains("<svg"));
    }

    #[test]
    fn to_html() {
        let h = html(&parse(NOTE, &kw()), "x", Path::new("/notes"), &Ids::new(), false);
        for want in [
            "<title>Plans &lt;&amp;&gt;</title>",
            "<h1>Plans &lt;&amp;&gt;</h1>",
            "<h2 id=\"ship-it\"><span class=\"kw todo\">TODO</span> <span class=\"prio\">[#A]</span> Ship <b>it</b> <span class=\"tags\"><span>work</span><span>urgent</span></span></h2>",
            "<p>Some <b>bold</b> <i>italic</i> <u>under</u> <code>a&lt;b</code> <code>code</code> <del>gone</del> text, a*b*c and 3 + 4. Second line.</p>",
            "<li>one<ul>\n<li>nested <a href=\"https://x.org/?a=1&amp;b=2\">site</a></li>\n</ul>\n</li>",
            "<li><input type=\"checkbox\" disabled checked> done item</li>",
            "<li><input type=\"checkbox\" disabled> open item</li>",
            "<ol>\n<li>first</li>\n<li>second</li>\n</ol>",
            "<thead>\n<tr><th>Name</th><th>Qty</th></tr>\n</thead>\n<tbody>\n<tr><td>&lt;a&gt;</td><td>1</td></tr>\n</tbody>\n<tbody>\n<tr><td>Sum</td>",
            "<pre class=\"src\" data-lang=\"rust\"><code>fn x() -&gt; &amp;&#39;a str { &quot;&lt;&quot; }</code></pre>",
            "<blockquote>\n<p>Quoted <i>line</i>.</p>\n</blockquote>",
            "<pre class=\"example\"><code>&lt;raw&gt;</code></pre>",
            "<h3 id=\"see-other-note-and-file-pic-png-and-doc\"><span class=\"kw done\">DONE</span> See Other note and <img src=\"file:///notes/pic.png\" alt=\"\"> and <a href=\"file:///notes/doc.org\">doc</a></h3>",
            "<hr>",
        ] {
            assert!(h.contains(want), "missing {want}\n---\n{h}");
        }
        for gone in ["PROPERTIES", "SCHEDULED", "filetags", "TBLFM", "comment", "abc", "<script"] {
            assert!(!h.contains(gone), "{gone} leaked\n{h}");
        }
        assert!(html(&parse("x", &kw()), "t", Path::new("/"), &Ids::new(), true).contains("print()"));
    }

    #[test]
    fn to_markdown() {
        let m = markdown(&parse(NOTE, &kw()), "x", Path::new("/notes"), &Ids::new());
        for want in [
            "# Plans \\<&\\>\n\n## <a id=\"ship-it\"></a>**TODO** \\[#A\\] Ship **it** `work` `urgent`\n\n",
            "Some **bold** *italic* <u>under</u> `a<b` `code` ~~gone~~ text, a\\*b\\*c and 3 + 4. Second line.\n\n",
            "- one\n  - nested [site](https://x.org/?a=1&b=2)\n- [x] done item\n- [ ] open item\n\n1. first\n2. second\n\n",
            "| Name | Qty |\n| --- | --- |\n| \\<a\\> | 1 |\n| Sum | 1 |\n\n",
            "```rust\nfn x() -> &'a str { \"<\" }\n```\n\n> Quoted *line*.\n\n",
            "```\n<raw>\n```\n\n### <a id=\"see-other-note-and-file-pic-png-and-doc\"></a>**DONE** See Other note and ![](file:///notes/pic.png) and [doc](file:///notes/doc.org)\n\n---\n",
        ] {
            assert!(m.contains(want), "missing {want}\n---\n{m}");
        }
        assert!(!m.contains("PROPERTIES") && !m.contains("SCHEDULED") && !m.contains("TBLFM"));
    }

    #[test]
    fn markdown_fragment_and_unresolved_id() {
        let m = markdown(&parse("Intro [[id:nope][Other]] end\n\n- one\n", &kw()), "x", Path::new("/n"), &Ids::new());
        assert!(m.contains("Intro Other end") && !m.contains("id:nope") && m.contains("- one"), "{m}");
    }

    #[test]
    fn same_page_links() {
        let t = "#+title: T <<top>>\n* Plan [1/2]\nSee [[*plan]], [[#cid][custom]], [[Goals]], [[Tbl]], [[Ship  IT]] and [[top]].\n\
* Plan\n* Ship it\n:PROPERTIES:\n:CUSTOM_ID: cid\n:END:\nThe <<goals>> are *<<b>>bold*; <<<radio>>> stays.\n#+NAME: tbl\n| a |\n\
* Goals\n[[*Missing]] [[#nope][gone]] [[Nowhere]] [[*Hidden]] [[inside]] [[doc.org]] [[sub/x]] [[b.org::*H]] [[https://x.org]]\n\
* Hidden :noexport:\n#+begin_src sh\n<<inside>>\n#+end_src\n";
        let d = parse(t, &kw());
        let h = html_body(&d, "x", Path::new("/n"), &Ids::new());
        for want in [
            "<h1>T <a id=\"target-top\"></a></h1>",
            "<h2 id=\"plan\">Plan [1/2]</h2>",
            "See <a href=\"#plan\">plan</a>, <a href=\"#cid\">custom</a>, <a href=\"#target-goals\">Goals</a>, <a href=\"#name-tbl\">Tbl</a>, <a href=\"#cid\">Ship  IT</a> and <a href=\"#target-top\">top</a>.",
            "<h2 id=\"plan-2\">Plan</h2>",
            "<h2 id=\"cid\">Ship it</h2>",
            "The <a id=\"target-goals\"></a> are <b><a id=\"target-b\"></a>bold</b>; &lt;&lt;&lt;radio&gt;&gt;&gt; stays.",
            "<a id=\"name-tbl\"></a>\n<table>",
            "<h2 id=\"goals\">Goals</h2>",
            "<p>Missing gone Nowhere Hidden inside <a href=\"doc.org\">doc.org</a> <a href=\"sub/x\">sub/x</a> <a href=\"b.org::*H\">b.org::*H</a> <a href=\"https://x.org\">",
        ] {
            assert!(h.contains(want), "missing {want:?} in\n{h}");
        }
        // A heading wins a bare target only when no <<target>> or #+NAME has that name.
        let g = html_body(&parse("* Goals\n* Tbl\n[[goals]] [[tbl]]\n#+name: tbl\n", &kw()), "x", Path::new("/n"), &Ids::new());
        assert!(g.contains("<a href=\"#goals\">goals</a> <a href=\"#name-tbl\">tbl</a>"), "{g}");
        let m = markdown(&d, "x", Path::new("/n"), &Ids::new());
        for want in ["## <a id=\"plan\"></a>Plan \\[1/2\\]", "See [plan](#plan), [custom](#cid), [Goals](#target-goals)", "The <a id=\"target-goals\"></a> are", "<a id=\"name-tbl\"></a>\n\n| a |", "Missing gone Nowhere Hidden inside [doc.org](doc.org)"] {
            assert!(m.contains(want), "missing {want:?} in\n{m}");
        }
        // Site publish resolves them the same way, before the file lookup.
        let site = Ids { ids: HashMap::new(), site: Some(HashMap::new()) };
        assert!(html_body(&d, "x", Path::new("/n"), &site).contains("See <a href=\"#plan\">plan</a>, <a href=\"#cid\">custom</a>, <a href=\"#target-goals\">Goals</a>"));
    }

    #[test]
    fn followups() {
        let dir = std::env::temp_dir().join(format!("margin-export-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("dot.png"), [137u8, 80, 78, 71]).unwrap();
        let note = "See https://example.com/a_b, then (http://x.y).\n\n- Apple :: a fruit\n- Kale :: *green*\n\n#+begin_verse\nRoses /red/\n  violets\n#+end_verse\n\n[[file:dot.png]] [[file:missing.png]]\n";
        let d = parse(note, &kw());
        let h = html(&d, "x", &dir, &Ids::new(), false);
        for want in [
            "<a href=\"https://example.com/a_b\">https://example.com/a_b</a>, then (<a href=\"http://x.y\">http://x.y</a>).",
            "<dl>\n<dt>Apple</dt><dd>a fruit</dd>\n<dt>Kale</dt><dd><b>green</b></dd>\n</dl>",
            "<p class=\"verse\">Roses <i>red</i><br>\n  violets</p>",
            "<img src=\"data:image/png;base64,iVBORw==\" alt=\"\">",
            "missing.png\" alt",
        ] {
            assert!(h.contains(want), "missing {want:?} in\n{h}");
        }
        let m = markdown(&d, "x", &dir, &Ids::new());
        for want in ["- **Apple**: a fruit", "Roses *red*  \n  violets", "[https://example.com/a\\_b](https://example.com/a_b)"] {
            assert!(m.contains(want), "missing {want:?} in\n{m}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn free_names() {
        let taken = ["a.html", "a (2).html", "b"];
        let t = |n: &str| taken.contains(&n);
        assert_eq!(free_name("c.html", t), "c.html");
        assert_eq!(free_name("a.html", t), "a (3).html");
        assert_eq!(free_name("b", t), "b (2)");
    }

    #[test]
    fn linked_notes() {
        let p = |path: &str, text: &str| org::parse(Path::new(path), text, &kw());
        let a = p("/n/a.org", ":PROPERTIES:\n:ID: a\n:END:\nTo [[id:bh][B's heading]], [[id:gone][lost]], [[id:a][me]].\n");
        let b = p("/n/b.org", "* H\n:PROPERTIES:\n:ID: bh\n:END:\nOn to [[id:c]], up to [[*H]].\n");
        let c = p("/n/sub/b.org", ":PROPERTIES:\n:ID: c\n:END:\nBack to [[id:a][A]].\n");
        let stale = p("/n/a.org", "old text\n");
        let files = [&stale, &b, &c];
        let names = |d| bundle(&files, &a, d, "html").0.into_iter().map(|(f, n)| (f.path.to_string_lossy().to_string(), n)).collect::<Vec<_>>();
        assert_eq!(names(1), [("/n/a.org".into(), "a.html".into()), ("/n/b.org".into(), "b.html".into())]);
        assert_eq!(names(2)[2], ("/n/sub/b.org".into(), "b (2).html".into()));
        let (_, ids) = bundle(&files, &a, 1, "html");
        let h = html(&parse(&a.text, &kw()), "a", Path::new("/n"), &ids, false);
        assert!(h.contains("To <a href=\"b.html\">B&#39;s heading</a>, lost, <a href=\"a.html\">me</a>."), "{h}");

        let (pages, ids) = bundle(&files, &a, 2, "md");
        assert_eq!(pages[2].1, "b (2).md");
        let m = markdown(&parse(&a.text, &kw()), "a", Path::new("/n"), &ids);
        assert!(m.contains("To [B's heading](b.md), lost, [me](a.md)."), "{m}");
        assert!(markdown(&parse(&c.text, &kw()), "b", Path::new("/n/sub"), &ids).contains("Back to [A](a.md)."));

        let texts: Vec<String> = pages.iter().map(|(f, _)| f.text.clone()).collect();
        let p = combined(&pages, &texts, &ids, &kw(), &HashMap::new());
        assert_eq!(p.matches("<section id=").count(), 3);
        assert_eq!(p.matches("print()").count(), 1);
        for want in ["<title>a</title>", "<section id=\"p-0\">\n<h1>a</h1>", "To <a href=\"#p-1\">B&#39;s heading</a>, lost, <a href=\"#p-0\">me</a>.", "On to <a href=\"#p-2\">", "<h2 id=\"p-1-h\">H</h2>", ", up to <a href=\"#p-1-h\">H</a>.", "<section id=\"p-2\">\n<h1>b</h1>"] {
            assert!(p.contains(want), "missing {want:?} in\n{p}");
        }
    }

    #[test]
    fn noexport_and_comment_subtrees_are_skipped() {
        let t = "* Keep\nk1\n* Hidden :noexport:\nh1\n#+begin_src sh\n* not a heading\nh2\n#+end_src\n** Child\nh3\n* TODO COMMENT Gone\nh4\n* Commentary\nc1\n* After :Noexport:\nh5\n* Last\nl1\n";
        let md = markdown(&parse(t, &kw()), "t", Path::new("."), &Ids::new());
        for want in ["Keep", "k1", "Commentary", "c1", "Last", "l1"] {
            assert!(md.contains(want), "missing {want:?} in\n{md}");
        }
        for no in ["Hidden", "h1", "h2", "h3", "Gone", "h4", "After", "h5", "not a heading"] {
            assert!(!md.contains(no), "{no:?} leaked into\n{md}");
        }
        assert!(!markdown(&parse("* A\nx\n* B :noexport:\ny", &kw()), "t", Path::new("."), &Ids::new()).contains('y'));
    }

    #[test]
    fn manifest_names() {
        let keep = vec!["a.html".to_string(), "index.html".into()];
        let m = "a.html\nold.html\nassets/pic.png\n\n../x.html\n/abs.html\nsub/x.html\nassets/../x\nassets/sub/x\nC:x\n..\nassets/\n";
        assert_eq!(manifest_stale(m, &keep), ["old.html", "assets/pic.png"]);
        assert!(manifest_stale("A.html\nassets/Pic.png\n", &["a.html".to_string(), "assets/pic.png".into()]).is_empty());
    }

    #[test]
    fn publish_case_only_rename() {
        let root = std::env::temp_dir().join(format!("margin-publish-case-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (notes, dir) = (root.join("notes"), root.join("site"));
        std::fs::create_dir_all(&notes).unwrap();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(MANIFEST), "A.html\nindex.html\n").unwrap();
        std::fs::write(dir.join("A.html"), "old").unwrap();
        let a = org::parse(&notes.join("a.org"), "#+filetags: :publish:\nHi.\n", &kw());
        assert_eq!(publish(&dir, &notes, &[&a], "publish", &kw(), |f| f.text.clone()), Ok(1));
        assert!(dir.join("a.html").exists(), "the renamed page must survive the stale sweep");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn publish_site() {
        let root = std::env::temp_dir().join(format!("margin-publish-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (notes, dir) = (root.join("notes"), root.join("site"));
        std::fs::create_dir_all(notes.join("img")).unwrap();
        for (n, b) in [("img/pic.png", "png"), ("doc.pdf", "pdf"), ("../out.png", "out")] {
            std::fs::write(notes.join(n), b).unwrap();
        }
        let p = |n: &str, text: &str| org::parse(&notes.join(n), text, &kw());
        let a = p("a.org", "#+title: Alpha\n#+filetags: :publish:x:\nTo [[file:b.org::*H][B]], [[id:bid][B by id]], [[file:secret.org][Secret]], [[./sub/x/../index.org]] and [[https://x.org][web]].\n\n[[file:img/pic.png]] [[./doc.pdf][Doc]] [[file:missing.png]] [[file:../out.png]] [[file:/etc/hosts][hosts]]\n");
        let b = p("b.org", ":PROPERTIES:\n:ID: bid\n:END:\n#+filetags: :Publish:\nBack to [[file:a.org][A]].\n");
        let idx = p("sub/index.org", "#+filetags: :publish:\nI.\n");
        let secret = p("secret.org", "Private.\n");
        let files = [&a, &b, &idx, &secret];
        let run = || publish(&dir, &notes, &files, "publish", &kw(), |f| f.text.clone());
        assert_eq!(run(), Ok(3));
        let read = |n: &str| std::fs::read_to_string(dir.join(n)).unwrap_or_default();
        let h = read("a.html");
        for want in [
            "To <a href=\"b.html\">B</a>, <a href=\"b.html\">B by id</a>, Secret, <a href=\"index (2).html\">./sub/x/../index.org</a> and <a href=\"https://x.org\">web</a>.",
            "<img src=\"assets/pic.png\" alt=\"\"> <a href=\"assets/doc.pdf\">Doc</a> missing.png out.png hosts",
        ] {
            assert!(h.contains(want), "missing {want:?} in\n{h}");
        }
        assert!(!h.contains("file:") && !h.contains(&*root.to_string_lossy()), "local path leaked\n{h}");
        assert!(read("b.html").contains("Back to <a href=\"a.html\">A</a>."));
        assert_eq!((read("assets/pic.png").as_str(), read("assets/doc.pdf").as_str(), read("secret.html").as_str()), ("png", "pdf", ""));
        let i = read("index.html");
        assert!(i.contains("<h1>site</h1>") && i.contains("<li><a href=\"a.html\">Alpha</a> <span class=\"tags\"><span>publish</span><span>x</span></span></li>") && i.find("Alpha") < i.find(">b<"), "{i}");
        assert_eq!(read(MANIFEST), "a.html\nb.html\nindex (2).html\nassets/pic.png\nassets/doc.pdf\nindex.html\n");

        // Re-run: stale names of the manifest go, foreign files and anything outside the folder stay.
        std::fs::write(dir.join("old.html"), "").unwrap();
        std::fs::write(dir.join("mine.txt"), "").unwrap();
        std::fs::write(root.join("victim.html"), "").unwrap();
        std::fs::write(dir.join(MANIFEST), read(MANIFEST) + "old.html\n../victim.html\nassets/../../victim.html\n" + &root.join("victim.html").to_string_lossy() + "\n").unwrap();
        assert_eq!(run(), Ok(3));
        assert!(!dir.join("old.html").exists() && dir.join("mine.txt").exists() && root.join("victim.html").exists());
        assert!(!read(MANIFEST).contains("victim") && !read(MANIFEST).contains("old.html"));

        // A file a run didn't write is never overwritten; nothing tagged is an error.
        std::fs::remove_file(dir.join(MANIFEST)).unwrap();
        assert!(run().is_err_and(|e| e.contains("a.html is in the way")));
        assert!(publish(&dir, &notes, &[&secret], "publish", &kw(), |f| f.text.clone()).is_err_and(|e| e.contains("No notes tagged :publish:")));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn publish_site_links_everywhere() {
        let root = std::env::temp_dir().join(format!("margin-publish2-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (notes, dir) = (root.join("notes"), root.join("site"));
        std::fs::create_dir_all(notes.join("x")).unwrap();
        std::fs::create_dir_all(notes.join("y")).unwrap();
        std::fs::write(notes.join("x/f.txt"), "1").unwrap();
        std::fs::write(notes.join("y/f.txt"), "2").unwrap();
        let text = "#+title: T\n#+filetags: :publish:\n* H [[x/f.txt][in head]]\n- term :: [[y/f.txt][in term]] *[[x/f.txt][bold]]*\n  - [[y/f.txt]]\n| [[x/f.txt]] |\n#+begin_quote\n[[y/f.txt]]\n#+end_quote\n#+begin_verse\n[[x/f.txt]]\n#+end_verse\n";
        let mut a = org::parse(&notes.join("a.org"), text, &kw());
        a.mtime = Some(std::time::SystemTime::now());
        assert_eq!(publish(&dir, &notes, &[&a], "publish", &kw(), |f| f.text.clone()), Ok(1));
        let h = std::fs::read_to_string(dir.join("a.html")).unwrap();
        assert!(h.contains("assets/f.txt") && h.contains("assets/f (2).txt") || h.contains("assets/f.txt") && h.contains("assets/f.txt (2)"), "{h}");
        assert!(dir.join("assets").is_dir());
        let i = std::fs::read_to_string(dir.join("index.html")).unwrap();
        assert!(i.contains("class=\"date\""), "{i}");
        // Errors surface instead of panicking: DIR is a file; an asset vanishes before it is copied.
        let file = root.join("file");
        std::fs::write(&file, "").unwrap();
        assert!(publish(&file, &notes, &[&a], "publish", &kw(), |f| f.text.clone()).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn stale_pages() {
        let v = |s: &[&str]| s.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(stale(&v(&["a.html", "old.html", "pic.png", "b.md", "x.html.bak"]), &v(&["a.html", "b.html"]), "html"), ["old.html"]);
        assert_eq!(stale(&v(&["a.html", "b.md", "old.md"]), &v(&["b.md"]), "md"), ["old.md"]);
    }
}
