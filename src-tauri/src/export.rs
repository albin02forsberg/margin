// Org → HTML / Markdown for exporting notes and reports. A small block parser
// (headings, paragraphs, lists, tables, blocks) plus inline markup; drawers,
// planning lines, comments and #+ keywords other than title/subtitle are dropped.

use crate::org::{self, Kw};
use regex::Regex;
use std::path::Path;
use std::sync::LazyLock;

static LINK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\[\[([^\]]+)\](?:\[([^\]]*)\])?\]").unwrap());
static ITEM: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\s*)([-+*]|\d+[.)])\s+(?:\[([ Xx-])\]\s+)?(.*)$").unwrap());
static BLOCK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^\s*#\+begin_(\w+)\s*(\S*)").unwrap());
static DRAWER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*:[A-Za-z_]+:\s*$").unwrap());

#[derive(Debug, PartialEq)]
pub struct Heading {
    level: usize,
    kw: Option<(String, bool)>, // (keyword, done)
    prio: Option<String>,
    title: String,
    tags: Vec<String>,
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
}

pub struct Doc {
    pub title: Option<String>,
    subtitle: Option<String>,
    blocks: Vec<Block>,
}

fn indent(l: &str) -> usize {
    l.len() - l.trim_start().len()
}

pub fn parse(text: &str, kw: &Kw) -> Doc {
    let lines: Vec<&str> = text.lines().collect();
    let mut d = Doc { title: None, subtitle: None, blocks: vec![] };
    let b = blocks(&lines, kw, &mut d);
    d.blocks = b;
    d
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
            out.push(Block::Heading(Heading { level, kw: k.map(|k| (k.clone(), kw.is_done(&k))), prio, title, tags }));
        } else if t.is_empty() || org::is_planning(l) || t == "#" || t.starts_with("# ") {
            flush(&mut para, &mut out);
        } else if DRAWER.is_match(l) && lower != ":end:" {
            flush(&mut para, &mut out);
            while i < lines.len() && lines[i].trim().to_lowercase() != ":end:" {
                i += 1;
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

/// Resolve a link target: Ok(url) or Err(plain text) for id: links. file: paths
/// become absolute file:// URLs (relative to BASE, the note's folder).
fn target(t: &str, desc: Option<&str>, base: &Path) -> Result<String, String> {
    if let Some(id) = t.strip_prefix("id:") {
        return Err(desc.unwrap_or(id).to_string());
    }
    match t.strip_prefix("file:") {
        Some(p) => {
            let p = base.join(p.split("::").next().unwrap_or(p));
            Ok(format!("file:///{}", p.to_string_lossy().replace('\\', "/").trim_start_matches('/')))
        }
        None => Ok(t.to_string()),
    }
}

// ---------------------------------------------------------------- HTML

pub fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#39;")
}

fn html_spans(s: &[Span], base: &Path) -> String {
    s.iter()
        .map(|sp| match sp {
            Span::Text(t) => esc(t),
            Span::Mark(m, inner) => {
                let tag = match m { '*' => "b", '/' => "i", '_' => "u", '+' => "del", _ => "code" };
                format!("<{tag}>{}</{tag}>", html_spans(inner, base))
            }
            Span::Link(t, d) => match target(t, d.as_deref(), base) {
                Err(text) => html_spans(&spans(&text), base),
                Ok(u) if d.is_none() && is_image(t) => format!("<img src=\"{}\" alt=\"\">", esc(&embed(t, base).unwrap_or(u))),
                Ok(u) => format!("<a href=\"{}\">{}</a>", esc(&u), d.as_deref().map_or_else(|| esc(t), |d| html_spans(&spans(d), base))),
            },
        })
        .collect()
}

fn html_blocks(bs: &[Block], base: &Path) -> String {
    let inl = |s: &str| html_spans(&spans(s), base);
    let mut o = String::new();
    for b in bs {
        match b {
            Block::Heading(h) => {
                let n = (h.level + 1).min(6);
                o += &format!("<h{n}>");
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
                    o += &format!("<dt>{}</dt><dd>{}{}</dd>\n", inl(it.term.as_deref().unwrap_or("")), inl(&it.text), html_blocks(&it.children, base));
                }
                o += "</dl>\n";
            }
            Block::List(ordered, items) => {
                let tag = if *ordered { "ol" } else { "ul" };
                o += &format!("<{tag}>\n");
                for it in items {
                    let cb = it.check.map_or(String::new(), |c| format!("<input type=\"checkbox\" disabled{}> ", if c == 'X' || c == 'x' { " checked" } else { "" }));
                    o += &format!("<li>{cb}{}{}</li>\n", inl(&it.text), html_blocks(&it.children, base));
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
            Block::Pre(kind, lang, body) => {
                let lang = if lang.is_empty() { String::new() } else { format!(" data-lang=\"{}\"", esc(lang)) };
                o += &format!("<pre class=\"{}\"{lang}><code>{}</code></pre>\n", esc(kind), esc(body));
            }
            Block::Quote(inner) => o += &format!("<blockquote>\n{}</blockquote>\n", html_blocks(inner, base)),
            Block::Rule => o += "<hr>\n",
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
hr { border: 0; border-top: 1px solid var(--line); }
.kw { font: 600 .75em ui-monospace, monospace; } .kw.todo { color: var(--todo); } .kw.done { color: var(--done); }
.prio { color: var(--dim); font-weight: 400; font-size: .8em; }
.tags span { font-size: .6em; font-weight: 400; color: var(--dim); border: 1px solid var(--line); border-radius: 1em; padding: 0 .5em; margin-left: .3em; vertical-align: middle; }
li:has(> input[type=checkbox]) { list-style: none; margin-left: -1.3em; }
@media print { body { margin: 0; max-width: none; } a { color: inherit; } }
";

/// Self-contained HTML page. BASE resolves relative file: links; PRINT opens the print dialog on load.
pub fn html(d: &Doc, fallback_title: &str, base: &Path, print: bool) -> String {
    let title = d.title.as_deref().unwrap_or(fallback_title);
    let mut o = format!("<!doctype html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>{}</title>\n<style>{CSS}</style>\n</head>\n<body>\n", esc(title));
    o += &format!("<h1>{}</h1>\n", html_spans(&spans(title), base));
    if let Some(s) = &d.subtitle {
        o += &format!("<p class=\"subtitle\">{}</p>\n", html_spans(&spans(s), base));
    }
    o += &html_blocks(&d.blocks, base);
    if print {
        o += "<script>addEventListener(\"load\", () => print())</script>\n";
    }
    o + "</body>\n</html>\n"
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

fn md_spans(s: &[Span], base: &Path) -> String {
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
                format!("{a}{}{b}", md_spans(inner, base))
            }
            Span::Link(t, d) => match target(t, d.as_deref(), base) {
                Err(text) => md_spans(&spans(&text), base),
                Ok(u) => {
                    let u = if u.contains([' ', '(', ')', '<', '>']) { format!("<{}>", u.replace('<', "%3C").replace('>', "%3E")) } else { u };
                    let img = if d.is_none() && is_image(t) { "!" } else { "" };
                    let text = if img.is_empty() { d.as_deref().map_or_else(|| md_esc(t), |d| md_spans(&spans(d), base)) } else { String::new() };
                    format!("{img}[{text}]({u})")
                }
            },
        })
        .collect()
}

fn md_blocks(bs: &[Block], base: &Path) -> String {
    let inl = |s: &str| md_spans(&spans(s), base);
    let mut parts: Vec<String> = vec![];
    for b in bs {
        parts.push(match b {
            Block::Heading(h) => {
                let mut o = format!("{} ", "#".repeat((h.level + 1).min(6)));
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
                    let kids = md_blocks(&it.children, base);
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
            Block::Quote(inner) => md_blocks(inner, base).lines().map(|l| if l.is_empty() { ">".into() } else { format!("> {l}") }).collect::<Vec<_>>().join("\n"),
            Block::Rule => "---".into(),
        });
    }
    parts.join("\n\n")
}

pub fn markdown(d: &Doc, fallback_title: &str, base: &Path) -> String {
    let title = d.title.as_deref().unwrap_or(fallback_title);
    let mut o = format!("# {}\n\n", md_spans(&spans(title), base));
    if let Some(s) = &d.subtitle {
        o += &format!("*{}*\n\n", md_spans(&spans(s), base));
    }
    o + &md_blocks(&d.blocks, base) + "\n"
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
    fn parses_blocks() {
        let d = parse(NOTE, &kw());
        assert_eq!(d.title.as_deref(), Some("Plans <&>"));
        let kinds: Vec<&str> = d.blocks.iter().map(|b| match b {
            Block::Heading(_) => "h", Block::Para(_) => "p", Block::List(..) => "l", Block::Table(_) => "t", Block::Pre(..) => "pre", Block::Quote(_) => "q", Block::Rule => "hr",
        }).collect();
        assert_eq!(kinds, ["h", "p", "l", "l", "t", "pre", "q", "pre", "h", "hr"]);
        let Block::List(false, items) = &d.blocks[2] else { panic!() };
        assert_eq!((items.len(), items[1].check, items[2].check), (3, Some('X'), Some(' ')));
        assert!(matches!(&items[0].children[..], [Block::List(false, _)]));
        let Block::Table(g) = &d.blocks[4] else { panic!() };
        assert_eq!((g.len(), g[1][0][0].as_str()), (3, "<a>"));
    }

    #[test]
    fn inline_markup() {
        assert_eq!(spans("a*b*c"), [Span::Text("a*b*c".into())]);
        assert_eq!(spans("3 + 4 + 5"), [Span::Text("3 + 4 + 5".into())]);
        assert_eq!(spans("(*x /y/*)"), [Span::Text("(".into()), Span::Mark('*', vec![Span::Text("x ".into()), Span::Mark('/', vec![Span::Text("y".into())])]), Span::Text(")".into())]);
        assert_eq!(spans("=*no*="), [Span::Mark('=', vec![Span::Text("*no*".into())])]);
    }

    #[test]
    fn to_html() {
        let h = html(&parse(NOTE, &kw()), "x", Path::new("/notes"), false);
        for want in [
            "<title>Plans &lt;&amp;&gt;</title>",
            "<h1>Plans &lt;&amp;&gt;</h1>",
            "<h2><span class=\"kw todo\">TODO</span> <span class=\"prio\">[#A]</span> Ship <b>it</b> <span class=\"tags\"><span>work</span><span>urgent</span></span></h2>",
            "<p>Some <b>bold</b> <i>italic</i> <u>under</u> <code>a&lt;b</code> <code>code</code> <del>gone</del> text, a*b*c and 3 + 4. Second line.</p>",
            "<li>one<ul>\n<li>nested <a href=\"https://x.org/?a=1&amp;b=2\">site</a></li>\n</ul>\n</li>",
            "<li><input type=\"checkbox\" disabled checked> done item</li>",
            "<li><input type=\"checkbox\" disabled> open item</li>",
            "<ol>\n<li>first</li>\n<li>second</li>\n</ol>",
            "<thead>\n<tr><th>Name</th><th>Qty</th></tr>\n</thead>\n<tbody>\n<tr><td>&lt;a&gt;</td><td>1</td></tr>\n</tbody>\n<tbody>\n<tr><td>Sum</td>",
            "<pre class=\"src\" data-lang=\"rust\"><code>fn x() -&gt; &amp;&#39;a str { &quot;&lt;&quot; }</code></pre>",
            "<blockquote>\n<p>Quoted <i>line</i>.</p>\n</blockquote>",
            "<pre class=\"example\"><code>&lt;raw&gt;</code></pre>",
            "<h3><span class=\"kw done\">DONE</span> See Other note and <img src=\"file:///notes/pic.png\" alt=\"\"> and <a href=\"file:///notes/doc.org\">doc</a></h3>",
            "<hr>",
        ] {
            assert!(h.contains(want), "missing {want}\n---\n{h}");
        }
        for gone in ["PROPERTIES", "SCHEDULED", "filetags", "TBLFM", "comment", "abc", "<script"] {
            assert!(!h.contains(gone), "{gone} leaked\n{h}");
        }
        assert!(html(&parse("x", &kw()), "t", Path::new("/"), true).contains("print()"));
    }

    #[test]
    fn to_markdown() {
        let m = markdown(&parse(NOTE, &kw()), "x", Path::new("/notes"));
        for want in [
            "# Plans \\<&\\>\n\n## **TODO** \\[#A\\] Ship **it** `work` `urgent`\n\n",
            "Some **bold** *italic* <u>under</u> `a<b` `code` ~~gone~~ text, a\\*b\\*c and 3 + 4. Second line.\n\n",
            "- one\n  - nested [site](https://x.org/?a=1&b=2)\n- [x] done item\n- [ ] open item\n\n1. first\n2. second\n\n",
            "| Name | Qty |\n| --- | --- |\n| \\<a\\> | 1 |\n| Sum | 1 |\n\n",
            "```rust\nfn x() -> &'a str { \"<\" }\n```\n\n> Quoted *line*.\n\n",
            "```\n<raw>\n```\n\n### **DONE** See Other note and ![](file:///notes/pic.png) and [doc](file:///notes/doc.org)\n\n---\n",
        ] {
            assert!(m.contains(want), "missing {want}\n---\n{m}");
        }
        assert!(!m.contains("PROPERTIES") && !m.contains("SCHEDULED") && !m.contains("TBLFM"));
    }

    #[test]
    fn followups() {
        let dir = std::env::temp_dir().join(format!("margin-export-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("dot.png"), [137u8, 80, 78, 71]).unwrap();
        let note = "See https://example.com/a_b, then (http://x.y).\n\n- Apple :: a fruit\n- Kale :: *green*\n\n#+begin_verse\nRoses /red/\n  violets\n#+end_verse\n\n[[file:dot.png]] [[file:missing.png]]\n";
        let d = parse(note, &kw());
        let h = html(&d, "x", &dir, false);
        for want in [
            "<a href=\"https://example.com/a_b\">https://example.com/a_b</a>, then (<a href=\"http://x.y\">http://x.y</a>).",
            "<dl>\n<dt>Apple</dt><dd>a fruit</dd>\n<dt>Kale</dt><dd><b>green</b></dd>\n</dl>",
            "<p class=\"verse\">Roses <i>red</i><br>\n  violets</p>",
            "<img src=\"data:image/png;base64,iVBORw==\" alt=\"\">",
            "missing.png\" alt",
        ] {
            assert!(h.contains(want), "missing {want:?} in\n{h}");
        }
        let m = markdown(&d, "x", &dir);
        for want in ["- **Apple**: a fruit", "Roses *red*  \n  violets", "[https://example.com/a\\_b](https://example.com/a_b)"] {
            assert!(m.contains(want), "missing {want:?} in\n{m}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
