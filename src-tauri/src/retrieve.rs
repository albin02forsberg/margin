// "Use my notes" in the AI chat (#304): the notes that answer a question, found by keyword and
// one hop of links, as excerpts small enough for a local model's context.

use crate::notes::adjacency;
use crate::org::OrgFile;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// An excerpt of a note sent to the model; LINE (0-based) is where TEXT starts.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Source {
    pub path: PathBuf,
    pub title: String,
    pub line: usize,
    pub text: String,
}

/// Bytes of packed sources (labels included) a chat gets.
pub const BUDGET: usize = 3500;
/// Best-matching notes; their linked notes follow.
const SEEDS: usize = 4;
const MAX_SOURCES: usize = 8;
/// One excerpt's size, so several notes fit.
const EXCERPT: usize = 1000;
/// Don't bother with a cut excerpt smaller than this.
const MIN_PART: usize = 200;

const STOP: &[&str] = &[
    "the", "and", "for", "are", "was", "were", "what", "when", "where", "which", "who", "whom", "why", "how", "does", "did", "has", "have",
    "had", "this", "that", "these", "those", "with", "from", "about", "into", "can", "could", "would", "should", "will", "you", "your",
    "our", "their", "there", "they", "them", "its", "not", "but", "all", "any", "some", "tell", "know", "notes", "note", "please", "find",
    "och", "att", "det", "som", "för", "med", "har", "jag", "vad", "hur", "när", "var", "den", "till", "inte", "ett", "min", "mina", "mig",
];

/// The search terms of QUESTION: lowercase words of 3+ letters (2+ with a digit), minus stop-words.
fn terms(question: &str) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for w in question.to_lowercase().split(|c: char| !c.is_alphanumeric()) {
        let long = w.chars().count() >= 3 || (w.chars().count() == 2 && w.chars().any(|c| c.is_ascii_digit()));
        if long && !STOP.contains(&w) && !out.iter().any(|t| t == w) {
            out.push(w.to_string());
        }
    }
    out
}

/// Property drawers and `#+keyword:` lines say nothing to the model.
fn meta(l: &str) -> bool {
    let t = l.trim_start();
    t.starts_with("#+") || (t.starts_with(':') && t[1..].find(':').is_some_and(|i| i > 0 && t[1..=i].chars().all(|c| c.is_alphanumeric() || "_-".contains(c))))
}

/// F's score for TERMS (each counts 3 in the title, 2 in a heading, 1 in the text; more terms beat
/// more repeats) and its best line, if any term occurs.
fn score(f: &OrgFile, terms: &[String]) -> (usize, Option<usize>) {
    let lines: Vec<String> = f.text.lines().map(str::to_lowercase).collect();
    let heading = |i: usize| f.headlines.iter().any(|h| h.line == i);
    let title = std::iter::once(&f.title).chain(&f.aliases).map(|t| t.to_lowercase()).collect::<Vec<_>>().join(" ");
    let (mut sum, mut hits, mut best) = (0, 0, None::<(usize, usize)>);
    for t in terms {
        let mut w = if title.contains(t.as_str()) { 3 } else { 0 };
        for (i, l) in lines.iter().enumerate().filter(|(_, l)| l.contains(t.as_str()) && !meta(l)) {
            w = w.max(if heading(i) { 2 } else { 1 });
            hits += 1;
        }
        sum += w;
    }
    for (i, l) in lines.iter().enumerate().filter(|(_, l)| !meta(l)) {
        let n = terms.iter().filter(|t| l.contains(t.as_str())).count() * if heading(i) { 2 } else { 1 };
        if n > 0 && best.is_none_or(|(b, _)| n > b) {
            best = Some((n, i));
        }
    }
    (sum * 10 + hits.min(9), best.map(|(_, i)| i))
}

/// The excerpt of F around line AT (its start when None): the section holding it when that fits in
/// EXCERPT bytes, else the lines around it that do. Returns (first line, text).
fn excerpt(f: &OrgFile, at: Option<usize>) -> (usize, String) {
    let lines: Vec<&str> = f.text.lines().collect();
    let at = at.unwrap_or_else(|| lines.iter().position(|l| !meta(l) && !l.trim().is_empty()).unwrap_or(0)).min(lines.len().saturating_sub(1));
    let head = f.headlines.iter().rfind(|h| h.line <= at);
    let start = head.map_or(0, |h| h.line);
    let end = f.headlines.iter().find(|h| h.line > at && head.is_none_or(|x| h.level <= x.level)).map_or(lines.len(), |h| h.line);
    let size = |a: usize, b: usize| lines[a..b].iter().filter(|l| !meta(l)).map(|l| l.len() + 1).sum::<usize>();
    let (mut a, mut b) = (start, end);
    if size(a, b) > EXCERPT {
        (a, b) = (at, at + 1);
        loop {
            let down = b < end && size(a, b + 1) <= EXCERPT;
            if down {
                b += 1;
            }
            let up = a > start && size(a - 1, b) <= EXCERPT;
            if up {
                a -= 1;
            }
            if !up && !down {
                break;
            }
        }
    }
    let text = lines[a..b].iter().filter(|l| !meta(l)).copied().collect::<Vec<_>>().join("\n");
    (a, cut(text.trim(), EXCERPT).to_string())
}

/// TEXT cut to at most MAX bytes, on a char boundary.
fn cut(text: &str, max: usize) -> &str {
    let mut n = max.min(text.len());
    while !text.is_char_boundary(n) {
        n -= 1;
    }
    &text[..n]
}

fn label(s: &Source) -> String {
    let file = s.path.file_name().map_or_else(|| s.path.display().to_string(), |n| n.to_string_lossy().into_owned());
    format!("[note: {} | {file}:{}]\n", s.title, s.line + 1)
}

/// SOURCES as the model reads them: each labelled, at most BUDGET bytes in all.
pub fn pack(sources: &[Source]) -> String {
    let mut out = String::new();
    for s in sources {
        out += &label(s);
        out += &s.text;
        out += "\n\n";
    }
    cut(out.trim_end(), BUDGET).to_string()
}

/// The notes of FILES that answer QUESTION: the SEEDS best matches, then the notes linked to or from
/// them or the OPEN note (one hop), most-linked to good matches first; the open note itself is left
/// out (the chat sends it on its own). As many excerpts as fit in BUDGET once packed.
pub fn retrieve(files: &[Arc<OrgFile>], question: &str, open: Option<&Path>) -> Vec<Source> {
    let terms = terms(question);
    if terms.is_empty() {
        return vec![];
    }
    let scored: Vec<(usize, Option<usize>)> = files.iter().map(|f| score(f, &terms)).collect();
    let open = open.and_then(|p| files.iter().position(|f| f.path == p));
    let mut seeds: Vec<usize> = (0..files.len()).filter(|&i| scored[i].0 > 0 && Some(i) != open).collect();
    seeds.sort_by_key(|&i| (std::cmp::Reverse(scored[i].0), i));
    seeds.truncate(SEEDS);
    let adj = adjacency(files);
    // A neighbour ranks by the scores of the seeds it touches, then by how many it touches (the open note counts as one).
    let mut near: Vec<(usize, usize, usize)> = (0..files.len())
        .filter(|&i| Some(i) != open && !seeds.contains(&i))
        .filter_map(|i| {
            let linked: Vec<usize> = seeds.iter().copied().filter(|s| adj[i].contains(s)).collect();
            let to_open = open.is_some_and(|o| adj[i].contains(&o)) as usize;
            (linked.len() + to_open > 0).then(|| (linked.iter().map(|&s| scored[s].0).sum::<usize>() + scored[i].0, linked.len() + to_open, i))
        })
        .collect();
    near.sort_by_key(|&(s, n, i)| (std::cmp::Reverse(s), std::cmp::Reverse(n), i));
    let mut out = vec![];
    let mut left = BUDGET;
    for i in seeds.into_iter().chain(near.into_iter().map(|(.., i)| i)).take(MAX_SOURCES) {
        let (line, text) = excerpt(&files[i], scored[i].1);
        if text.is_empty() {
            continue;
        }
        let mut s = Source { path: files[i].path.clone(), title: files[i].title.clone(), line, text };
        let need = label(&s).len() + 2;
        if need + s.text.len() > left {
            if left < need + MIN_PART {
                break;
            }
            s.text = cut(&s.text, left - need).to_string();
        }
        left -= need + s.text.len();
        out.push(s);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::org::{parse, Kw};

    fn note(name: &str, text: &str) -> Arc<OrgFile> {
        Arc::new(parse(Path::new(&format!("/n/{name}.org")), text, &Kw { todo: vec![], done: vec![] }))
    }
    fn names(s: &[Source]) -> Vec<&str> {
        s.iter().map(|s| s.title.as_str()).collect()
    }

    #[test]
    fn terms_drop_stop_words() {
        assert_eq!(terms("What did we decide about the Q3 budget, and the budget?"), ["decide", "q3", "budget"]);
        assert_eq!(terms("Vad är det?"), Vec::<String>::new());
    }

    #[test]
    fn seeds_then_linked_notes() {
        let files = [
            note("a", ":PROPERTIES:\n:ID: a\n:END:\n#+title: Alpha\nThe launch date is May 3.\n"),
            note("b", "#+title: Budget\n* Launch budget\nWe spend 10k. See [[id:a][Alpha]].\n* Other\nunrelated\n"),
            note("c", "#+title: Misc\nlaunch mentioned once\n"),
            note("d", "#+title: Daily\nnothing here, but see [[file:b.org][the plan]]\n"),
            note("e", "#+title: Lonely\nno match, no link\n"),
        ];
        let got = retrieve(&files, "When is the launch budget due?", None);
        // Budget has both terms (one in a heading) > Alpha, Misc (one term each); Daily only links to Budget.
        assert_eq!(names(&got), ["Budget", "Alpha", "Misc", "Daily"]);
        // The matching section only, without the drawer or keywords.
        assert_eq!((got[0].line, got[0].text.as_str()), (1, "* Launch budget\nWe spend 10k. See [[id:a][Alpha]]."));
        assert_eq!(got[1].text, "The launch date is May 3.");
        assert_eq!(got[3].text, "nothing here, but see [[file:b.org][the plan]]");
        // The open note is left out, but its links count.
        let got = retrieve(&files, "launch", Some(Path::new("/n/b.org")));
        assert_eq!(names(&got), ["Alpha", "Misc", "Daily"]);
        assert!(retrieve(&files, "the and", None).is_empty());
        assert!(retrieve(&files, "zebra", None).is_empty());
    }

    #[test]
    fn stays_in_budget() {
        let big = format!("#+title: Big\n{}", "ålpha ".repeat(2000));
        let files: Vec<_> = (0..6).map(|i| note(&format!("n{i}"), &big)).collect();
        let got = retrieve(&files, "ålpha", None);
        let packed = pack(&got);
        assert!(packed.len() <= BUDGET, "{}", packed.len());
        assert!(got.len() >= 3 && got.iter().all(|s| s.text.len() <= EXCERPT));
        assert!(packed.starts_with("[note: Big | n0.org:2]\nålpha"));
        // a long section: the lines around the match
        let long = note("l", &format!("#+title: L\n* H\n{}needle here\n{}", "filler line\n".repeat(200), "more\n".repeat(200)));
        let got = retrieve(&[long], "needle", None);
        assert!(got[0].text.contains("needle here") && got[0].text.len() <= EXCERPT && got[0].line > 100);
    }
}
