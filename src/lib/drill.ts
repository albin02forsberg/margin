/** A `:drill:` heading from the backend's `drill_cards`; due/interval/ease are null on a new card. */
export type Card = { path: string; line: number; title: string; body: string; due: string | null; interval: number | null; ease: number | null };
export type Grade = 1 | 2 | 3 | 4; // again, hard, good, easy

/** "YYYY-MM-DD" N days after DAY (calendar days, no time zone drift). */
export function addDays(day: string, n: number) {
  const [y, m, d] = day.split("-").map(Number);
  const t = new Date(Date.UTC(y, m - 1, d + n));
  return t.toISOString().slice(0, 10);
}

/** Cards due by TODAY (new cards included), most overdue first, new ones last. */
export function due(cards: Card[], today: string) {
  const k = (c: Card) => c.due ?? "~"; // "~" sorts after digits
  return cards.filter((c) => !c.due || c.due <= today).sort((a, b) => (k(a) < k(b) ? -1 : k(a) > k(b) ? 1 : 0));
}

/** Next schedule after grading (an SM-2 variant): again resets to 1 day, hard grows slowly,
 *  good multiplies by the ease, easy by 1.3 × ease; the ease (default 2.5) moves by -0.2/-0.15/0/+0.15, min 1.3. */
export function grade(card: Pick<Card, "interval" | "ease">, g: Grade, today: string) {
  const i = card.interval ?? 0, e = card.ease ?? 2.5;
  const ease = Math.round(Math.max(1.3, e + [-0.2, -0.15, 0, 0.15][g - 1]) * 100) / 100;
  const interval =
    g === 1 ? 1 : g === 2 ? Math.max(1, Math.round(i * 1.2)) : g === 3 ? (i ? Math.max(i + 1, Math.round(i * e)) : 1) : i ? Math.max(i + 2, Math.round(i * e * 1.3)) : 4;
  return { due: addDays(today, interval), interval, ease };
}

/** The `org_edit` "prop" value that stores a schedule in the card's drawer. */
export const propValue = (s: { due: string; interval: number; ease: number }) => `DRILL_DUE ${s.due}\nDRILL_INTERVAL ${s.interval}\nDRILL_EASE ${s.ease}`;
