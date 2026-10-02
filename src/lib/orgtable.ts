// Org tables as pure functions over the table's lines. Every edit re-aligns
// the table and returns where the cursor should go (row = line index, col = cell).

export type Table = { lines: string[]; row: number; col: number };

export const isTableLine = (s: string) => /^\s*\|/.test(s);
export const isSep = (s: string) => /^\s*\|-/.test(s);

/** Cells of a row, or null for a separator line. */
export function cells(s: string): string[] | null {
  if (isSep(s)) return null;
  let t = s.trim().slice(1);
  if (t.endsWith("|")) t = t.slice(0, -1);
  return t.split("|").map((c) => c.trim());
}

const width = (s: string) => [...s].length;
const NUMBER = /^[-+]?(\d[\d\s,]*)?\.?\d+([eE][-+]?\d+)?\s*[%a-zA-Z]{0,3}$/;

/** Align columns; columns that are mostly numbers are right-aligned, like org. */
export function align(lines: string[]): string[] {
  const indent = /^\s*/.exec(lines[0] ?? "")![0];
  const rows = lines.map(cells);
  const n = Math.max(1, ...rows.map((r) => r?.length ?? 0));
  const w = Array.from({ length: n }, (_, i) => Math.max(1, ...rows.map((r) => (r ? width(r[i] ?? "") : 0))));
  const right = w.map((_, i) => {
    const filled = rows.flatMap((r) => (r && r[i] ? [r[i]] : []));
    return filled.length > 0 && filled.filter((c) => NUMBER.test(c)).length / filled.length > 0.5;
  });
  const pad = (c: string, i: number) => {
    const fill = " ".repeat(w[i] - width(c));
    return right[i] ? fill + c : c + fill;
  };
  return rows.map((r) =>
    r === null
      ? `${indent}|${w.map((x) => "-".repeat(x + 2)).join("+")}|`
      : `${indent}| ${w.map((_, i) => pad(r[i] ?? "", i)).join(" | ")} |`,
  );
}

/** Index of the cell containing column COL of LINE. */
export function cellIndex(line: string, col: number): number {
  return Math.max(0, (line.slice(0, col).match(/\|/g)?.length ?? 1) - 1);
}

/** Offset in LINE where the text of cell I starts (first non-space, or just after "| "). */
export function cellOffset(line: string, i: number): number {
  let bar = -1;
  for (let k = 0; k <= i; k++) {
    const next = line.indexOf("|", bar + 1);
    if (next < 0) break;
    bar = next;
  }
  const end = line.indexOf("|", bar + 1);
  const inner = line.slice(bar + 1, end < 0 ? undefined : end);
  const lead = inner.search(/\S/);
  return bar + 1 + (lead < 0 ? Math.min(1, inner.length) : lead);
}

const ncols = (lines: string[]) => Math.max(1, ...lines.map((l) => cells(l)?.length ?? 0));
const emptyRow = (lines: string[]) => `|${" |".repeat(ncols(lines))}`;
const done = (lines: string[], row: number, col: number): Table => ({ lines: align(lines), row, col });

/** Tab / Shift-Tab: next or previous cell; Tab past the last cell adds a row. */
export function nextCell(t: Table, dir: 1 | -1): Table {
  const lines = align(t.lines);
  const n = ncols(lines);
  let { row, col } = t;
  if (isSep(lines[row])) col = dir > 0 ? -1 : n;
  col += dir;
  for (;;) {
    if (col >= n) { row++; col = 0; }
    if (col < 0) { row--; col = n - 1; }
    if (row < 0) return { lines, row: 0, col: 0 };
    if (row >= lines.length) {
      lines.push(emptyRow(lines));
      return done(lines, lines.length - 1, 0);
    }
    if (!isSep(lines[row])) return { lines, row, col };
    col = dir > 0 ? n : -1;
  }
}

/** Enter: same column in the next row, adding a row at the end or before a separator. */
export function nextRow(t: Table): Table {
  const lines = [...t.lines];
  const row = t.row + 1;
  if (row >= lines.length || isSep(lines[row])) lines.splice(row, 0, emptyRow(lines));
  return done(lines, row, t.col);
}

export function insertRow(t: Table, below: boolean): Table {
  const lines = [...t.lines];
  const at = below ? t.row + 1 : t.row;
  lines.splice(at, 0, emptyRow(lines));
  return done(lines, at, t.col);
}

export function insertSep(t: Table): Table {
  const lines = [...t.lines];
  lines.splice(t.row + 1, 0, "|-");
  return done(lines, t.row, t.col);
}

export function deleteRow(t: Table): Table | null {
  if (t.lines.length <= 1) return null;
  const lines = t.lines.filter((_, i) => i !== t.row);
  return done(lines, Math.min(t.row, lines.length - 1), t.col);
}

/** Apply F to the cell arrays of every non-separator row. */
function mapRows(lines: string[], f: (c: string[]) => string[]) {
  const n = ncols(lines);
  return lines.map((l) => {
    const c = cells(l);
    if (!c) return l;
    while (c.length < n) c.push("");
    return `| ${f(c).join(" | ")} |`;
  });
}

export function insertCol(t: Table): Table {
  return done(mapRows(t.lines, (c) => [...c.slice(0, t.col + 1), "", ...c.slice(t.col + 1)]), t.row, t.col + 1);
}

export function deleteCol(t: Table): Table | null {
  if (ncols(t.lines) <= 1) return null;
  return done(mapRows(t.lines, (c) => c.filter((_, i) => i !== t.col)), t.row, Math.max(0, t.col - 1));
}

export function moveRow(t: Table, d: 1 | -1): Table {
  const to = t.row + d;
  if (to < 0 || to >= t.lines.length) return t;
  const lines = [...t.lines];
  [lines[t.row], lines[to]] = [lines[to], lines[t.row]];
  return done(lines, to, t.col);
}

export function moveCol(t: Table, d: 1 | -1): Table {
  const to = t.col + d;
  if (to < 0 || to >= ncols(t.lines)) return t;
  return done(mapRows(t.lines, (c) => { [c[t.col], c[to]] = [c[to], c[t.col]]; return c; }), t.row, to);
}

/** Sort the rows between the separators around the cursor by the current column. */
export function sortRows(t: Table, desc = false): Table {
  let a = t.row, b = t.row;
  while (a > 0 && !isSep(t.lines[a - 1])) a--;
  while (b < t.lines.length - 1 && !isSep(t.lines[b + 1])) b++;
  const key = (l: string) => cells(l)?.[t.col] ?? "";
  const numeric = t.lines.slice(a, b + 1).every((l) => !key(l) || NUMBER.test(key(l)));
  const cmp = (x: string, y: string) =>
    numeric ? parseFloat(key(x).replace(/[\s,]/g, "")) - parseFloat(key(y).replace(/[\s,]/g, "")) || 0 : key(x).localeCompare(key(y), undefined, { numeric: true });
  const sorted = t.lines.slice(a, b + 1).sort((x, y) => (desc ? -cmp(x, y) : cmp(x, y)));
  return done([...t.lines.slice(0, a), ...sorted, ...t.lines.slice(b + 1)], a, t.col);
}

/** A new table with a header row and separator. */
export function create(cols: number, rows: number): string[] {
  const head = `|${Array.from({ length: cols }, (_, i) => ` Column ${i + 1} |`).join("")}`;
  const body = Array.from({ length: Math.max(1, rows) }, () => `|${" |".repeat(cols)}`);
  return align([head, "|-", ...body]);
}
