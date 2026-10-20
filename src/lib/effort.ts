/** `:EFFORT:` minutes as a badge: "30m", "2h", "1h 30m". */
export const fmtEffort = (m: number) => (m >= 60 ? `${Math.floor(m / 60)}h${m % 60 ? ` ${m % 60}m` : ""}` : `${m}m`);

/** Total effort of the open tasks among ITEMS (one day's rows), each heading counted once
 *  (a task can show as both scheduled and due). 0 when none has an estimate. */
export function planned(items: { path: string; line: number; effort?: number | null }[], done: (it: any) => boolean) {
  const seen = new Set<string>();
  let sum = 0;
  for (const it of items) {
    const k = `${it.path}\n${it.line}`;
    if (!it.effort || done(it) || seen.has(k)) continue;
    seen.add(k);
    sum += it.effort;
  }
  return sum;
}
