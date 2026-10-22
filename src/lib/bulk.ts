/** Bulk task ops on marked tasks (#633). */
type Loc = { path: string; line: number };

/** A mark's key: the task's file and heading line. */
export const markKey = (it: Loc) => `${it.path}:${it.line}`;

/** Edit order for a batch: per file from the bottom up, so an edit that adds or removes lines
 *  (archive, a logbook) never shifts the line of a task still to come. */
export const orderForEdit = <T extends Loc>(items: T[]) =>
  [...items].sort((a, b) => (a.path < b.path ? -1 : a.path > b.path ? 1 : b.line - a.line));

/** A bulk tag change (#638): add TAG to every marked task, or remove it from every one. */
export type TagChange = { add: boolean; tag: string };
export const tagChange = (tags: string[], c: TagChange) =>
  c.add ? (tags.includes(c.tag) ? tags : [...tags, c.tag]) : tags.filter((t) => t !== c.tag);

/** The LINES (0-based headings of TEXT) not inside another listed heading's subtree: a bulk move
 *  takes those with their parent instead of moving them out first. */
export function outermost(text: string, lines: number[]) {
  const ls = text.split("\n"), lvl = (i: number) => /^(\*+) /.exec(ls[i] ?? "")?.[1].length ?? 0;
  const set = new Set(lines);
  return lines.filter((l) => {
    let m = lvl(l) || Infinity;
    for (let i = l - 1; i >= 0 && m > 1; i--) {
      const v = lvl(i);
      if (v && v < m) { if (set.has(i)) return false; m = v; }
    }
    return true;
  });
}

/** A marked task: its location plus the title it had when marked. */
export type Mark = Loc & { title: string };

/** Re-point MARKS at ITEMS after a refresh (#649): a mark stays if its `path:line` still has the same
 *  title, else follows the same-titled task in that file (nearest line, each task used once), else is
 *  dropped. A renamed heading loses its mark. Matches distinct keys, since one task can be several rows. */
export function reresolve(marks: Map<string, Mark>, items: (Loc & { title: string })[]) {
  const cur = new Map(items.map((i) => [markKey(i), i]));
  const out = new Map<string, Mark>(), moved: Mark[] = [];
  for (const m of marks.values()) {
    const c = cur.get(markKey(m));
    if (c && c.title === m.title) out.set(markKey(m), m); else moved.push(m);
  }
  for (const m of moved) {
    let best: Loc | undefined;
    for (const c of cur.values())
      if (c.path === m.path && c.title === m.title && !out.has(markKey(c)) && (!best || Math.abs(c.line - m.line) < Math.abs(best.line - m.line))) best = c;
    if (best) out.set(markKey(best), { path: m.path, line: best.line, title: m.title });
  }
  return out;
}
