// Kindle `My Clippings.txt` / Readwise CSV import (#1040): one org note per book under
// <notes>/Reading/, one heading per highlight. A re-import only appends highlights whose
// `:HIGHLIGHT_KEY:` the note doesn't hold yet; existing text is never rewritten.

use chrono::NaiveDateTime;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Highlight {
    pub book: String,
    pub author: String,
    pub text: String,
    /// As shown, e.g. `page 12, location 180-182`.
    pub location: String,
    /// An org inactive timestamp, or the source's date text when it couldn't be read.
    pub added: String,
    pub note: String,
    /// Org heading tags (Readwise `Tags` column), already sanitized.
    pub tags: Vec<String>,
}

/// Readwise tags (`.fav, to read`) as org tags: split on comma/whitespace, leading `.` dropped,
/// anything but `[A-Za-z0-9_@#%]` removed, empties and repeats skipped.
fn org_tags(s: &str) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for t in s.split(|c: char| c == ',' || c.is_whitespace()) {
        let t: String = t.trim_start_matches('.').chars().filter(|c| c.is_ascii_alphanumeric() || "_@#%".contains(*c)).collect();
        if !t.is_empty() && !out.contains(&t) {
            out.push(t);
        }
    }
    out
}

fn org_ts(t: NaiveDateTime) -> String {
    t.format("[%Y-%m-%d %a %H:%M]").to_string()
}

/// Kindle `Added on Monday, January 1, 2024 10:00:00 AM` (US) or `…, 1 January 2024 10:00:00` (UK).
fn kindle_date(s: &str) -> String {
    let s = s.trim();
    ["%A, %B %d, %Y %I:%M:%S %p", "%A, %d %B %Y %H:%M:%S", "%A, %B %d, %Y %H:%M:%S"]
        .iter()
        .find_map(|f| NaiveDateTime::parse_from_str(s, f).ok())
        .map_or_else(|| s.to_string(), org_ts)
}

/// Readwise `2023-05-01 12:34:56+00:00` / `2023-05-01T12:34:56Z` (date and minute kept, zone ignored).
fn readwise_date(s: &str) -> String {
    let s = s.trim();
    let t = s.get(..16).map(|x| x.replace('T', " "));
    t.and_then(|t| NaiveDateTime::parse_from_str(&t, "%Y-%m-%d %H:%M").ok())
        .or_else(|| s.get(..10).and_then(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()).map(|d| d.and_hms_opt(0, 0, 0).unwrap()))
        .map_or_else(|| s.to_string(), org_ts)
}

/// `Title (Author)` split at the last parenthesized group.
fn title_author(line: &str) -> (String, String) {
    let l = line.trim_start_matches('\u{feff}').trim();
    if let (true, Some(i)) = (l.ends_with(')'), l.rfind(" (")) {
        return (l[..i].trim().into(), l[i + 2..l.len() - 1].trim().into());
    }
    (l.into(), String::new())
}

/// The `a-b` (or `a`) Kindle location range in a LOCATION string.
fn loc_range(s: &str) -> Option<(u32, u32)> {
    let i = s.find("location ")?;
    let r = s[i + 9..].split(',').next()?.trim();
    let (a, b) = r.split_once('-').unwrap_or((r, r));
    Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
}

/// The page in a LOCATION string (`page 5, location 1-2` gives `5`).
fn page_of(s: &str) -> Option<&str> {
    s.strip_prefix("page ")?.split(',').next().map(str::trim)
}

/// Kindle metadata-line phrases (lowercase) by locale, rewritten to the English form the parser
/// reads (de, fr, es, it, pt, nl, sv; an unknown language is left as is). Longest phrases first.
const LOCALIZED: &[(&str, &str)] = &[
    ("ihre notiz", "Your Note"), ("votre note", "Your Note"), ("tu nota", "Your Note"), ("la tua nota", "Your Note"), ("sua nota", "Your Note"), ("uw notitie", "Your Note"), ("din anteckning", "Your Note"),
    ("ihr lesezeichen", "Your Bookmark"), ("votre signet", "Your Bookmark"), ("tu marcador", "Your Bookmark"), ("il tuo segnalibro", "Your Bookmark"), ("seu marcador", "Your Bookmark"), ("uw bladwijzer", "Your Bookmark"), ("ditt bokmärke", "Your Bookmark"),
    ("hinzugefügt am ", "Added on "), ("ajouté le ", "Added on "), ("añadido el ", "Added on "), ("aggiunto il ", "Added on "), ("adicionado: ", "Added on "), ("toegevoegd op ", "Added on "), ("tillagd den ", "Added on "),
    ("bei position ", "location "), ("à l'emplacement ", "location "), ("en la posición ", "location "), ("alla posizione ", "location "), ("na posição ", "location "), ("op locatie ", "location "), ("på plats ", "location "),
    ("auf seite ", "page "), ("sur la page ", "page "), ("en la página ", "page "), ("alla pagina ", "page "), ("na página ", "page "), ("op pagina ", "page "), ("på sidan ", "page "),
];

/// The metadata line META with the `LOCALIZED` phrases replaced (case-insensitively, char by char so
/// non-ASCII text never shifts byte offsets).
fn english_meta(meta: &str) -> String {
    let (mut out, cs) = (String::new(), meta.chars().collect::<Vec<_>>());
    let low: Vec<char> = cs.iter().map(|c| c.to_lowercase().next().unwrap_or(*c)).collect();
    let mut i = 0;
    'next: while i < cs.len() {
        for (from, to) in LOCALIZED {
            let f: Vec<char> = from.chars().collect();
            if low[i..].starts_with(&f) {
                out += to;
                i += f.len();
                continue 'next;
            }
        }
        out.push(cs[i]);
        i += 1;
    }
    out
}

/// The entries of a Kindle `My Clippings.txt`: bookmarks and empty highlights skipped, a note
/// attached to the book's highlight at the same location (else kept on its own).
pub fn parse_clippings(text: &str) -> Vec<Highlight> {
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut out: Vec<Highlight> = vec![];
    for block in text.split("==========") {
        let mut lines = block.trim_matches(['\n', '\u{feff}', ' ']).lines();
        let (Some(head), Some(meta)) = (lines.next(), lines.next()) else { continue };
        let body = lines.collect::<Vec<_>>().join("\n").trim().to_string();
        let (book, author) = title_author(head);
        let meta = &english_meta(meta);
        let lower = meta.to_lowercase();
        if lower.contains("bookmark") || body.is_empty() {
            continue;
        }
        let (mut page, mut loc, mut added) = (None, None, String::new());
        for part in meta.trim_start_matches(['-', ' ']).split('|') {
            let p = part.trim();
            let pl = p.to_lowercase();
            if let Some(a) = p.strip_prefix("Added on ") {
                added = kindle_date(a);
            } else if let Some(i) = pl.find("page ") {
                page = Some(p[i + 5..].split_whitespace().next().unwrap_or("").to_string());
            }
            if let Some(i) = pl.find("location ").or_else(|| pl.find("loc. ")) {
                let v = p[i..].split_once(' ').map_or("", |x| x.1).split_whitespace().next().unwrap_or("");
                loc = Some(v.to_string());
            }
        }
        let location = [page.map(|p| format!("page {p}")), loc.map(|l| format!("location {l}"))].into_iter().flatten().collect::<Vec<_>>().join(", ");
        if lower.contains("your note") {
            let at = loc_range(&location).map(|r| r.1);
            let pg = page_of(&location).filter(|_| at.is_none()); // page-only note (PDFs): match on the page
            let host = out.iter_mut().rev().find(|h| {
                h.book == book && h.author == author && h.note.is_empty() && !h.text.is_empty() && match (at, pg) {
                    (Some(n), _) => loc_range(&h.location).is_some_and(|(a, b)| (a..=b).contains(&n)),
                    (None, Some(p)) => page_of(&h.location) == Some(p),
                    _ => false,
                }
            });
            if let Some(h) = host {
                h.note = body;
                continue;
            }
            if out.iter().any(|h| h.book == book && h.author == author && h.location == location && norm(&h.note) == norm(&body)) {
                continue; // the same note again (a repeated clipping)
            }
            out.push(Highlight { book, author, location, added, note: body, ..Default::default() });
            continue;
        }
        // a re-highlight that widened/shrank an earlier one (overlapping range, one text inside the other) replaces it in place
        let nb = norm(&body);
        let widened = loc_range(&location).and_then(|(a, b)| {
            out.iter().position(|h| {
                let t = norm(&h.text);
                h.book == book && h.author == author && !t.is_empty() && !nb.is_empty() && (t.contains(&nb) || nb.contains(&t)) && loc_range(&h.location).is_some_and(|(c, d)| a <= d && c <= b)
            })
        });
        let new = Highlight { book, author, text: body, location, added, ..Default::default() };
        match widened {
            Some(i) => out[i] = Highlight { note: std::mem::take(&mut out[i].note), ..new },
            None => out.push(new),
        }
    }
    out
}

/// A Readwise export CSV (columns found by header name: Highlight, Book Title, Book Author, Note,
/// Location Type, Location, Highlighted at). Errors when there is no Highlight column.
pub fn parse_readwise_csv(text: &str) -> Result<Vec<Highlight>, String> {
    let rows = crate::ledger::csv_rows(text, ',');
    let Some((head, rows)) = rows.split_first() else { return Ok(vec![]) };
    let col = |names: &[&str]| head.iter().position(|h| names.iter().any(|n| h.trim().eq_ignore_ascii_case(n)));
    let hl = col(&["Highlight", "Text"]).ok_or("not a Readwise CSV: no Highlight column")?;
    let (title, author, note, ltype, loc, date) =
        (col(&["Book Title", "Title"]), col(&["Book Author", "Author"]), col(&["Note"]), col(&["Location Type"]), col(&["Location"]), col(&["Highlighted at", "Date"]));
    let tags = col(&["Tags"]);
    let get = |r: &Vec<String>, c: Option<usize>| c.and_then(|c| r.get(c)).map_or(String::new(), |s| s.trim().to_string());
    Ok(rows
        .iter()
        .filter(|r| !get(r, Some(hl)).is_empty())
        .map(|r| {
            let (t, l) = (get(r, ltype), get(r, loc));
            let location = if l.is_empty() || t.is_empty() || t.eq_ignore_ascii_case("order") { l } else { format!("{} {l}", t.to_lowercase()) };
            let added = get(r, date);
            Highlight {
                book: get(r, title),
                author: get(r, author),
                text: get(r, Some(hl)),
                location,
                added: if added.is_empty() { added } else { readwise_date(&added) },
                note: get(r, note),
                tags: org_tags(&get(r, tags)),
            }
        })
        .collect())
}

fn norm(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Key of a note added later to the highlight keyed K.
fn note_key(k: &str, note: &str) -> String {
    format!("{:x}", Sha256::digest(format!("{k}\nnote\n{}", norm(note))))[..16].to_string()
}

/// A note added to an already-imported highlight (key K, note key NK) as its own entry.
fn render_note(h: &Highlight, nk: &str) -> String {
    let mut o = format!("* Note: {}\n:PROPERTIES:\n:HIGHLIGHT_KEY: {nk}\n", heading(&Highlight { note: h.note.clone(), ..Default::default() }));
    if !h.added.is_empty() {
        o += &format!(":ADDED: {}\n", h.added);
    }
    o += ":END:\n";
    for l in h.note.lines() {
        o += &format!("{}{l}\n", if l.starts_with('*') { " " } else { "" });
    }
    o
}

/// The dedupe key: book, author, location and whitespace-normalized text, hashed.
pub fn key(h: &Highlight) -> String {
    let d = Sha256::digest(format!("{}\n{}\n{}\n{}\n{}", norm(&h.book), norm(&h.author), h.location, norm(&h.text), norm(if h.text.is_empty() { &h.note } else { "" })));
    format!("{d:x}")[..16].to_string()
}

/// The heading line: the first words of the quote (or note), at most ~60 characters.
fn heading(h: &Highlight) -> String {
    let src = if h.text.is_empty() { &h.note } else { &h.text };
    let words: Vec<&str> = src.split_whitespace().collect();
    let mut s = String::new();
    for w in &words {
        if s.chars().count() + w.chars().count() > 60 && !s.is_empty() {
            return format!("{}…", s.trim_end_matches(':'));
        }
        if !s.is_empty() {
            s.push(' ');
        }
        s += w;
    }
    s.trim_end_matches(':').to_string() // a trailing `:x:` would read as tags
}

/// One highlight as an org heading with its properties, quote and note.
pub fn render(h: &Highlight) -> String {
    let tags = if h.tags.is_empty() { String::new() } else { format!(" :{}:", h.tags.join(":")) };
    let mut o = format!("* {}{tags}\n:PROPERTIES:\n:HIGHLIGHT_KEY: {}\n", heading(h), key(h));
    if !h.location.is_empty() {
        o += &format!(":LOCATION: {}\n", h.location);
    }
    if !h.added.is_empty() {
        o += &format!(":ADDED: {}\n", h.added);
    }
    if !h.text.is_empty() && !h.note.is_empty() {
        o += &format!(":NOTE_KEY: {}\n", note_key(&key(h), &h.note)); // so a re-import knows the inline note
    }
    o += ":END:\n";
    if !h.text.is_empty() {
        o += "#+begin_quote\n";
        for l in h.text.lines() {
            let esc = l.starts_with('*') || l.starts_with(",*") || l.trim_start().starts_with("#+") || l.trim_start().starts_with(",#+");
            o += &format!("{}{l}\n", if esc { "," } else { "" });
        }
        o += "#+end_quote\n";
    }
    for l in h.note.lines() {
        o += &format!("{}{l}\n", if l.starts_with('*') { " " } else { "" });
    }
    o
}

/// Normalized body of each entry by its `:HIGHLIGHT_KEY:` (up to the next top-level heading); the first entry wins on a repeated key.
fn entry_norms(text: &str) -> std::collections::HashMap<String, String> {
    let mut m = std::collections::HashMap::new();
    let mut cur: Option<(String, Vec<&str>)> = None;
    let mut flush = |c: Option<(String, Vec<&str>)>| {
        if let Some((k, b)) = c {
            m.entry(k).or_insert_with(|| norm(&b.join("\n")));
        }
    };
    for l in text.lines() {
        if l.starts_with("* ") {
            flush(cur.take());
        } else if let Some((_, b)) = cur.as_mut() {
            b.push(l);
        } else if let Some(k) = l.trim().strip_prefix(":HIGHLIGHT_KEY:") {
            cur = Some((k.trim().to_string(), vec![]));
        }
    }
    flush(cur);
    m
}

/// The value of the `#+KEY:` line of an org TEXT (any case), trimmed.
fn keyword<'a>(text: &'a str, k: &str) -> Option<&'a str> {
    text.lines().find_map(|l| {
        let l = l.trim_start();
        let rest = l.get(..k.len() + 3).filter(|p| p.eq_ignore_ascii_case(&format!("#+{k}:")))?;
        Some(l[rest.len()..].trim())
    })
}

#[derive(Serialize, Default, Debug)]
pub struct Book {
    pub title: String,
    pub author: String,
    /// Relative to the notes folder.
    pub file: String,
    pub new: usize,
    pub dupes: usize,
    pub created: bool,
}

#[derive(Serialize, Default, Debug)]
pub struct Summary {
    pub books: Vec<Book>,
    pub new: usize,
    pub dupes: usize,
}

/// Import the clippings/CSV file SRC into NOTES/Reading/<slug>.org (`-2`… for the same title by
/// another author). DRY only reports; DRILL also tags the newly added highlights `drill`. The caller holds the note-write lock.
pub fn import(src: &Path, notes: &Path, dry: bool, drill: bool) -> Result<Summary, String> {
    if !src.is_absolute() || !src.is_file() {
        return Err(format!("{} must be an absolute path to a file", src.display()));
    }
    let text = std::fs::read_to_string(src).map_err(|e| format!("{}: {e}", src.display()))?;
    let csv = src.extension().is_some_and(|e| e.eq_ignore_ascii_case("csv"));
    let hs = if csv { parse_readwise_csv(&text)? } else { parse_clippings(&text) };
    let mut books: Vec<((String, String), Vec<Highlight>)> = vec![];
    let mut idx: std::collections::HashMap<(String, String), usize> = Default::default();
    for h in hs {
        let k = (if h.book.is_empty() { "Untitled".into() } else { h.book.clone() }, h.author.clone());
        match idx.get(&k) {
            Some(&i) => books[i].1.push(h),
            None => {
                idx.insert(k.clone(), books.len());
                books.push((k, vec![h]));
            }
        }
    }
    let dir = notes.join("Reading");
    let mut taken: Vec<PathBuf> = vec![];
    let mut s = Summary::default();
    for ((title, author), hs) in books {
        let slug = Some(crate::notes::slug(&title)).filter(|s| !s.is_empty()).unwrap_or_else(|| "untitled".into());
        let (path, old) = (1..)
            .map(|n| dir.join(if n == 1 { format!("{slug}.org") } else { format!("{slug}-{n}.org") }))
            .filter(|p| !taken.contains(p))
            .find_map(|p| match std::fs::read_to_string(&p) {
                Ok(t) if keyword(&t, "title") == Some(title.as_str()) && keyword(&t, "author").unwrap_or("") == author => Some((p, Some(t))),
                Ok(_) => None,
                Err(_) if p.exists() => None,
                Err(_) => Some((p, None)),
            })
            .unwrap();
        taken.push(path.clone());
        let mut seen: std::collections::HashSet<String> = old.as_deref().map_or(Default::default(), |t| t.lines().filter_map(|l| l.trim().strip_prefix(":HIGHLIGHT_KEY:").or_else(|| l.trim().strip_prefix(":NOTE_KEY:"))).map(|k| k.trim().to_string()).collect());
        let mut norms = None;
        let mut add = String::new();
        let mut b = Book { title: title.clone(), author: author.clone(), file: path.strip_prefix(notes).unwrap().to_string_lossy().into_owned(), created: old.is_none(), ..Default::default() };
        for h in &hs {
            // imported before localized metadata was read: no location in the old key
            let legacy_k = key(&Highlight { location: String::new(), ..h.clone() });
            let is_legacy = !h.location.is_empty() && seen.contains(&legacy_k);
            let k = if is_legacy { legacy_k } else { key(h) };
            if !seen.insert(k.clone()) {
                // a note added to a known highlight later becomes its own `* Note:` entry, once
                let nk = note_key(&k, &h.note);
                // an earlier import kept a page-only note as its own entry (empty text, same location)
                let legacy = key(&Highlight { book: h.book.clone(), author: h.author.clone(), location: h.location.clone(), note: h.note.clone(), ..Default::default() });
                let fresh = !h.note.is_empty() && !h.text.is_empty() && !seen.contains(&nk) && !seen.contains(&legacy) && !(is_legacy && seen.contains(&key(&Highlight { book: h.book.clone(), author: h.author.clone(), text: h.note.clone(), ..Default::default() }))) && !norms.get_or_insert_with(|| entry_norms(old.as_deref().unwrap_or_default())).get(&k).map_or("", String::as_str).contains(&norm(&h.note));
                if !fresh {
                    seen.insert(nk);
                    b.dupes += 1;
                    continue;
                }
                seen.insert(nk.clone());
                add += &format!("\n{}", render_note(h, &nk));
                b.new += 1;
                continue;
            }
            add += "\n";
            let mut h = h.clone();
            if drill && !h.tags.iter().any(|t| t == "drill") {
                h.tags.push("drill".into());
            }
            add += &render(&h);
            seen.insert(note_key(&k, &h.note));
            b.new += 1;
        }
        if !dry && b.new > 0 {
            let full = match &old {
                Some(t) => format!("{}{}{add}", t, if t.is_empty() || t.ends_with('\n') { "" } else { "\n" }),
                None => format!("#+title: {title}\n{}{add}", if author.is_empty() { String::new() } else { format!("#+author: {author}\n") }),
            };
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            crate::config::write_atomic(&path, full).map_err(|e| format!("{}: {e}", path.display()))?;
        }
        s.new += b.new;
        s.dupes += b.dupes;
        s.books.push(b);
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLIP: &str = "\u{feff}Dune (Frank Herbert)\r\n- Your Highlight on page 12 | Location 180-182 | Added on Monday, January 1, 2024 10:00:00 AM\r\n\r\nFear is the mind-killer.\r\n* Not a heading\r\n==========\r\n\u{feff}Dune (Frank Herbert)\r\n- Your Note on page 12 | Location 182 | Added on Monday, January 1, 2024 10:01:00 AM\r\n\r\nGreat line\r\n==========\r\nDune (Frank Herbert)\r\n- Your Bookmark on page 20 | Location 300 | Added on Monday, January 1, 2024 10:02:00 AM\r\n\r\n\r\n==========\r\nDune (Frank Herbert)\r\n- Your Highlight on page 12 | Location 180-182 | Added on Monday, January 1, 2024 10:00:00 AM\r\n\r\nFear is the mind-killer.\r\n* Not a heading\r\n==========\r\nThe Name of the Book (Smith, J.)\r\n- Your Highlight at location 5-6 | Added on Tuesday, 2 January 2024 08:30:00\r\n\r\nFirst line\r\nsecond line\r\n==========\r\n";

    #[test]
    fn large_import_dedups_and_reimport_adds_nothing() {
        let dir = std::env::temp_dir().join(format!("margin-hl-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut csv = String::from("Highlight,Book Title,Book Author,Amazon Book ID,Note,Color,Tags,Location Type,Location,Highlighted at,Document tags\n");
        for i in 0..3000 {
            csv += &format!("text {i},Book {},A,,,,,location,{i},,\n", i % 3);
        }
        csv += "text 0,Book 0,A,,,,,location,0,,\n"; // duplicate
        let src = dir.join("in.csv");
        std::fs::write(&src, csv).unwrap();
        let s = import(&src, &dir, false, false).unwrap();
        assert_eq!((s.new, s.dupes), (3000, 1));
        assert_eq!(s.books.iter().map(|b| b.title.as_str()).collect::<Vec<_>>(), ["Book 0", "Book 1", "Book 2"]);
        let s = import(&src, &dir, false, false).unwrap();
        assert_eq!((s.new, s.dupes), (0, 3001));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn note_dedupe_is_per_entry() {
        let dir = std::env::temp_dir().join(format!("margin-hl-nk-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let clip = |a: &str, b: &str, note: &str| format!("B (A)\n- Your Highlight at location 1-2 | Added on Monday, January 1, 2024 10:00:00 AM\n\n{a}\n==========\nB (A)\n- Your Highlight at location 3-4 | Added on Monday, January 1, 2024 10:00:00 AM\n\n{b}\n==========\n{note}");
        let src = dir.join("c.txt");
        std::fs::write(&src, clip("an important point", "other", "")).unwrap();
        assert_eq!(import(&src, &dir, false, false).unwrap().new, 2);
        let note = "B (A)\n- Your Note at location 3-4 | Added on Monday, January 1, 2024 10:01:00 AM\n\nimportant\n==========\n";
        std::fs::write(&src, clip("an important point", "other", note)).unwrap();
        assert_eq!(import(&src, &dir, false, false).unwrap().new, 1);
        assert_eq!(import(&src, &dir, false, false).unwrap().new, 0);
        // inline note: written with a NOTE_KEY, not re-added; same clipping twice adds one entry
        let inl = "B (A)\n- Your Highlight at location 9 | Added on Monday, January 1, 2024 10:00:00 AM\n\nquote\n==========\nB (A)\n- Your Note at location 9 | Added on Monday, January 1, 2024 10:01:00 AM\n\nmine\n==========\n";
        std::fs::write(&src, inl.repeat(2)).unwrap();
        assert_eq!(import(&src, &dir, false, false).unwrap().new, 1);
        let f = dir.join("Reading/b.org");
        assert_eq!(std::fs::read_to_string(&f).unwrap().matches(":NOTE_KEY:").count(), 1);
        assert_eq!(import(&src, &dir, false, false).unwrap().new, 0);
        // legacy file (inline note, no NOTE_KEY) is not duplicated
        let t = std::fs::read_to_string(&f).unwrap().lines().filter(|l| !l.starts_with(":NOTE_KEY:")).collect::<Vec<_>>().join("\n") + "\n";
        std::fs::write(&f, t).unwrap();
        assert_eq!(import(&src, &dir, false, false).unwrap().new, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn entry_norms_per_key_first_wins() {
        let m = entry_norms("#+title: t\n* a\n:PROPERTIES:\n:HIGHLIGHT_KEY: k1\n:END:\nFoo  Bar\n* b\n:HIGHLIGHT_KEY: k2\nx\n* c\n:HIGHLIGHT_KEY: k1\nother\n");
        assert_eq!(m.len(), 2);
        assert_eq!(m["k1"], norm(":END:\nFoo  Bar"));
        assert_eq!(m["k2"], norm("x"));
    }

    #[test]
    fn parses_clippings() {
        let hs = parse_clippings(CLIP);
        assert_eq!(hs.len(), 2); // the bookmark is skipped, the exact duplicate collapses
        assert_eq!(hs[0], Highlight { book: "Dune".into(), author: "Frank Herbert".into(), text: "Fear is the mind-killer.\n* Not a heading".into(), location: "page 12, location 180-182".into(), added: "[2024-01-01 Mon 10:00]".into(), note: "Great line".into(), tags: vec![] });
        assert_eq!(hs[1].book, "The Name of the Book");
        assert_eq!(hs[1].author, "Smith, J.");
        assert_eq!(hs[1].text, "First line\nsecond line");
        assert_eq!(hs[1].location, "location 5-6");
        assert_eq!(hs[1].added, "[2024-01-02 Tue 08:30]");
        assert!(parse_clippings("").is_empty());
        assert_eq!(kindle_date("Monday, January 6, 2025 9:12:00 PM"), "[2025-01-06 Mon 21:12]");
        assert_eq!(kindle_date("lundi 6 janvier 2025"), "lundi 6 janvier 2025"); // unreadable: kept as written
    }

    #[test]
    fn page_only_note_attaches_to_same_page_highlight() {
        let c = |loc: &str| format!("B (A)\n- Your Highlight on page 5 | Added on Monday, January 1, 2024 10:00:00 AM\n\nquote\n==========\nB (A)\n- Your Note on page {loc} | Added on Monday, January 1, 2024 10:01:00 AM\n\nmine\n==========\n");
        let hs = parse_clippings(&c("5"));
        assert_eq!((hs.len(), hs[0].note.as_str()), (1, "mine"));
        let hs = parse_clippings(&c("6"));
        assert_eq!((hs.len(), hs[0].note.as_str(), hs[1].note.as_str()), (2, "", "mine"));
        // re-import over a file that holds the note as a standalone entry adds nothing
        let root = std::env::temp_dir().join(format!("margin-hl-pg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("Reading")).unwrap();
        let src = root.join("c.txt");
        std::fs::write(&src, c("5")).unwrap();
        let (host, note) = (Highlight { book: "B".into(), author: "A".into(), text: "quote".into(), location: "page 5".into(), ..Default::default() }, Highlight { book: "B".into(), author: "A".into(), location: "page 5".into(), note: "mine".into(), ..Default::default() });
        std::fs::write(root.join("Reading/b.org"), format!("#+title: B\n#+author: A\n\n{}\n{}", render(&host), render(&note))).unwrap();
        let s = import(&src, &root, false, false).unwrap();
        assert_eq!((s.new, s.dupes), (0, 1));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn widened_highlight_replaces_earlier_one() {
        let c = |loc: &str, t: &str| format!("B (A)\n- Your Highlight at location {loc} | Added on Monday, January 1, 2024 10:00:00 AM\n\n{t}\n==========\n");
        let note = "B (A)\n- Your Note at location 10 | Added on Monday, January 1, 2024 10:00:00 AM\n\nmine\n==========\n";
        // widened: later wins, order kept, note carried over
        let hs = parse_clippings(&format!("{}{note}{}{}", c("10-11", "one two"), c("10-12", "one two three"), c("50", "other")));
        assert_eq!(hs.iter().map(|h| (h.location.as_str(), h.text.as_str(), h.note.as_str())).collect::<Vec<_>>(), [("location 10-12", "one two three", "mine"), ("location 50", "other", "")]);
        // shrunk
        let hs = parse_clippings(&format!("{}{}", c("10-12", "one two three"), c("10-11", "one two")));
        assert_eq!((hs.len(), hs[0].text.as_str()), (1, "one two"));
        // same text at distant locations stays separate; overlapping but unrelated text too
        assert_eq!(parse_clippings(&format!("{}{}", c("10-11", "same"), c("90-91", "same"))).len(), 2);
        assert_eq!(parse_clippings(&format!("{}{}", c("10-11", "alpha"), c("11-12", "beta"))).len(), 2);
        // a standalone note is never replaced
        assert_eq!(parse_clippings(&format!("{}{}", "B (A)\n- Your Note at location 10 | Added on Monday, January 1, 2024 10:00:00 AM\n\nn\n==========\n", c("10-11", "x"))).len(), 2);
    }

    #[test]
    fn parses_localized_clippings() {
        let cases = [
            ("Ihre Markierung auf Seite 12 | bei Position 180-182 | Hinzugefügt am Montag, 1. Januar 2024 10:00:00", "Ihre Notiz auf Seite 12 | bei Position 182 | Hinzugefügt am Montag"),
            ("Votre surlignement sur la page 12 | à l'emplacement 180-182 | Ajouté le lundi 1 janvier 2024", "Votre note sur la page 12 | à l'emplacement 182 | Ajouté le lundi"),
            ("Tu subrayado en la página 12 | en la posición 180-182 | Añadido el lunes, 1 de enero de 2024", "Tu nota en la página 12 | en la posición 182 | Añadido el lunes"),
            ("La tua evidenziazione alla pagina 12 | alla posizione 180-182 | Aggiunto il lunedì 1 gennaio 2024", "La tua nota alla pagina 12 | alla posizione 182 | Aggiunto il lunedì"),
            ("Seu destaque na página 12 | na posição 180-182 | Adicionado: segunda-feira", "Sua nota na página 12 | na posição 182 | Adicionado: segunda-feira"),
            ("Uw markering op pagina 12 | op locatie 180-182 | Toegevoegd op maandag 1 januari 2024", "Uw notitie op pagina 12 | op locatie 182 | Toegevoegd op maandag"),
            ("Din markering på sidan 12 | på plats 180-182 | Tillagd den måndag 1 januari 2024", "Din anteckning på sidan 12 | på plats 182 | Tillagd den måndag"),
        ];
        for (hl, note) in cases {
            let hs = parse_clippings(&format!("Bok (Au)\n- {hl}\n\nquote\n==========\nBok (Au)\n- {note}\n\nmine\n==========\nBok (Au)\n- Mi Marcador | x\n\n\n==========\n"));
            assert_eq!(hs.len(), 1, "{hl}");
            assert_eq!((hs[0].location.as_str(), hs[0].text.as_str(), hs[0].note.as_str()), ("page 12, location 180-182", "quote", "mine"), "{hl}");
            assert!(hs[0].added.starts_with(char::is_alphabetic) || hs[0].added.starts_with('['), "{hl}");
        }
        let hs = parse_clippings("B (A)\n- Ihr Lesezeichen bei Position 5\n\ntext\n==========\n");
        assert!(hs.is_empty());
    }

    #[test]
    fn reimport_skips_legacy_locationless_keys() {
        let dir = std::env::temp_dir().join(format!("margin-hl-lg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let old = Highlight { book: "Bok".into(), author: "Au".into(), text: "quote".into(), ..Default::default() };
        std::fs::create_dir_all(dir.join("Reading")).unwrap();
        std::fs::write(dir.join("Reading/bok.org"), format!("#+title: Bok\n#+author: Au\n\n{}", render(&old))).unwrap();
        let src = dir.join("c.txt");
        std::fs::write(&src, "Bok (Au)\n- Ihre Markierung bei Position 5-6 | Hinzugefügt am x\n\nquote\n==========\n").unwrap();
        let s = import(&src, &dir, false, false).unwrap();
        assert_eq!((s.new, s.dupes), (0, 1));
        // a later note on the legacy-keyed highlight is imported once
        std::fs::write(&src, "Bok (Au)\n- Ihre Markierung bei Position 5-6 | Hinzugefügt am x\n\nquote\n==========\nBok (Au)\n- Ihre Notiz bei Position 6 | Hinzugefügt am x\n\nnew thought\n==========\n").unwrap();
        let s = import(&src, &dir, false, false).unwrap();
        assert_eq!(s.new, 1);
        assert_eq!(std::fs::read_to_string(dir.join("Reading/bok.org")).unwrap().matches("* Note: new thought").count(), 1);
        assert_eq!(import(&src, &dir, false, false).unwrap().new, 0);
        // a note already imported pre-fix as its own quote entry is not written again
        let nq = Highlight { book: "Bok".into(), author: "Au".into(), text: "old thought".into(), ..Default::default() };
        let f = dir.join("Reading/bok.org");
        std::fs::write(&f, format!("#+title: Bok\n#+author: Au\n\n{}\n{}", render(&old), render(&nq))).unwrap();
        std::fs::write(&src, "Bok (Au)\n- Ihre Markierung bei Position 5-6 | Hinzugefügt am x\n\nquote\n==========\nBok (Au)\n- Ihre Notiz bei Position 6 | Hinzugefügt am x\n\nold thought\n==========\n").unwrap();
        assert_eq!(import(&src, &dir, false, false).unwrap().new, 0);
        assert_eq!(std::fs::read_to_string(&f).unwrap().matches("old thought").count(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parses_readwise_csv() {
        let csv = "\u{feff}Highlight,Book Title,Book Author,Amazon Book ID,Note,Color,Tags,Location Type,Location,Highlighted at,Document tags\r\n\"Hello, \"\"world\"\"\nline two\",Book A,Ann,X,my note,yellow,,location,42,2023-05-01 12:34:56+00:00,\r\n,Empty,Ann,,,,,,,,\r\nPlain,Book A,Ann,,,,,order,7,,\r\n";
        let hs = parse_readwise_csv(csv).unwrap();
        assert_eq!(hs.len(), 2);
        assert_eq!(hs[0].text, "Hello, \"world\"\nline two");
        assert_eq!((hs[0].book.as_str(), hs[0].author.as_str(), hs[0].note.as_str()), ("Book A", "Ann", "my note"));
        assert_eq!(hs[0].location, "location 42");
        assert_eq!(hs[0].added, "[2023-05-01 Mon 12:34]");
        assert_eq!((hs[1].location.as_str(), hs[1].added.as_str()), ("7", ""));
        assert!(parse_readwise_csv("a,b\n1,2\n").is_err());
        assert!(parse_readwise_csv("").unwrap().is_empty());
    }

    #[test]
    fn tags_and_drill() {
        assert_eq!(org_tags(".fav, to read,fav, ,a-b!,#x"), ["fav", "to", "read", "ab", "#x"]);
        let csv = "Highlight,Book Title,Book Author,Note,Tags,Location,Document tags\nOne: two:,B,A,,\".fav, deep\",1,ignored\nTwo,B,A,,,2,\n";
        let hs = parse_readwise_csv(csv).unwrap();
        assert_eq!((hs[0].tags.clone(), hs[1].tags.len()), (vec!["fav".to_string(), "deep".into()], 0));
        assert!(render(&hs[0]).starts_with("* One: two :fav:deep:\n"));
        let root = std::env::temp_dir().join(format!("margin-hl-drill-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let src = root.join("in.csv");
        std::fs::write(&src, csv).unwrap();
        assert_eq!(import(&src, &root, false, true).unwrap().new, 2);
        let f = root.join("Reading/b.org");
        let text = std::fs::read_to_string(&f).unwrap();
        let kw = crate::org::Kw { todo: vec![], done: vec![] };
        let cards = crate::org::drill_cards(&[std::sync::Arc::new(crate::org::parse(&f, &text, &kw))]);
        assert_eq!(cards.iter().map(|c| c.title.as_str()).collect::<Vec<_>>(), ["One: two", "Two"]);
        assert!(text.contains(":fav:deep:drill:"));
        let again = import(&src, &root, false, true).unwrap();
        assert_eq!((again.new, again.dupes), (0, 2));
        assert_eq!(std::fs::read_to_string(&f).unwrap(), text);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn later_note_appends_once() {
        let root = std::env::temp_dir().join(format!("margin-hl-note-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let src = root.join("in.csv");
        let csv = |n: &str| format!("Highlight,Book Title,Book Author,Note,Tags,Location\nQuote one,B,A,,.x,1\nQuote two,B,A,{n},,2\n");
        std::fs::write(&src, csv("")).unwrap();
        assert_eq!(import(&src, &root, false, true).unwrap().new, 2);
        std::fs::write(&src, csv("a new thought")).unwrap();
        let f = root.join("Reading/b.org");
        let before = std::fs::read_to_string(&f).unwrap();
        assert_eq!(import(&src, &root, true, true).unwrap().new, 1); // dry run counts it, writes nothing
        assert_eq!(std::fs::read_to_string(&f).unwrap(), before);
        let s = import(&src, &root, false, true).unwrap();
        assert_eq!((s.new, s.dupes), (1, 1));
        let t = std::fs::read_to_string(&f).unwrap();
        assert!(t.starts_with(&before));
        assert!(t.contains("* Note: a new thought\n:PROPERTIES:\n:HIGHLIGHT_KEY:") && t.contains(":END:\na new thought\n"));
        assert!(!t.contains("* Note: a new thought :drill:"));
        let s = import(&src, &root, false, true).unwrap();
        assert_eq!((s.new, s.dupes), (0, 2));
        assert_eq!(std::fs::read_to_string(&f).unwrap(), t);
        // a note that came inline on the first import is not appended again
        let r2 = root.join("two");
        std::fs::create_dir_all(&r2).unwrap();
        let src2 = r2.join("in.csv");
        std::fs::write(&src2, csv("inline note")).unwrap();
        assert_eq!(import(&src2, &r2, false, false).unwrap().new, 2);
        let s = import(&src2, &r2, false, false).unwrap();
        assert_eq!((s.new, s.dupes), (0, 2));
        assert!(!std::fs::read_to_string(r2.join("Reading/b.org")).unwrap().contains("* Note:"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn renders_escaped_quote() {
        let h = &parse_clippings(CLIP)[0];
        assert_eq!(render(h), format!("* Fear is the mind-killer. * Not a heading\n:PROPERTIES:\n:HIGHLIGHT_KEY: {}\n:LOCATION: page 12, location 180-182\n:ADDED: [2024-01-01 Mon 10:00]\n:NOTE_KEY: d3f11f1f2f4a726a\n:END:\n#+begin_quote\nFear is the mind-killer.\n,* Not a heading\n#+end_quote\nGreat line\n", key(h)));
        let long = Highlight { text: "word ".repeat(30), ..Default::default() };
        assert!(heading(&long).ends_with('…') && heading(&long).chars().count() <= 62);
    }

    #[test]
    fn imports_appends_only_new_and_splits_authors() {
        let root = std::env::temp_dir().join(format!("margin-highlights-{}", std::process::id()));
        let notes = root.join("notes");
        std::fs::create_dir_all(&notes).unwrap();
        let src = root.join("My Clippings.txt");
        std::fs::write(&src, CLIP).unwrap();
        let dry = import(&src, &notes, true, false).unwrap();
        assert_eq!((dry.new, dry.dupes, dry.books.len()), (2, 0, 2));
        assert!(!notes.join("Reading").exists()); // dry writes nothing
        let s = import(&src, &notes, false, false).unwrap();
        assert_eq!((s.new, s.dupes), (2, 0));
        let dune = notes.join("Reading/dune.org");
        let text = std::fs::read_to_string(&dune).unwrap();
        assert!(text.starts_with("#+title: Dune\n#+author: Frank Herbert\n\n* Fear is the mind-killer."));
        assert_eq!(text.matches(":HIGHLIGHT_KEY:").count(), 1);
        // the user edits the note; a re-import adds nothing and keeps the edit
        std::fs::write(&dune, format!("{text}my own line")).unwrap();
        let again = import(&src, &notes, false, false).unwrap();
        assert_eq!((again.new, again.dupes), (0, 2));
        assert_eq!(std::fs::read_to_string(&dune).unwrap(), format!("{text}my own line"));
        // a new highlight is appended after the edit
        let more = format!("{CLIP}Dune (Frank Herbert)\n- Your Highlight on page 99 | Location 900-901 | Added on Monday, January 1, 2024 11:00:00 AM\n\nNew one\n==========\nDune (Someone Else)\n- Your Highlight on page 1 | Location 1-2 | Added on Monday, January 1, 2024 11:00:00 AM\n\nOther dune\n==========\n");
        std::fs::write(&src, more).unwrap();
        let s = import(&src, &notes, false, false).unwrap();
        assert_eq!(s.new, 2);
        let t = std::fs::read_to_string(&dune).unwrap();
        assert!(t.starts_with(&format!("{text}my own line\n\n* New one\n")));
        let other = std::fs::read_to_string(notes.join("Reading/dune-2.org")).unwrap();
        assert!(other.starts_with("#+title: Dune\n#+author: Someone Else\n") && other.contains("Other dune"));
        std::fs::write(&src, "").unwrap();
        assert_eq!(import(&src, &notes, false, false).unwrap().books.len(), 0);
        assert!(import(Path::new("rel.txt"), &notes, true, false).is_err());
        std::fs::remove_dir_all(&root).unwrap();
    }
}
