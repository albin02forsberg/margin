const lv = (l: string) => /^(\*+)\s/.exec(l)?.[1].length ?? 0;

/** True when turning the sibling headings `all[a..b]` into body items would change the outline:
 *  the nearest heading above must be their parent, and the next heading below must not be deeper
 *  (it would be reparented under the previous sibling or the items). */
export function reparents(all: string[], a: number, b: number): boolean {
  const L = lv(all[a]);
  for (let i = a - 1; i >= 0; i--) if (lv(all[i])) { if (lv(all[i]) >= L) return true; break; }
  const n = all.findIndex((l, i) => i > b && lv(l));
  return n >= 0 && lv(all[n]) > L;
}
