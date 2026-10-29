// Yearly activity calendar (#916): a GitHub-style grid of per-day values, with shading levels and streaks.

/** ISO date N days after ISO date D (UTC arithmetic, so no DST surprises). */
export function addDays(d: string, n: number): string {
  const [y, m, dd] = d.split("-").map(Number);
  return new Date(Date.UTC(y, m - 1, dd + n)).toISOString().slice(0, 10);
}

/** Shade 0..4 per value: 0 for none, else the quartile among the nonzero VALUES (the largest is always 4). */
export function levels(values: number[]): number[] {
  const nz = values.filter((v) => v > 0).sort((a, b) => a - b);
  return values.map((v) => {
    if (v <= 0) return 0;
    let le = 0; // nonzero values <= v
    while (le < nz.length && nz[le] <= v) le++;
    return Math.ceil((4 * le) / nz.length);
  });
}

/** YEAR as week columns, Monday first: 7 ISO dates each, null outside the year. */
export function weeks(year: number): (string | null)[][] {
  const jan1 = `${year}-01-01`, start = addDays(jan1, -((new Date(Date.UTC(year, 0, 1)).getUTCDay() + 6) % 7));
  const out: (string | null)[][] = [];
  for (let d = start; d.slice(0, 4) <= `${year}`; ) {
    const col: (string | null)[] = [];
    for (let i = 0; i < 7; i++, d = addDays(d, 1)) col.push(d.startsWith(`${year}-`) ? d : null);
    out.push(col);
  }
  return out;
}

/** Runs of consecutive days with a value in DAYS: the longest one reaching into YEAR (counted from its real
 *  start, so a run from December carries over), and the current one ending TODAY, or yesterday while today is empty. */
export function streaks(days: Record<string, number>, year: number, today: string): { longest: number; current: number } {
  const on = (d: string) => (days[d] ?? 0) > 0;
  let [run, longest] = [0, 0];
  const keys = Object.keys(days).filter((d) => days[d] > 0).sort();
  let d = keys[0] ?? `${year}-01-01`;
  for (const end = `${year}-12-31` < today ? `${year}-12-31` : today; d <= end; d = addDays(d, 1)) {
    run = on(d) ? run + 1 : 0;
    if (d.startsWith(`${year}-`)) longest = Math.max(longest, run);
  }
  let current = 0;
  for (let c = on(today) ? today : addDays(today, -1); on(c); c = addDays(c, -1)) current++;
  return { longest, current };
}

/** "3.5 h", "25 min" or "–" for SECONDS. */
export const hours = (s: number) => (s <= 0 ? "–" : s < 3600 ? `${Math.max(1, Math.round(s / 60))} min` : `${+(s / 3600).toFixed(1)} h`);
