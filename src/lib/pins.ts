/** FILES sorted for Ctrl+P: PINNED first (in pin order), then RECENT (most recent first), then the rest in their order. */
export function orderNotes(files: string[], recent: string[], pinned: string[]) {
  const rank = (p: string) => {
    const i = pinned.indexOf(p);
    if (i >= 0) return i - pinned.length;
    const j = recent.indexOf(p);
    return j >= 0 ? j : 1e9;
  };
  return [...files].sort((a, b) => rank(a) - rank(b));
}

/** PINNED with PATH added at the end, or removed when it is already there. */
export const togglePin = (pinned: string[], path: string) => (pinned.includes(path) ? pinned.filter((p) => p !== path) : [...pinned, path]);

/** Parse a stored pin list; anything that isn't an array of strings gives []. */
export function parsePins(s: string): string[] {
  try {
    const v = JSON.parse(s);
    return Array.isArray(v) ? v.filter((p) => typeof p === "string") : [];
  } catch {
    return [];
  }
}
