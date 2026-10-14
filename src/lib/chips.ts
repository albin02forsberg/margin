// Agenda chips strip (#357): a day's timed items as "time title" chips, plus the running clock.

export type Chip = { time: string; title: string; i: number; state: "" | "done" | "live" };

/** Chips for DAY: ITEMS (indexed) with a time on that day, by start time; `done` when done or already past (NOW is "HH:MM" on TODAY).
 *  CLOCK (the running clock, `since` an ISO date-time) adds a `live` chip on TODAY; its `i` is -1. */
export function chips<T extends { date: string; time: string | null; title: string }>(
  items: T[], day: string, today: string, now: string, isDone: (it: T) => boolean, clock?: { project: string; since: string } | null,
): Chip[] {
  const start = (t: string) => t.split(/[-–]/)[0].padStart(5, "0");
  const xs: Chip[] = items.flatMap((it, i) => it.date === day && it.time
    ? [{ time: start(it.time), title: it.title, i, state: isDone(it) || (day === today && start(it.time) < now) || day < today ? "done" as const : "" as const }] : []);
  if (clock && day === today) xs.push({ time: clock.since.slice(11, 16), title: `${clock.project || "No project"} · tracking`, i: -1, state: "live" });
  return xs.sort((a, b) => a.time.localeCompare(b.time));
}
