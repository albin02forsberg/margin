/** Recently run palette commands, keyed by label (#800). */

/** Move `id` to the front, dedupe, cap. */
export const pushRecent = (list: string[], id: string, cap = 30): string[] => [id, ...list.filter((x) => x !== id)].slice(0, cap);

/** Items whose label is in `recents` first, in recency order; the rest keep their order. Unknown ids are ignored. */
export function sortByRecent<T extends { label: string }>(items: T[], recents: string[]): T[] {
  const rank = new Map(recents.map((r, i) => [r, i] as const));
  const hit = items.filter((i) => rank.has(i.label)).sort((a, b) => rank.get(a.label)! - rank.get(b.label)!);
  return [...hit, ...items.filter((i) => !rank.has(i.label))];
}

/** Parse the stored JSON; anything but a string array gives []. */
export function parseRecents(s: string): string[] {
  try { const v = JSON.parse(s); return Array.isArray(v) ? v.filter((x) => typeof x === "string") : []; } catch { return []; }
}
