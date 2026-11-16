// User commands registered by init scripts (`init/*.js` in the config dir). Built-in palette commands stay in +page.svelte.
export type UserCmd = { label: string; keys?: string[]; leader?: string; ctx?: "editor" | "normal"; run: () => unknown };

/** Registry of user commands: a duplicate label replaces the earlier one. */
export function registry() {
  const list: UserCmd[] = [];
  const register = (c: UserCmd) => {
    if (!c || typeof c.label !== "string" || !c.label.trim() || typeof c.run !== "function") throw new Error("register needs { label, run }");
    const i = list.findIndex((x) => x.label === c.label);
    i < 0 ? list.push(c) : (list[i] = c);
  };
  return { list, register };
}

/** Merge user commands after the built-ins; a user label equal to a built-in replaces it. */
export function mergeCmds<T extends { label: string }>(builtin: T[], user: T[]): T[] {
  const over = new Set(user.map((c) => c.label));
  return [...builtin.filter((c) => !over.has(c.label)), ...user];
}

/** Run each script (sorted by the caller) with a `margin` API; a throwing script is reported by name and the rest still run. */
export function runScripts(scripts: { name: string; text: string }[], api: { register: (c: UserCmd) => void }, report: (msg: string) => void) {
  for (const s of scripts) {
    try { new Function("margin", s.text)(api); } catch (e) { report(`⚠ init/${s.name}: ${e instanceof Error ? e.message : e}`); }
  }
}
