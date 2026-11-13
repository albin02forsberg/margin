// org-cite citations (#1335): `[cite:@a;@b]`, `[cite/t:see @a p. 3]`. Prefixes and suffixes are kept in the text but ignored here.

/** A `bib_entries` entry (src-tauri/src/bib.rs). */
export type BibEntry = { key: string; author: string; year: string; title: string; container: string; label: string };
export type Cite = { from: number; to: number; keys: { key: string; from: number; to: number }[] };

const CITE = /(?<!\[)\[cite(?:\/[\w/-]*)?:([^\]\n]*)\]/g;
const KEY = /@([\w\-.:?!'/*+|<>&^$#%~]+)/g;

/** The cites with keys on one LINE (not `[[cite:…]]` links), with each key's column range (keys without trailing punctuation). */
export function cites(line: string): Cite[] {
  const out: Cite[] = [];
  for (const m of line.matchAll(CITE)) {
    const body = m.index! + m[0].length - 1 - m[1].length, keys = [];
    for (const k of m[1].matchAll(KEY)) {
      const key = k[1].replace(/[.:?!]+$/, ""), from = body + k.index!;
      if (key) keys.push({ key, from, to: from + 1 + key.length });
    }
    if (keys.length) out.push({ from: m.index!, to: m.index! + m[0].length, keys });
  }
  return out;
}

/** The key under column COL of LINE: the one the column is on, else the cite's first key; null outside a cite. */
export function citeAt(line: string, col: number): string | null {
  const c = cites(line).find((c) => c.from <= col && col <= c.to);
  return c ? (c.keys.find((k) => k.from <= col && col <= k.to) ?? c.keys[0]).key : null;
}

/** The key prefix being typed after `@` inside an unclosed cite at the end of BEFORE (completion), else null. */
export const citeQuery = (before: string): string | null => /\[cite(?:\/[\w/-]*)?:[^\]\n]*@([\w\-.:/]*)$/.exec(before)?.[1] ?? null;

/** One entry as hover text: `Smith, J. (2020). Title. Journal.` */
export const citeText = (e: BibEntry) =>
  [e.author || e.key, e.year && `(${e.year})`].filter(Boolean).join(" ") + [e.title, e.container].filter(Boolean).map((s) => `. ${s}`).join("") + ".";

/** The chip shown for KEYS: entries' author-year labels, unknown keys as `@key`; `missing` when one isn't in BIB. */
export function chip(keys: string[], bib: Map<string, BibEntry>): { text: string; missing: boolean } {
  const text = keys.map((k) => bib.get(k)?.label || (bib.has(k) ? k : `@${k}`)).join("; ");
  return { text, missing: keys.some((k) => !bib.has(k)) };
}

/** A `#+PRINT_BIBLIOGRAPHY:` line (#1336): shown as the note's cited entries, expanded only at export, never written. */
export const printBibDirective = (line: string) => /^\s*#\+print_bibliography:/i.test(line);

/** The keys cited in TEXT in order of first citation, deduplicated, with their BIB entry (none: unknown); src/example blocks skipped. */
export function bibliography(text: string, bib: Map<string, BibEntry>): { key: string; entry?: BibEntry }[] {
  const seen = new Set<string>(), out: { key: string; entry?: BibEntry }[] = [];
  let code = false;
  for (const l of text.split("\n")) {
    if (/^\s*#\+begin_(src|example)\b/i.test(l)) code = true;
    else if (/^\s*#\+end_(src|example)\b/i.test(l)) code = false;
    else if (!code) for (const c of cites(l)) for (const { key } of c.keys) if (!seen.has(key)) seen.add(key), out.push({ key, entry: bib.get(key) });
  }
  return out;
}

/** One bibliography line: `Smith 2020. Title. Journal.`; an unknown key as `[key?]`. */
export const bibLine = (key: string, e?: BibEntry) => (e ? [e.label || e.key, e.title, e.container].filter(Boolean).join(". ") + "." : `[${key}?]`);
