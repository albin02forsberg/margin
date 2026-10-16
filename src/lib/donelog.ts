// Today's Done log (#424): tasks closed on each day shown, from their CLOSED: stamps (like org agenda log mode).

type It = { date: string; path: string; line: number };
const at = (i: It) => `${i.path}\n${i.line}\n${i.date}`;

/** CLOSED (done tasks) as log rows on their close day (kind "closed", time "HH:MM"), by close time. */
export function doneLog<T extends It & { time: string | null; label: string; kind: string; closed?: string | null }>(closed: T[]): T[] {
  return closed
    .flatMap((c) => (c.closed ? [{ ...c, kind: "closed", label: "", date: c.closed.slice(0, 10), time: c.closed.slice(11, 16) }] : []))
    .sort((a, b) => a.closed!.localeCompare(b.closed!));
}

/** ITEMS (by date) and LOG together, each day's log rows after that day's items; a task in the log
 *  leaves that day's items, so one done today shows once, under Done. */
export function withLog<T extends It>(items: T[], log: T[]) {
  const logged = new Set(log.map(at));
  return [...items.filter((i) => !logged.has(at(i))), ...log].sort((a, b) => a.date.localeCompare(b.date));
}
