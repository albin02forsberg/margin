//! Encrypted headings (#1084): a heading's body becomes an armoured
//! ChaCha20-Poly1305 block keyed by argon2id over the passphrase. Pure
//! functions only; the heading line and its tags stay plaintext.
//!
//! ```text
//! -----BEGIN MARGIN ENCRYPTED-----
//! v=1 kdf=argon2id m=65536 t=3 p=1 salt=<b64> nonce=<b64>
//! <base64 ciphertext, 64 columns>
//! -----END MARGIN ENCRYPTED-----
//! ```
//! The header line is the AEAD associated data, so a changed parameter,
//! salt or nonce fails exactly like a changed ciphertext.

use argon2::{Algorithm, Argon2, Params, Version};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use chacha20poly1305::aead::{rand_core::RngCore, Aead, AeadCore, KeyInit, OsRng, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use std::ops::Range;
use std::path::{Path, PathBuf};

pub const BEGIN: &str = "-----BEGIN MARGIN ENCRYPTED-----";
pub const END: &str = "-----END MARGIN ENCRYPTED-----";
/// Argon2id cost for new blocks: 64 MiB, 3 passes, 1 lane.
const KDF: (u32, u32, u32) = (65536, 3, 1);
/// Upper bounds accepted on decrypt, so a hostile file can't exhaust memory or time.
const MAX_M: u32 = 1 << 20;
const MAX_T: u32 = 16;
const MAX_P: u32 = 16;

fn key(pass: &str, salt: &[u8], (m, t, p): (u32, u32, u32)) -> Result<[u8; 32], String> {
    let params = Params::new(m, t, p, Some(32)).map_err(|e| format!("bad kdf parameters: {e}"))?;
    let mut k = [0u8; 32];
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(pass.as_bytes(), salt, &mut k)
        .map_err(|e| format!("key derivation failed: {e}"))?;
    Ok(k)
}

/// Encrypt a section body into an armoured block (no trailing newline).
pub fn encrypt_section(plain: &str, pass: &str) -> Result<String, String> {
    encrypt_with(plain, pass, KDF)
}

fn encrypt_with(plain: &str, pass: &str, kdf: (u32, u32, u32)) -> Result<String, String> {
    if pass.is_empty() {
        return Err("empty passphrase".into());
    }
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
    let (m, t, p) = kdf;
    let header = format!(
        "v=1 kdf=argon2id m={m} t={t} p={p} salt={} nonce={}",
        B64.encode(salt),
        B64.encode(nonce)
    );
    let ct = ChaCha20Poly1305::new(Key::from_slice(&key(pass, &salt, kdf)?))
        .encrypt(&nonce, Payload { msg: plain.as_bytes(), aad: header.as_bytes() })
        .map_err(|_| "encryption failed".to_string())?;
    let b64 = B64.encode(ct);
    let mut out = format!("{BEGIN}\n{header}\n");
    // base64 is ASCII, so byte chunks are valid str slices.
    for c in b64.as_bytes().chunks(64) {
        out.push_str(std::str::from_utf8(c).unwrap());
        out.push('\n');
    }
    out.push_str(END);
    Ok(out)
}

/// Decrypt an armoured block. Leading indentation and surrounding blank lines are ignored.
pub fn decrypt_section(block: &str, pass: &str) -> Result<String, String> {
    let mut lines = block.lines().map(str::trim).filter(|l| !l.is_empty());
    if lines.next() != Some(BEGIN) {
        return Err("not an encrypted block".into());
    }
    let header = lines.next().ok_or("truncated encrypted block")?;
    let mut body = String::new();
    let mut closed = false;
    for l in lines.by_ref() {
        if l == END {
            closed = true;
            break;
        }
        body.push_str(l);
    }
    if !closed || lines.next().is_some() {
        return Err("truncated encrypted block".into());
    }
    let field = |k: &str| {
        header.split(' ').find_map(|f| f.strip_prefix(k)?.strip_prefix('=')).ok_or(format!("missing {k} in encrypted block"))
    };
    if field("v")? != "1" {
        return Err(format!("unsupported encrypted block version {}", field("v")?));
    }
    if field("kdf")? != "argon2id" {
        return Err(format!("unsupported kdf {}", field("kdf")?));
    }
    let num = |k: &str, max: u32| -> Result<u32, String> {
        field(k)?.parse::<u32>().ok().filter(|&n| n <= max).ok_or(format!("bad {k} in encrypted block"))
    };
    let kdf = (num("m", MAX_M)?, num("t", MAX_T)?, num("p", MAX_P)?);
    let dec = |k: &str| B64.decode(field(k)?).map_err(|_| format!("bad {k} in encrypted block"));
    let (salt, nonce) = (dec("salt")?, dec("nonce")?);
    if nonce.len() != 12 {
        return Err("bad nonce in encrypted block".into());
    }
    let ct = B64.decode(&body).map_err(|_| "corrupt encrypted block".to_string())?;
    let pt = ChaCha20Poly1305::new(Key::from_slice(&key(pass, &salt, kdf)?))
        .decrypt(Nonce::from_slice(&nonce), Payload { msg: &ct, aad: header.as_bytes() })
        .map_err(|_| "wrong passphrase or tampered block".to_string())?;
    String::from_utf8(pt).map_err(|_| "decrypted text is not UTF-8".into())
}

/// Where the pre-encryption copy of FILE lives: LOCAL is the app's local data dir, never the notes
/// or data dir (those are committed by backups and synced, which would publish the plaintext).
pub fn backup_path(local: &Path, file: &Path) -> PathBuf {
    local.join("crypt-backup").join(format!("{:016x}.org", crate::embed::hash(&file.to_string_lossy())))
}

/// Call before the first ciphertext write to FILE. Checks that BLOCK decrypts back to PLAIN (the
/// body being locked), then keeps a copy of ORIGINAL (the file as it is now) in `backup_path`
/// unless one exists. Returns true when a new copy was made (show the one-time notice then).
pub fn backup_before_lock(local: &Path, file: &Path, original: &str, plain: &str, block: &str, pass: &str) -> Result<bool, String> {
    if decrypt_section(block, pass)? != plain {
        return Err("encrypted block does not decrypt to the original text".into());
    }
    let dst = backup_path(local, file);
    if dst.exists() {
        return Ok(false);
    }
    std::fs::create_dir_all(dst.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(&dst, original).map_err(|e| format!("backup failed: {e}"))?;
    Ok(true)
}

/// Byte range of the body of the heading starting at byte `start` of `text`:
/// after the heading line, up to the next heading of the same or higher level
/// (deeper sub-headings belong to the body). Empty if `start` isn't a heading.
pub fn section_body(text: &str, start: usize) -> Range<usize> {
    let rest = &text[start..];
    let Some(level) = crate::org::heading_level(rest) else { return start..start };
    let from = rest.find('\n').map_or(text.len(), |i| start + i + 1);
    let mut pos = from;
    for line in text[from..].split_inclusive('\n') {
        if crate::org::heading_level(line).is_some_and(|l| l <= level) {
            break;
        }
        pos += line.len();
    }
    from..pos
}

/// TEXT with every armoured block (an unterminated one runs to the end) blanked line by line, so
/// line numbers hold. Readers that index, search, export or hand text to an AI use this (#1087).
pub fn strip(text: &str) -> std::borrow::Cow<'_, str> {
    if !text.contains(BEGIN) {
        return text.into();
    }
    let mut open = false;
    text.split_inclusive('\n')
        .map(|l| {
            let t = l.trim();
            let hide = open || t == BEGIN;
            open = hide && t != END;
            if hide { l.trim_start_matches(|c| c != '\n') } else { l }
        })
        .collect::<String>()
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cheap argon2 cost so debug-mode tests stay fast; real blocks use `KDF`.
    const FAST: (u32, u32, u32) = (64, 1, 1);
    fn enc(plain: &str, pass: &str) -> String {
        encrypt_with(plain, pass, FAST).unwrap()
    }

    #[test]
    fn backup_is_local_verified_and_once() {
        let d = std::env::temp_dir().join(format!("margin-crypt-bk-{}", std::process::id()));
        let (local, notes, data) = (d.join("local"), d.join("notes"), d.join("data"));
        let file = notes.join("a.org");
        let orig = "* H\nsecret\n";
        let block = enc("secret\n", "pw");
        let p = backup_path(&local, &file);
        assert!(!p.starts_with(&notes) && !p.starts_with(&data) && p.starts_with(&local));
        // A block that doesn't decrypt to the text, or a wrong passphrase, makes no backup.
        assert!(backup_before_lock(&local, &file, orig, "other\n", &block, "pw").is_err());
        assert!(backup_before_lock(&local, &file, orig, "secret\n", &block, "bad").is_err());
        assert!(!p.exists());
        assert!(backup_before_lock(&local, &file, orig, "secret\n", &block, "pw").unwrap());
        assert_eq!(std::fs::read_to_string(&p).unwrap(), orig);
        // Second lock keeps the first copy and reports no new backup.
        assert!(!backup_before_lock(&local, &file, "* H\n(locked)\n", "secret\n", &block, "pw").unwrap());
        assert_eq!(std::fs::read_to_string(&p).unwrap(), orig);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn round_trip() {
        for plain in ["", "secret", "line 1\n** Sub heading\n* top-level star\nåäö 🔒\n", &"x".repeat(1000)] {
            let b = enc(plain, "pw");
            assert_eq!(decrypt_section(&b, "pw").unwrap(), plain);
            // No armour line can be mistaken for a heading or leak the plaintext.
            assert!(b.lines().all(|l| crate::org::heading_level(l).is_none() && !l.starts_with('*')));
            if plain.len() > 3 {
                assert!(!b.contains(plain));
            }
        }
    }

    #[test]
    fn default_params_round_trip() {
        let b = encrypt_section("body", "pw").unwrap();
        assert!(b.lines().nth(1).unwrap().starts_with("v=1 kdf=argon2id m=65536 t=3 p=1 "));
        assert_eq!(decrypt_section(&b, "pw").unwrap(), "body");
    }

    #[test]
    fn fresh_salt_and_nonce() {
        assert_ne!(enc("same", "pw"), enc("same", "pw"));
    }

    #[test]
    fn indented_block_decrypts() {
        let b = enc("hi", "pw");
        let indented: String = b.lines().map(|l| format!("  {l}\n")).collect();
        assert_eq!(decrypt_section(&format!("\n{indented}\n"), "pw").unwrap(), "hi");
    }

    #[test]
    fn rejects_bad_input() {
        let b = enc("secret body", "pw");
        let err = |s: &str| decrypt_section(s, "pw").unwrap_err();
        assert_eq!(decrypt_section(&b, "nope").unwrap_err(), "wrong passphrase or tampered block");
        assert!(encrypt_with("x", "", FAST).is_err());
        // Tampered ciphertext: flip one base64 char of the body.
        let mut lines: Vec<String> = b.lines().map(String::from).collect();
        let c = lines[2].remove(0);
        lines[2].insert(0, if c == 'A' { 'B' } else { 'A' });
        assert_eq!(err(&lines.join("\n")), "wrong passphrase or tampered block");
        // Tampered header (associated data): bump the cost.
        assert_eq!(err(&b.replace(" t=1 ", " t=2 ")), "wrong passphrase or tampered block");
        // Truncated: no END line, or a body line missing.
        assert_eq!(err(b.trim_end_matches(END)), "truncated encrypted block");
        assert_eq!(err(BEGIN), "truncated encrypted block");
        let long = enc(&"y".repeat(200), "pw");
        let cut: Vec<&str> = long.lines().enumerate().filter(|&(i, _)| i != 2).map(|(_, l)| l).collect();
        assert!(decrypt_section(&cut.join("\n"), "pw").is_err());
        assert!(err(&format!("{b}\ntrailing")).contains("truncated"));
        assert_eq!(err("plain text"), "not an encrypted block");
        assert_eq!(err(&b.replace("v=1 ", "v=2 ")), "unsupported encrypted block version 2");
        assert_eq!(err(&b.replace(" m=64 ", " m=99999999 ")), "bad m in encrypted block");
    }

    #[test]
    fn strip_blanks_blocks_keeping_lines() {
        let b = enc("secret", "pw");
        let t = format!("* A :crypt:\n{b}\n* B\nplain\n");
        let s = strip(&t);
        assert!(!s.contains(BEGIN) && !s.contains("kdf=") && !s.contains(&b[b.len() - 20..]));
        assert_eq!(s.lines().count(), t.lines().count());
        assert_eq!(s.lines().last(), Some("plain"));
        assert_eq!(strip("no block\n"), "no block\n");
        assert_eq!(strip(&format!("x\n{BEGIN}\nabc\n")), "x\n\n\n");
    }

    #[test]
    fn section_body_boundaries() {
        let t = "* A :crypt:\nbody\n** Sub\nmore\n* B\nb\n";
        let r = section_body(t, 0);
        assert_eq!(&t[r], "body\n** Sub\nmore\n");
        let sub = t.find("** Sub").unwrap();
        assert_eq!(&t[section_body(t, sub)], "more\n");
        let b = t.find("* B").unwrap();
        assert_eq!(&t[section_body(t, b)], "b\n");
        // Last heading without trailing newline, and a non-heading start.
        assert_eq!(section_body("* X", 0), 3..3);
        assert_eq!(section_body("text", 0), 0..0);
        // Round trip through encrypt: the `*` lines inside the body come back intact.
        let body = &t[section_body(t, 0)];
        let locked = format!("{}{}\n{}", &t[..12], enc(body, "pw"), &t[b..]);
        let r2 = section_body(&locked, 0);
        assert_eq!(decrypt_section(&locked[r2], "pw").unwrap(), body);
    }
}
