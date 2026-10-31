import { money } from "./ledger.ts";
import { waitAge, type Waiting } from "./waiting.ts";

/** People for `Space n @` (#704): `@name` tags, one per name in any case (first spelling kept), sorted. */
export function peopleFrom(tags: string[]): string[] {
  const seen = new Map<string, string>();
  for (const t of tags) {
    const n = personName(t);
    if (t.trim().startsWith("@") && n && !seen.has(n.toLowerCase())) seen.set(n.toLowerCase(), n);
  }
  return [...seen.values()].sort((a, b) => a.localeCompare(b, undefined, { sensitivity: "base" }));
}

/** Last contact (#969) per lowercased name: `[date, source]` (journal, note, task or ledger), as `people_last` returns it. */
export type LastContact = Record<string, [string, string]>;

const days = (from: string, to: string) => Math.round((Date.parse(to) - Date.parse(from)) / 86_400_000);

/** "41 d ago" from DATE (YYYY-MM-DD) to TODAY; "today" for today or later, "never dated" without a date. */
export function contactAge(date: string | undefined, today: string): string {
  if (!date) return "never dated";
  const n = days(date, today);
  return n <= 0 ? "today" : `${n} d ago`;
}

/** Picker items for PEOPLE, longest out of touch first (never dated first, then by name), the age as detail. */
export function byLastContact(people: string[], last: LastContact, today: string) {
  const at = (n: string) => last[n.toLowerCase()]?.[0] ?? "";
  return [...people].sort((a, b) => at(a).localeCompare(at(b)) || a.localeCompare(b, undefined, { sensitivity: "base" }))
    .map((n) => ({ label: `@${n}`, detail: contactAge(at(n) || undefined, today), value: n as unknown }));
}

/** The person view's first line: "Last mentioned 41 days ago (journal 2026-09-19)". */
export function lastLine(last: { date: string; source: string } | null | undefined, today: string): string {
  if (!last) return "Never mentioned with a date";
  const n = days(last.date, today);
  return `Last mentioned ${n <= 0 ? "today" : `${n} day${n === 1 ? "" : "s"} ago`} (${last.source} ${last.date})`;
}

/** A typed or picked person without the leading `@`s and spaces. */
export const personName = (s: string) => s.trim().replace(/^@+/, "").trim();

type Task = { path: string; line: number; keyword: string | null; title: string; date: string; priority: string | null };
type Note = { path: string; line: number; title: string; text: string };
type Payment = { path: string; line: number; date: string; cents: number; currency: string; text: string };

/** Picker rows for a person view: tasks waiting on them (#808, with age, as of TODAY), then their other open tasks (with
 *  priority and date), the notes mentioning them, then their ledger entries (#790). REL names a file. */
export function personRows(v: { tasks: Task[]; notes: Note[]; payments?: Payment[]; waiting?: Waiting[] }, rel: (p: string) => string, today = "") {
  const due = (d: string) => (d.startsWith("+") ? "" : ` · ${d}`);
  const waits = new Set((v.waiting ?? []).map((w) => `${w.path}:${w.line}`));
  return [
    ...(v.waiting ?? []).map((w) => ({ label: `Waiting on them: ${w.title}`, detail: `${waitAge(w.since, today)} · ${rel(w.path)}${due(w.date)}`, value: [w.path, w.line] as unknown })),
    ...v.tasks.filter((t) => !waits.has(`${t.path}:${t.line}`)).map((t) => ({ label: `${t.keyword ?? ""} ${t.priority ? `[#${t.priority}] ` : ""}${t.title}`.trim(), detail: `task · ${rel(t.path)}${due(t.date)}`, value: [t.path, t.line] as unknown })),
    ...v.notes.map((n) => ({ label: n.text, detail: `${n.title || rel(n.path)} · line ${n.line + 1}`, value: [n.path, n.line] as unknown })),
    ...(v.payments ?? []).map((p) => ({ label: `${money(p.cents, p.currency)} ${p.text}`.trim(), detail: `ledger · ${p.date}`, value: [p.path, p.line] as unknown })),
  ];
}
