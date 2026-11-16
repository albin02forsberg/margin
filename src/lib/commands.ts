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

const msg = (e: unknown) => (e instanceof Error ? e.message : String(e));

/** Run each script (sorted by the caller) with a `margin` API; a throwing script is reported by name and the rest still run. */
export function runScripts(scripts: { name: string; text: string }[], api: object, report: (msg: string) => void) {
  for (const s of scripts) {
    try { new Function("margin", s.text)(api); } catch (e) { report(`⚠ init/${s.name}: ${msg(e)}`); }
  }
}

export type Hook = "save" | "open" | "startup";
/** What the page lends the scripts: the live config (overrides included), an in-memory config setter, the focused buffer, reload. */
export type Host = { config: () => Record<string, unknown>; setConfig: (key: string, value: unknown) => void; buffer: object; reload: () => unknown };

/** One run of the init scripts and everything it added (commands, key bindings, hooks, config overrides).
 *  Reload throws the whole session away and starts a new one, so nothing from the previous run survives. */
export function session(host: Host, report: (msg: string) => void) {
  const reg = registry(), binds = new Map<string, string[]>(), over: Record<string, unknown> = {};
  const hooks: Record<Hook, ((...a: unknown[]) => unknown)[]> = { save: [], open: [], startup: [] };
  const api = {
    command: reg.register,
    register: reg.register,
    /** Bind KEYS ("Ctrl+Alt+H" or a list) to the command labelled LABEL, built-in or user; the keys leave any other command. */
    bind(keys: string | string[], label: string) {
      const ks = typeof keys === "string" ? [keys] : keys;
      if (!Array.isArray(ks) || !ks.length || ks.some((k) => typeof k !== "string" || !k) || typeof label !== "string") throw new Error("bind needs (keys, label)");
      binds.set(label, [...(binds.get(label) ?? []), ...ks]);
    },
    on(h: Hook, fn: (...a: unknown[]) => unknown) {
      if (!Object.hasOwn(hooks, h) || typeof fn !== "function") throw new Error("on needs ('save' | 'open' | 'startup', fn)");
      hooks[h].push(fn);
    },
    /** Like Emacs `setq`: set changes the setting for this session only and never writes config.toml. */
    config: { get: (k: string) => host.config()[k], set: (k: string, v: unknown) => { over[k] = v; host.setConfig(k, v); } },
    buffer: host.buffer,
    reload: host.reload,
  };
  /** Call H's hooks with ARGS; a throwing or rejecting hook is reported and the others still run. */
  const emit = (h: Hook, ...args: unknown[]) => {
    for (const f of hooks[h]) {
      const bad = (e: unknown) => report(`⚠ init ${h} hook: ${msg(e)}`);
      try { const r = f(...args); if (r instanceof Promise) r.catch(bad); } catch (e) { bad(e); }
    }
  };
  /** Run SCRIPTS, then report bindings to commands that are neither in KNOWN (the built-ins) nor registered. */
  const run = (scripts: { name: string; text: string }[], known: string[]) => {
    runScripts(scripts, api, report);
    for (const l of binds.keys()) if (!known.includes(l) && !reg.list.some((c) => c.label === l)) report(`⚠ init: bind: no command “${l}”`);
  };
  return { api, cmds: reg.list, binds, over, emit, run };
}

/** CMDS with the user BINDS applied: a bound key is added to its command and taken from every other command. */
export function applyBinds<T extends { label: string; keys?: string[] }>(cmds: T[], binds: Map<string, string[]>): T[] {
  if (!binds.size) return cmds;
  const taken = new Set([...binds.values()].flat().map((k) => k.toLowerCase()));
  return cmds.map((c) => {
    const b = binds.get(c.label), keys = c.keys?.filter((k) => !taken.has(k.toLowerCase()));
    return !b && keys?.length === c.keys?.length ? c : { ...c, keys: [...(b ?? []), ...(keys ?? [])] };
  });
}
