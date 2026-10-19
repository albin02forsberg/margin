/** Vim `:set` line-number options (session only). */
export type NumOpts = { number: boolean; relativenumber: boolean };

/** Apply `:set` args like ["nu", "nornu"] to opts; returns the new opts (unknown args ignored). */
export function applySet(o: NumOpts, args: string[]): NumOpts {
  const r = { ...o };
  for (const a of args) {
    const m = /^(no)?(number|nu|relativenumber|rnu)$/.exec(a);
    if (!m) continue;
    r[m[2].startsWith("r") ? "relativenumber" : "number"] = !m[1];
  }
  return r;
}
