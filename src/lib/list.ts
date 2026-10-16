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
