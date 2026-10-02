// Attachments: files dropped or pasted into a note are stored under
// <notes>/<attachments_dir>/<note stem>/ and linked relative to the note.

use regex::Regex;
use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};
use std::sync::LazyLock;

/// Link targets: `[[target]]`, `[[target][desc]]` and plain `file:target`.
static LINK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[\[([^\]]+)\]|\bfile:([^\s\[\]]+)").unwrap());

/// `[` and `]` would end the org link early.
fn clean(s: &str) -> String {
    s.replace(['[', ']'], "_")
}

/// Where attachment NAME of NOTE goes: (destination, link target relative to the
/// note's folder). Names already TAKEN get -1, -2… before the extension.
pub fn target(notes: &Path, dir: &str, note: &Path, name: &str, taken: impl Fn(&Path) -> bool) -> Result<(PathBuf, String), String> {
    let name = clean(&Path::new(name).file_name().ok_or_else(|| format!("bad file name {name}"))?.to_string_lossy());
    let stem = clean(&note.file_stem().ok_or("the note has no name")?.to_string_lossy());
    let rel_dir = Path::new(dir).join(&stem);
    let folder = notes.join(&rel_dir);
    let note_dir = note.parent().and_then(|p| p.strip_prefix(notes).ok()).ok_or("the note is outside the notes folder")?;
    if [&rel_dir, note_dir].iter().any(|p| p.components().any(|c| !matches!(c, Component::Normal(_)))) {
        return Err(format!("{} is outside the notes folder", folder.display()));
    }
    let (base, ext) = match name.rsplit_once('.') {
        Some((b, e)) if !b.is_empty() => (b, format!(".{e}")),
        _ => (name.as_str(), String::new()),
    };
    let file = (0..)
        .map(|i| if i == 0 { name.clone() } else { format!("{base}-{i}{ext}") })
        .find(|f| !taken(&folder.join(f)))
        .unwrap();
    let up = "../".repeat(note_dir.components().count());
    let link = format!("{up}{}/{file}", rel_dir.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/"));
    Ok((folder.join(file), link))
}

/// FILES that no note (path, text) links to, relative or absolute; `file:` and
/// `::search` suffixes are ignored, and `..`/`.` are resolved lexically.
pub fn unused(notes: &[(&Path, &str)], files: &[PathBuf]) -> Vec<PathBuf> {
    let mut used = HashSet::new();
    for (note, text) in notes {
        let dir = note.parent().unwrap_or(Path::new(""));
        for c in LINK.captures_iter(text) {
            let t = c.get(1).or(c.get(2)).unwrap().as_str();
            let t = t.strip_prefix("file:").unwrap_or(t);
            let t = t.split_once("::").map_or(t, |(p, _)| p);
            used.insert(normalize(&dir.join(t)));
        }
    }
    files.iter().filter(|f| !used.contains(&normalize(f))).cloned().collect()
}

fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_links() {
        let n = Path::new("/n");
        let none = |_: &Path| false;
        let t = |note: &str, name: &str| target(n, "attachments", Path::new(note), name, none);
        assert_eq!(t("/n/idea.org", "pic.png").unwrap(), (PathBuf::from("/n/attachments/idea/pic.png"), "attachments/idea/pic.png".into()));
        assert_eq!(t("/n/daily/2026-10-02.org", "/tmp/a b.pdf").unwrap().1, "../attachments/2026-10-02/a b.pdf");
        assert_eq!(t("/n/idea.org", "x[1].png").unwrap().1, "attachments/idea/x_1_.png");
        // collisions
        let taken = |p: &Path| ["/n/attachments/idea/pic.png", "/n/attachments/idea/pic-1.png", "/n/attachments/idea/Makefile"].contains(&p.to_str().unwrap());
        assert_eq!(target(n, "attachments", Path::new("/n/idea.org"), "pic.png", taken).unwrap().1, "attachments/idea/pic-2.png");
        assert_eq!(target(n, "attachments", Path::new("/n/idea.org"), "Makefile", taken).unwrap().1, "attachments/idea/Makefile-1");
        // escapes
        assert!(t("/n/idea.org", "..").is_err());
        assert!(t("/elsewhere/idea.org", "pic.png").is_err());
        assert!(target(n, "../out", Path::new("/n/idea.org"), "pic.png", none).is_err());
        assert!(target(n, "/abs", Path::new("/n/idea.org"), "pic.png", none).is_err());
    }

    #[test]
    fn unused_files() {
        let f = |s: &[&str]| s.iter().map(PathBuf::from).collect::<Vec<_>>();
        let files = f(&["/n/att/idea/a.png", "/n/att/idea/b c.pdf", "/n/att/day/x.png", "/n/att/day/y.png", "/n/att/idea/z.txt", "/n/att/idea/w.png"]);
        let notes = [
            (Path::new("/n/idea.org"), "[[file:att/idea/a.png]] and [[file:./att/idea/b c.pdf::3][doc]]"),
            (Path::new("/n/daily/day.org"), "see file:../att/day/x.png here\n[[/n/att/idea/w.png]]"),
        ];
        assert_eq!(unused(&notes, &files), f(&["/n/att/day/y.png", "/n/att/idea/z.txt"]));
        assert_eq!(unused(&[], &files), files);
    }
}
