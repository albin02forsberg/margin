// "Use my notes" in the AI chat (#304): the notes that answer a question, found by keyword and
// one hop of links, as excerpts small enough for a local model's context; plus the open tasks,
// the agenda and the time log when the question asks about them (#468).

use crate::notes::adjacency;
use crate::org::{Item, Kw, OrgFile};
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
pub fn meta(l: &str) -> bool {
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
/// case-insensitive substring of its `/`-separated path below ROOT, which starts with `/` so
/// `/private/` anchors at the top. PATTERNS may use `\` or `/`; a lone `/` is ignored.
pub fn excluded(path: &Path, root: &Path, patterns: &[String]) -> bool {
    let p = format!("/{}", path.strip_prefix(root).unwrap_or(path).to_string_lossy().replace('\\', "/").to_lowercase());
    patterns.iter().map(|x| x.trim().replace('\\', "/").to_lowercase()).any(|x| !x.is_empty() && x != "/" && p.contains(&x))
}

/// The note of FILES titled TITLE (or with that alias), any case; of several, the first by path.
pub fn by_title<'a>(files: &'a [Arc<OrgFile>], title: &str) -> Option<&'a Arc<OrgFile>> {
    let t = title.trim().to_lowercase();
    files.iter().filter(|f| f.title.to_lowercase() == t || f.aliases.iter().any(|a| a.to_lowercase() == t)).min_by(|a, b| a.path.cmp(&b.path))
}

/// Bytes one tool result may take, so a few calls fit in BUDGET (#469).
pub const TOOL_RESULT: usize = 1500;
/// The longest range the agenda and time tools read.
const MAX_DAYS: i64 = 62;

/// What the chat's tools may read besides the `ai_exclude`-filtered notes (#990): the switches
/// (`ai_source_notes`, `ai_source_tasks`) and the time log when `ai_source_time` is on.
pub struct ToolCtx {
    pub kw: Kw,
    pub today: NaiveDate,
    pub notes: bool,
    pub tasks: bool,
    pub sessions: Option<Vec<Session>>,
}

/// The tools the chat model may call (Ollama's schema), as CTX switches them on: read-only, notes
/// by title, never by path (#469); tasks, the agenda and the time log (#990).
pub fn tools(ctx: &ToolCtx) -> serde_json::Value {
    let tool = |name: &str, desc: &str, args: &[(&str, &str)], required: bool| {
        let props: serde_json::Map<String, serde_json::Value> = args.iter().map(|(a, about)| (a.to_string(), serde_json::json!({ "type": "string", "description": about }))).collect();
        let req: Vec<&str> = if required { args.iter().map(|a| a.0).collect() } else { vec![] };
        serde_json::json!({ "type": "function", "function": { "name": name, "description": desc, "parameters": { "type": "object", "properties": props, "required": req } } })
    };
    let (from, to) = (("from", "First day, e.g. 2026-11-02, today, mon or -7d"), ("to", "Last day, same forms"));
    let mut out = vec![];
    if ctx.notes {
        out.push(tool("search_notes", "Excerpts of the user's notes that best match QUERY, plus notes linked to them.", &[("query", "Search words")], true));
        out.push(tool("open_note", "The text of one note, by its title.", &[("title", "The note's title, as search_notes labels it")], true));
        out.push(tool("neighbors", "The titles of the notes linked to or from one note.", &[("title", "The note's title")], true));
    }
    if ctx.tasks {
        out.push(tool("list_tasks", "The user's open tasks (state, priority, title, tags, file, date), most urgent first.", &[("state", "Only this state, e.g. TODO, NEXT or DONE"), ("project", "Only tasks with this tag or category")], false));
        out.push(tool("agenda", "What is scheduled or due from FROM to TO (default: the next 7 days).", &[from, to], false));
    }
    if ctx.sessions.is_some() {
        out.push(tool("time_report", "The time tracked from FROM to TO (default: the last 7 days): per-project totals, then each session.", &[from, to], false));
    }
    serde_json::Value::Array(out)
}

/// The system prompt's hint at the tools CTX offers, with today's date for the date ones.
pub fn tool_note(ctx: &ToolCtx) -> String {
    let mut can = vec![];
    if ctx.notes {
        can.push("search and read more of the user's notes");
    }
    if ctx.tasks {
        can.push("list their tasks or read their agenda");
    }
    if ctx.sessions.is_some() {
        can.push("read their time log");
    }
    let day = if ctx.tasks || ctx.sessions.is_some() { format!(" Today is {}.", ctx.today.format("%A %Y-%m-%d")) } else { String::new() };
    format!("If the context above doesn't answer it, use the tools to {}.{day}", can.join(", or "))
}

/// FROM..TO of ARGS relative to TODAY: one side alone gives 7 days; missing or unreadable dates
/// give DEFAULT, a reversed TO 7 days from FROM; at most MAX_DAYS long.
fn range(args: &serde_json::Value, today: NaiveDate, default: (NaiveDate, NaiveDate)) -> (NaiveDate, NaiveDate) {
    let d = |k: &str| args.get(k).and_then(|v| v.as_str()).and_then(|s| crate::org::read_date(s, today)).map(|w| w.0);
    let (from, to) = match (d("from"), d("to")) {
        (Some(f), Some(t)) if t >= f => (f, t),
        (Some(f), _) => (f, f + Duration::days(6)),
        (None, Some(t)) => (t - Duration::days(6), t),
        (None, None) => default,
    };
    (from, to.min(from + Duration::days(MAX_DAYS - 1)))
}

/// LINES as a tool result of KIND: labelled, whole lines up to TOOL_RESULT bytes; EMPTY when none.
fn listing(kind: &str, title: &str, lines: impl Iterator<Item = String>, empty: &str) -> String {
    let l = label(&Source { kind: kind.into(), path: PathBuf::new(), title: title.into(), line: 0, text: String::new() });
    let text = whole_lines(lines, TOOL_RESULT - l.len());
    l + if text.is_empty() { empty } else { &text }
}

/// The result of tool NAME with ARGS over FILES (already without `ai_exclude`d notes), if CTX
/// offers it, at most TOOL_RESULT bytes; errors are told to the model as text.
/// Also the notes it used (open_note's, search_notes' hits, texts emptied) for the chat's source list.
pub fn tool_sources(files: &[Arc<OrgFile>], ctx: &ToolCtx, name: &str, args: &serde_json::Value) -> (String, Vec<Source>) {
    let mut used = vec![];
    let opt = |k: &str| args.get(k).and_then(|v| v.as_str()).map(str::trim).filter(|s| !s.is_empty());
    let arg = |k: &str| opt(k).ok_or(format!("Error: {name} needs a {k}."));
    let note = |t: &str| by_title(files, t).ok_or(format!("Error: no note titled {t}. Use search_notes to find titles."));
    let offered: Vec<String> = tools(ctx).as_array().into_iter().flatten().filter_map(|t| t["function"]["name"].as_str().map(String::from)).collect();
    let day = ctx.today;
    let out = match name {
        _ if !offered.iter().any(|n| n == name) => Err(format!("Error: unknown tool {name}. Use {}.", offered.join(", "))),
        "search_notes" => arg("query").map(|q| {
            used = retrieve(files, q, None, TOOL_RESULT);
            match pack(&used) {
                p if p.is_empty() => format!("No note matches {q}."),
                p => p,
            }
        }),
        "open_note" => arg("title").and_then(note).map(|f| {
            let text = f.text.lines().filter(|l| !meta(l)).collect::<Vec<_>>().join("\n");
            let s = Source { kind: note_kind(), path: f.path.clone(), title: f.title.clone(), line: 0, text: text.trim().into() };
            let out = label(&s) + &s.text;
            used = vec![Source { text: String::new(), ..s }];
            out
        }),
        "neighbors" => arg("title").and_then(note).map(|f| {
            let i = files.iter().position(|g| Arc::ptr_eq(g, f)).unwrap_or(0);
            let near: Vec<String> = adjacency(files)[i].iter().map(|&j| format!("- {} ({})", files[j].title, file_name(&files[j].path))).collect();
            if near.is_empty() { format!("No note links to or from {}.", f.title) } else { format!("Notes linked to or from {}:\n{}", f.title, near.join("\n")) }
        }),
        "list_tasks" => {
            let state = opt("state").map(str::to_uppercase);
            let project = opt("project").map(str::to_lowercase);
            let done = state.as_deref().is_some_and(|s| ctx.kw.is_done(s));
            let items = crate::org::todos(files, &ctx.kw, done, |_, h| state.as_ref().is_none_or(|s| h.keyword.as_deref().is_some_and(|k| k.eq_ignore_ascii_case(s))), day);
            let has = |i: &&Item| project.as_ref().is_none_or(|p| std::iter::once(&i.category).chain(&i.tags).chain(&i.inherited).any(|t| t.to_lowercase() == *p));
            let title = format!("{} tasks{}", state.as_deref().unwrap_or("open"), project.as_ref().map(|p| format!(", project {p}")).unwrap_or_default());
            Ok(listing("tasks", &title, items.iter().filter(has).map(task_line), "No matching tasks."))
        }
        "agenda" => {
            let (from, to) = range(args, day, (day, day + Duration::days(6)));
            let items = crate::org::agenda(files, &ctx.kw, from, (to - from).num_days() + 1, day);
            Ok(listing("agenda", &format!("{from}..{to}"), items.iter().map(agenda_line), "Nothing scheduled then."))
        }
        "time_report" => {
            let (from, to) = range(args, day, (day - Duration::days(6), day));
            let lines = time_lines(ctx.sessions.as_deref().unwrap_or_default(), from, to);
            Ok(listing("time", &format!("{from}..{to}"), lines.into_iter(), "Nothing tracked then."))
        }
        _ => Err(format!("Error: unknown tool {name}.")),
    };
    for u in &mut used {
        u.text.clear(); // the list only needs where to jump
    }
    (cut(&out.unwrap_or_else(|e| e), TOOL_RESULT).to_string(), used)
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

/// LINES joined while they fit in MAX bytes, whole lines only.
fn whole_lines(lines: impl Iterator<Item = String>, max: usize) -> String {
    let mut text = String::new();
    for l in lines {
        if text.len() + l.len() + 1 > max {
            break;
        }
        text += &l;
        text.push('\n');
    }
    text.trim_end().to_string()
}

/// LINES as one source of KIND, whole lines up to APP bytes; EMPTY when there are none.
fn app_source(kind: &str, title: &str, lines: impl Iterator<Item = String>, empty: &str) -> Source {
    let text = whole_lines(lines, APP);
    let text = if text.is_empty() { empty.to_string() } else { text };
    Source { kind: kind.into(), path: PathBuf::new(), title: title.into(), line: 0, text }
}

fn file_name(p: &Path) -> String {
    p.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned())
}

/// A task or agenda entry: state, priority, title, tags and file.
fn head(i: &Item) -> String {
    let k = i.keyword.as_deref().map(|k| format!("{k} ")).unwrap_or_default();
    let p = i.priority.as_deref().map(|p| format!("[#{p}] ")).unwrap_or_default();
    let tags = if i.tags.is_empty() { String::new() } else { format!(" :{}:", i.tags.join(":")) };
    format!("{k}{p}{}{tags} ({})", i.title, file_name(&i.path))
}

fn task_line(i: &Item) -> String {
    let date = if i.date == NaiveDate::MAX { String::new() } else { format!(", dated {}", i.date) };
    format!("- {}{date}", head(i))
}

fn agenda_line(i: &Item) -> String {
    let what = if !i.label.is_empty() { i.label.clone() } else if i.kind == "timestamp" { "Event".into() } else { i.kind.to_string() };
    let time = i.time.as_deref().map(|t| format!(" {t}")).unwrap_or_default();
    format!("- {}{time} {what}: {}", i.date.format("%Y-%m-%d %a"), head(i))
}

/// The SESSIONS from FROM to TO: a line of per-project totals (most first), then one per session,
/// newest first; none when nothing was tracked.
fn time_lines(sessions: &[Session], from: NaiveDate, to: NaiveDate) -> Vec<String> {
    let week: Vec<&Session> = sessions.iter().filter(|s| s.date >= from && s.date <= to).collect();
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
    total.into_iter().chain(lines).collect()
}

/// The open TASKS, the AGENDA (both from org) and SESSIONS of the 7 days up to TODAY as sources,
/// one line per entry, at most APP bytes each; None is left out (switched off or not asked about).
pub fn app_sources(tasks: Option<&[Item]>, agenda: Option<&[Item]>, sessions: Option<&[Session]>, today: NaiveDate) -> Vec<Source> {
    let mut out = vec![];
    if let Some(t) = tasks {
        out.push(app_source("tasks", "open tasks", t.iter().map(task_line), "No open tasks."));
    }
    if let Some(a) = agenda {
        out.push(app_source("agenda", "next 7 days", a.iter().map(agenda_line), "Nothing scheduled in the next 7 days."));
    }
    if let Some(s) = sessions {
        out.push(app_source("time", "last 7 days", time_lines(s, today - Duration::days(6), today).into_iter(), "Nothing tracked in the last 7 days."));
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
        // a leading `/` is fine, `\` works as a separator, the root itself and a lone `/` never match
        let top = |p: &str| excluded(Path::new(p), root, &["/Private/".to_string()]);
        assert!(top("/n/private/diary.org") && top("/n/a/private/d.org") && !top("/n/privates/d.org"));
        assert!(excluded(Path::new("/n/private/d.org"), root, &["private\\".to_string()]));
        assert!(!excluded(Path::new("/n/x.org"), root, &["/n/".to_string()]));
        assert!(!excluded(Path::new("/n/x.org"), root, &["/".to_string()]));
        // an excluded note isn't found, nor reached as a linked note
        let files = [note("plan", "#+title: Plan\nlaunch soon, see [[file:secret.org][s]]\n"), note("secret", "#+title: Secret\ncodes\n")];
        let kept: Vec<_> = files.iter().filter(|f| !excluded(&f.path, root, &["secret".to_string()])).cloned().collect();
        assert_eq!(names(&retrieve(&kept, "launch", None, BUDGET)), ["Plan"]);
        assert_eq!(names(&retrieve(&files, "launch", None, BUDGET)), ["Plan", "Secret"]);
    }

    #[test]
    fn tools_over_filtered_notes() {
        let files = [
            note("plan", ":PROPERTIES:\n:ID: p\n:END:\n#+title: Plan\nlaunch soon, see [[file:secret.org][s]] and [[file:budget.org][b]]\n"),
            note("budget", "#+title: Budget\n#+filetags: :money:\nWe spend 10k.\n"),
            note("daily", "#+title: Daily\nsee [[id:p][Plan]]\n"),
            note("secret", "#+title: Secret\nlaunch codes\n"),
            note("big", &format!("#+title: Big\n{}", "ålpha ".repeat(2000))),
        ];
        let kept: Vec<_> = files.iter().filter(|f| !excluded(&f.path, Path::new("/n"), &["secret".to_string()])).cloned().collect();
        let ctx = ToolCtx { kw: Kw { todo: vec![], done: vec![] }, today: NaiveDate::from_ymd_opt(2026, 10, 18).unwrap(), notes: true, tasks: false, sessions: None };
        let run = |name: &str, args: serde_json::Value| tool_sources(&kept, &ctx, name, &args).0;
        assert_eq!(tool_names(&ctx), ["search_notes", "open_note", "neighbors"]);
        assert_eq!(tool_note(&ctx), "If the context above doesn't answer it, use the tools to search and read more of the user's notes.");
        // search: the matching note, not the excluded one
        let got = run("search_notes", serde_json::json!({ "query": "launch" }));
        assert!(got.starts_with("[note: Plan | plan.org:1]\nlaunch soon") && !got.contains("codes"), "{got}");
        assert_eq!(run("search_notes", serde_json::json!({ "query": "zebra" })), "No note matches zebra.");
        // open: any case, without drawers or keywords; excluded and unknown titles aren't found
        assert_eq!(run("open_note", serde_json::json!({ "title": "budget" })), "[note: Budget | budget.org:1]\nWe spend 10k.");
        for t in ["Secret", "Nope"] {
            assert!(run("open_note", serde_json::json!({ "title": t })).starts_with(&format!("Error: no note titled {t}.")));
        }
        // neighbors: outgoing (Budget) and backlinks (Daily), never the excluded note
        assert_eq!(run("neighbors", serde_json::json!({ "title": "Plan" })), "Notes linked to or from Plan:\n- Budget (budget.org)\n- Daily (daily.org)");
        assert_eq!(run("neighbors", serde_json::json!({ "title": "Big" })), "No note links to or from Big.");
        // each result stays in TOOL_RESULT, cut on a char boundary
        let big = run("open_note", serde_json::json!({ "title": "Big" }));
        assert!(big.len() <= TOOL_RESULT && big.len() > TOOL_RESULT - 4 && big.starts_with("[note: Big"));
        assert!(run("search_notes", serde_json::json!({ "query": "ålpha" })).len() <= TOOL_RESULT);
        // the notes a call used, for the source list
        let used = |name: &str, args: serde_json::Value| tool_sources(&kept, &ctx, name, &args).1.iter().map(|s| (s.title.clone(), s.text.is_empty())).collect::<Vec<_>>();
        assert_eq!(used("open_note", serde_json::json!({ "title": "Big" })), [("Big".to_string(), true)]);
        assert!(used("open_note", serde_json::json!({ "title": "Nope" })).is_empty());
        assert!(used("neighbors", serde_json::json!({ "title": "Big" })).is_empty());
        assert!(used("search_notes", serde_json::json!({ "query": "nothingmatchesthis" })).is_empty());
        assert!(!used("search_notes", serde_json::json!({ "query": "launch" })).is_empty());
        // bad calls are told to the model
        assert_eq!(run("open_note", serde_json::json!({})), "Error: open_note needs a title.");
        assert_eq!(run("open_note", serde_json::json!({ "title": 3 })), "Error: open_note needs a title.");
        assert_eq!(run("rm", serde_json::json!({})), "Error: unknown tool rm. Use search_notes, open_note, neighbors.");
        // switched off: not offered, and not run
        assert!(run("list_tasks", serde_json::json!({})).starts_with("Error: unknown tool list_tasks."));
    }

    fn tool_names(ctx: &ToolCtx) -> Vec<String> {
        tools(ctx).as_array().unwrap().iter().map(|t| t["function"]["name"].as_str().unwrap().to_string()).collect()
    }

    #[test]
    fn task_agenda_and_time_tools() {
        let kw = Kw { todo: vec!["TODO".into(), "NEXT".into()], done: vec!["DONE".into()] };
        let day = NaiveDate::from_ymd_opt(2026, 10, 18).unwrap(); // a Sunday
        let file = |name: &str, text: &str| Arc::new(parse(Path::new(&format!("/n/{name}.org")), text, &kw));
        let files = [
            file("work", "* TODO [#A] Ship it :acme:\nDEADLINE: <2026-10-20 Tue>\n* NEXT Call Bob\nSCHEDULED: <2026-11-02 Mon>\n* DONE Invoice :acme:\n"),
            file("secret", "* TODO Hidden plan\nSCHEDULED: <2026-10-19 Mon>\n"),
        ];
        let kept: Vec<_> = files.iter().filter(|f| !excluded(&f.path, Path::new("/n"), &["secret".to_string()])).cloned().collect();
        let t = |d: u32, p: &str| Session {
            date: NaiveDate::from_ymd_opt(2026, 10, d).unwrap(),
            project: p.into(),
            desc: String::new(),
            hours: 1.5,
            start: chrono::NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            end: chrono::NaiveTime::from_hms_opt(10, 30, 0).unwrap(),
            line: None,
        };
        let ctx = ToolCtx { kw: kw.clone(), today: day, notes: false, tasks: true, sessions: Some(vec![t(5, "Old"), t(16, "Acme"), t(18, "Acme")]) };
        let run = |name: &str, args: serde_json::Value| tool_sources(&kept, &ctx, name, &args).0;
        // notes off: only the task, agenda and time tools; tasks off and no time log: none of them
        assert_eq!(tool_names(&ctx), ["list_tasks", "agenda", "time_report"]);
        assert!(run("search_notes", serde_json::json!({ "query": "ship" })).starts_with("Error: unknown tool search_notes."));
        assert!(tool_names(&ToolCtx { kw: kw.clone(), today: day, notes: false, tasks: false, sessions: None }).is_empty());
        assert_eq!(tool_note(&ctx), "If the context above doesn't answer it, use the tools to list their tasks or read their agenda, or read their time log. Today is Sunday 2026-10-18.");
        // the optional args aren't required
        assert_eq!(tools(&ctx)[0]["function"]["parameters"]["required"], serde_json::json!([]));
        // tasks: open ones by default, by state (done too) and project; never from an excluded note
        assert_eq!(run("list_tasks", serde_json::json!({})), "[tasks: open tasks]\n- TODO [#A] Ship it :acme: (work.org), dated 2026-10-20\n- NEXT Call Bob (work.org), dated 2026-11-02");
        assert_eq!(run("list_tasks", serde_json::json!({ "state": "done" })), "[tasks: DONE tasks]\n- DONE Invoice :acme: (work.org)");
        assert_eq!(run("list_tasks", serde_json::json!({ "project": "ACME" })), "[tasks: open tasks, project acme]\n- TODO [#A] Ship it :acme: (work.org), dated 2026-10-20");
        assert_eq!(run("list_tasks", serde_json::json!({ "project": "nope" })), "[tasks: open tasks, project nope]\nNo matching tasks.");
        // agenda: the next 7 days by default, or the range asked for; bad dates fall back
        let next = run("agenda", serde_json::json!({}));
        assert!(next.starts_with("[agenda: 2026-10-18..2026-10-24]\n") && next.contains("Ship it") && !next.contains("Call Bob") && !next.contains("Hidden"), "{next}");
        assert_eq!(run("agenda", serde_json::json!({ "from": "2026-11-02", "to": "2026-11-02" })), "[agenda: 2026-11-02..2026-11-02]\n- 2026-11-02 Mon scheduled: NEXT Call Bob (work.org)");
        assert!(run("agenda", serde_json::json!({ "from": "whenever", "to": 3 })).starts_with("[agenda: 2026-10-18..2026-10-24]"));
        assert!(run("agenda", serde_json::json!({ "from": "2026-12-01", "to": "2026-11-01" })).starts_with("[agenda: 2026-12-01..2026-12-07]\nNothing scheduled then."));
        assert!(run("agenda", serde_json::json!({ "from": "2026-01-01", "to": "2026-12-31" })).starts_with("[agenda: 2026-01-01..2026-03-03]"));
        // time: the last 7 days by default, totals first; a range with nothing says so
        assert_eq!(run("time_report", serde_json::json!({})), "[time: 2026-10-12..2026-10-18]\nTotals: Acme 3h 00m\n- 2026-10-18 Sun 09:00–10:30 Acme: 1h 30m\n- 2026-10-16 Fri 09:00–10:30 Acme: 1h 30m");
        assert_eq!(run("time_report", serde_json::json!({ "from": "2026-10-05", "to": "2026-10-05" })), "[time: 2026-10-05..2026-10-05]\nTotals: Old 1h 30m\n- 2026-10-05 Mon 09:00–10:30 Old: 1h 30m");
        assert_eq!(run("time_report", serde_json::json!({ "to": "2026-10-01" })), "[time: 2026-09-25..2026-10-01]\nNothing tracked then.");
        // a long result stays in TOOL_RESULT, on whole lines; these tools use no notes
        let many = ToolCtx { sessions: Some((0..200).map(|_| t(18, "Acme")).collect()), ..ctx };
        let (big, used) = tool_sources(&kept, &many, "time_report", &serde_json::json!({}));
        assert!(big.len() <= TOOL_RESULT && big.ends_with(": 1h 30m") && used.is_empty());
    }

    #[test]
    fn title_lookup_first_by_path() {
        let files = [note("b", "#+title: Same\n"), note("a", ":PROPERTIES:\n:ROAM_ALIASES: same\n:END:\n#+title: Other\n"), note("c", "#+title: same\n")];
        assert_eq!(by_title(&files, " SAME ").unwrap().path, Path::new("/n/a.org"));
        assert_eq!(by_title(&files, "other").unwrap().path, Path::new("/n/a.org"));
        assert!(by_title(&files, "none").is_none());
    }
}
