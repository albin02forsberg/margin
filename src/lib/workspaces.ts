/** Named sets of open tabs and the split (#1300), stored like the last session under localStorage `workspaces`. */
import { toSession, type Session } from "./session.ts";

export type Workspaces = Record<string, Session>;

/** The stored workspaces from RAW: each session validated like `parseSession`, empty ones and blank names dropped. */
export function parseWorkspaces(raw: string | null): Workspaces {
  let v: any;
  try { v = JSON.parse(raw ?? ""); } catch { return {}; }
  if (!v || typeof v !== "object" || Array.isArray(v)) return {};
  const out: Workspaces = {};
  for (const [k, s] of Object.entries(v)) {
    const n = workspaceName(k), w = toSession(s);
    if (n && w.tabs.length) out[n] = w;
  }
  return out;
}

/** NAME trimmed, or null when blank. */
export const workspaceName = (name: string | null | undefined) => name?.trim() || null;
/** WS with NAME set to S (replacing one with that name); an empty S removes it instead. */
export const putWorkspace = (ws: Workspaces, name: string, s: Session): Workspaces => (s.tabs.length ? { ...ws, [name]: s } : dropWorkspace(ws, name));
/** WS with every tab on FROM pointing at TO (a renamed or moved note); WS itself when none does. */
export function renameInWorkspaces(ws: Workspaces, from: string, to: string): Workspaces {
  const on = (e: Session["tabs"][number]) => "path" in e && e.path === from;
  if (!Object.values(ws).some((s) => s.tabs.some(on))) return ws;
  return Object.fromEntries(Object.entries(ws).map(([k, s]) => [k, { ...s, tabs: s.tabs.map((e) => (on(e) ? { ...e, path: to } : e)) }]));
}
export const dropWorkspace =(ws: Workspaces, name: string): Workspaces => Object.fromEntries(Object.entries(ws).filter(([k]) => k !== name));
