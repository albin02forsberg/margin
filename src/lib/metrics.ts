// Journal metrics trend (#721): what the backend's `metric_trend` returns, and the sparkline geometry.

/** A metric's summary; `long`/`rest` are its mean on days with hours >= expected vs the other workdays. */
export type Stat = { name: string; n: number; min: number | null; max: number | null; mean: number | null; latest: number | null; long: number | null; rest: number | null };
/** One day (yyyy-mm-dd): each metric's value (null: not logged) and hours clocked. */
export type Row = { date: string; values: (number | null)[]; hours: number };
export type Trend = { stats: Stat[]; days: Row[] };

const dayNo = (s: string) => Date.UTC(+s.slice(0, 4), +s.slice(5, 7) - 1, +s.slice(8, 10)) / 864e5;

/** Points of metric I in ROWS on a W×H box: x spans the SPAN days ending TO, y the values' min (bottom)
 *  to max (top); equal values sit mid-height. */
export function sparkline(rows: Row[], i: number, to: string, span: number, w: number, h: number): [number, number][] {
  const pts = rows.flatMap((r) => (r.values[i] == null ? [] : [[dayNo(r.date), r.values[i]!] as const]));
  const lo = Math.min(...pts.map((p) => p[1])), hi = Math.max(...pts.map((p) => p[1])), end = dayNo(to);
  const x = (d: number) => ((span - 1 - (end - d)) / Math.max(1, span - 1)) * w;
  return pts.map(([d, v]) => [x(d), hi > lo ? h - ((v - lo) / (hi - lo)) * h : h / 2]);
}

/** A value with at most one decimal, "–" for none. */
export const num = (x: number | null) => (x == null ? "–" : String(Math.round(x * 10) / 10));
