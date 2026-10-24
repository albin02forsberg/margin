// Tasks board (#357): one column per configured open keyword, then one Done column.

export type Column<T> = { key: string; title: string; done: boolean; items: T[] };

/** Group ITEMS into the board's columns, in keyword order; a keyword not in TODO or DONE goes to the first column. */
export function columns<T extends { keyword: string | null }>(items: T[], todo: string[], done: string[]): Column<T>[] {
  const cols: Column<T>[] = [...todo.map((k) => ({ key: k, title: k, done: false, items: [] as T[] })),
    { key: "", title: done[0] ?? "DONE", done: true, items: [] }];
  for (const it of items) {
    const k = it.keyword ?? "";
    cols[done.includes(k) ? todo.length : Math.max(0, todo.indexOf(k))].items.push(it);
  }
  return cols;
}

/** The keyword one column left (DIR -1) or right (+1) of KEYWORD, or null at the edge. */
export function moveTo(keyword: string | null, dir: number, todo: string[], done: string[]): string | null {
  const seq = [...todo, done[0] ?? "DONE"];
  const i = done.includes(keyword ?? "") ? seq.length - 1 : Math.max(0, todo.indexOf(keyword ?? ""));
  return seq[i + dir] ?? null;
}

/** How many of ITEMS (blocked flag, keyword) an op moving them to TARGETS (see bulkTargets) would finish while still blocked (#729). */
export const blockedDone = (items: { blocked?: boolean; keyword: string | null }[], targets: (string | null)[], done: string[]) =>
  items.filter((x, i) => x.blocked && targets[i] && done.includes(targets[i]!) && !done.includes(x.keyword ?? "")).length;

/** The keyword each task gets from a bulk OP over KEYWORDS (null = leave it alone). `done`: open tasks get the first
 *  DONE keyword and done ones stay, unless all are done, then all reopen to the first TODO (#654). `column`: ARG (-1/1)
 *  columns over, skipping tasks at the edge. `keyword`: ARG for every task. */
export function bulkTargets(op: string, keywords: (string | null)[], arg: string | undefined, todo: string[], done: string[]): (string | null)[] {
  const isDone = (k: string | null) => done.includes(k ?? "");
  if (op === "done") {
    const reopen = keywords.length > 0 && keywords.every(isDone);
    return keywords.map((k) => (reopen ? todo[0] ?? null : isDone(k) ? null : done[0] ?? null));
  }
  if (op === "column") return keywords.map((k) => moveTo(k, Number(arg), todo, done));
  return keywords.map(() => (op === "keyword" ? arg ?? null : null));
}
