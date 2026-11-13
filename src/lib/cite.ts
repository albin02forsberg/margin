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
