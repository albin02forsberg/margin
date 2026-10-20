import { fmtEffort } from "./effort.ts";

const DAYS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

type Row = { date: string; title: string; priority: string | null; effort?: number | null; done: boolean };

/** `2026-10-20` as `Tue 20 Oct`. */
const heading = (iso: string) => {
  const d = new Date(+iso.slice(0, 4), +iso.slice(5, 7) - 1, +iso.slice(8, 10));
  return `${DAYS[d.getDay()]} ${d.getDate()} ${MONTHS[d.getMonth()]}`;
};

/** The list as Markdown checkboxes (`- [ ] [#A] Title (30m)`); with GROUPBYDAY, a `## Tue 20 Oct` heading per date. */
export function agendaText(items: Row[], groupByDay = false) {
  const out: string[] = [];
  let last = "";
  for (const it of items) {
    if (groupByDay && it.date !== last) { if (out.length) out.push(""); out.push(`## ${heading(it.date)}`); last = it.date; }
    out.push(`- [${it.done ? "x" : " "}] ${it.priority ? `[#${it.priority}] ` : ""}${it.title}${it.effort ? ` (${fmtEffort(it.effort)})` : ""}`);
  }
  return out.join("\n");
}
