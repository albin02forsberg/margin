// Weekly review (#670): a `* Weekly review` section in today's journal entry, built from the week's tasks and time.
import { hm } from "./clock.ts";

export const BEGIN = "# review-begin", END = "# review-end";

type Task = { title: string; category: string; date: string; closed?: string | null };
type Hours = { project: string; worked: number };

/** ISO week of DAY (yyyy-mm-dd) as "2026-W43". */
export function isoWeek(day: string) {
  const d = new Date(`${day}T00:00:00Z`), wd = (d.getUTCDay() + 6) % 7;
  d.setUTCDate(d.getUTCDate() - wd + 3); // the week's Thursday decides the year
  const n = 1 + Math.floor((d.getTime() - Date.UTC(d.getUTCFullYear(), 0, 1)) / 864e5 / 7);
  return `${d.getUTCFullYear()}-W${String(n).padStart(2, "0")}`;
}

const list = (xs: string[], none: string) => (xs.length ? xs.map((x) => `- ${x}`) : [`- ${none}`]);
const task = (t: Task) => `${t.title}${t.category ? ` (${t.category})` : ""}`;

/** The generated part, between the BEGIN/END markers: DONE (doneLog rows, by close time), the week's HOURS
 *  per project (tc_dashboard `projects`), OVERDUE and NEXT (scheduled next week) tasks and the INBOX count. */
export function reviewBody(done: Task[], hours: Hours[], overdue: Task[], next: Task[], inbox: number) {
  const worked = hours.filter((h) => h.worked > 0), total = worked.reduce((a, h) => a + h.worked, 0);
  return [
    BEGIN,
    `Inbox: ${inbox} entr${inbox === 1 ? "y" : "ies"} waiting.`,
    `** Done (${done.length})`,
    ...list(done.map((t) => `${t.date} ${task(t)}`), "nothing closed"),
    `** Hours per project (${hm(total)})`,
    ...list(worked.map((h) => `${h.project}: ${hm(h.worked)}`), "no time tracked"),
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
