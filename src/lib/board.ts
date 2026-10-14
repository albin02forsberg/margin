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
