// Evening shutdown (#757): walk today's unfinished tasks one at a time, then sum up the day.
import { hm } from "./clock.ts";

export type ShutdownAction = "done" | "tomorrow" | "week" | "date" | "skip";

/** One key per step: x done, t tomorrow, w next week, s pick a date, n / space leave it. */
export const shutdownKeys: Record<string, ShutdownAction> = { x: "done", t: "tomorrow", w: "week", s: "date", n: "skip", " ": "skip" };

type Row = { kind: string; keyword: string | null; habit?: unknown; path: string; line: number };

/** Today's open scheduled / due / overdue rows of one day's agenda ITEMS, one per heading (TODO: open keywords), with the
 *  planning line a move changes: SCHEDULED when the task shows as scheduled, else DEADLINE. Habits are left out. */
export function shutdownTasks<T extends Row>(items: T[], todo: string[]): (T & { plan: "SCHEDULED" | "DEADLINE" })[] {
  const out = new Map<string, T & { plan: "SCHEDULED" | "DEADLINE" }>();
  for (const it of items) {
    if (!/^(scheduled|deadline|overdue)$/.test(it.kind) || !it.keyword || !todo.includes(it.keyword) || it.habit) continue;
    const k = `${it.path}\n${it.line}`, plan = it.kind === "scheduled" ? "SCHEDULED" : "DEADLINE";
    const seen = out.get(k);
    if (!seen) out.set(k, { ...it, plan });
    else if (plan === "SCHEDULED") seen.plan = plan;
  }
  return [...out.values()];
}

/** "Done 2, moved 1 · 6h 30m clocked of 8h 00m" (WORKED: today's hours and target, see clock.ts `worked`). */
export const summarise = (done: number, moved: number, w: { hours: number; expected: number }) =>
  `Done ${done}, moved ${moved} · ${hm(w.hours)} clocked${w.expected > 0 ? ` of ${hm(w.expected)}` : ""}`;
