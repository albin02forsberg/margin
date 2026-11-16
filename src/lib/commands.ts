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
export type Host = { config: () => Record<string, unknown>; setConfig: (key: string, value: unknown) => void; buffer: { path?: string; [k: string]: any }; reload: () => unknown };

/** One run of the init scripts and everything it added (commands, key bindings, hooks, config overrides).
 *  Reload throws the whole session away and starts a new one, so nothing from the previous run survives. */
export function session(host: Host, report: (msg: string) => void) {
  const reg = registry(), binds = new Map<string, string[]>(), over: Record<string, unknown> = {};
  const hooks: Record<Hook, ((...a: unknown[]) => unknown)[]> = { save: [], open: [], startup: [] };
  /** Hooks of each kind still running (an async one until it settles), and paths a `save` hook edited. */
  const active: Record<Hook, number> = { save: 0, open: 0, startup: 0 }, edited = new Set<string>();
  /** Buffer edits made by a `save` hook mark the buffer, so the autosave they cause skips the hooks (no edit → save → edit loop). */
  const marking = (f: (...a: any[]) => unknown) => (...a: any[]) => { if (active.save && host.buffer.path) edited.add(host.buffer.path); return f(...a); };
  const buffer: Host["buffer"] = Object.create(host.buffer, Object.fromEntries(["insert", "replace"].filter((k) => typeof host.buffer[k] === "function").map((k) => [k, { value: marking(host.buffer[k]) }])));
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
    buffer,
    /** Ignored inside a `startup` hook, which would otherwise rerun itself forever. */
    reload: () => (active.startup ? report("⚠ init: reload() ignored inside a startup hook") : host.reload()),
  };
  /** Call H's hooks with ARGS; a throwing or rejecting hook is reported and the others still run.
   *  A save of a path a `save` hook edited skips the hooks once. */
  const emit = (h: Hook, ...args: unknown[]) => {
    if (h === "save" && edited.delete(args[0] as string)) return;
    for (const f of hooks[h]) {
      const bad = (e: unknown) => report(`⚠ init ${h} hook: ${msg(e)}`), done = () => void active[h]--;
      active[h]++;
      try {
        const r = f(...args);
        if (r instanceof Promise) { r.catch(bad).finally(done); continue; }
      } catch (e) { bad(e); }
      done();
    }
  };
  /** Run SCRIPTS, then report bindings to commands that are neither in KNOWN (the built-ins) nor registered. */
  const run = (scripts: { name: string; text: string }[], known: string[]) => {
    runScripts(scripts, api, report);
    for (const l of binds.keys()) if (!known.includes(l) && !reg.list.some((c) => c.label === l)) report(`⚠ init: bind: no command “${l}”`);
  };
  return { api, cmds: reg.list, binds, over, emit, run };
}

/** The `margin` object init scripts and bundled packages get. */
export type Margin = ReturnType<typeof session>["api"];
/** A bundled package (`src/lib/packages/`): loaded through the same API before the user's scripts; `disabled_packages` skips it by ID. */
export type Package = { id: string; load: (margin: Margin) => void };

/** CMDS with the user BINDS applied: a bound key is added to its command and taken from every other command. */
export function applyBinds<T extends { label: string; keys?: string[] }>(cmds: T[], binds: Map<string, string[]>): T[] {
  if (!binds.size) return cmds;
  const taken = new Set([...binds.values()].flat().map((k) => k.toLowerCase()));
  return cmds.map((c) => {
    const b = binds.get(c.label), keys = c.keys?.filter((k) => !taken.has(k.toLowerCase()));
    return !b && keys?.length === c.keys?.length ? c : { ...c, keys: [...(b ?? []), ...(keys ?? [])] };
  });
}
