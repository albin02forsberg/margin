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
