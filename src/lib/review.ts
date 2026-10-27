// Weekly review (#670): a `* Weekly review` section in today's journal entry, built from the week's tasks and time.
import { hm } from "./clock.ts";
import { estimateLine } from "./estimates.ts";

export const BEGIN = "# review-begin", END = "# review-end";

type Task = { title: string; category: string; date: string; closed?: string | null; effort?: number | null; spent?: number | null };
type Hours = { project: string; worked: number };
/** An `org_goals` row (#739). */
export type Goal = { title: string; done: number; total: number; percent: number | null; days_left: number | null; bad_target: boolean };

/** When a goal's target falls: "8d left", "3d overdue", "no target" or "bad target". */
export const goalWhen = (g: Pick<Goal, "bad_target" | "days_left">) => g.bad_target ? "bad target" : g.days_left == null ? "no target" : g.days_left < 0 ? `${-g.days_left}d overdue` : `${g.days_left}d left`;

/** ISO week of DAY (yyyy-mm-dd) as "2026-W43". */
export function isoWeek(day: string) {
  const d = new Date(`${day}T00:00:00Z`), wd = (d.getUTCDay() + 6) % 7;
  d.setUTCDate(d.getUTCDate() - wd + 3); // the week's Thursday decides the year
  const n = 1 + Math.floor((d.getTime() - Date.UTC(d.getUTCFullYear(), 0, 1)) / 864e5 / 7);
  return `${d.getUTCFullYear()}-W${String(n).padStart(2, "0")}`;
}

const list = (xs: string[], none: string) => (xs.length ? xs.map((x) => `- ${x}`) : [`- ${none}`]);
const task = (t: Task) => `${t.title}${t.category ? ` (${t.category})` : ""}`;

/** The generated part, between the BEGIN/END markers: DONE (doneLog rows, by close time; with their estimates vs clocked time), the week's HOURS
 *  per project (tc_dashboard `projects`), GOALS (section omitted without any), OVERDUE and NEXT (scheduled next week) tasks, the INBOX count and the SOMEDAY count (#833; line omitted at 0). */
export function reviewBody(done: Task[], hours: Hours[], overdue: Task[], next: Task[], inbox: number, goals: Goal[] = [], someday = 0) {
  const worked = hours.filter((h) => h.worked > 0), total = worked.reduce((a, h) => a + h.worked, 0);
  return [
    BEGIN,
    `Inbox: ${inbox} entr${inbox === 1 ? "y" : "ies"} waiting.`,
    ...(someday ? [`Someday/maybe: ${someday} parked task${someday === 1 ? "" : "s"} (Someday/maybe review).`] : []),
    `** Done (${done.length})`,
    ...list(done.map((t) => `${t.date} ${task(t)}`), "nothing closed"),
    ...[estimateLine(done)].filter((l) => l !== null),
    `** Hours per project (${hm(total)})`,
    ...list(worked.map((h) => `${h.project}: ${hm(h.worked)}`), "no time tracked"),
    ...(goals.length ? [`** Goals (${goals.length})`, ...goals.map((g) => `- ${g.title}: ${g.total ? `${g.done}/${g.total} done (${g.percent}%)` : "no tasks"}, ${goalWhen(g)}`)] : []),
    `** Overdue (${overdue.length})`,
    ...list(overdue.map((t) => `${task(t)}, since ${t.date}`), "nothing overdue"),
    `** Scheduled next week (${next.length})`,
    ...list(next.map((t) => `${t.date} ${task(t)}`), "nothing scheduled"),
    END,
  ].join("\n");
}

/** A new section for WEEK: heading, BODY and the headings to fill in (outside the markers, so a re-run keeps them). */
export const reviewEntry = (week: string, body: string) => [`* Weekly review ${week}`, body, "** Went well", "** To change"].join("\n");

/** TEXT with its BEGIN..END span replaced by BODY, or null if it has no such span (then file a reviewEntry). */
export function putReview(text: string, body: string): string | null {
  const eol = text.includes("\r\n") ? "\r\n" : "\n", lines = text.split(/\r?\n/);
  const b = lines.findIndex((l) => l.trim() === BEGIN), e = b < 0 ? -1 : lines.findIndex((l, i) => i > b && l.trim() === END);
  if (e < 0) return null;
  return [...lines.slice(0, b), ...body.split("\n"), ...lines.slice(e + 1)].join(eol);
}
