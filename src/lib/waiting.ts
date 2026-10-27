// Waiting-for list (#808): WAIT tasks with whom they wait on, since when and the follow-up date.

export type Waiting = { path: string; line: number; title: string; date: string; who: string | null; since: string | null };

const DAY = 86_400_000;

/** "waiting 9 days" from SINCE (YYYY-MM-DD, or null: "age unknown") to TODAY. */
export function waitAge(since: string | null, today: string): string {
  if (!since) return "age unknown";
  const n = Math.round((Date.parse(today) - Date.parse(since)) / DAY);
  return n <= 0 ? "waiting since today" : `waiting ${n} day${n === 1 ? "" : "s"}`;
}

/** The task's own follow-up date (SCHEDULED, else DEADLINE); none for the backend's far-future "no date". */
export const followUp = (w: Waiting) => (w.date.startsWith("+") ? null : w.date);

/** Picker rows (backend order: by who, oldest first): "who · title", age, follow-up (warn once due) and file. REL names a file. */
export function waitingRows(ws: Waiting[], today: string, rel: (p: string) => string) {
  return ws.map((w) => {
    const f = followUp(w);
    return {
      label: `${w.who ?? "(unspecified)"} · ${w.title}`,
      detail: [waitAge(w.since, today), f && `follow up ${f}`, rel(w.path)].filter(Boolean).join(" · "),
      warn: !!f && f <= today,
      value: [w.path, w.line] as unknown,
    };
  });
}
