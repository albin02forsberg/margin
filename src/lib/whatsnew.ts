/** Release notes remembered across a self-update (#513): `pending-notes` is written before install, read on the next start. */
export type Notes = { from: string; to: string; body: string };

/** Notes to show for the running VERSION from the stored RAW json, or null (empty, malformed, or the update didn't land). */
export function notesToShow(raw: string | null, version: string): Notes | null {
  try {
    const n = JSON.parse(raw ?? "null");
    if (n && typeof n.to === "string" && typeof n.body === "string" && n.to === version && n.body.trim()) return { from: String(n.from ?? ""), to: n.to, body: n.body };
  } catch {}
  return null;
}

export const notesTitle = (n: Notes) => `What's new in ${n.to}${n.from ? ` (from ${n.from})` : ""}`;
export const notesText = (n: Notes) => `${notesTitle(n)}\n\n${n.body.trim()}\n`;
