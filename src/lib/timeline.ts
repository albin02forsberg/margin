// Day timeline (#694): one day's clocked sessions, tasks closed and notes edited, in time order.
import { hm } from "./clock.ts";

type Session = { project: string; desc: string; hours: number; start: string; end: string; line: number | null };
type Done = { title: string; category: string; time: string | null; path: string; line: number };
/** The open clock (tc_status): SINCE naive ISO, or null when not clocked in. */
type Running = { since: string | null; project: string | null; task: string | null };

export type Entry =
  | { kind: "session"; time: string; text: string; session: Session | null }
  | { kind: "done" | "note"; time: string; text: string; path: string; line: number };

const pad = (n: number) => String(n).padStart(2, "0");
const what = (project: string | null, desc: string | null) => `${project || "Other"}: ${desc || "(no description)"}`;

/** DAY's (yyyy-mm-dd) entries by time: finished SESSIONS on that day (a session past midnight stays on its start
 *  day, as in the log), the RUNNING clock when it started on or before DAY (open-ended), DONE (doneLog rows of
 *  DAY) and NOTES ([path, title, mtime secs], notes_recent) changed that day in local time. */
export function buildTimeline(day: string, sessions: Session[], running: Running | null, done: Done[], notes: [string, string, number][]): Entry[] {
  const out: Entry[] = sessions.map((s) => ({ kind: "session", time: s.start.slice(0, 5), text: `${s.start.slice(0, 5)}–${s.end.slice(0, 5)} ${what(s.project, s.desc)} (${hm(s.hours)})`, session: s }));
  if (running?.since && running.since.slice(0, 10) <= day) {
    const t = running.since.slice(0, 10) === day ? running.since.slice(11, 16) : "00:00";
    out.push({ kind: "session", time: t, text: `${running.since.slice(0, 10) === day ? t : running.since.slice(0, 16).replace("T", " ")}– (running) ${what(running.project, running.task)}`, session: null });
  }
  for (const d of done) out.push({ kind: "done", time: d.time ?? "00:00", text: `${d.time ?? ""} Done: ${d.title}${d.category ? ` (${d.category})` : ""}`.trim(), path: d.path, line: d.line });
  for (const [path, title, secs] of notes) {
    const t = new Date(secs * 1000), local = `${t.getFullYear()}-${pad(t.getMonth() + 1)}-${pad(t.getDate())}`;
    if (local !== day) continue;
    const hhmm = `${pad(t.getHours())}:${pad(t.getMinutes())}`;
    out.push({ kind: "note", time: hhmm, text: `${hhmm} Edited: ${title || path.split("/").pop()}`, path, line: 0 });
  }
  return out.sort((a, b) => a.time.localeCompare(b.time)); // stable: same minute keeps sessions, done, notes
}

/** The timeline as a Markdown list under a heading for DAY. */
export const timelineMarkdown = (day: string, entries: Entry[]) =>
  [`## ${day}`, "", ...(entries.length ? entries.map((e) => `- ${e.text}`) : ["- nothing logged"])].join("\n");
