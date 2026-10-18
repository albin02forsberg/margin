// "Use my notes" in the AI chat (#304): the notes that answer a question, found by keyword and
// one hop of links, as excerpts small enough for a local model's context; plus the open tasks,
// the agenda and the time log when the question asks about them (#468).

use crate::notes::adjacency;
use crate::org::{Item, OrgFile};
use crate::timeclock::{format_hm, Session};
use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// An excerpt sent to the model: of a note (LINE, 0-based, is where TEXT starts), or of the open
/// tasks, the agenda or the time log (KIND tasks/agenda/time, no path).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Source {
    #[serde(default = "note_kind")]
    pub kind: String,
    pub path: PathBuf,
    pub title: String,
    pub line: usize,
    pub text: String,
}

fn note_kind() -> String {
    "note".into()
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
/// One tasks/agenda/time source's size, so notes still fit.
const APP: usize = 700;

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
    let text = f.text.to_lowercase();
    let title = std::iter::once(&f.title).chain(&f.aliases).map(|t| t.to_lowercase()).collect::<Vec<_>>().join(" ");
    // Most notes match nothing: skip them before splitting into lines.
    if !terms.iter().any(|t| text.contains(t.as_str()) || title.contains(t.as_str())) {
        return (0, None);
    }
    // The non-meta lines, each with its heading flag.
    let mut heading = vec![false; f.text.lines().count()];
    for h in &f.headlines {
        if let Some(x) = heading.get_mut(h.line) {
            *x = true;
        }
    }
    let lines: Vec<(usize, &str, bool)> = text.lines().enumerate().filter(|(_, l)| !meta(l)).map(|(i, l)| (i, l, heading[i])).collect();
    let (mut sum, mut hits, mut best) = (0, 0, None::<(usize, usize)>);
    for t in terms {
        let mut w = if title.contains(t.as_str()) { 3 } else { 0 };
        for &(_, _, h) in lines.iter().filter(|(_, l, _)| l.contains(t.as_str())) {
            w = w.max(if h { 2 } else { 1 });
            hits += 1;
        }
        sum += w;
    }
    for &(i, l, h) in &lines {
        let n = terms.iter().filter(|t| l.contains(t.as_str())).count();
        let n = if h { n * 2 } else { n };
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
    if s.kind != "note" {
        return format!("[{}: {}]\n", s.kind, s.title);
    }
    let file = s.path.file_name().map_or_else(|| s.path.display().to_string(), |n| n.to_string_lossy().into_owned());
    format!("[note: {} | {file}:{}]\n", s.title, s.line + 1)
}

/// Bytes S takes once packed.
pub fn cost(s: &Source) -> usize {
    label(s).len() + s.text.len() + 2
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
/// out (the chat sends it on its own). As many excerpts as fit in LEFT bytes once packed.
pub fn retrieve(files: &[Arc<OrgFile>], question: &str, open: Option<&Path>, mut left: usize) -> Vec<Source> {
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
    for i in seeds.into_iter().chain(near.into_iter().map(|(.., i)| i)).take(MAX_SOURCES) {
        let (line, text) = excerpt(&files[i], scored[i].1);
        if text.is_empty() {
            continue;
        }
        let mut s = Source { kind: note_kind(), path: files[i].path.clone(), title: files[i].title.clone(), line, text };
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

/// Whether PATH (under ROOT, the notes folder) matches one of `ai_exclude`'s PATTERNS: a
/// case-insensitive substring of its `/`-separated path below ROOT.
pub fn excluded(path: &Path, root: &Path, patterns: &[String]) -> bool {
    let p = path.strip_prefix(root).unwrap_or(path).to_string_lossy().replace('\\', "/").to_lowercase();
    patterns.iter().map(|x| x.trim().to_lowercase()).any(|x| !x.is_empty() && p.contains(&x))
}

/// Which of the open tasks, the agenda and the time log QUESTION asks about, by word prefixes.
pub fn wants(question: &str) -> [bool; 3] {
    const KEYS: [&[&str]; 3] = [
        &["task", "todo", "project", "deadline", "priorit", "uppgift"],
        &["today", "tomorrow", "week", "agenda", "deadline", "schedul", "meeting", "calendar", "idag", "imorgon", "vecka", "möte"],
        &["hour", "time", "tracked", "worked", "project", "week", "timmar", "jobbat", "vecka"],
    ];
    let q = question.to_lowercase();
    let words: Vec<&str> = q.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect();
    KEYS.map(|keys| words.iter().any(|w| keys.iter().any(|k| w.starts_with(k))))
}

/// LINES as one source of KIND, whole lines up to APP bytes; EMPTY when there are none.
fn app_source(kind: &str, title: &str, lines: impl Iterator<Item = String>, empty: &str) -> Source {
    let mut text = String::new();
    for l in lines {
        if text.len() + l.len() + 1 > APP {
            break;
        }
        text += &l;
        text.push('\n');
    }
    let text = if text.is_empty() { empty.to_string() } else { text.trim_end().to_string() };
    Source { kind: kind.into(), path: PathBuf::new(), title: title.into(), line: 0, text }
}

fn file_name(p: &Path) -> String {
    p.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned())
}

/// The open TASKS, the AGENDA (both from org) and SESSIONS of the 7 days up to TODAY as sources,
/// one line per entry, at most APP bytes each; None is left out (switched off or not asked about).
pub fn app_sources(tasks: Option<&[Item]>, agenda: Option<&[Item]>, sessions: Option<&[Session]>, today: NaiveDate) -> Vec<Source> {
    let head = |i: &Item| {
        let k = i.keyword.as_deref().map(|k| format!("{k} ")).unwrap_or_default();
        let p = i.priority.as_deref().map(|p| format!("[#{p}] ")).unwrap_or_default();
        let tags = if i.tags.is_empty() { String::new() } else { format!(" :{}:", i.tags.join(":")) };
        format!("{k}{p}{}{tags} ({})", i.title, file_name(&i.path))
    };
    let mut out = vec![];
    if let Some(t) = tasks {
        let lines = t.iter().map(|i| {
            let date = if i.date == NaiveDate::MAX { String::new() } else { format!(", dated {}", i.date) };
            format!("- {}{date}", head(i))
        });
        out.push(app_source("tasks", "open tasks", lines, "No open tasks."));
    }
    if let Some(a) = agenda {
        let lines = a.iter().map(|i| {
            let what = if !i.label.is_empty() { i.label.clone() } else if i.kind == "timestamp" { "Event".into() } else { i.kind.to_string() };
            let time = i.time.as_deref().map(|t| format!(" {t}")).unwrap_or_default();
            format!("- {}{time} {what}: {}", i.date.format("%Y-%m-%d %a"), head(i))
        });
        out.push(app_source("agenda", "next 7 days", lines, "Nothing scheduled in the next 7 days."));
    }
    if let Some(s) = sessions {
        let week: Vec<&Session> = s.iter().filter(|s| s.date > today - Duration::days(7) && s.date <= today).collect();
        let mut totals: Vec<(&str, f64)> = vec![];
        for s in &week {
            match totals.iter_mut().find(|(p, _)| *p == s.project) {
                Some(t) => t.1 += s.hours,
                None => totals.push((s.project.as_str(), s.hours)),
            }
        }
        totals.sort_by(|a, b| b.1.total_cmp(&a.1));
        let total = (!totals.is_empty()).then(|| format!("Totals: {}", totals.iter().map(|(p, h)| format!("{p} {}", format_hm(*h))).collect::<Vec<_>>().join(", ")));
        let lines = week.iter().rev().map(|s| {
            let desc = if s.desc.is_empty() { String::new() } else { format!(" ({})", s.desc) };
            format!("- {} {}–{} {}{desc}: {}", s.date.format("%Y-%m-%d %a"), s.start.format("%H:%M"), s.end.format("%H:%M"), s.project, format_hm(s.hours))
        });
        out.push(app_source("time", "last 7 days", total.into_iter().chain(lines), "Nothing tracked in the last 7 days."));
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
        let got = retrieve(&files, "When is the launch budget due?", None, BUDGET);
        // Budget has both terms (one in a heading) > Alpha, Misc (one term each); Daily only links to Budget.
        assert_eq!(names(&got), ["Budget", "Alpha", "Misc", "Daily"]);
        // The matching section only, without the drawer or keywords.
        assert_eq!((got[0].line, got[0].text.as_str()), (1, "* Launch budget\nWe spend 10k. See [[id:a][Alpha]]."));
        assert_eq!(got[1].text, "The launch date is May 3.");
        assert_eq!(got[3].text, "nothing here, but see [[file:b.org][the plan]]");
        // The open note is left out, but its links count.
        let got = retrieve(&files, "launch", Some(Path::new("/n/b.org")), BUDGET);
        assert_eq!(names(&got), ["Alpha", "Misc", "Daily"]);
        assert!(retrieve(&files, "the and", None, BUDGET).is_empty());
        assert!(retrieve(&files, "zebra", None, BUDGET).is_empty());
        // No term in text or title: no score; a title-only match still counts.
        assert_eq!(score(&files[4], &terms("zebra")), (0, None));
        assert_eq!(score(&files[4], &terms("lonely zebra")), (30, None));
    }

    #[test]
    fn stays_in_budget() {
        let big = format!("#+title: Big\n{}", "ålpha ".repeat(2000));
        let files: Vec<_> = (0..6).map(|i| note(&format!("n{i}"), &big)).collect();
        let got = retrieve(&files, "ålpha", None, BUDGET);
        let packed = pack(&got);
        assert!(packed.len() <= BUDGET, "{}", packed.len());
        assert!(got.len() >= 3 && got.iter().all(|s| s.text.len() <= EXCERPT));
        assert!(packed.starts_with("[note: Big | n0.org:2]\nålpha"));
        // a long section: the lines around the match
        let long = note("l", &format!("#+title: L\n* H\n{}needle here\n{}", "filler line\n".repeat(200), "more\n".repeat(200)));
        let got = retrieve(&[long], "needle", None, BUDGET);
        assert!(got[0].text.contains("needle here") && got[0].text.len() <= EXCERPT && got[0].line > 100);
    }

    #[test]
    fn wants_by_keyword() {
        assert_eq!(wants("Which tasks are open?"), [true, false, false]);
        assert_eq!(wants("What's on my agenda today?"), [false, true, false]);
        assert_eq!(wants("How many hours did I log on Margin?"), [false, false, true]);
        assert_eq!(wants("What did I do this week?"), [false, true, true]);
        assert_eq!(wants("Hur många timmar idag?"), [false, true, true]);
        assert_eq!(wants("When is the launch?"), [false, false, false]);
    }

    #[test]
    fn app_sources_and_mixed_packing() {
        let kw = Kw { todo: vec!["TODO".into()], done: vec!["DONE".into()] };
        let day = NaiveDate::from_ymd_opt(2026, 10, 18).unwrap();
        let files = [Arc::new(parse(Path::new("/n/work.org"), "* TODO [#A] Ship it :work:\nDEADLINE: <2026-10-20 Tue>\n* TODO Later\n* DONE Old\n", &kw))];
        let tasks = crate::org::todos(&files, &kw, false, |_, _| true, day);
        let agenda = crate::org::agenda(&files, &kw, day, 7, day);
        let t = |d: u32, p: &str, h: f64| Session {
            date: NaiveDate::from_ymd_opt(2026, 10, d).unwrap(),
            project: p.into(),
            desc: if p == "Margin" { "chat".into() } else { String::new() },
            hours: h,
            start: chrono::NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            end: chrono::NaiveTime::from_hms_opt(10, 30, 0).unwrap(),
            line: None,
        };
        // the 11th is more than 7 days back
        let sessions = [t(11, "Old", 5.0), t(16, "Margin", 1.5), t(17, "Other", 0.5), t(18, "Margin", 1.5)];
        let got = app_sources(Some(tasks.as_slice()), Some(agenda.as_slice()), Some(&sessions[..]), day);
        assert_eq!(got[0].text, "- TODO [#A] Ship it :work: (work.org), dated 2026-10-20\n- TODO Later (work.org)");
        assert_eq!(got[1].text, "- 2026-10-18 Sun Due in 2d: TODO [#A] Ship it :work: (work.org)\n- 2026-10-20 Tue Due: TODO [#A] Ship it :work: (work.org)");
        assert_eq!(
            got[2].text,
            "Totals: Margin 3h 00m, Other 30m\n- 2026-10-18 Sun 09:00–10:30 Margin (chat): 1h 30m\n- 2026-10-17 Sat 09:00–10:30 Other: 30m\n- 2026-10-16 Fri 09:00–10:30 Margin (chat): 1h 30m"
        );
        assert!(pack(&got[2..]).starts_with("[time: last 7 days]\nTotals:"));
        // empty data says so; switched off is left out
        let got = app_sources(Some(&[][..]), Some(&[][..]), None, day);
        assert_eq!(got.iter().map(|s| (s.kind.as_str(), s.text.as_str())).collect::<Vec<_>>(), [("tasks", "No open tasks."), ("agenda", "Nothing scheduled in the next 7 days.")]);
        // a long list is cut on whole lines; notes fill the rest of the budget
        let many: Vec<_> = (0..100).map(|i| Session { desc: "é".repeat(i % 7), ..t(18, "Margin", 1.0) }).collect();
        let mut got = app_sources(None, None, Some(&many[..]), day);
        assert!(got[0].text.len() <= APP && got[0].text.ends_with(": 1h 00m"));
        let big = format!("#+title: Big\n{}", "ålpha ".repeat(2000));
        let notes: Vec<_> = (0..6).map(|i| note(&format!("n{i}"), &big)).collect();
        let used: usize = got.iter().map(cost).sum();
        got.extend(retrieve(&notes, "ålpha", None, BUDGET - used));
        assert!(got.len() >= 3 && got.iter().map(cost).sum::<usize>() <= BUDGET);
        assert!(pack(&got).starts_with("[time: last 7 days]\nTotals: Margin 100h 00m\n"));
    }

    #[test]
    fn exclude_patterns() {
        let root = Path::new("/n");
        let ex = ["Private/".to_string(), " ".into()];
        assert!(excluded(Path::new("/n/private/diary.org"), root, &ex));
        assert!(!excluded(Path::new("/n/work.org"), root, &ex));
        // only the path below the notes folder counts
        assert!(!excluded(Path::new("/n/work.org"), root, &["n/".to_string()]));
        assert!(!excluded(Path::new("/n/work.org"), root, &[]));
        // an excluded note isn't found, nor reached as a linked note
        let files = [note("plan", "#+title: Plan\nlaunch soon, see [[file:secret.org][s]]\n"), note("secret", "#+title: Secret\ncodes\n")];
        let kept: Vec<_> = files.iter().filter(|f| !excluded(&f.path, root, &["secret".to_string()])).cloned().collect();
        assert_eq!(names(&retrieve(&kept, "launch", None, BUDGET)), ["Plan"]);
        assert_eq!(names(&retrieve(&files, "launch", None, BUDGET)), ["Plan", "Secret"]);
    }
}
