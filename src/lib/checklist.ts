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
