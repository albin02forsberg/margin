// Weekday x hour heatmap of clocked time (#738): what `tc_rhythm` returns and how it's worded.

export type Piece = { day: number; hour: number; start: string; end: string; project: string; minutes: number };
export type Rhythm = { minutes: number[][]; peak: [number, number] | null; evening: number; hours: number; pieces: Piece[] };

export const DAYS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

/** "10–11" for HOUR. */
export const hourSpan = (h: number) => `${h}–${h + 1}`;

/** Whole minutes, "–" for none. */
export const mins = (m: number) => (m > 0 ? `${Math.max(1, Math.round(m))} min` : "–");

/** One line: the peak cell, the share after 17:00 and the total. */
export function summary(r: Rhythm): string {
  if (!r.peak) return "No clocked time in this range.";
  const [d, h] = r.peak;
  return `Peak: ${DAYS[d]} ${hourSpan(h)}, ${mins(r.minutes[d][h])} avg · ${Math.round(r.evening * 100)}% after 17:00 · ${+r.hours.toFixed(1)} h in total`;
}

/** Shade 0..4 of M against the busiest cell MAX (0 only for no time), for a 5-step single-hue ramp. */
export const shade = (m: number, max: number) => (m <= 0 || max <= 0 ? 0 : Math.min(4, Math.ceil((m / max) * 4)));
