// Ordered-list renumbering (org-list-repair), as a pure function over lines.

const NUM = /^(\s*)(\d+)[.)]\s/;
const indent = (t: string) => /^\s*/.exec(t)![0].length;

/** Digit fixes for the ordered list containing item line I (LINE returns
 *  undefined past either end). Items share the indent; deeper lines and single
 *  blank lines belong to the list; two blanks, a heading or a shallower / non-item
 *  line end it. Numbers run on from the first item. */
export function renumber(line: (i: number) => string | undefined, i: number) {
  const m = NUM.exec(line(i) ?? "");
  if (!m) return [];
  const ind = m[1].length;
  const items = [i];
  for (const d of [-1, 1]) {
    for (let j = i + d, blanks = 0; ; j += d) {
      const t = line(j);
      if (t === undefined) break;
      if (!t.trim()) { if (++blanks > 1) break; continue; }
      blanks = 0;
      if (/^\*+ /.test(t)) break;
      const n = indent(t);
      if (n > ind) continue;
      if (n < ind || !NUM.test(t)) break;
      d < 0 ? items.unshift(j) : items.push(j);
    }
  }
  const out: { line: number; from: number; to: number; insert: string }[] = [];
  let k = +NUM.exec(line(items[0])!)![2];
  for (const j of items) {
    const digits = NUM.exec(line(j)!)![2];
    if (digits !== String(k)) out.push({ line: j, from: ind, to: ind + digits.length, insert: String(k) });
    k++;
  }
  return out;
}

const ITEM = /^(\s*)([-+]|\d+[.)]|\*(?=\s))\s/;
const isItem = (t: string) => ITEM.test(t) && !/^\*+ /.test(t);

/** Line ranges [first, last] of the list item at line I and its sibling in
 *  direction D (same indent, at most one blank line between; deeper lines belong
 *  to an item), in document order. null: I is not an item line; "end": no sibling
 *  that way. Same list bounds as `renumber`. */
export function itemSwap(line: (i: number) => string | undefined, i: number, d: -1 | 1): [[number, number], [number, number]] | null | "end" {
  const t0 = line(i);
  if (t0 === undefined || !isItem(t0)) return null;
  const ind = indent(t0);
  /** Last non-blank line of the item starting at S (deeper lines follow it). */
  const blockEnd = (s: number) => {
    let e = s;
    for (let j = s + 1; ; j++) {
      const t = line(j);
      if (t === undefined) break;
      if (!t.trim()) continue;
      if (/^\*+ /.test(t) || indent(t) <= ind) break;
      e = j;
    }
    return e;
  };
  /** First non-blank line from J going in direction D, over at most one blank; 0 if none. */
  const sibling = (j: number) => {
    for (let blanks = 0; ; j += d) {
      const t = line(j);
      if (t === undefined || /^\*+ /.test(t)) return 0;
      if (!t.trim()) { if (++blanks > 1) return 0; continue; }
      return j;
    }
  };
  if (d > 0) {
    const e = blockEnd(i);
    const s = sibling(e + 1);
    if (!s || indent(line(s)!) !== ind || !isItem(line(s)!)) return "end";
    return [[i, e], [s, blockEnd(s)]];
  }
  let s = sibling(i - 1);
  while (s && indent(line(s)!) > ind) s = sibling(s - 1);
  if (!s || indent(line(s)!) !== ind || !isItem(line(s)!)) return "end";
  return [[s, blockEnd(s)], [i, blockEnd(i)]];
}
