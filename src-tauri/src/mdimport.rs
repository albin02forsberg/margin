// Markdown/Obsidian import (#900): every `.md` file of a folder becomes an org note under
// <notes>/imported/<name>/ (`<name>-2/`… on a re-run), keeping the subfolder layout. The
// source is only read; notes are created new, never written over.

use regex::{Captures, Regex};
use serde::Serialize;
use std::collections::HashMap;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::LazyLock;

/// What the converter asks the importer to resolve: a wiki/`.md` link target (a note name
/// without `.md`) or an embedded file (as written, relative to the Markdown file).
pub enum Ref<'a> {
    Wiki(&'a str),
    File(&'a str),
}

static INLINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?x)
        `(?P<code>[^`]+)`
        | !\[\[(?P<emb>[^\]|]+)(?:\|[^\]]*)?\]\]
        | !\[(?P<alt>[^\]]*)\]\((?P<src>[^)\s]+)(?:\s+"[^"]*")?\)
        | \[\[(?P<wiki>[^\]|]+)(?:\|(?P<alias>[^\]]+))?\]\]
        | \[(?P<text>[^\]]+)\]\((?P<url>[^)\s]+)(?:\s+"[^"]*")?\)
        | \*\*(?P<b1>\S(?:.*?\S)?)\*\*
        | \b__(?P<b2>\S(?:.*?\S)?)__\b
        | ~~(?P<st>\S(?:.*?\S)?)~~
        | \*(?P<i1>[^*\s](?:[^*]*[^*\s])?)\*
        | \b_(?P<i2>[^_\s](?:[^_]*[^_\s])?)_\b
        "#,
    )
    .unwrap()
});
static HEADING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(#{1,6})\s+(.*?)(?:\s+#+)?\s*$").unwrap());
static FENCE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\s*)(```+|~~~+)\s*([\w+#.-]*)").unwrap());
static LIST: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\s*)(?:[*+-]|(\d+[.)]))\s+(?:\[([ xX])\]\s)?(.*)$").unwrap());
static HR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^ {0,3}(?:(?:-\s*){3,}|(?:\*\s*){3,}|(?:_\s*){3,})$").unwrap());

fn has_scheme(u: &str) -> bool {
    u.contains("://") || u.starts_with("mailto:")
}

fn decode(s: &str) -> String {
    percent_encoding::percent_decode_str(s).decode_utf8_lossy().into_owned()
}

/// A wiki target `folder/Note.md#Heading` as (note name `Note`, org search `::*Heading` or "").
fn wiki_target(t: &str) -> (&str, String) {
    let (t, anchor) = t.split_once('#').unwrap_or((t, ""));
    let name = t.rsplit('/').next().unwrap_or(t);
    let name = if name.to_ascii_lowercase().ends_with(".md") { &name[..name.len() - 3] } else { name };
    let search = if anchor.is_empty() || anchor.starts_with('^') { String::new() } else { format!("::*{anchor}") };
    (name.trim(), search)
}

/// Inline Markdown (links, embeds, emphasis, code) as org; N counts the links rewritten.
fn inline(s: &str, r: &mut dyn FnMut(Ref) -> Option<String>, n: &mut usize) -> String {
    INLINE
        .replace_all(s, |c: &Captures| {
            let g = |k: &str| c.name(k).map(|m| m.as_str());
            if let Some(code) = g("code") {
                return if code.contains('~') { format!("={code}=") } else { format!("~{code}~") };
            }
            if let Some(e) = g("emb") {
                let (name, search) = wiki_target(e);
                let file = Path::new(name).extension().is_some_and(|x| !x.eq_ignore_ascii_case("md"));
                let to = if file { r(Ref::File(e.trim())) } else { r(Ref::Wiki(name)).map(|t| format!("{t}{search}")) };
                return match to {
                    Some(t) => {
                        *n += 1;
                        format!("[[file:{t}]]")
                    }
                    None => format!("[[{e}]]"),
                };
            }
            if let Some(src) = g("src") {
                *n += 1;
                if has_scheme(src) {
                    return format!("[[{src}]]");
                }
                return format!("[[file:{}]]", r(Ref::File(&decode(src))).unwrap_or_else(|| decode(src)));
            }
            if let Some(w) = g("wiki") {
                let (name, search) = wiki_target(w);
                let desc = g("alias").unwrap_or(w).trim();
                if name.is_empty() && !search.is_empty() {
                    *n += 1;
                    return format!("[[{}][{desc}]]", &search[2..]);
                }
                return match r(Ref::Wiki(name)) {
                    Some(t) => {
                        *n += 1;
                        format!("[[file:{t}{search}][{desc}]]")
                    }
                    None if desc == w => format!("[[{w}]]"),
                    None => format!("[[{w}][{desc}]]"),
                };
            }
            if let (Some(text), Some(url)) = (g("text"), g("url")) {
                *n += 1;
                let text = inline(text, r, n);
                if has_scheme(url) || url.starts_with('#') {
                    return format!("[[{url}][{text}]]");
                }
                let path = decode(url.split('#').next().unwrap_or(url));
                let to = if path.to_ascii_lowercase().ends_with(".md") {
                    let (name, search) = wiki_target(&path); // `#slug` anchors don't name an org heading
                    r(Ref::Wiki(name)).map(|t| format!("{t}{search}"))
                } else {
                    r(Ref::File(&path))
                };
                return format!("[[file:{}][{text}]]", to.unwrap_or(path));
            }
            if let Some(b) = g("b1").or(g("b2")) {
                return format!("*{}*", inline(b, r, n));
            }
            if let Some(st) = g("st") {
                return format!("+{}+", inline(st, r, n));
            }
            let i = g("i1").or(g("i2")).unwrap();
            format!("/{}/", inline(i, r, n))
        })
        .into_owned()
}

/// YAML front matter lines as (key, values): `k: v`, `k: [a, b]` and `- item` lines under `k:`.
fn front_matter(lines: &[&str]) -> Vec<(String, Vec<String>)> {
    let unquote = |s: &str| s.trim().trim_matches(|c| c == '"' || c == '\'').to_string();
    let mut v: Vec<(String, Vec<String>)> = vec![];
    for l in lines {
        if let Some(item) = l.trim().strip_prefix("- ") {
            if let Some(last) = v.last_mut() {
                last.1.push(unquote(item));
            }
        } else if let Some((k, val)) = l.split_once(':').filter(|_| !l.starts_with([' ', '\t'])) {
            let val = val.trim();
            let vals = match val.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                Some(list) => list.split(',').map(unquote).collect(),
                None => vec![unquote(val)],
            };
            v.push((k.trim().to_string(), vals.into_iter().filter(|s| !s.is_empty()).collect()));
        }
    }
    v
}

/// A Markdown note as org. STEM (the file name) is the title unless front matter or a lone
/// leading `# Title` gives one; that heading becomes `#+title:` and the rest move up a level.
/// RESOLVE maps link targets to paths relative to the new note. Returns (org, links rewritten).
pub fn md_to_org(text: &str, stem: &str, resolve: &mut dyn FnMut(Ref) -> Option<String>) -> (String, usize) {
    let mut lines: Vec<&str> = text.lines().collect();
    let mut head: Vec<String> = vec![];
    let mut title = None;
    if lines.first().is_some_and(|l| l.trim_end() == "---") {
        if let Some(end) = lines.iter().skip(1).position(|l| matches!(l.trim_end(), "---" | "...")) {
            for (k, vals) in front_matter(&lines[1..=end]) {
                match k.to_ascii_lowercase().as_str() {
                    _ if vals.is_empty() => {}
                    "title" => title = Some(vals[0].clone()),
                    "tags" | "tag" => {
                        let tags: Vec<String> = vals
                            .iter()
                            .flat_map(|v| v.split([',', ' ']))
                            .map(|t| t.trim_start_matches('#').replace(|c: char| !(c.is_alphanumeric() || "_@#%".contains(c)), "_"))
                            .filter(|t| !t.is_empty())
                            .collect();
                        head.push(format!("#+FILETAGS: :{}:", tags.join(":")));
                    }
                    _ => head.push(format!("#+{k}: {}", vals.join(", "))),
                }
            }
            lines.drain(..end + 2);
        }
    }
    // Heading levels outside code fences: a lone level-1 first heading is the title.
    let (mut fenced, mut levels) = (false, vec![]);
    for (i, l) in lines.iter().enumerate() {
        if FENCE.is_match(l) {
            fenced = !fenced;
        } else if let Some(c) = HEADING.captures(l).filter(|_| !fenced) {
            levels.push((i, c[1].len()));
        }
    }
    let mut shift = 0;
    let mut skip = None;
    if title.is_none() && levels.first().is_some_and(|f| f.1 == 1) && levels.iter().filter(|l| l.1 == 1).count() == 1 {
        let i = levels[0].0;
        title = Some(HEADING.captures(lines[i]).unwrap()[2].to_string());
        (shift, skip) = (1, Some(i));
    }
    let mut n = 0;
    let title = inline(title.as_deref().unwrap_or(stem), resolve, &mut n);
    let mut out = vec![format!("#+title: {title}")];
    out.extend(head);
    let mut fence: Option<(String, &str)> = None; // (closing marker, END line)
    for (i, l) in lines.iter().enumerate() {
        if let Some((marker, end)) = &fence {
            let t = l.trim();
            if t.starts_with(marker.as_str()) && t.trim_start_matches(&marker[..1]).is_empty() {
                out.push(format!("{}{end}", &l[..l.len() - l.trim_start().len()]));
                fence = None;
            } else {
                // org-escape lines that would read as a heading or keyword
                let body = l.trim_start();
                let bare = body.trim_start_matches(',');
                let ind = &l[..l.len() - body.len()];
                out.push(if bare.starts_with('*') || bare.starts_with("#+") { format!("{ind},{body}") } else { l.to_string() });
            }
            continue;
        }
        if skip == Some(i) {
            continue;
        }
        if let Some(c) = FENCE.captures(l) {
            let (ind, lang) = (&c[1], &c[3]);
            let (begin, end) = if lang.is_empty() { ("#+BEGIN_EXAMPLE".to_string(), "#+END_EXAMPLE") } else { (format!("#+BEGIN_SRC {lang}"), "#+END_SRC") };
            out.push(format!("{ind}{begin}"));
            fence = Some((c[2].to_string(), end));
        } else if let Some(c) = HEADING.captures(l) {
            out.push(format!("{} {}", "*".repeat(c[1].len().saturating_sub(shift).max(1)), inline(&c[2], resolve, &mut n)));
        } else if HR.is_match(l) {
            out.push("-----".into());
        } else if let Some(c) = LIST.captures(l) {
            let bullet = c.get(2).map_or("-", |m| m.as_str());
            let check = c.get(3).map_or(String::new(), |m| format!("[{}] ", if m.as_str() == " " { " " } else { "X" }));
            out.push(format!("{}{bullet} {check}{}", &c[1], inline(&c[4], resolve, &mut n)));
        } else if l.trim_start().starts_with('|') && l.contains('-') && l.trim().chars().all(|c| "|-: ".contains(c)) {
            let cells = l.trim().trim_matches('|').split('|').map(|_| "---").collect::<Vec<_>>();
            out.push(format!("|{}|", cells.join("+")));
        } else {
            out.push(inline(l, resolve, &mut n));
        }
    }
    if let Some((_, end)) = fence {
        out.push(end.into());
    }
    (out.join("\n") + "\n", n)
}

/// A Markdown folder may be anywhere the user picks, but only an absolute path with no `..`,
/// and never one overlapping the notes, data or config folder (a re-run would import its own output).
pub fn source_ok(p: &Path, forbidden: &[PathBuf]) -> Result<(), String> {
    if !p.is_absolute() || p.components().any(|c| c == Component::ParentDir) {
        return Err(format!("{} must be an absolute path to a folder", p.display()));
    }
    if forbidden.iter().any(|r| p.starts_with(r) || r.starts_with(p)) {
        return Err(format!("{} overlaps the notes, data or config folder", p.display()));
    }
    if !p.is_dir() {
        return Err(format!("{} isn't a folder", p.display()));
    }
    Ok(())
}

/// TO relative to the folder FROM, `/`-separated.
fn rel(from: &Path, to: &Path) -> String {
    let (a, b): (Vec<_>, Vec<_>) = (from.components().collect(), to.components().collect());
    let k = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let mut parts = vec!["..".to_string(); a.len() - k];
    parts.extend(b[k..].iter().map(|c| c.as_os_str().to_string_lossy().into_owned()));
    parts.join("/")
}

#[derive(Serialize, Default, Debug)]
pub struct Summary {
    /// The new notes, relative to TARGET.
    pub notes: Vec<String>,
    pub links: usize,
    /// Images and attachments copied.
    pub files: usize,
    /// Markdown files that couldn't be read as text.
    pub skipped: Vec<String>,
    pub target: PathBuf,
}

/// Convert every Markdown file under SRC (dot folders skipped) into NOTES/imported/<name>/;
/// embedded files go where attachments go (`attach_in`). DRY only reports what would happen.
pub fn import(src: &Path, notes: &Path, attachments: &str, images: &str, dry: bool) -> Result<Summary, String> {
    let all: Vec<PathBuf> = walkdir::WalkDir::new(src)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|e| e.depth() == 0 || !e.file_name().to_string_lossy().starts_with('.'))
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .collect();
    let is_md = |p: &Path| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("md") || e.eq_ignore_ascii_case("markdown"));
    let name = src.file_name().map_or("markdown".into(), |n| n.to_string_lossy().into_owned());
    let base = notes.join("imported");
    let target = (1..).map(|i| base.join(if i == 1 { name.clone() } else { format!("{name}-{i}") })).find(|p| !p.exists()).unwrap();
    let natural = |p: &Path| target.join(p.strip_prefix(src).unwrap()).with_extension("org");
    // Destinations, deduplicated (`a.md` + `a.markdown` -> `a.org`, `a-2.org`; case-insensitive, never a name another source maps to).
    let mut dests: HashMap<PathBuf, PathBuf> = HashMap::new();
    let key = |p: &Path| p.to_string_lossy().to_lowercase();
    let mut used: std::collections::HashSet<String> = std::collections::HashSet::new();
    let reserved: std::collections::HashSet<String> = all.iter().filter(|p| is_md(p)).map(|p| key(&natural(p))).collect();
    for p in all.iter().filter(|p| is_md(p)) {
        let (mut d, mut i) = (natural(p), 1);
        while used.contains(&key(&d)) || (i > 1 && reserved.contains(&key(&d))) {
            i += 1;
            d = natural(p).with_file_name(format!("{}-{i}.org", natural(p).file_stem().unwrap().to_string_lossy()));
        }
        used.insert(key(&d));
        dests.insert(p.clone(), d);
    }
    let dest = |p: &Path| dests[p].clone();
    let lower = |p: &Path, f: fn(&Path) -> Option<&std::ffi::OsStr>| f(p).map_or(String::new(), |s| s.to_string_lossy().to_lowercase());
    let (mut stems, mut others) = (HashMap::new(), HashMap::new());
    for p in &all {
        if is_md(p) {
            stems.entry(lower(p, Path::file_stem)).or_insert_with(|| dest(p));
        } else {
            others.entry(lower(p, Path::file_name)).or_insert_with(|| p.clone());
        }
    }
    let mut s = Summary { target: target.clone(), ..Default::default() };
    let mut copied: HashMap<(PathBuf, PathBuf), String> = HashMap::new();
    let created = std::cell::RefCell::new(Vec::<PathBuf>::new()); // attachments written by this run
    let mut run = || -> Result<(), String> {
    for p in all.iter().filter(|p| is_md(p)) {
        let shown = p.strip_prefix(src).unwrap().to_string_lossy().into_owned();
        let Ok(text) = std::fs::read_to_string(p) else {
            s.skipped.push(shown);
            continue;
        };
        let note = dest(p);
        let dir = note.parent().unwrap().to_path_buf();
        let mut files = 0;
        let mut err = None;
        let (org, links) = md_to_org(&text, &p.file_stem().unwrap().to_string_lossy(), &mut |r| match r {
            Ref::Wiki(n) => stems.get(&n.to_lowercase()).map(|t| rel(&dir, t)),
            Ref::File(f) => {
                let near = crate::attach::normalize(&p.parent().unwrap().join(f));
                let from = if near.starts_with(src) && near.is_file() { near } else { others.get(&f.rsplit('/').next().unwrap_or(f).to_lowercase())?.clone() };
                let file = from.file_name()?.to_string_lossy().into_owned();
                let key = (if crate::attach::is_image(&file) && !images.is_empty() { dir.clone() } else { note.clone() }, from.clone());
                if let Some(l) = copied.get(&key) {
                    return Some(l.clone());
                }
                files += 1;
                let link = if dry { file } else { crate::attach_in(notes, attachments, images, &note, &file, |q| q.exists(), |d| std::fs::copy(&from, d).map(|_| created.borrow_mut().push(d.to_path_buf()))).map_err(|e| err = Some(e)).ok()? };
                copied.insert(key, link.clone());
                Some(link)
            }
        });
        if let Some(e) = err {
            return Err(e);
        }
        if !dry {
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let mut f = std::fs::File::create_new(&note).map_err(|e| format!("{}: {e}", note.display()))?;
            f.write_all(org.as_bytes()).map_err(|e| format!("{}: {e}", note.display()))?;
        }
        s.links += links;
        s.files += files;
        s.notes.push(note.strip_prefix(&target).unwrap().to_string_lossy().into_owned());
    }
    Ok(())
    };
    if let Err(e) = run() {
        // Undo this run: the target folder is new, and the attachments are files it created.
        let mut gone = std::fs::remove_dir_all(&target).or_else(|e| if e.kind() == std::io::ErrorKind::NotFound { Ok(()) } else { Err(e) });
        for f in created.borrow().iter() {
            let _ = std::fs::remove_file(f);
            let _ = std::fs::remove_dir(f.parent().unwrap()); // only if now empty
        }
        if let Err(r) = gone.as_mut() {
            return Err(format!("{e} (and couldn't remove {}: {r})", target.display()));
        }
        return Err(e);
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conv(md: &str) -> String {
        let known = ["Other", "deep note"];
        md_to_org(md, "stem", &mut |r| match r {
            Ref::Wiki(n) => known.iter().find(|k| k.eq_ignore_ascii_case(n)).map(|k| format!("{k}.org")),
            Ref::File(f) => (f == "pic.png").then(|| "images/pic.png".into()),
        })
        .0
    }

    #[test]
    fn converts_each_construct() {
        let body = |md: &str| conv(md).strip_prefix("#+title: stem\n").unwrap().trim_end().to_string();
        for (md, org) in [
            ("## Sub ##", "** Sub"),
            ("text #tag and C# here", "text #tag and C# here"),
            ("#tag at start", "#tag at start"),
            ("* item\n  + nested\n- [ ] open\n* [x] done\n3. third", "- item\n  - nested\n- [ ] open\n- [X] done\n3. third"),
            ("**bold** and *it* and _it2_ and __b2__ ~~gone~~", "*bold* and /it/ and /it2/ and *b2* +gone+"),
            ("snake_case_name and 2 * 3 * 4", "snake_case_name and 2 * 3 * 4"),
            ("`a *b* [[Other]]` and `x~y`", "~a *b* [[Other]]~ and =x~y="),
            ("[site](https://x.org/a_b_c) and [**b**](Other.md#Top) and [doc](a%20b.pdf)", "[[https://x.org/a_b_c][site]] and [[file:Other.org][*b*]] and [[file:a b.pdf][doc]]"),
            ("[[Other]] [[other|that one]] [[Missing]] [[Missing|m]] [[Deep Note#Sec]] [[#Here]]", "[[file:Other.org][Other]] [[file:Other.org][that one]] [[Missing]] [[Missing][m]] [[file:deep note.org::*Sec][Deep Note#Sec]] [[*Here][#Here]]"),
            ("![a](pic.png) ![](https://x/y.png) ![[pic.png]] ![[gone.png]]", "[[file:images/pic.png]] [[https://x/y.png]] [[file:images/pic.png]] [[gone.png]]"),
            ("```rust\n* not a heading\n#+x\n**b**\n```\nafter", "#+BEGIN_SRC rust\n,* not a heading\n,#+x\n,**b**\n#+END_SRC\nafter"),
            ("~~~\nplain\n", "#+BEGIN_EXAMPLE\nplain\n#+END_EXAMPLE"),
            ("---\n***", "-----\n-----"),
            ("| a | b |\n|:--|--:|\n| *x* | 1 |", "| a | b |\n|---+---|\n| /x/ | 1 |"),
        ] {
            assert_eq!(body(md), org, "{md}");
        }
    }

    #[test]
    fn titles_and_front_matter() {
        assert_eq!(conv("# Big Title\n\nintro\n## Part\n### Sub"), "#+title: Big Title\n\nintro\n* Part\n** Sub\n");
        // two level-1 headings: no title taken, levels kept
        assert_eq!(conv("# A\n# B"), "#+title: stem\n* A\n* B\n");
        // a `#` inside code isn't a heading
        assert_eq!(conv("```sh\n# comment\n```\n# T"), "#+title: T\n#+BEGIN_SRC sh\n# comment\n#+END_SRC\n");
        let fm = "---\ntitle: \"From FM\"\ntags: [work, project/x]\naliases:\n  - one\n  - two\nempty:\n---\n# Heading\nbody";
        assert_eq!(conv(fm), "#+title: From FM\n#+FILETAGS: :work:project_x:\n#+aliases: one, two\n* Heading\nbody\n");
        assert!(conv("---\ntags:\n  - a\n  - b c\n---\nx").contains("#+FILETAGS: :a:b:c:\n"));
        // an unclosed `---` is a rule, not front matter
        assert_eq!(conv("---\nx"), "#+title: stem\n-----\nx\n");
    }

    #[test]
    fn rel_paths() {
        assert_eq!(rel(Path::new("/t/a"), Path::new("/t/a/n.org")), "n.org");
        assert_eq!(rel(Path::new("/t/a/b"), Path::new("/t/c/n.org")), "../../c/n.org");
    }

    #[test]
    fn source_must_be_an_absolute_folder_away_from_app_dirs() {
        let root = std::env::temp_dir().join(format!("margin-mdsrc-{}", std::process::id()));
        let (src, notes) = (root.join("vault"), root.join("notes"));
        std::fs::create_dir_all(&src).unwrap();
        let forbidden = [notes.clone()];
        assert_eq!(source_ok(&src, &forbidden), Ok(()));
        assert!(source_ok(Path::new("vault"), &forbidden).is_err());
        assert!(source_ok(&src.join("../vault"), &forbidden).is_err());
        assert!(source_ok(&notes.join("sub"), &forbidden).is_err());
        assert!(source_ok(&root, &forbidden).is_err()); // contains the notes folder
        assert!(source_ok(&root.join("missing"), &forbidden).is_err());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn imports_a_vault_without_touching_it() {
        let root = std::env::temp_dir().join(format!("margin-mdimport-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (src, notes) = (root.join("Vault"), root.join("notes"));
        let w = |p: &str, b: &[u8]| {
            let p = src.join(p);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, b).unwrap();
        };
        w("Home.md", b"# Home\nSee [[Deep]] and ![](img/logo.png) and ![[logo.png]]\n");
        w("sub/Deep.md", b"Back to [[home]] ![[logo.png]] ![[spec.pdf]]\n");
        w("img/logo.png", b"PNG");
        w("spec.pdf", b"PDF");
        w("bad.md", &[0xff, 0xfe]);
        w(".obsidian/hidden.md", b"x");
        std::fs::create_dir_all(&notes).unwrap();
        let before: Vec<_> = walkdir::WalkDir::new(&src).into_iter().map(|e| e.unwrap().into_path()).collect();

        let dry = import(&src, &notes, "attachments", "images", true).unwrap();
        assert_eq!((dry.notes.len(), dry.links, dry.files, dry.skipped.clone()), (2, 6, 3, vec!["bad.md".to_string()]));
        assert!(!notes.join("imported").exists(), "a dry run writes nothing");

        let s = import(&src, &notes, "attachments", "images", false).unwrap();
        assert_eq!(s.target, notes.join("imported/Vault"));
        let t = &s.target;
        assert_eq!(std::fs::read_to_string(t.join("Home.org")).unwrap(), "#+title: Home\nSee [[file:sub/Deep.org][Deep]] and [[file:images/logo.png]] and [[file:images/logo.png]]\n");
        assert_eq!(std::fs::read_to_string(t.join("sub/Deep.org")).unwrap(), "#+title: Deep\nBack to [[file:../Home.org][home]] [[file:images/logo.png]] [[file:../../../attachments/Deep/spec.pdf]]\n");
        assert_eq!(std::fs::read(t.join("images/logo.png")).unwrap(), b"PNG");
        assert_eq!(std::fs::read(t.join("sub/images/logo.png")).unwrap(), b"PNG");
        assert_eq!(std::fs::read(notes.join("attachments/Deep/spec.pdf")).unwrap(), b"PDF");
        assert!(!t.join("hidden.org").exists() && !t.join(".obsidian").exists());

        // a re-run goes to a fresh folder; the source is unchanged
        assert_eq!(import(&src, &notes, "attachments", "images", false).unwrap().target, notes.join("imported/Vault-2"));
        let after: Vec<_> = walkdir::WalkDir::new(&src).into_iter().map(|e| e.unwrap().into_path()).collect();
        assert_eq!(before, after);
        assert_eq!(std::fs::read(src.join("Home.md")).unwrap(), b"# Home\nSee [[Deep]] and ![](img/logo.png) and ![[logo.png]]\n");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn colliding_names_get_a_suffix_and_a_failed_run_leaves_nothing() {
        let root = std::env::temp_dir().join(format!("margin-mdcollide-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (src, notes) = (root.join("V"), root.join("notes"));
        let w = |p: &str, b: &[u8]| std::fs::write(src.join(p), b).unwrap();
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&notes).unwrap();
        w("a.md", b"first [[a]]");
        w("a.markdown", b"second");
        w("a-2.md", b"own");
        let dry = import(&src, &notes, "att", "", true).unwrap();
        let real = import(&src, &notes, "att", "", false).unwrap();
        assert_eq!(dry.notes, real.notes);
        let mut n = real.notes.clone();
        n.sort();
        assert_eq!(n, ["a-2.org", "a-3.org", "a.org"]);
        assert_eq!(std::fs::read_to_string(real.target.join("a-2.org")).unwrap(), "#+title: a-2\nown\n");

        // a failure on the second note removes the folder and the attachment copied for the first
        let (src2, notes2) = (root.join("W"), root.join("notes2"));
        std::fs::create_dir_all(&src2).unwrap();
        std::fs::create_dir_all(notes2.join("att")).unwrap();
        std::fs::write(src2.join("a.md"), "![[a.pdf]]").unwrap();
        std::fs::write(src2.join("b.md"), "![[b.pdf]]").unwrap();
        std::fs::write(src2.join("a.pdf"), "x").unwrap();
        std::fs::write(src2.join("b.pdf"), "x").unwrap();
        std::fs::write(notes2.join("att/b"), "blocker").unwrap();
        assert!(import(&src2, &notes2, "att", "", false).is_err());
        assert!(!notes2.join("imported/W").exists());
        assert!(!notes2.join("att/a").exists());
        assert!(notes2.join("att/b").is_file());
        std::fs::remove_dir_all(&root).unwrap();
    }
}
