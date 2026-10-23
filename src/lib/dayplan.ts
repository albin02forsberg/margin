// Day plan (#705): a day's timed tasks as blocks on an hour grid, with overlaps and free gaps.
import { fmtEffort } from "./effort.ts";

export type Block = { i: number; start: number; end: number; title: string; done: boolean; overlap: boolean; lane: number; lanes: number };
export type Gap = { start: number; end: number };

/** "9:05" / "09:05" as minutes after midnight. */
export const toMin = (t: string) => { const [h, m] = t.split(":"); return +h * 60 + +m; };
export const hhmm = (m: number) => `${String(Math.floor(m / 60)).padStart(2, "0")}:${String(m % 60).padStart(2, "0")}`;

/** Blocks of ITEMS (indexed) timed on DAY: a range stamp ("14:00-15:30") ends at its own end, else start + `:EFFORT:`
 *  (30 min without one); clamped at midnight; one block per heading and start (a task can show as scheduled and due).
 *  Overlaps and lanes are set among the open blocks; done ones stay in lane 0, dimmed by the view. */
export function blocks<T extends { date: string; time: string | null; title: string; path: string; line: number; effort?: number | null }>(
  items: T[], day: string, isDone: (it: T) => boolean,
): Block[] {
  const seen = new Set<string>(), out: Block[] = [];
  items.forEach((it, i) => {
    if (it.date !== day || !it.time) return;
    const [a, b] = it.time.split(/[-–]/), start = toMin(a);
    const k = `${it.path}\n${it.line}\n${start}`;
    if (seen.has(k)) return;
    seen.add(k);
    const end = Math.min(24 * 60, b && toMin(b) > start ? toMin(b) : start + (it.effort || 30));
    out.push({ i, start, end, title: it.title, done: isDone(it), overlap: false, lane: 0, lanes: 1 });
  });
  out.sort((x, y) => x.start - y.start || x.end - y.end);
  // clusters of transitively overlapping open blocks; each block takes the first free lane in its cluster
  let cluster: Block[] = [], laneEnd: number[] = [], until = -1;
  const close = () => { for (const c of cluster) { c.lanes = laneEnd.length; c.overlap = cluster.length > 1; } };
  for (const x of out.filter((x) => !x.done)) {
    if (x.start >= until) { close(); cluster = []; laneEnd = []; }
    let l = laneEnd.findIndex((e) => e <= x.start);
    if (l < 0) l = laneEnd.push(0) - 1;
    laneEnd[l] = x.end;
    x.lane = l;
    cluster.push(x);
    until = Math.max(until, x.end);
  }
  close();
  return out;
}

/** Free spans of at least 15 min between FROM and TO (minutes; working hours 08:00-18:00) not covered by an open block;
 *  NOW (minutes, on today) moves the start up so past time isn't offered. */
export function gaps(bs: Block[], from = 8 * 60, to = 18 * 60, now?: number): Gap[] {
  let t = Math.max(from, now ?? 0);
  const out: Gap[] = [];
  for (const b of bs) {
    if (b.done) continue;
    if (b.start >= to) break;
    if (b.start - t >= 15) out.push({ start: t, end: b.start });
    t = Math.max(t, b.end);
  }
  if (to - t >= 15) out.push({ start: t, end: to });
  return out;
}

/** "1h 30m free 10:00–11:30". */
export const gapLabel = (g: Gap) => `${fmtEffort(g.end - g.start)} free ${hhmm(g.start)}–${hhmm(g.end)}`;
