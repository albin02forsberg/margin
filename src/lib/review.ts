// Weekly review (#670): a `* Weekly review` section in today's journal entry, built from the week's tasks and time; `* Monthly review` (#1220) and `* Yearly review` (#1221) likewise.
import { hm } from "./clock.ts";
import { estimateLine } from "./estimates.ts";
import { isPushed } from "./pushed.ts";
import { money } from "./ledger.ts";
import { weekStartOf } from "./week.ts";

export const BEGIN = "# review-begin", END = "# review-end";
export type ReviewKind = "week" | "month" | "year";
/** Heading word per KIND. */
export const reviewName = { week: "Weekly", month: "Monthly", year: "Yearly" } as const;
/** KIND's begin/end markers: the week keeps the plain ones (old blocks stay valid), the others get a ` month` / ` year` suffix (#1220). */
export const markers = (kind: ReviewKind = "week") => (kind === "week" ? [BEGIN, END] : [`${BEGIN} ${kind}`, `${END} ${kind}`]);

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
/** "Pushed: A (5x), B (3x)": the 3 open tasks rescheduled most often (lifetime count, not this week's), most first; null if none. */
export function pushedLine(open: (Task & { pushed?: number })[]) {
  const top = open.filter((t) => isPushed(t.pushed)).sort((a, b) => b.pushed! - a.pushed!).slice(0, 3);
  return top.length ? `Pushed: ${top.map((t) => `${t.title.length > 40 ? `${t.title.slice(0, 39)}…` : t.title} (${t.pushed}x)`).join(", ")}` : null;
}
/** `ledger_week` (#1197): per currency, cents (SPENT / LAST expenses as positive sums), then budgets this month went over. */
export type LedgerWeek = [{ currency: string; spent: number; last: number; income: number; top: [string, number] | null }[], { category: string; currency: string; budget: number; used: number }[]];

/** "Ledger: spent 412.00 EUR (last week 530.00 EUR), income 0.00 EUR; top: #food 180.00 EUR" per currency, then one "Over budget this month" line. */
export function ledgerLines([weeks, over]: LedgerWeek, period = "week") {
  const m = (n: number, cur: string) => money(n, cur, false);
  return [
    ...weeks.map((w) => `Ledger: spent ${m(w.spent, w.currency)} (last ${period} ${m(w.last, w.currency)}), income ${m(w.income, w.currency)}${w.top ? `; top: #${w.top[0]} ${m(w.top[1], w.currency)}` : ""}`),
    ...(over.length ? [`Over budget this month: ${over.map((b) => `#${b.category} ${m(b.used, b.currency).split(" ")[0]}/${m(b.budget, b.currency)}`).join(", ")}`] : []),
  ];
}
const task = (t: Task) => `${t.title}${t.category ? ` (${t.category})` : ""}`;

/** The generated part, between the BEGIN/END markers: DONE (doneLog rows, by close time; with their estimates vs clocked time), the week's HOURS
 *  per project (tc_dashboard `projects`) with the LEDGER lines (#1197, none without entries), GOALS (section omitted without any), OVERDUE and NEXT (scheduled next week) tasks, the INBOX count the SOMEDAY count (#833; line omitted at 0) and a PUSHED line (#1062) after the estimates. */
export function reviewBody(done: Task[], hours: Hours[], overdue: Task[], next: Task[], inbox: number, goals: Goal[] = [], someday = 0, pushed: (Task & { pushed?: number })[] = [], ledger: LedgerWeek = [[], []]) {
  const worked = hours.filter((h) => h.worked > 0), total = worked.reduce((a, h) => a + h.worked, 0);
  return [
    BEGIN,
    `Inbox: ${inbox} entr${inbox === 1 ? "y" : "ies"} waiting.`,
    ...(someday ? [`Someday/maybe: ${someday} parked task${someday === 1 ? "" : "s"} (Someday/maybe review).`] : []),
    `** Done (${done.length})`,
    ...list(done.map((t) => `${t.date} ${task(t)}`), "nothing closed"),
    ...[estimateLine(done)].filter((l) => l !== null),
    ...[pushedLine(pushed)].filter((l) => l !== null),
    `** Hours per project (${hm(total)})`,
    ...list(worked.map((h) => `${h.project}: ${hm(h.worked)}`), "no time tracked"),
    ...ledgerLines(ledger),
    ...(goals.length ? [`** Goals (${goals.length})`, ...goals.map((g) => `- ${g.title}: ${g.total ? `${g.done}/${g.total} done (${g.percent}%)` : "no tasks"}, ${goalWhen(g)}`)] : []),
    `** Overdue (${overdue.length})`,
    ...list(overdue.map((t) => `${task(t)}, since ${t.date}`), "nothing overdue"),
    `** Scheduled next week (${next.length})`,
    ...list(next.map((t) => `${t.date} ${task(t)}`), "nothing scheduled"),
    END,
  ].join("\n");
}

/** A new section for WEEK: heading, BODY and the headings to fill in (outside the markers, so a re-run keeps them). */
/** Heading label for a review of the 7-day span from START: the ISO week holding most of it (start + 3 days), so Sunday/Saturday starts aren't labelled by the week they end in (#1100). */
export const reviewWeekLabel = (start: Date) => {
  const d = new Date(start.getFullYear(), start.getMonth(), start.getDate() + 3);
  return isoWeek(`${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`);
};

export const reviewEntry = (label: string, body: string, kind: ReviewKind = "week") =>
  [`* ${reviewName[kind]} review ${label}`, body, "** Went well", "** To change"].join("\n");

const ymd = (d: Date) => `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
/** The KIND of period holding DATE (weeks from WEEKSTART): FROM..TO (YYYY-MM-DD, TO clipped to TODAY while it runs) and its heading label (#1220, #1221). */
export function reviewRange(kind: ReviewKind, date: Date, weekStart?: string, today = date) {
  const first = kind === "week" ? weekStartOf(date, weekStart) : new Date(date.getFullYear(), kind === "month" ? date.getMonth() : 0, 1);
  const last = kind === "week" ? new Date(first.getFullYear(), first.getMonth(), first.getDate() + 6) : new Date(first.getFullYear(), kind === "month" ? first.getMonth() + 1 : 12, 0);
  return { from: ymd(first), to: ymd(last < today ? last : today), label: kind === "week" ? reviewWeekLabel(first) : ymd(first).slice(0, kind === "month" ? 7 : 4) };
}

/** `tc_project_hours` (#1220): worked hours per project over a range, their total and the hours expected (less leave). */
export type ProjectHours = { projects: Hours[]; total: number; expected: number };

/** The monthly review's generated part (#1220): DONE count with its top 5 categories, HOURS per project against expected,
 *  the LEDGER lines (none without entries) and GOALS (omitted without any). */
export const monthBody = (done: Task[], hours: ProjectHours, goals: Goal[] = [], ledger: LedgerWeek = [[], []]) => periodBody("month", done, hours, goals, ledgerLines(ledger, "month"));

/** monthBody's layout for KIND, with EXTRA lines after the hours and ledger (#1221). */
function periodBody(kind: ReviewKind, done: Task[], hours: ProjectHours, goals: Goal[], ledger: string[], extra: string[] = []) {
  const [begin, end] = markers(kind), by = new Map<string, number>();
  for (const t of done) by.set(t.category || "no category", (by.get(t.category || "no category") ?? 0) + 1);
  return [
    begin,
    `** Done (${done.length})`,
    ...list([...by].sort((a, b) => b[1] - a[1]).slice(0, 5).map(([c, n]) => `${c}: ${n}`), "nothing closed"),
    ...[estimateLine(done)].filter((l) => l !== null),
    `** Hours per project (${hm(hours.total)} of ${hm(hours.expected)} expected)`,
    ...list(hours.projects.filter((h) => h.worked > 0).map((h) => `${h.project}: ${hm(h.worked)}`), "no time tracked"),
    ...ledger,
    ...extra,
    ...(goals.length ? [`** Goals (${goals.length})`, ...goals.map((g) => `- ${g.title}: ${g.total ? `${g.done}/${g.total} done (${g.percent}%)` : "no tasks"}, ${goalWhen(g)}`)] : []),
    end,
  ].join("\n");
}

const addDays = (d: Date, n: number) => new Date(d.getFullYear(), d.getMonth(), d.getDate() + n);
/** Busiest and quietest of YEAR's weeks (starting on WEEKSTART) by clocked SECS per `YYYY-MM-DD` (`tc_activity`), in hours: only weeks wholly
 *  inside the year and ended by TODAY count, so a partial edge week or the running one never comes out quietest; null with none (#1221). */
export function weekExtremes(secs: Record<string, number>, year: number, weekStart: string | undefined, today: Date) {
  let d = weekStartOf(new Date(year, 0, 1), weekStart);
  if (d.getFullYear() < year) d = addDays(d, 7);
  const weeks: { label: string; hours: number }[] = [];
  for (; ymd(addDays(d, 6)) <= ymd(today) && addDays(d, 6).getFullYear() === year; d = addDays(d, 7))
    weeks.push({ label: reviewWeekLabel(d), hours: Array.from({ length: 7 }, (_, i) => secs[ymd(addDays(d, i))] ?? 0).reduce((a, b) => a + b, 0) / 3600 });
  if (!weeks.length) return null;
  return { busiest: weeks.reduce((a, w) => (w.hours > a.hours ? w : a)), quietest: weeks.reduce((a, w) => (w.hours < a.hours ? w : a)) };
}

/** The sum of COUNTS (per `YYYY-MM-DD`, e.g. `activity_counts` `notes`, which spans the year before too) over YEAR. */
export const yearCount = (counts: Record<string, number> | undefined, year: number) =>
  Object.entries(counts ?? {}).reduce((a, [d, n]) => (d.startsWith(`${year}-`) ? a + n : a), 0);

/** `ledger_year` (#1196): one currency's year in cents per month (expense negative) and each category's net. */
export type LedgerYear = { currency: string; income: number[]; expense: number[]; categories: [string | null, number[]][] };
const sum = (xs: number[]) => xs.reduce((a, b) => a + b, 0);
/** "Ledger: spent 4120.00 EUR (last year 5300.00 EUR), income 100.00 EUR (last year 0.00 EUR); top: #food 1800.00 EUR" per currency of YEAR, LAST matched by currency. */
export function yearLedgerLines(year: LedgerYear[], last: LedgerYear[]) {
  const m = (n: number, cur: string) => money(n, cur, false);
  return year.map((y) => {
    const l = last.find((x) => x.currency === y.currency), cur = y.currency;
    const top = y.categories.filter(([c, ms]) => c && sum(ms) < 0).sort((a, b) => sum(a[1]) - sum(b[1]))[0];
    return `Ledger: spent ${m(-sum(y.expense), cur)} (last year ${m(-sum(l?.expense ?? []), cur)}), income ${m(sum(y.income), cur)} (last year ${m(sum(l?.income ?? []), cur)})${top ? `; top: #${top[0]} ${m(-sum(top[1]), cur)}` : ""}`;
  });
}

/** A `habit_overview` row. */
export type HabitRow = { title: string; habit?: { streak: number; best: number; pct: number } | null };
/** The yearly review's generated part (#1221): monthBody's parts over the year plus the busiest/quietest WEEKS (weekExtremes), NOTES edited
 *  (by last change, so a note counts once) and each open habit's current and best streak, longest first (section omitted without habits). */
export function yearBody(done: Task[], hours: ProjectHours, goals: Goal[], weeks: ReturnType<typeof weekExtremes>, notes: number, habits: HabitRow[], ledger: string[] = []) {
  const hs = habits.filter((h) => h.habit).sort((a, b) => b.habit!.streak - a.habit!.streak || b.habit!.best - a.habit!.best);
  return periodBody("year", done, hours, goals, ledger, [
    ...(weeks ? [`Busiest week: ${weeks.busiest.label} (${hm(weeks.busiest.hours)}); quietest: ${weeks.quietest.label} (${hm(weeks.quietest.hours)})`] : []),
    `Notes edited: ${notes} (by last change)`,
    ...(hs.length ? [`** Habit streaks (${hs.length})`, ...hs.map((h) => `- ${h.title}: streak ${h.habit!.streak}, best ${h.habit!.best}, ${h.habit!.pct}% on time`)] : []),
  ]);
}

/** TEXT with its KIND's begin..end span replaced by BODY, or null if it has none (then file a reviewEntry). Markers match exactly, so week and month blocks never collide. */
export function putReview(text: string, body: string, kind: ReviewKind = "week"): string | null {
  const eol = text.includes("\r\n") ? "\r\n" : "\n", lines = text.split(/\r?\n/), [BEGIN, END] = markers(kind);
  const b = lines.findIndex((l) => l.trim() === BEGIN), e = b < 0 ? -1 : lines.findIndex((l, i) => i > b && l.trim() === END);
  if (e < 0) return null;
  return [...lines.slice(0, b), ...body.split("\n"), ...lines.slice(e + 1)].join(eol);
}
