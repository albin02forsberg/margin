// Semantic note search, part 1 of #470: notes split into sections, an on-disk vector index that
// re-embeds only changed sections, and cosine ranking. The embedder is a parameter, so this needs
// no model; the chat wires it up in a later step.
#![allow(dead_code)]

use crate::org::OrgFile;
use crate::retrieve::meta;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// A section longer than this is split on line boundaries.
const CHUNK: usize = 1500;
const MAGIC: &[u8; 8] = b"MRGNEMB\0";
const VERSION: u32 = 1;

/// A piece of a note to embed: LINE (0-based) is where it starts; TEXT is the note's title and
/// the section's non-meta lines.
#[derive(Debug, Clone, PartialEq)]
pub struct Chunk {
    pub line: usize,
    pub text: String,
}

/// F split by headline: the text before the first one, then each section up to the next headline
/// of any level, longer ones cut into CHUNK-sized runs of lines. Blank and meta-only parts are left out.
pub fn chunks(f: &OrgFile) -> Vec<Chunk> {
    let lines: Vec<&str> = f.text.lines().collect();
    let mut starts: Vec<usize> = std::iter::once(0).chain(f.headlines.iter().map(|h| h.line)).filter(|&l| l < lines.len()).collect();
    starts.dedup();
    let mut out = vec![];
    for (n, &a) in starts.iter().enumerate() {
        let b = starts.get(n + 1).copied().unwrap_or(lines.len());
        let (mut at, mut buf) = (a, String::new());
        let mut flush = |at: usize, buf: &mut String| {
            if !buf.trim().is_empty() {
                out.push(Chunk { line: at, text: format!("{}\n{}", f.title, buf.trim_end()) });
            }
            buf.clear();
        };
        for (i, l) in lines.iter().enumerate().take(b).skip(a).filter(|(_, l)| !meta(l)) {
            if !buf.is_empty() && buf.len() + l.len() + 1 > CHUNK {
                flush(at, &mut buf);
            }
            if buf.is_empty() {
                at = i;
            }
            buf.push_str(l);
            buf.push('\n');
        }
        flush(at, &mut buf);
    }
    out
}

fn hash(text: &str) -> u64 {
    u64::from_le_bytes(Sha256::digest(text.as_bytes())[..8].try_into().unwrap())
}

fn nanos(t: Option<SystemTime>) -> u64 {
    t.and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_nanos() as u64)
}

/// One section's vector; HASH is of the text that was embedded.
#[derive(Debug, Clone, PartialEq)]
pub struct Vector {
    pub line: usize,
    pub hash: u64,
    pub vec: Vec<f32>,
}

/// One note's vectors; MTIME (ns, 0 = unknown or not fully embedded) lets an unchanged note skip chunking.
#[derive(Debug, Clone, PartialEq)]
pub struct Indexed {
    pub path: PathBuf,
    pub mtime: u64,
    pub chunks: Vec<Vector>,
}

/// The vectors of a notes folder made by MODEL, all of DIM floats (0 until the first one).
#[derive(Debug, Clone, PartialEq)]
pub struct Index {
    pub model: String,
    pub dim: usize,
    pub files: Vec<Indexed>,
}

/// Where the index of the notes folder ROOT lives: LOCAL is the app's local data dir, never the
/// data dir (backups commit that); one file per notes folder, so indexes don't mix.
pub fn index_path(local: &Path, root: &Path) -> PathBuf {
    local.join("embeddings").join(format!("{:016x}.bin", hash(&root.to_string_lossy())))
}

impl Index {
    pub fn new(model: &str) -> Index {
        Index { model: model.into(), dim: 0, files: vec![] }
    }

    /// Bring the index up to date with FILES (already filtered by `ai_exclude`): notes no longer
    /// listed lose their vectors, unchanged ones (same scanned mtime) are kept, and changed or new
    /// sections are embedded, reusing any vector of the same text (so a moved note costs nothing).
    /// At most CAP sections are embedded per run; a note left short is retried next run. Returns
    /// how many were embedded; on an embedder error the work so far is kept and the error returned.
    pub fn update(&mut self, files: &[Arc<OrgFile>], embed: &dyn Fn(&str) -> Result<Vec<f32>, String>, cap: usize) -> Result<usize, String> {
        let old = std::mem::take(&mut self.files);
        let known: HashMap<u64, &Vec<f32>> = old.iter().flat_map(|f| &f.chunks).map(|c| (c.hash, &c.vec)).collect();
        let by_path: HashMap<&Path, &Indexed> = old.iter().map(|f| (f.path.as_path(), f)).collect();
        let (mut done, mut err) = (0, None);
        for f in files {
            // The scanned text's mtime, so a note edited while indexing is redone next run.
            let mtime = nanos(f.mtime);
            if let Some(o) = by_path.get(f.path.as_path()).filter(|o| mtime != 0 && o.mtime == mtime) {
                self.files.push((*o).clone());
                continue;
            }
            let mut ix = Indexed { path: f.path.clone(), mtime, chunks: vec![] };
            for c in chunks(f) {
                let h = hash(&c.text);
                let vec = match known.get(&h) {
                    Some(v) => (*v).clone(),
                    None if done < cap && err.is_none() => match embed(&c.text) {
                        Ok(v) if v.is_empty() || (self.dim != 0 && v.len() != self.dim) => {
                            err = Some(format!("embedding has {} dimensions, expected {}", v.len(), self.dim));
                            continue;
                        }
                        Ok(v) => {
                            done += 1;
                            self.dim = v.len();
                            v
                        }
                        Err(e) => {
                            err = Some(e);
                            continue;
                        }
                    },
                    None => {
                        ix.mtime = 0;
                        continue;
                    }
                };
                ix.chunks.push(Vector { line: c.line, hash: h, vec });
            }
            if err.is_some() {
                ix.mtime = 0;
            }
            self.files.push(ix);
        }
        err.map_or(Ok(done), Err)
    }

    /// The K sections most like QUERY by cosine similarity, best first: (path, line, score).
    pub fn top_k(&self, query: &[f32], k: usize) -> Vec<(&Path, usize, f32)> {
        let norm = |v: &[f32]| v.iter().map(|x| x * x).sum::<f32>().sqrt();
        let qn = norm(query);
        if qn == 0.0 {
            return vec![];
        }
        let mut out: Vec<(&Path, usize, f32)> = self
            .files
            .iter()
            .flat_map(|f| f.chunks.iter().map(move |c| (f.path.as_path(), c)))
            .filter(|(_, c)| c.vec.len() == query.len())
            .filter_map(|(p, c)| {
                let n = norm(&c.vec);
                (n > 0.0).then(|| (p, c.line, c.vec.iter().zip(query).map(|(a, b)| a * b).sum::<f32>() / (n * qn)))
            })
            .collect();
        out.sort_by(|a, b| b.2.total_cmp(&a.2));
        out.truncate(k);
        out
    }

    /// MAGIC, then little-endian u32 VERSION, model, dim and file count; per file its path, u64
    /// mtime and chunk count; per chunk u32 line, u64 hash and DIM f32s. Strings are u32 length + UTF-8.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut b = MAGIC.to_vec();
        let u32le = |b: &mut Vec<u8>, n: usize| b.extend((n as u32).to_le_bytes());
        let str = |b: &mut Vec<u8>, s: &str| {
            u32le(b, s.len());
            b.extend(s.as_bytes());
        };
        u32le(&mut b, VERSION as usize);
        str(&mut b, &self.model);
        u32le(&mut b, self.dim);
        u32le(&mut b, self.files.len());
        for f in &self.files {
            str(&mut b, &f.path.to_string_lossy());
            b.extend(f.mtime.to_le_bytes());
            u32le(&mut b, f.chunks.len());
            for c in &f.chunks {
                u32le(&mut b, c.line);
                b.extend(c.hash.to_le_bytes());
                c.vec.iter().for_each(|x| b.extend(x.to_le_bytes()));
            }
        }
        b
    }

    /// The index in B, if it is a whole one of this VERSION made by MODEL.
    pub fn from_bytes(b: &[u8], model: &str) -> Option<Index> {
        let mut r = b.strip_prefix(MAGIC)?;
        let mut take = |n: usize| -> Option<&[u8]> {
            let (a, rest) = r.split_at_checked(n)?;
            r = rest;
            Some(a)
        };
        macro_rules! u32 {
            () => {
                u32::from_le_bytes(take(4)?.try_into().ok()?) as usize
            };
        }
        macro_rules! u64 {
            () => {
                u64::from_le_bytes(take(8)?.try_into().ok()?)
            };
        }
        macro_rules! str {
            () => {{
                let n = u32!();
                String::from_utf8(take(n)?.to_vec()).ok()?
            }};
        }
        if u32!() != VERSION as usize || str!() != model {
            return None;
        }
        let dim = u32!();
        let mut files = vec![];
        for _ in 0..u32!() {
            let (path, mtime) = (PathBuf::from(str!()), u64!());
            let mut chunks = vec![];
            for _ in 0..u32!() {
                let (line, hash) = (u32!(), u64!());
                let vec = take(dim.checked_mul(4)?)?.as_chunks::<4>().0.iter().map(|x| f32::from_le_bytes(*x)).collect();
                chunks.push(Vector { line, hash, vec });
            }
            files.push(Indexed { path, mtime, chunks });
        }
        r.is_empty().then(|| Index { model: model.into(), dim, files })
    }

    /// The index at PATH, or an empty one when it is missing, corrupt, old or of another model.
    pub fn load(path: &Path, model: &str) -> Index {
        std::fs::read(path).ok().and_then(|b| Index::from_bytes(&b, model)).unwrap_or_else(|| Index::new(model))
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d)?;
        }
        crate::config::write_atomic(path, self.to_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::org::{parse, Kw};
    use std::cell::RefCell;

    fn note(path: &str, text: &str, mtime: u64) -> Arc<OrgFile> {
        let mut f = parse(Path::new(path), text, &Kw { todo: vec![], done: vec![] });
        f.mtime = Some(UNIX_EPOCH + std::time::Duration::from_nanos(mtime));
        Arc::new(f)
    }
    /// A fake embedder: counts of a, e and o, recording what it was asked.
    fn fake(log: &RefCell<Vec<String>>) -> impl Fn(&str) -> Result<Vec<f32>, String> + '_ {
        move |t: &str| {
            log.borrow_mut().push(t.to_string());
            Ok("aeo".chars().map(|c| t.matches(c).count() as f32).collect())
        }
    }

    #[test]
    fn chunks_by_section() {
        let f = note("/n/a.org", "#+title: Alpha\nintro\n* One\n:PROPERTIES:\n:ID: x\n:END:\nbody one\n** Two\n* Empty\n#+begin: x\n", 1);
        let c = chunks(&f);
        assert_eq!(c.iter().map(|c| c.line).collect::<Vec<_>>(), [1, 2, 7, 8]);
        assert_eq!(c[0].text, "Alpha\nintro");
        assert_eq!(c[1].text, "Alpha\n* One\nbody one");
        assert_eq!(c[2].text, "Alpha\n** Two");
        let body: Vec<String> = (0..40).map(|i| format!("line {i} {}", "x".repeat(90))).collect();
        let c = chunks(&note("/n/b.org", &format!("#+title: B\n* H\n{}", body.join("\n")), 1));
        assert!(c.len() > 1 && c.iter().all(|c| c.text.len() <= CHUNK + 10));
        assert_eq!(c[1].line, c[0].line + c[0].text.lines().count() - 1);
    }

    #[test]
    fn cosine_ranking() {
        let v = |line, hash, vec: Vec<f32>| Vector { line, hash, vec };
        let ix = Index {
            model: "m".into(),
            dim: 2,
            files: vec![
                Indexed { path: "/a".into(), mtime: 1, chunks: vec![v(0, 1, vec![1.0, 0.0]), v(5, 2, vec![0.0, 0.0])] },
                Indexed { path: "/b".into(), mtime: 1, chunks: vec![v(3, 3, vec![1.0, 1.0]), v(9, 4, vec![-1.0, 0.0])] },
            ],
        };
        let top: Vec<(&str, usize)> = ix.top_k(&[2.0, 0.1], 2).into_iter().map(|(p, l, _)| (p.to_str().unwrap(), l)).collect();
        assert_eq!(top, [("/a", 0), ("/b", 3)]);
        assert_eq!(ix.top_k(&[0.0, 1.0], 9).len(), 3); // a zero vector never matches
        assert!(ix.top_k(&[0.0, 0.0], 9).is_empty() && ix.top_k(&[1.0], 9).is_empty());
    }

    #[test]
    fn update_reuses_unchanged_and_moved() {
        let log = RefCell::new(vec![]);
        let mut ix = Index::new("m");
        let a = note("/n/a.org", "#+title: A\n* One\naaa\n* Two\neee\n", 1);
        let b = note("/n/b.org", "#+title: B\n* Three\nooo\n", 1);
        assert_eq!(ix.update(&[a.clone(), b.clone()], &fake(&log), 100), Ok(3));
        assert_eq!(ix.dim, 3);
        // Same mtime: not even chunked.
        assert_eq!(ix.update(&[a.clone(), b.clone()], &fake(&log), 100), Ok(0));
        // One section edited: only it is embedded.
        let a2 = note("/n/a.org", "#+title: A\n* One\naaa\n* Two\neee eee\n", 2);
        log.borrow_mut().clear();
        assert_eq!(ix.update(&[a2.clone(), b.clone()], &fake(&log), 100), Ok(1));
        assert_eq!(*log.borrow(), ["A\n* Two\neee eee"]);
        // Moved note with a new mtime: vectors are found by text, nothing embedded.
        let moved = note("/n/sub/a.org", &a2.text, 3);
        assert_eq!(ix.update(&[moved, b.clone()], &fake(&log), 100), Ok(0));
        assert_eq!(ix.files.iter().map(|f| f.path.to_str().unwrap()).collect::<Vec<_>>(), ["/n/sub/a.org", "/n/b.org"]);
        assert_eq!(ix.files[0].chunks.len(), 2);
        // A note no longer listed (deleted or newly excluded) loses its vectors.
        ix.update(std::slice::from_ref(&b), &fake(&log), 100).unwrap();
        assert_eq!(ix.files.len(), 1);
        assert!(ix.top_k(&[1.0, 1.0, 1.0], 9).iter().all(|(p, ..)| *p == Path::new("/n/b.org")));
    }

    #[test]
    fn cap_and_errors_retry_next_run() {
        let log = RefCell::new(vec![]);
        let mut ix = Index::new("m");
        let a = note("/n/a.org", "* One\naaa\n* Two\neee\n* Three\nooo\n", 1);
        assert_eq!(ix.update(std::slice::from_ref(&a), &fake(&log), 2), Ok(2));
        assert_eq!((ix.files[0].chunks.len(), ix.files[0].mtime), (2, 0));
        assert_eq!(ix.update(std::slice::from_ref(&a), &fake(&log), 2), Ok(1));
        assert_eq!((ix.files[0].chunks.len(), ix.files[0].mtime), (3, nanos(a.mtime)));
        let b = note("/n/b.org", "* New\nx\n", 1);
        let fail = |_: &str| Err::<Vec<f32>, String>("down".into());
        assert_eq!(ix.update(&[a.clone(), b.clone()], &fail, 9), Err("down".into()));
        assert_eq!((ix.files[0].chunks.len(), ix.files[1].chunks.len(), ix.files[1].mtime), (3, 0, 0));
        let wrong = |_: &str| Ok(vec![1.0]);
        assert!(ix.update(&[a, b], &wrong, 9).unwrap_err().contains("dimensions"));
    }

    #[test]
    fn file_roundtrip_and_rejects() {
        let log = RefCell::new(vec![]);
        let mut ix = Index::new("nomic");
        ix.update(&[note("/n/å.org", "hej\n* One\naaa\n", 1)], &fake(&log), 9).unwrap();
        let dir = std::env::temp_dir().join(format!("margin-embed-{}", std::process::id()));
        let p = index_path(&dir, Path::new("/home/me/notes"));
        assert!(p.starts_with(dir.join("embeddings")) && p != index_path(&dir, Path::new("/other")));
        ix.save(&p).unwrap();
        assert_eq!(Index::load(&p, "nomic"), ix);
        assert_eq!(Index::load(&p, "other"), Index::new("other"));
        let b = ix.to_bytes();
        assert!(Index::from_bytes(&b[..b.len() - 1], "nomic").is_none());
        assert!(Index::from_bytes(&[b.as_slice(), &[0]].concat(), "nomic").is_none());
        let mut old = b.clone();
        old[8] = 0;
        assert!(Index::from_bytes(&old, "nomic").is_none());
        assert_eq!(Index::load(&dir.join("missing.bin"), "nomic"), Index::new("nomic"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
