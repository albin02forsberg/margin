/** Bulk task ops on marked tasks (#633). */
type Loc = { path: string; line: number };

/** A mark's key: the task's file and heading line. */
export const markKey = (it: Loc) => `${it.path}:${it.line}`;

/** Edit order for a batch: per file from the bottom up, so an edit that adds or removes lines
 *  (archive, a logbook) never shifts the line of a task still to come. */
export const orderForEdit = <T extends Loc>(items: T[]) =>
  [...items].sort((a, b) => (a.path < b.path ? -1 : a.path > b.path ? 1 : b.line - a.line));
