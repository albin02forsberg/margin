/** Checklist -> task conversion for `Space x k`. */
const BOX = /^(\s*)(?:[-+]|\d+[.)])\s+\[([ xX-])\]\s*(.*)$/;
const HEAD = /^(\*+)\s/;

/** Level of the nearest heading above `lines[i]`, plus one (1 when none). */
export function levelAt(lines: string[], i: number): number {
  for (let j = i - 1; j >= 0; j--) { const m = HEAD.exec(lines[j]); if (m) return m[1].length + 1; }
  return 1;
}

/** Turn the top-level checkbox items of `lines` into `level`-star headings
 *  (`[X]` -> done, else todo); nested items and other lines pass through. */
export function checklistToTasks(lines: string[], level: number, todo: string, done: string): string[] {
  const top = lines.map((l) => BOX.exec(l)).find((m) => m)?.[1].length;
  return lines.map((l) => {
    const m = BOX.exec(l);
    return m && m[1].length === top ? `${"*".repeat(level)} ${m[2] === "x" || m[2] === "X" ? done : todo} ${m[3]}` : l;
  });
}

/** Line range [a, b] (0-based) of the contiguous list around line `i`:
 *  non-blank, non-heading lines. */
export function listRange(lines: string[], i: number): [number, number] {
  const ok = (l: string) => l.trim() !== "" && !HEAD.test(l);
  let a = i, b = i;
  while (a > 0 && ok(lines[a - 1])) a--;
  while (b < lines.length - 1 && ok(lines[b + 1])) b++;
  return [a, b];
}

const PLAN = /^\s*(?:SCHEDULED|DEADLINE|CLOSED):/;

/** Inverse of `checklistToTasks`: sibling headings (all one level, no child
 *  headings) -> checklist items. Done keyword -> `- [X]`, other keyword ->
 *  `- [ ]`, none -> `- `. Body lines indent 2; SCHEDULED/DEADLINE/CLOSED and an
 *  `:ID:`-only drawer are dropped. Null when anything else would be lost. */
export function tasksToChecklist(lines: string[], todo: string[], done: string[]): string[] | null {
  const lvl = HEAD.exec(lines[0] ?? "")?.[1].length;
  if (!lvl) return null;
  const out: string[] = [];
  for (let i = 0; i < lines.length; i++) {
    const l = lines[i], h = HEAD.exec(l);
    if (h) {
      if (h[1].length !== lvl) return null;
      const t = l.slice(h[0].length).trim(), w = t.split(/\s+/)[0], rest = t.slice(w.length).trim();
      out.push((done.includes(w) ? `- [X] ${rest}` : todo.includes(w) ? `- [ ] ${rest}` : `- ${t}`).trimEnd());
    } else if (PLAN.test(l)) continue;
    else if (l.trim() === ":PROPERTIES:") {
      let j = i + 1;
      while (j < lines.length && /^\s*:ID:/.test(lines[j])) j++;
      if (lines[j]?.trim() !== ":END:") return null;
      i = j;
    } else if (/^\s*:[A-Za-z_]+:\s*$/.test(l)) return null;
    else out.push(l.trim() === "" ? l : `  ${l.replace(/^\s+/, "")}`);
  }
  return out;
}
