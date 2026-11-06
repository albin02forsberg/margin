// Inbox: process (#1137): walk the open top-level inbox entries one key each, like shutdown / plan the day.

export type InboxAction = "today" | "tomorrow" | "date" | "refile" | "tags" | "park" | "drop" | "skip";

/** t today, m tomorrow, s pick a date, r refile, g tags, p park (someday), d drop, n / space leave it. */
export const inboxKeys: Record<string, InboxAction> = { t: "today", m: "tomorrow", s: "date", r: "refile", g: "tags", p: "park", d: "drop", n: "skip", " ": "skip" };

export type InboxCounts = { scheduled: number; refiled: number; tagged: number; parked: number; dropped: number };

/** A key per entry that survives a re-fetch (lines move, keywords change): its title plus which same-titled entry it is. */
export function entryKeys<T extends { title: string }>(entries: T[]): (T & { key: string })[] {
  const n = new Map<string, number>();
  return entries.map((e) => {
    const i = n.get(e.title) ?? 0;
    n.set(e.title, i + 1);
    return { ...e, key: `${e.title}\n${i}` };
  });
}

/** "Processed 7: 3 scheduled, 2 refiled, 1 parked, 1 dropped" (zero counts left out). */
export function summariseInbox(c: InboxCounts): string {
  const parts = (Object.keys(c) as (keyof InboxCounts)[]).filter((k) => c[k]).map((k) => `${c[k]} ${k}`);
  return `Processed ${Object.values(c).reduce((a, b) => a + b, 0)}${parts.length ? `: ${parts.join(", ")}` : ""}`;
}
