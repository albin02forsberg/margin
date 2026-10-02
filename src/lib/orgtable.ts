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

// ---------------------------------------------------------------- formulas (#+TBLFM)
//
// Supported: column formulas `$4=$2*$3`, field formulas `@>$4=vsum(@2..@-1)`,
// references `$N @N @N$N`, relative `@-1 $+1`, `@< @> $< $>`, separators
// `@I @II` (in ranges), ranges `@2$1..@4$3`, + - * / ^ and parentheses,
// functions vsum vmean vmin vmax vcount vmedian abs round floor ceil sqrt exp ln,
// and a trailing format `;%.2f` / `;%d`. Empty fields are dropped from ranges.
// ponytail: no Calc units/dates/Lisp formulas or remote references; add when a table needs them.

type Val = number | number[];
type Grid = { rows: (string[] | null)[]; data: number[]; hlines: number[]; n: number };

function grid(lines: string[]): Grid {
  const rows = lines.map(cells);
  const n = ncols(lines);
  rows.forEach((r) => { while (r && r.length < n) r.push(""); });
  const data = rows.flatMap((r, i) => (r ? [i] : []));
  const hlines = rows.flatMap((r, i) => (r ? [] : [i]));
  return { rows, data, hlines, n };
}

const toNum = (s: string) => {
  const x = parseFloat(s.replace(/[\s,]/g, ""));
  return isNaN(x) ? 0 : x;
};

/** Data-row index for a row reference. Separator refs (@I) resolve to the first
 *  row after the separator, or with END to the last row before it. */
function rowRef(tok: string | undefined, g: Grid, cur: number, end = false): number {
  if (!tok) return cur;
  if (tok === "<") return 0;
  if (tok === ">") return g.data.length - 1;
  if (/^I+$/.test(tok)) {
    const h = g.hlines[tok.length - 1];
    if (h == null) throw new Error(`no separator @${tok}`);
    const after = g.data.findIndex((d) => d > h);
    return end ? (after < 0 ? g.data.length : after) - 1 : after;
  }
  if (/^[+-]/.test(tok)) return cur + parseInt(tok);
  return parseInt(tok) - 1;
}

function colRef(tok: string | undefined, g: Grid, cur: number): number {
  if (!tok) return cur;
  if (tok === "<") return 0;
  if (tok === ">") return g.n - 1;
  if (/^[+-]/.test(tok)) return cur + parseInt(tok);
  return parseInt(tok) - 1;
}

const FUNCS: Record<string, (a: Val[]) => number> = {
  vsum: (a) => flat(a).reduce((x, y) => x + y, 0),
  vmean: (a) => { const f = flat(a); return f.length ? f.reduce((x, y) => x + y, 0) / f.length : 0; },
  vmin: (a) => Math.min(...flat(a)),
  vmax: (a) => Math.max(...flat(a)),
  vcount: (a) => flat(a).length,
  vmedian: (a) => { const f = flat(a).sort((x, y) => x - y); const m = f.length >> 1; return f.length % 2 ? f[m] : (f[m - 1] + f[m]) / 2; },
  abs: (a) => Math.abs(scalar(a[0])),
  round: (a) => { const p = 10 ** scalar(a[1] ?? 0); return Math.round(scalar(a[0]) * p) / p; },
  floor: (a) => Math.floor(scalar(a[0])),
  ceil: (a) => Math.ceil(scalar(a[0])),
  sqrt: (a) => Math.sqrt(scalar(a[0])),
  exp: (a) => Math.exp(scalar(a[0])),
  ln: (a) => Math.log(scalar(a[0])),
};
const flat = (a: Val[]) => a.flatMap((v) => v);
function scalar(v: Val): number {
  if (typeof v === "number") return v;
  if (v.length === 1) return v[0];
  throw new Error("a range needs a function like vsum()");
}

/** Evaluate EXPR for the field at data row ROW, column COL. */
export function evaluate(expr: string, g: Grid, row: number, col: number): number {
  const s = expr.replace(/\s+/g, "");
  let i = 0;
  const REF = /@(<|>|[+-]?\d+|I+)?(?:\$(<|>|[+-]?\d+))?|\$(<|>|[+-]?\d+)/y;
  const eat = (re: RegExp) => { re.lastIndex = i; const m = re.exec(s); if (m) i = re.lastIndex; return m; };
  const ref = (end: boolean) => {
    const m = eat(REF);
    if (!m || m[0] === "@") return null;
    return { r: m[3] ? row : rowRef(m[1], g, row, end), c: colRef(m[2] ?? m[3], g, col), rowOnly: !!m[1] && !m[2] };
  };
  const field = (r: number, c: number) => {
    const d = g.data[r];
    if (d == null || c < 0 || c >= g.n) throw new Error("reference outside the table");
    return g.rows[d]![c];
  };
  const primary = (): Val => {
    const n = eat(/\d+(\.\d+)?([eE][-+]?\d+)?|\.\d+/y);
    if (n) return parseFloat(n[0]);
    const f = eat(/([a-z]\w*)\(/y);
    if (f) {
      const args: Val[] = [];
      if (s[i] !== ")") for (;;) { args.push(sum()); if (s[i] !== ",") break; i++; }
      if (s[i++] !== ")") throw new Error("missing )");
      const fn = FUNCS[f[1]];
      if (!fn) throw new Error(`unknown function ${f[1]}`);
      return fn(args);
    }
    if (s[i] === "(") {
      i++;
      const v = sum();
      if (s[i++] !== ")") throw new Error("missing )");
      return v;
    }
    const a = ref(false);
    if (!a) throw new Error(`can't read “${s.slice(i)}”`);
    if (s.startsWith("..", i)) {
      i += 2;
      const b = ref(true);
      if (!b) throw new Error("bad range");
      // @2..@-1 is a range down the current column.
      const [c1, c2] = a.rowOnly && b.rowOnly ? [col, col] : [a.c, b.c];
      const out: number[] = [];
      for (let r = Math.min(a.r, b.r); r <= Math.max(a.r, b.r); r++)
        for (let c = Math.min(c1, c2); c <= Math.max(c1, c2); c++) { const v = field(r, c); if (v !== "") out.push(toNum(v)); }
      return out;
    }
    return toNum(field(a.r, a.rowOnly ? col : a.c));
  };
  const power = (): Val => {
    const b = primary();
    if (s[i] !== "^") return b;
    i++;
    return scalar(b) ** scalar(unary());
  };
  const unary = (): Val => (s[i] === "-" ? (i++, -scalar(unary())) : s[i] === "+" ? (i++, unary()) : power());
  const product = (): Val => {
    let v = unary();
    while (s[i] === "*" || s[i] === "/") { const op = s[i++]; const r = scalar(unary()); v = op === "*" ? scalar(v) * r : scalar(v) / r; }
    return v;
  };
  const sum = (): Val => {
    let v = product();
    while (s[i] === "+" || s[i] === "-") { const op = s[i++]; const r = scalar(product()); v = op === "+" ? scalar(v) + r : scalar(v) - r; }
    return v;
  };
  const v = scalar(sum());
  if (i < s.length) throw new Error(`unexpected “${s.slice(i)}”`);
  return v;
}

function format(x: number, fmt?: string): string {
  if (!isFinite(x)) return "#ERROR";
  const f = fmt && /^%\.?(\d*)([fd])$/.exec(fmt.trim());
  if (f) return f[2] === "d" ? String(Math.round(x)) : x.toFixed(f[1] ? +f[1] : 6);
  return Number.isInteger(x) ? String(x) : String(Number(x.toPrecision(10)));
}

export const parseTblfm = (s: string) =>
  s.split("::").map((f) => f.trim()).filter((f) => f.includes("=")).map((f) => {
    const eq = f.indexOf("=");
    return { lhs: f.slice(0, eq).trim(), rhs: f.slice(eq + 1).trim() };
  });

/** Recalculate the table: column formulas first, then field formulas (twice, so
 *  formulas that depend on other formulas settle). Bad formulas write #ERROR. */
export function applyFormulas(lines: string[], tblfm: string): string[] {
  const g = grid(lines);
  const indent = /^\s*/.exec(lines[0] ?? "")![0];
  const fs = parseTblfm(tblfm);
  const body = g.hlines.length ? g.data.map((_, k) => k).filter((k) => g.data[k] > g.hlines[0]) : g.data.map((_, k) => k);
  const set = (r: number, c: number, rhs: string) => {
    const [expr, fmt] = rhs.split(";");
    let out: string;
    try { out = format(evaluate(expr, g, r, c), fmt); } catch { out = "#ERROR"; }
    g.rows[g.data[r]]![c] = out;
  };
  for (let pass = 0; pass < 2; pass++) {
    for (const { lhs, rhs } of fs) {
      const col = /^\$(<|>|\d+)$/.exec(lhs);
      if (col) {
        const c = colRef(col[1], g, 0);
        // Skip rows that are otherwise empty, e.g. a row just added with Tab.
        for (const r of body) if (g.rows[g.data[r]]!.some((v, k) => k !== c && v !== "")) set(r, c, rhs);
        continue;
      }
      const fld = /^@(<|>|\d+|I+)\$(<|>|\d+)$/.exec(lhs);
      if (fld) set(rowRef(fld[1], g, 0), colRef(fld[2], g, 0), rhs);
    }
  }
  return g.rows.map((r, i) => (r ? `${indent}| ${r.join(" | ")} |` : lines[i]));
}

/** Cells typed as `=expr` (column formula) or `:=expr` (field formula) move into the formula list. */
export function extractCellFormulas(lines: string[]): { lines: string[]; formulas: string[] } {
  const formulas: string[] = [];
  let k = 0;
  const out = lines.map((l) => {
    const r = cells(l);
    if (!r) return l;
    k++;
    let changed = false;
    r.forEach((cell, c) => {
      const m = /^(:?)=(.+)$/.exec(cell);
      if (!m) return;
      formulas.push(m[1] ? `@${k}$${c + 1}=${m[2].trim()}` : `$${c + 1}=${m[2].trim()}`);
      r[c] = "";
      changed = true;
    });
    return changed ? `${/^\s*/.exec(l)![0]}| ${r.join(" | ")} |` : l;
  });
  return { lines: out, formulas };
}

/** Add formulas to a TBLFM string, replacing ones with the same target. */
export function mergeTblfm(existing: string, add: string[]): string {
  const fs = parseTblfm(existing);
  for (const f of parseTblfm(add.join("::"))) {
    const i = fs.findIndex((x) => x.lhs === f.lhs);
    if (i >= 0) fs[i] = f;
    else fs.push(f);
  }
  return fs.map((f) => `${f.lhs}=${f.rhs}`).join("::");
}
