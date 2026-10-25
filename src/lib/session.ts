/** A tab to reopen: a file at a line, or a view / task search (also the "Reopen closed tab" stack). */
export type Entry = { path: string; line: number } | { kind: "agenda" | "todo" | "time" | "projects" | "ledger" | "habits" | "settings"; query?: string; title: string };
export type Session = { tabs: Entry[]; active: number };

const KINDS = ["agenda", "todo", "time", "projects", "ledger", "habits", "settings"];
const ok = (e: any): e is Entry =>
  !!e && typeof e === "object" &&
  (typeof e.path === "string" ? Number.isInteger(e.line) && e.line >= 0
    : KINDS.includes(e.kind) && typeof e.title === "string" && (e.query == null || typeof e.query === "string"));

/** The last session's tabs from RAW (the stored JSON): bad entries dropped, at most MAX kept,
 *  ACTIVE an index into them or -1; anything unreadable is an empty session. */
export function parseSession(raw: string | null, max = 20): Session {
  let s: any;
  try { s = JSON.parse(raw ?? ""); } catch { return { tabs: [], active: -1 }; }
  if (!s || !Array.isArray(s.tabs)) return { tabs: [], active: -1 };
  const keep: [Entry, number][] = s.tabs.flatMap((e: unknown, i: number) => (ok(e) ? [[e, i]] : [])).slice(0, max);
  return { tabs: keep.map(([e]) => e), active: keep.findIndex(([, i]) => i === s.active) };
}
