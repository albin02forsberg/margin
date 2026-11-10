// Folded headings remembered per note across tab close and restart (#1236). Text-only so node tests can import it.
// A heading is keyed by its `:ID:` property, else by its line text plus which occurrence of that text it is,
// so edits elsewhere in the note don't shift it. Stored in localStorage, never in the note.

export type FoldMap = Record<string, string[]>;

const isHead = (t: string) => /^\*+ /.test(t);

/** The `:ID:` in the property drawer right under heading line N (1-based), after at most one planning line. */
function headId(lines: string[], n: number): string | undefined {
  for (let j = n; j < Math.min(n + 2, lines.length) && !isHead(lines[j]); j++) {
    if (!/^\s*:PROPERTIES:\s*$/i.test(lines[j])) continue;
    for (let k = j + 1; k < lines.length && !/^\s*:END:\s*$/i.test(lines[k]) && !isHead(lines[k]); k++) {
      const m = /^\s*:ID:\s*(\S+)\s*$/i.exec(lines[k]);
      if (m) return m[1];
    }
    return;
  }
}

/** Every heading line (1-based) with its key. */
function keyed(lines: string[]): [number, string][] {
  const seen = new Map<string, number>();
  const out: [number, string][] = [];
  lines.forEach((t, i) => {
    if (!isHead(t)) return;
    const id = headId(lines, i + 1);
    if (id) return out.push([i + 1, `id:${id}`]);
    const text = t.trimEnd(), k = seen.get(text) ?? 0;
    seen.set(text, k + 1);
    out.push([i + 1, `h:${text}#${k}`]);
  });
  return out;
}

/** Keys of the headings on 1-based lines HEADS (non-heading lines are skipped). */
export function headingKeys(lines: string[], heads: number[]): string[] {
  const want = new Set(heads);
  return keyed(lines).filter(([n]) => want.has(n)).map(([, k]) => k);
}

/** The 1-based heading lines KEYS name in LINES; keys of headings that are gone are dropped. */
export function keyLines(lines: string[], keys: string[]): number[] {
  const want = new Set(keys);
  return keyed(lines).filter(([, k]) => want.has(k)).map(([n]) => n);
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
