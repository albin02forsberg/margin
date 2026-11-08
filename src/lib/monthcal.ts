/** Month calendar helpers (#1039): a grid of ISO days in full weeks and the agenda's items bucketed per day. */
import { planned } from "./effort.ts";
import { bdayIcon } from "./people.ts";
import { intoWeek } from "./week.ts";

const utc = (d: string) => Date.UTC(+d.slice(0, 4), +d.slice(5, 7) - 1, +d.slice(8, 10));
const isoOf = (t: number) => new Date(t).toISOString().slice(0, 10);

/** ISO day D moved by N days (UTC math, so DST never shifts it). */
export const addDays = (d: string, n: number) => isoOf(utc(d) + n * 86_400_000);

/** The full weeks (starting on WS, config `week_start`; Monday by default) covering MONTH (`YYYY-MM`): 28 to 42 ISO days. */
export function monthGrid(month: string, ws?: string) {
  const first = `${month}-01`, lead = intoWeek(new Date(utc(first)).getUTCDay(), ws);
  const len = new Date(Date.UTC(+month.slice(0, 4), +month.slice(5, 7), 0)).getUTCDate();
  const start = addDays(first, -lead);
  return Array.from({ length: Math.ceil((lead + len) / 7) * 7 }, (_, i) => addDays(start, i));
}

export type Day = { scheduled: number; deadline: number; effort: number; titles: string[]; birthdays: string[] };
type Item = { date: string; kind: string; title: string; path: string; line: number; effort?: number | null; label?: string };

/** ITEMS (from `agenda`) per day: open scheduled tasks and plain timestamps, deadlines and their planned effort
 *  (a heading counted once a day), and birthdays / anniversaries ("🎂 Anna", not counted as tasks, #1187).
 *  Done tasks, the today-only `overdue` / `warning` rows and birthday heads-ups ("Turns 42 in 3d") are left out. */
export function bucket<T extends Item>(items: T[], done: (it: T) => boolean) {
  const by: Record<string, T[]> = {}, bdays: Record<string, string[]> = {};
  for (const it of items) {
    if (it.kind === "birthday") { if (!/ in \d+d$/.test(it.label ?? "")) (bdays[it.date] ??= []).push(`${bdayIcon(it.label ?? "")} ${it.title}`); }
    else if (!done(it) && (it.kind === "scheduled" || it.kind === "timestamp" || it.kind === "deadline")) (by[it.date] ??= []).push(it);
  }
  const out: Record<string, Day> = {};
  for (const d of new Set([...Object.keys(by), ...Object.keys(bdays)])) {
    const xs = by[d] ?? [];
    out[d] = {
      scheduled: xs.filter((x) => x.kind !== "deadline").length,
      deadline: xs.filter((x) => x.kind === "deadline").length,
      effort: planned(xs, () => false),
      titles: [...new Set(xs.map((x) => x.title))],
      birthdays: bdays[d] ?? [],
    };
  }
  return out;
}
