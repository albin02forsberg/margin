/** Reschedules after which a task counts as stuck (chip, nudge, review). */
export const PUSHED_MIN = 3;

/** Whether a task was rescheduled often enough to flag. */
export const isPushed = (n?: number | null): boolean => (n ?? 0) >= PUSHED_MIN;

export type StuckAction = "park" | "drop" | "split";

/** Extra keys for stuck (pushed) rows in shutdown / morning plan (#1061): p parks as someday, d drops (CANCELLED), b adds a subtask (#1098). */
export const stuckKeys: Record<string, StuckAction> = { p: "park", d: "drop", b: "split" };

/** The done keyword that cancels a task (CANCELLED / CANCELED), if configured. */
export const cancelKeyword = (done: string[]): string | undefined => done.find((k) => /^cancell?ed$/i.test(k));

/** Menu items and keys for one row: the stuck actions are added only when it was pushed often enough (drop: and a cancel keyword exists). */
export function stuckMenu(pushed: number | null | undefined, done: string[]): { items: { label: string; value: StuckAction }[]; keys: Record<string, StuckAction> } {
  if (!isPushed(pushed)) return { items: [], keys: {} };
  const drop = !!cancelKeyword(done), keys: Record<string, StuckAction> = { p: "park", b: "split" };
  if (drop) keys.d = "drop";
  return { items: [{ label: "p  Park (someday)", value: "park" as const }, { label: "b  Break down (add subtask)", value: "split" as const }, ...(drop ? [{ label: "d  Drop (cancel)", value: "drop" as const }] : [])], keys };
}
