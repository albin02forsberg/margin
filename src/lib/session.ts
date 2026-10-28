/** A tab to reopen: a file at a line, or a view / task search (also the "Reopen closed tab" stack). */
export type Entry = { path: string; line: number } | { kind: "agenda" | "todo" | "time" | "projects" | "ledger" | "habits" | "settings"; query?: string; title: string };
export type Session = { tabs: Entry[]; active: number; split: number };

const KINDS = ["agenda", "todo", "time", "projects", "ledger", "habits", "settings"];
const ok = (e: any): e is Entry =>
  !!e && typeof e === "object" &&
  (typeof e.path === "string" ? Number.isInteger(e.line) && e.line >= 0
    : KINDS.includes(e.kind) && typeof e.title === "string" && (e.query == null || typeof e.query === "string"));

/** The last session's tabs from RAW (the stored JSON): bad entries dropped, at most MAX kept,
 *  ACTIVE (the left pane's tab) and SPLIT (the right pane's, -1 without a split) indexes into them or -1;
 *  anything unreadable is an empty session. */
export function parseSession(raw: string | null, max = 20): Session {
  const none = { tabs: [], active: -1, split: -1 };
  let s: any;
  try { s = JSON.parse(raw ?? ""); } catch { return none; }
  if (!s || !Array.isArray(s.tabs)) return none;
  const keep: [Entry, number][] = s.tabs.flatMap((e: unknown, i: number) => (ok(e) ? [[e, i]] : [])).slice(0, max);
  const at = (n: unknown) => keep.findIndex(([, i]) => i === n);
  return { tabs: keep.map(([e]) => e), active: at(s.active), split: at(s.split) };
}
