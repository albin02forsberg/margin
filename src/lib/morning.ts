// Morning plan (#846): the twin of shutdown.ts — walk overdue, due-soon and undated A/B tasks and pick today's.
import { hm } from "./clock.ts";

export type MorningAction = "today" | "tomorrow" | "date" | "skip";

/** One key per step: t today, m tomorrow, s pick a date, n / space leave it (Esc stops). */
export const morningKeys: Record<string, MorningAction> = { t: "today", m: "tomorrow", s: "date", n: "skip", " ": "skip" };

type Row = { kind: string; label: string; date: string; keyword: string | null; habit?: unknown; blocked?: boolean; path: string; line: number };
export type Why = "overdue" | "soon" | "undated";

/** Candidates to plan, once per heading: overdue first (an overdue deadline, or "Scheduled Nd ago"), then scheduled / due
 *  in the AGENDA window, then UNDATED (open A/B tasks without dates), each group in input order. Leaves out tasks already
 *  scheduled for TODAY, closed or keyword-less ones (TODO: open keywords), habits, blocked tasks and plain timestamps. */
export function planCandidates<T extends Row>(agenda: T[], undated: T[], today: string, todo: string[]): (T & { why: Why })[] {
  const key = (it: Row) => `${it.path}\n${it.line}`;
  const onToday = new Set(agenda.filter((it) => it.kind === "scheduled" && it.date === today && !it.label).map(key));
  const out = new Map<string, T & { why: Why }>(), rank: Why[] = ["overdue", "soon", "undated"];
  const add = (it: T, why: Why) => {
    if (!it.keyword || !todo.includes(it.keyword) || it.habit || it.blocked || onToday.has(key(it))) return;
    const seen = out.get(key(it));
    if (!seen || rank.indexOf(why) < rank.indexOf(seen.why)) out.set(key(it), { ...it, why });
  };
  for (const it of agenda) {
    if (it.kind === "overdue" || (it.kind === "scheduled" && it.label)) add(it, "overdue");
    else if (it.kind === "scheduled" || it.kind === "deadline") add(it, "soon");
  }
  for (const it of undated) add(it, "undated");
  return [...out.values()].sort((a, b) => rank.indexOf(a.why) - rank.indexOf(b.why));
}

/** "Planned 2 for today · 3h 30m estimated of 8h 00m" (EFFORT: minutes of today's open tasks; EXPECTED: hours, 0 = no target). */
export const planSummary = (planned: number, effort: number, expected: number) =>
  `Planned ${planned} for today · ${hm(effort / 60)} estimated${expected > 0 ? ` of ${hm(expected)}` : ""}`;
