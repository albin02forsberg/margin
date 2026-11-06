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

/** The vim options `:setlocal` sets per buffer (#1112), by name and alias. */
export type LocalOpts = NumOpts & { wrap: boolean };
const LOCAL: Record<string, keyof LocalOpts> = { number: "number", nu: "number", relativenumber: "relativenumber", rnu: "relativenumber", wrap: "wrap" };
export type SetScope = "set" | "local" | "global";
/** One `:set` / `:setlocal` / `:setglobal` arg: like setArg, but the buffer-local options read their effective value
 *  (LOCAL's override, else GET's global one) and write it into LOCAL, except for `:setglobal`, which only touches the global value.
 *  Returns the global option to set (none for `:setlocal` of a buffer-local option) or a message. */
export function scopedArg(a: string, scope: SetScope, local: Partial<LocalOpts>, get: (name: string) => unknown): { set?: [string, unknown] } | { msg: string } {
  const r = setArg(a, (n) => (scope !== "global" && local[LOCAL[n]] !== undefined ? local[LOCAL[n]] : get(n)));
  if (!("set" in r)) return r;
  const k = LOCAL[r.set[0]];
  if (k && scope !== "global") local[k] = r.set[1] as boolean;
  return k && scope === "local" ? {} : r;
}

/** Whether a search for PATTERN ignores case: `ignorecase`, unless `smartcase` and it has an uppercase letter (not a backslash-escaped one like `\\S`). */
export const foldCase = (pattern: string, ic: boolean, scs: boolean) => ic && !(scs && /[A-Z]/.test(pattern.replace(/\\./g, "")));
