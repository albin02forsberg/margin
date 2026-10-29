/** Back/forward history of jump origins (links, headings, ...): two stacks of note positions. */
export type Pos = { path: string; line: number };
export type NavHistory = { back: Pos[]; fwd: Pos[] };
const CAP = 50;
const same = (a?: Pos, b?: Pos) => !!a && !!b && a.path === b.path && a.line === b.line;

export const newHistory = (): NavHistory => ({ back: [], fwd: [] });

/** Record the position a jump leaves; clears forward. Consecutive duplicates are skipped, the oldest drop past 50. */
export function push(h: NavHistory, from: Pos) {
  h.fwd.length = 0;
  if (same(h.back.at(-1), from)) return;
  h.back.push(from);
  if (h.back.length > CAP) h.back.shift();
}

/** Step back from CUR: the position to go to (CUR becomes forwardable), or undefined. */
export function back(h: NavHistory, cur: Pos): Pos | undefined {
  const p = h.back.pop();
  if (p && !same(p, cur)) h.fwd.push(cur);
  return p;
}

/** Step forward from CUR. */
export function forward(h: NavHistory, cur: Pos): Pos | undefined {
  const p = h.fwd.pop();
  if (p && !same(p, cur)) h.back.push(cur);
  return p;
}

/** Remove entries whose note fails KEEP (trashed or vanished). */
export function drop(h: NavHistory, keep: (path: string) => boolean) {
  h.back = h.back.filter((p) => keep(p.path));
  h.fwd = h.fwd.filter((p) => keep(p.path));
}
