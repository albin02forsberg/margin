// Estimates vs actuals (#719): how far clocked time on done tasks strays from their `:EFFORT:`.

type Done = { title: string; category: string; effort?: number | null; spent?: number | null };
type Sum = { n: number; est: number; spent: number };

/** DONE tasks with an estimate, summed per project (category) and in total, plus the 5 largest overruns
 *  (clocked minus estimate, positive only). Tasks with an estimate but no clocked time count as `unclocked`
 *  and stay out of the sums, so they don't skew the ratio. */
export function estimates<T extends Done>(done: T[]) {
  const by = new Map<string, Sum>(), total: Sum = { n: 0, est: 0, spent: 0 }, timed: T[] = [];
  let unclocked = 0;
  for (const t of done) {
    if (!t.effort) continue;
    if (!t.spent) { unclocked++; continue; }
    timed.push(t);
    const s = by.get(t.category) ?? by.set(t.category, { n: 0, est: 0, spent: 0 }).get(t.category)!;
    for (const x of [s, total]) { x.n++; x.est += t.effort; x.spent += t.spent; }
  }
  const projects = [...by].map(([project, s]) => ({ project, ...s })).sort((a, b) => b.spent / b.est - a.spent / a.est || a.project.localeCompare(b.project));
  const over = (t: T) => t.spent! - t.effort!;
  const worst = timed.filter((t) => over(t) > 0).sort((a, b) => over(b) - over(a)).slice(0, 5);
  return { projects, total, worst, unclocked };
}

/** Clocked time as a share of the estimate, "120%". */
export const pct = (s: { est: number; spent: number }) => `${Math.round((s.spent / s.est) * 100)}%`;

/** The weekly review line, "Estimates: 120% of plan (4 tasks).", or null without estimated, clocked tasks. */
export function estimateLine(done: Done[]) {
  const { total } = estimates(done);
  return total.n ? `Estimates: ${pct(total)} of plan (${total.n} task${total.n === 1 ? "" : "s"}).` : null;
}
