/** Vim `:set` line-number options (session only). */
export type NumOpts = { number: boolean; relativenumber: boolean };

/** One `:set` arg ("nu", "nonu", "invnu", "nu!", "nu?", "tw=80", "tw") against the options GET reads (an Error for an unknown name):
 *  the option and value to set, or a message to show. Boolean options are the ones whose value is a boolean. */
export function setArg(a: string, get: (name: string) => unknown): { set: [string, unknown] } | { msg: string } {
  const m = /^(\w+)(?:(\?)|(!)|=(.*))?$/.exec(a);
  if (!m) return { msg: `Invalid argument: ${a}` };
  let name = m[1], cur = get(name), no = false, inv = !!m[3];
  if (cur instanceof Error) {
    const p = /^(no|inv)(\w+)$/.exec(name);
    if (!p || get(p[2]) instanceof Error) return { msg: `Unknown option: ${name}` };
    [name, cur] = [p[2], get(p[2])];
    if (p[1] === "no") no = true; else inv = true;
  }
  if (typeof cur === "boolean") {
    if (m[2]) return no || inv ? { msg: `Invalid argument: ${a}` } : { msg: `  ${cur ? "" : "no"}${name}` };
    if (m[4] !== undefined || (no && inv)) return { msg: `Invalid argument: ${a}` };
    return { set: [name, inv ? !cur : !no] };
  }
  if (no || inv) return { msg: `Invalid argument: ${a}` };
  if (m[4] === undefined) return { msg: `  ${name}=${cur ?? ""}` };
  return { set: [name, m[4]] };
}
