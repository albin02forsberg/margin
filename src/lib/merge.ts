// Pure text diff/merge helpers for syncing open notes with the disk (no editor imports, so node --test can load it).

export type Change = { from: number; to: number; insert: string };

/** The single change turning OLD into TEXT (common prefix/suffix kept). */
export function diffChange(old: string, text: string): Change {
  let a = 0;
  while (a < old.length && a < text.length && old[a] === text[a]) a++;
  let b = 0;
  while (b < old.length - a && b < text.length - a && old[old.length - 1 - b] === text[text.length - 1 - b]) b++;
  return { from: a, to: old.length - b, insert: text.slice(a, text.length - b) };
}

/** The disk's change to SAVED, positioned for MINE (SAVED plus your unsaved edit), or null when
 *  the two edits overlap or touch, i.e. a conflict. */
export function mergeEdits(saved: string, mine: string, disk: string): Change | null {
  const m = diffChange(saved, mine);
  const t = diffChange(saved, disk);
  const shift = m.insert.length - (m.to - m.from);
  if (t.from > m.to) return { from: t.from + shift, to: t.to + shift, insert: t.insert };
  if (t.to < m.from) return t;
  return null;
}
