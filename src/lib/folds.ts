// Folded headings remembered per note across tab close and restart (#1236). Text-only so node tests can import it.
// A heading is keyed by its `:ID:` property, else by its line text plus which occurrence of that text it is,
// so edits elsewhere in the note don't shift it. The text is the stars plus the title: a TODO keyword, priority or tags changing keeps the fold. Stored in localStorage, never in the note.

import { headProp, startupFolds, startupMode } from "./startup.ts";

export type FoldMap = Record<string, string[]>;

const isHead = (t: string) => /^\*+ /.test(t);

/** Heading T as stars plus title: the TODO/DONE keyword in KW, `[#A]` priority and trailing `:tags:` removed. */
function headTitle(t: string, kw: string[]): string {
  const m = /^(\*+) (.*)$/.exec(t.trimEnd())!;
  let r = m[2].replace(/\s+:[^\s:]+(?::[^\s:]+)*:$/, "");
  const w = /^(\S+)(?:\s+|$)/.exec(r)?.[1];
  if (w && kw.includes(w)) r = r.slice(w.length).trimStart();
  return `${m[1]}|${r.replace(/^\[#[A-Z0-9]\]\s*/, "").trim()}`;
}

/** Every heading line (1-based) with its key. */
function keyed(lines: string[], kw: string[]): [number, string][] {
  const seen = new Map<string, number>();
  const out: [number, string][] = [];
  lines.forEach((t, i) => {
    if (!isHead(t)) return;
    const id = headProp(lines, i + 1, "ID");
    if (id) return out.push([i + 1, `id:${id}`]);
    const text = headTitle(t, kw), k = seen.get(text) ?? 0;
    seen.set(text, k + 1);
    out.push([i + 1, `h:${text}#${k}`]);
  });
  return out;
}

/** Keys of the headings on 1-based lines HEADS (non-heading lines are skipped). */
export function headingKeys(lines: string[], heads: number[], kw: string[] = ["TODO", "DONE"]): string[] {
  const want = new Set(heads);
  return keyed(lines, kw).filter(([n]) => want.has(n)).map(([, k]) => k);
}

/** The 1-based heading lines KEYS name in LINES; keys of headings that are gone are dropped. */
export function keyLines(lines: string[], keys: string[], kw: string[] = ["TODO", "DONE"]): number[] {
  const want = new Set(keys);
  return keyed(lines, kw).filter(([, k]) => want.has(k)).map(([n]) => n);
}

/** Whether A and B hold the same keys, in any order. */
export const sameKeys = (a: string[], b: string[]) => a.length === b.length && a.every((k) => b.includes(k)) && b.every((k) => a.includes(k));

/** KEYS to remember for DOC, or null when they equal what `#+STARTUP:` folds anyway (so editing that line still applies later). */
export function foldsToStore(doc: string, keys: string[], kw?: string[]): string[] | null {
  const lines = doc.split("\n");
  return sameKeys(keys, headingKeys(lines, startupFolds(lines, startupMode(doc)).map((f) => f.from), kw)) ? null : keys;
}

/** The stored map from RAW: entries that aren't string lists are dropped; unreadable is empty. */
export function parseFolds(raw: string | null): FoldMap {
  let m: any;
  try { m = JSON.parse(raw ?? ""); } catch { return {}; }
  if (!m || typeof m !== "object" || Array.isArray(m)) return {};
  return Object.fromEntries(Object.entries(m).filter(([, v]) => Array.isArray(v) && v.every((k) => typeof k === "string"))) as FoldMap;
}

/** MAP with PATH's folds set to KEYS (most recent last), the oldest dropped beyond MAX notes. */
export function putFolds(map: FoldMap, path: string, keys: string[], max = 200): FoldMap {
  const out = { ...map };
  delete out[path];
  out[path] = keys;
  const paths = Object.keys(out);
  for (const p of paths.slice(0, Math.max(0, paths.length - max))) delete out[p];
  return out;
}
