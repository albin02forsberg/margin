<script lang="ts">
  import "$lib/theme.css";
  import { onMount, tick } from "svelte";
  import { Channel, invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getVersion } from "@tauri-apps/api/app";
  import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
  import { Update } from "@tauri-apps/plugin-updater";
  import { relaunch } from "@tauri-apps/plugin-process";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { EditorView } from "@codemirror/view";
  import type { EditorState } from "@codemirror/state";
  import * as ed from "$lib/editor";
  import { changed, journalNeighbour, mergeChanged, relativeTo } from "$lib/paths";
  import Picker, { type PickOpts } from "$lib/Picker.svelte";
  import Menu, { type MenuNode } from "$lib/Menu.svelte";
  import Agenda, { type AgendaApi, type AgendaItem } from "$lib/Agenda.svelte";
  import Time, { hm, type Session, type Suggestion, type TimeApi } from "$lib/Time.svelte";
  import TaskDialog from "$lib/TaskDialog.svelte";
  import Projects from "$lib/Projects.svelte";
  import Settings from "$lib/Settings.svelte";
  import Graph, { type NoteGraph } from "$lib/Graph.svelte";
  import { chatContext } from "$lib/chatctx";
  import { bugReportUrl, isError, osName } from "$lib/bugreport";
  import { naive, parseWhen } from "$lib/clock";
  import { parseSession, type Entry } from "$lib/session";
  import Chat, { type ChatMsg } from "$lib/Chat.svelte";

  type View = "agenda" | "todo" | "time" | "projects" | "settings";
  /** SAVED: the text last read from / written to disk, to tell our own writes from outside changes.
   *  REPORT: how to regenerate a report tab when the data changes. */
  type Tab = {
    key: string; title: string; kind: "file" | "report" | View; path?: string; state?: EditorState; dirty: boolean;
    saved?: string; report?: { kind: string; dateInput?: string | null }; conflict?: boolean; missing?: boolean; query?: string;
  };
  type NoteNode = { id: string; title: string; path: string; line: number };
  type Hit = { path: string; title: string; line: number; text: string };
  type Mention = Hit & { col: number; len: number };
  type Target = { path: string; line: number | null; label: string };
  type Parts = { level: number; keyword: string | null; priority: string | null; title: string; tags: string[] };
  type Loc = { path: string; line: number };
  /** A command: shown in the palette, bound to KEYS, placed in the Space menu at LEADER.
   *  CTX limits keys to the editor ("editor") or vim normal mode ("normal"). RUN returning false passes the key on. */
  type Cmd = { label: string; keys?: string[]; leader?: string; ctx?: "editor" | "normal"; run: () => unknown };

  let cfg = $state<any>(null);
  let tabs = $state<Tab[]>([]);
  let cur = $state(-1);
  let editorEl = $state<HTMLDivElement>();
  let dropCue = $state(false);
  let view: EditorView;
  let picker = $state<Picker>();
  let menu = $state<Menu>();
  let taskDialog = $state<TaskDialog>();
  let mode = $state("normal");
  let message = $state("");
  let tc = $state<any>({});
  let backlinks = $state.raw<Hit[]>([]);
  let showBacklinks = $state(false);
  let mentions = $state.raw<Mention[]>([]);
  let graph = $state.raw<NoteGraph | null>(null);
  let showGraph = $state(false);
  let showChat = $state(false);
  let chat = $state<Chat>();
  let reload = $state(0);
  const stored = (k: string, d: string) => { try { return localStorage.getItem(k) ?? d; } catch { return d; } };
  const store = (k: string, v: string) => { try { localStorage.setItem(k, v); } catch {} };
  let motion = $state(false); // view-change motion, on after the first load
  let topbar = $state(stored("sidebar", "1") === "1"); // key kept from the old sidebar toggle
  let recent = $state<string[]>(JSON.parse(stored("recent", "[]")));

  const tab = $derived(tabs[cur]);
  const isText = (t?: Tab) => t?.kind === "file" || t?.kind === "report";
  const call = <T = any,>(cmd: string, args?: Record<string, unknown>) => invoke<T>(cmd, args);
  const pick = (o: PickOpts) => picker!.open(o);
  const ask = (prompt: string, initial = "", hint?: string) => pick({ prompt, initial, hint }) as Promise<string | null>;
  const sep = () => (cfg.notes.includes("\\") ? "\\" : "/");
  const join = (a: string, b: string) => a.replace(/[\\/]$/, "") + sep() + b;
  const base = (p: string) => p.split(/[\\/]/).pop()!;
  const rel = (p: string) => (p.startsWith(cfg.notes) ? p.slice(cfg.notes.length + 1) : p);
  const niceName = (p: string) => base(p).replace(/\.org(_archive)?$/, "").replace(/^\d{14}-/, "").replace(/_/g, " ");
  const pad = (n: number) => String(n).padStart(2, "0");
  const kw = () => ({ todo: cfg.config.todo_keywords as string[], done: cfg.config.done_keywords as string[] });
  /** Reset zoom. Tauri's hotkey script keeps its own zoom counter, so replay its Ctrl/Cmd+0 (resets counter and webview); setZoom covers Windows, which has no script. */
  function resetZoom() {
    setTimeout(() => window.dispatchEvent(new KeyboardEvent("keydown", { key: "0", ctrlKey: true, metaKey: true })), 0);
    getCurrentWebview().setZoom(1);
  }
  const isMac = typeof navigator !== "undefined" && /Mac/.test(navigator.platform);

  let msgTimer: ReturnType<typeof setTimeout>;
  function flash(m: string) {
    message = m;
    if (isError(m)) lastError = m;
    clearTimeout(msgTimer);
    msgTimer = setTimeout(() => (message = ""), 6000);
  }

  /** The last ⚠ message shown in the message line, for "Report a bug". */
  let lastError = "";
  /** Run an action, surfacing errors in the message line. */
  function act(f: () => unknown) {
    Promise.resolve().then(f).catch((e) => flash(`⚠ ${e}`));
  }

  // ---------------------------------------------------------------- tabs & files

  const stateOf = (t: Tab) => (t === tabs[cur] && isText(t) ? view.state : t.state!);
  const fileTab = (path: string) => tabs.find((t) => t.kind === "file" && t.path === path);

  function newState(key: string, text: string, path?: string, readOnly = false) {
    return ed.createState(text, {
      org: !path || /\.org(_archive)?$/.test(path),
      readOnly,
      ...kw(),
      path,
      onChange: () => {
        if (syncing) return;
        const t = tabs.find((x) => x.key === key);
        if (t?.kind !== "file") return;
        t.dirty = true;
        // Notes save themselves shortly after you stop typing, so tasks and links update live.
        // Config and the raw time log only save on :w / tab switch, since half-typed edits there break things.
        if (t.path!.startsWith(cfg.notes) && /\.org(_archive)?$/.test(t.path!)) {
          clearTimeout(saveTimers.get(key));
          saveTimers.set(key, setTimeout(() => act(() => saveTab(tabs.find((x) => x.key === key))), 700));
        }
      },
    });
  }

  async function show(i: number) {
    const old = tabs[cur];
    if (old && isText(old)) {
      old.state = view.state;
      await saveTab(old);
    }
    cur = i;
    const t = tabs[i];
    if (motion) tick().then(() => replay(isText(t) ? editorEl : document.querySelector(`.pane > .view[data-key="${CSS.escape(t?.key ?? "")}"]`)));
    if (t?.state) {
      view.setState(t.state);
      ed.onModeChange(view, (m) => (mode = m));
      mode = "normal";
      await tick();
      view.focus();
    }
    refreshBacklinks();
  }

  /** Restart the view-enter animation on EL (it only replays on its own when display toggles, not note to note). */
  function replay(el?: Element | null) {
    if (!el) return;
    el.classList.remove("view-in");
    void (el as HTMLElement).offsetWidth;
    el.classList.add("view-in");
  }

  /** FORCE (from :w / Ctrl+S) also overwrites a file that changed on disk in a conflicting way, or recreates a deleted one.
   *  Saves of one tab run one at a time, so `saved` is what the disk should hold when each starts. */
  const saveQueue = ed.perKey();
  const saveTab = (t?: Tab, force = false) => (t ? saveQueue(t.key, () => writeTab(t, force)) : Promise.resolve());
  async function writeTab(t: Tab, force: boolean): Promise<void> {
    if (t.kind !== "file" || !(t.dirty || (t.missing && force)) || ((t.conflict || t.missing) && !force)) return;
    t.conflict = t.missing = false;
    clearTimeout(saveTimers.get(t.key));
    const text = stateOf(t).doc.toString();
    let warn: string;
    try {
      warn = await call("write_file", { path: t.path, text, expect: force ? undefined : t.saved });
    } catch (e) {
      if (e !== "changed-on-disk") throw e;
      // Edited elsewhere since the last sync (the watcher hasn't caught up): merge or flag it, then save the result.
      const disk = ((await call("read_file", { path: t.path })) as string).replace(/\r\n?/g, "\n");
      if (disk !== t.saved) mergeFromDisk(t, disk);
      t.saved = disk;
      return writeTab(t, false);
    }
    t.saved = text;
    // Keystrokes typed while the write was in flight keep the tab dirty.
    if (stateOf(t).doc.toString() === text) t.dirty = false;
    if (t.path === cfg.log_path || t.path === cfg.projects_path) {
      const r: string = await call("tc_report", { kind: "doctor" });
      if (!r.includes("No issues")) flash("Saved — the time log or projects have problems, see Time → Check log");
    } else if (t.path === cfg.config_path) {
      cfg = await call("config");
      flash(warn ? `⚠ Settings reloaded. ${warn}` : "Settings reloaded.");
    }
    reload++;
    refreshBacklinks();
    refreshTc();
  }

  async function saveAll() {
    for (const t of tabs) await saveTab(t);
  }

  async function openFile(path: string, line?: number) {
    let i = tabs.findIndex((t) => t.kind === "file" && t.path === path);
    if (i < 0) {
      const text: string = await call("read_file", { path });
      tabs.push({ key: path, title: niceName(path), kind: "file", path, dirty: false, saved: text, state: newState(path, text, path) });
      i = tabs.length - 1;
    }
    await show(i);
    if (line != null) {
      const pos = view.state.doc.line(Math.min(line + 1, view.state.doc.lines)).from;
      view.dispatch({ selection: { anchor: pos }, effects: EditorView.scrollIntoView(pos, { y: "center" }) });
    }
    if (path.startsWith(cfg.notes) && path.endsWith(".org")) {
      recent = [path, ...recent.filter((p) => p !== path)].slice(0, 12);
      store("recent", JSON.stringify(recent));
    }
  }

  async function openReport(key: string, title: string, text: string, report?: Tab["report"]) {
    const state = newState(key, text, undefined, true);
    let i = tabs.findIndex((t) => t.key === key);
    if (i < 0) {
      tabs.push({ key, title, kind: "report", dirty: false, state, report });
      i = tabs.length - 1;
    } else {
      tabs[i].state = state;
      tabs[i].report = report;
      if (i === cur) cur = -1; // force the new state in
    }
    await show(i);
  }

  async function openView(kind: View) {
    let i = tabs.findIndex((t) => t.key === kind);
    if (i < 0) {
      tabs.push({ key: kind, title: { agenda: "Today", todo: "Tasks", time: "Time", projects: "Projects", settings: "Settings" }[kind], kind, dirty: false });
      i = tabs.length - 1;
    }
    await show(i);
  }

  // Saved views that don't fit in the top bar fold into its More button: FIT of them stay in the bar.
  let viewsEl = $state<HTMLElement>();
  let fit = $state(Infinity);
  const folded = $derived<{ name: string; query: string }[]>((cfg?.config.views ?? []).slice(fit));
  /** Fit saved views into the room the bar has (the views row plus the spacer after it), keeping room for More. */
  function fitViews() {
    if (!viewsEl) return;
    const btns = [...viewsEl.querySelectorAll<HTMLElement>(":scope > button")];
    const saved = btns.filter((b) => b.classList.contains("saved"));
    const more = btns.find((b) => b.classList.contains("more"))!;
    const gap = parseFloat(getComputedStyle(viewsEl).columnGap) || 0;
    const w = (b: HTMLElement) => b.offsetWidth + gap;
    let room = viewsEl.clientWidth + (viewsEl.nextElementSibling as HTMLElement).offsetWidth;
    for (const b of btns) if (b !== more && !saved.includes(b)) room -= w(b);
    let n = 0;
    for (; n < saved.length; n++) if ((room -= w(saved[n])) < (n < saved.length - 1 ? w(more) : 0)) break;
    fit = n;
    tick().then(placeInd);
  }
  // The open view's card in the bar: one indicator that slides to the `.on` button (More when the open view is folded).
  let ind = $state({ x: 0, y: 0, w: 0, h: 0, on: false });
  function placeInd() {
    const on = viewsEl?.querySelector<HTMLElement>(":scope > button.on:not(.fold)");
    ind = on ? { x: on.offsetLeft, y: on.offsetTop, w: on.offsetWidth, h: on.offsetHeight, on: true } : { ...ind, on: false };
  }
  $effect(() => {
    const nav = viewsEl;
    if (!nav) return;
    const ro = new ResizeObserver(fitViews);
    ro.observe(nav);
    ro.observe(nav.parentElement!);
    return () => ro.disconnect();
  });
  $effect(() => {
    void [JSON.stringify(cfg?.config.views), tab?.key]; // labels and the open view's weight change widths
    tick().then(fitViews);
  });
  async function moreViews() {
    const v = await pick({ prompt: "Saved views", items: folded.map((v) => ({ label: v.name, detail: v.query, value: v })) });
    if (v) await openSearch(v.name, v.query);
  }

  /** A task list filtered by QUERY (a saved view, or an ad-hoc search). */
  async function openSearch(title: string, query: string) {
    await call("search_todos", { query }); // report a bad query here rather than in an empty tab
    const key = `search:${query}`;
    let i = tabs.findIndex((t) => t.key === key);
    if (i < 0) {
      tabs.push({ key, title, kind: "todo", query, dirty: false });
      i = tabs.length - 1;
    }
    await show(i);
  }

  async function searchTasks() {
    const q = await ask("Search tasks", "", "e.g. todo:NEXT tag:work -tag:home pri:A file:inbox due:<=+7d scheduled:none \"some words\"");
    if (q?.trim()) await openSearch(q.trim(), q.trim());
  }

  async function saveView() {
    const query = tab?.query;
    if (query == null) return flash("Open a task search first (Space v f).");
    const old = cfg.config.views?.find((v: { query: string }) => v.query === query)?.name;
    const rename = old != null && (await pick({ prompt: `“${old}” already shows this search`, items: [{ label: "Rename existing view", value: "rename" }, { label: "Keep both", value: "both" }] }));
    if (rename === null) return;
    const name = await ask(rename === "rename" ? `Rename “${old}” to` : "Save this search as a view named", tab.title === query ? "" : tab.title, query);
    if (!name?.trim()) return;
    const warn: string = await ownCfg(() => call("save_view", { name: name.trim(), query, rename: rename === "rename" }));
    cfg = await call("config");
    flash(warn ? `⚠ Saved “${name.trim()}” to the top bar. ${warn}` : `Saved “${name.trim()}” to the top bar.`);
  }

  /** Setup prompts (first run, or from the palette); Esc at any step changes nothing. */
  async function setup() {
    const c = cfg.config;
    const hint = "Esc keeps the current settings";
    const notesDir = await ask("Setup: notes folder", c.notes_dir, hint);
    const dataDir = notesDir == null ? null : await ask("Setup: time-tracking folder", c.data_dir, hint);
    const profiles = dataDir == null ? null : await ask("Setup: profiles, comma-separated", c.profiles.join(", "), hint);
    if (profiles == null) return;
    const warn: string = await ownCfg(() => call("setup", { notesDir, dataDir, profiles }));
    cfg = await call("config");
    reload++;
    await refreshTc();
    flash(warn ? `⚠ Settings saved. ${warn}` : "Settings saved — Ctrl+, has the rest.");
  }

  /** The settings page's changed VALUES (and saved views or templates: TABLES); errors go back to the page. */
  async function saveSettings(values: Record<string, unknown>, reset: string[] = [], tables: Record<string, unknown> = {}) {
    const warn: string = await ownCfg(() => call("save_config", { values, reset, ...tables }));
    cfg = await call("config");
    reload++;
    await refreshTc();
    flash(warn ? `⚠ Settings saved. ${warn}` : "Settings saved.");
  }

  /** Write a missing config.toml from the current settings (Settings' button); errors go back to the page. */
  let ownCfgUntil = 0; // until when a config.toml watcher event is our own write's echo and needn't be announced
  /** Run a config-writing command, covering its in-flight time plus the watcher's debounce afterwards (not longer: a hand edit right after must still be announced). */
  async function ownCfg<T>(p: () => Promise<T>): Promise<T> {
    ownCfgUntil = Infinity;
    try { return await p(); } finally { ownCfgUntil = Date.now() + 600; }
  }
  async function recreateConfig() {
    const warn: string = await ownCfg(() => call("config_recreate"));
    cfg = await call("config");
    reload++;
    await refreshTc();
    syncFromDisk(); // an open config.toml tab shows the new file
    flash(warn ? `⚠ config.toml recreated. ${warn}` : "config.toml recreated from the current settings.");
  }

  /** Open config.toml, recreated from the current settings first if it's gone (saving an empty buffer would reset them). */
  async function editConfig() {
    cfg = await call("config");
    if (cfg.config_missing) await recreateConfig();
    await openFile(cfg.config_path);
  }

  // Search tabs take the name of the view with their query, or the query itself once no view has it.
  $effect(() => {
    for (const t of tabs) {
      if (t.query != null) t.title = cfg?.config.views?.find((v: { query: string }) => v.query === t.query)?.name ?? t.query;
    }
  });

  /** Recently closed tabs (newest last) for "Reopen closed tab": a file with its line, or a view / task search. */
  const closed: Entry[] = [];

  /** The open tabs (`restore_tabs`) for the next start: only paths and lines, never content; Today always opens first. */
  let restored = false; // nothing is written until the last session is back
  function saveSession() {
    if (!restored || !cfg?.config.restore_tabs) return;
    const ok = (t: Tab) => (t.kind === "file" ? !t.missing && ([cfg.config_path, cfg.log_path, cfg.projects_path].includes(t.path) || t.path!.startsWith(cfg.notes)) : t.kind !== "report" && t.kind !== "agenda");
    const keep = tabs.filter(ok);
    const entries = keep.map((t): Entry => {
      if (t.kind !== "file") return { kind: t.kind as View, query: t.query, title: t.title };
      const st = stateOf(t);
      return { path: t.path!, line: st.doc.lineAt(st.selection.main.head).number - 1 };
    });
    store("session", JSON.stringify({ tabs: entries, active: keep.indexOf(tabs[cur]) }));
  }
  let sessionTimer: ReturnType<typeof setTimeout>;
  $effect(() => {
    void [tabs.map((t) => t.key), cur];
    clearTimeout(sessionTimer);
    sessionTimer = setTimeout(saveSession, 1000);
  });

  /** Reopen the last session's tabs after Today (missing notes are skipped) and show its active one. */
  async function restoreSession() {
    const { tabs: entries, active } = cfg.config.restore_tabs ? parseSession(stored("session", "")) : { tabs: [], active: -1 };
    const r = recent; // reopening shouldn't reorder Recent
    let at = -1;
    for (const [i, c] of entries.entries()) {
      try {
        if ("path" in c) {
          if (!(await call<boolean>("file_exists", { path: c.path }))) continue; // a missing file would open empty
          await openFile(c.path, c.line);
        } else if (c.query != null) await openSearch(c.title, c.query);
        else await openView(c.kind);
        if (i === active) at = cur;
      } catch { /* e.g. a bad query or a path outside the folders */ }
    }
    if (entries.length) {
      store("recent", JSON.stringify((recent = r)));
      await show(at >= 0 ? at : 0);
    }
    restored = true;
  }

  /** Keep only the recent notes (Ctrl P's order) and the closed tabs whose path passes KEEP (for trashed or vanished notes). */
  function pruneRecent(keep: (p: string) => boolean) {
    const r = recent.filter(keep);
    if (r.length < recent.length) store("recent", JSON.stringify((recent = r)));
    for (let i = closed.length; i--; ) {
      const c = closed[i];
      if ("path" in c && !keep(c.path)) closed.splice(i, 1);
    }
  }

  async function reopenTab() {
    for (let c; (c = closed.pop()); ) {
      try {
        if ("path" in c) await openFile(c.path, c.line);
        else if (c.query != null) await openSearch(c.title, c.query);
        else await openView(c.kind);
        return;
      } catch { /* e.g. the file is gone; try the one before */ }
    }
    flash("No closed tabs");
  }

  /** Save tab T, ask before dropping unsaved edits that clash with the disk, put it on the Reopen stack and remove it.
   *  `cur` stays on the same tab (-1 if T was current); the caller shows a tab afterwards. Returns T's old index, or -1 if it stays. */
  async function detachTab(t: Tab) {
    await saveTab(t);
    // Unsaved edits that clash with the disk or whose file is gone (flagged earlier or by that save): ask before dropping them.
    if (t.kind === "file" && (t.conflict || t.missing) && t.dirty) {
      const c = await pick({
        prompt: `“${t.title}” has unsaved edits and ${t.missing ? "is gone from disk" : "changed on disk"}`,
        items: [{ label: "Keep editing", value: "keep" }, { label: t.missing ? "Save mine (recreates the file)" : "Save mine (overwrites the disk version)", value: "save" }, { label: "Discard mine", value: "discard" }],
      });
      if (c === "save") await saveTab(t, true);
      else if (c !== "discard") return -1;
    }
    const i = tabs.indexOf(t);
    if (i < 0) return -1;
    if (t.kind === "file" && t.path) {
      const st = stateOf(t);
      closed.push({ path: t.path, line: st.doc.lineAt(st.selection.main.head).number - 1 });
    } else if (t.kind !== "file" && t.kind !== "report") closed.push({ kind: t.kind, query: t.query, title: t.title });
    if (closed.length > 20) closed.shift();
    const c = tabs[cur];
    tabs.splice(i, 1);
    cur = c === t ? -1 : tabs.indexOf(c);
    return i;
  }

  /** After closing tabs: show the current tab if it's still open, else the one before index I, else Agenda. */
  const showAfterClose = (i: number) => (!tabs.length ? openView("agenda") : show(cur >= 0 ? cur : Math.min(Math.max(i - 1, 0), tabs.length - 1)));

  async function closeTab(i = cur) {
    const t = tabs[i];
    if (t && (i = await detachTab(t)) >= 0) await showAfterClose(i);
  }

  /** Close every tab but the current one (or ALL of them), highest first so each lands on the Reopen stack; conflict and deleted-with-edits tabs stay open.
   *  Shows a tab once at the end, so the editor and the backlinks update once. */
  async function closeOthers(all = false) {
    const keep = all ? undefined : tabs[cur];
    const at = cur;
    let kept = 0;
    for (const t of [...tabs].reverse()) {
      if (t === keep) continue;
      if (t.conflict || (t.missing && t.dirty)) kept++;
      else await detachTab(t);
    }
    if (kept) flash(`Kept ${kept} tab${kept > 1 ? "s" : ""} with conflicts or deleted files`);
    await showAfterClose(at + 1);
  }

  const cycleTab = (d: number) => tabs.length && show((cur + d + tabs.length) % tabs.length);

  async function switchTab() {
    const i = await pick({ prompt: "Switch to tab", items: tabs.map((t, i) => ({ label: t.title, detail: t.path ? rel(t.path) : "", value: i })) });
    if (i != null) await show(i);
  }

  async function textOf(path: string): Promise<string> {
    const t = fileTab(path);
    return t ? stateOf(t).doc.toString() : await call("read_file", { path });
  }

  // Undo for edits made from the task views: each action records the files' previous text.
  let recording: { path: string; text: string }[] | null = null;
  const undoStack: { path: string; text: string }[][] = [];

  async function recorded(f: () => Promise<void>) {
    recording = [];
    try {
      await f();
    } finally {
      if (recording.length) undoStack.push(recording);
      recording = null;
    }
  }

  async function undo() {
    const group = undoStack.pop();
    if (!group) return flash("Nothing to undo.");
    for (const r of group.reverse()) await toFile(r.path, async () => r.text);
    flash("Undone.");
  }

  /** Apply an async text transform to a file, through its open tab if there is one. */
  async function toFile(path: string, f: (text: string) => Promise<string>) {
    const t = fileTab(path);
    const old = await textOf(path);
    const next = await f(old);
    if (next === old) return;
    if (recording && !recording.some((r) => r.path === path)) recording.push({ path, text: old });
    if (!t) {
      const warn: string = await call("write_file", { path, text: next });
      if (warn) flash(`⚠ ${warn}`);
      reload++;
      return;
    }
    if (t === tabs[cur]) ed.applyText(view, next);
    else t.state = t.state!.update({ changes: ed.diffChange(old, next) }).state;
    t.dirty = true;
    await saveTab(t);
  }

  // ---------------------------------------------------------------- tasks

  const curLine0 = () => view.state.doc.lineAt(view.state.selection.main.head).number - 1;
  const inFile = () => tab?.kind === "file";
  const editorLoc = (): Loc | null => (inFile() ? { path: tab.path!, line: curLine0() } : null);
  const heading = async (loc: Loc) => call<Parts | null>("org_heading", { text: await textOf(loc.path), line: loc.line });
  const edit = (loc: Loc, op: string, value?: string) => toFile(loc.path, (text) => call("org_edit", { text, line: loc.line, op, value }));
  const dateKind = (it?: AgendaItem) => (it && /deadline|overdue|warning/.test(it.kind) ? "DEADLINE" : "SCHEDULED");

  function datePick(prompt: string) {
    return pick({
      prompt,
      items: [
        { label: "Today", value: "today" }, { label: "Tomorrow", value: "tomorrow" }, { label: "Next Monday", value: "mon" },
        { label: "In a week", value: "+1w" }, { label: "No date", value: "rm", detail: "remove" },
      ],
      allowCustom: true,
      preview: (q) => call("date_preview", { input: q }),
      hint: "or type fri, +3d, 12-24, 2026-12-24 14:00",
    }) as Promise<string | null>;
  }

  const taskOps: Record<string, (loc: Loc, it?: AgendaItem, arg?: number | string) => Promise<unknown>> = {
    async done(loc) {
      const h = await heading(loc);
      if (!h) return flash("Not on a task.");
      const k = kw();
      await edit(loc, "keyword", h.keyword && k.done.includes(h.keyword) ? k.todo[0] : k.done[0]);
    },
    async state(loc) {
      const k = kw();
      const v = await pick({ prompt: "Set status", items: [...k.todo, ...k.done, { label: "No status (plain heading)", value: "" }] });
      if (v != null) await edit(loc, "keyword", v);
    },
    cycle: (loc, _it, dir = 1) => edit(loc, "cycle", String(dir)),
    keyword: (loc, _it, k = "") => edit(loc, "keyword", String(k)),
    schedule: (loc) => planDate(loc, "SCHEDULED"),
    deadline: (loc) => planDate(loc, "DEADLINE"),
    async priority(loc) {
      const v = await pick({ prompt: "Priority", items: [{ label: "A — high", value: "A" }, { label: "B — medium", value: "B" }, { label: "C — low", value: "C" }, { label: "None", value: "" }] });
      if (v != null) await edit(loc, "priority", v);
    },
    "priority-up": (loc) => edit(loc, "priority-cycle", "1"),
    "priority-down": (loc) => edit(loc, "priority-cycle", "-1"),
    tags: (loc) => editTags(loc),
    move: (loc) => moveTask(loc),
    archive: (loc) => archiveTask(loc),
    later: (loc, it) => edit(loc, "shift", `${dateKind(it)} 1`),
    earlier: (loc, it) => edit(loc, "shift", `${dateKind(it)} -1`),
    clockIn: async (loc, it) => clockIn(it?.category ?? (await call("org_context", { path: loc.path, text: await textOf(loc.path), line: loc.line })).category),
    open: (loc) => openFile(loc.path, loc.line),
  };

  function atCursor(op: string, arg?: number) {
    const loc = editorLoc();
    if (!loc) return flash("Open a note and put the cursor on a task first.");
    return Promise.resolve(taskOps[op](loc, undefined, arg)).then(() => {
      if (tab === fileTab(loc.path)) ed.updateCookies(view);
    });
  }

  async function goToHeading() {
    if (!inFile()) return flash("Open a note first.");
    const hs = ed.headings(view.state);
    if (!hs.length) return flash("This note has no headings.");
    const h = await pick({ prompt: "Go to heading", items: hs.map((h) => ({ label: `${"  ".repeat(h.level - 1)}${h.text}`, value: h })) });
    if (h) await openFile(tab.path!, h.line);
  }

  async function insertTable() {
    if (!inFile()) return flash("Open a note first.");
    const size = await ask("Table size: columns × rows", "3x2", "e.g. 4x3 — then Tab moves between cells");
    const m = size?.match(/(\d+)\s*[x×*, ]\s*(\d+)/);
    if (m) ed.insertTable(view, Math.max(1, +m[1]), Math.max(1, +m[2]));
  }

  async function insertCodeBlock() {
    if (!inFile()) return flash("Open a note first.");
    const lang = await pick({ prompt: "Code block language", items: ["python", "javascript", "typescript", "rust", "shell", "sql", "json", "yaml", "html", "css", "go", "java", "c", "elisp", "text"], allowCustom: true });
    if (lang != null) ed.insertBlock(view, "src", lang === "text" ? "" : lang);
  }

  async function insertWebLink() {
    if (!inFile()) return flash("Open a note first.");
    const url = await ask("Link address (URL)");
    if (!url) return;
    const desc = await ask("Link text (optional)");
    if (desc == null) return;
    ed.insertText(view, desc ? `[[${url}][${desc}]]` : `[[${url}]]`);
    view.focus();
  }

  /** Alt+H/J/K/L act on the table when the cursor is in one, otherwise on headings. */
  const tableOr = (op: Parameters<typeof ed.tableOp>[1], other: () => unknown) => () => ed.tableOp(view, op) || other();
  const tableCmd = (op: Parameters<typeof ed.tableOp>[1]) => () => (inFile() && ed.tableOp(view, op)) || flash("Put the cursor in a table first.");
  let monoFont = $state(stored("mono-font", "0") === "1");

  async function planDate(loc: Loc, kind: "SCHEDULED" | "DEADLINE") {
    const input = await datePick(kind === "SCHEDULED" ? "Schedule for" : "Due date");
    if (input != null) await toFile(loc.path, (text) => call("org_planning", { text, line: loc.line, kind, input }));
  }

  async function editTags(loc: Loc) {
    const h = await heading(loc);
    if (!h) return flash("Not on a task.");
    let tags = [...h.tags];
    const all: string[] = await call("org_tags");
    const DONE = {};
    for (;;) {
      const names = [...new Set([...tags, ...all])];
      const v = await pick({
        prompt: `Tags: ${tags.length ? tags.join(", ") : "none"}`,
        items: [{ label: "✓ Done", value: DONE }, ...names.map((t) => ({ label: `${tags.includes(t) ? "☑" : "☐"} ${t}`, value: t }))],
        allowCustom: true,
        hint: "pick to toggle · type a new tag and press Ctrl+↵",
      });
      if (v == null) return;
      if (v === DONE || v === "") break;
      const t = String(v).trim().replace(/\s+/g, "_");
      tags = tags.includes(t) ? tags.filter((x) => x !== t) : [...tags, t];
    }
    await edit(loc, "tags", tags.join(":"));
  }

  async function moveTask(loc: Loc) {
    const targets: Target[] = await call("org_targets");
    const t: Target | null = await pick({
      prompt: "Move to…",
      items: targets.filter((x) => !(x.path === loc.path && x.line === loc.line)).map((x) => ({ label: x.label, value: x })),
      hint: "a file puts it at the top level; a heading files it underneath",
    });
    if (!t) return;
    if (t.path === loc.path) await toFile(loc.path, (text) => call("org_refile_same", { text, line: loc.line, dstLine: t.line }));
    else {
      const src = await textOf(loc.path);
      let newSrc = src;
      await toFile(t.path, async (dst) => {
        const [s, d] = await call<[string, string]>("org_refile", { src, line: loc.line, dst, dstLine: t.line });
        newSrc = s;
        return d;
      });
      await toFile(loc.path, async () => newSrc);
    }
    flash(`Moved to ${t.label}`);
  }

  async function archiveTask(loc: Loc) {
    const src = await textOf(loc.path);
    let newSrc = src;
    await toFile(loc.path + "_archive", async (dst) => {
      const [s, d] = await call<[string, string]>("org_archive", { src, line: loc.line, dst, srcPath: loc.path });
      newSrc = s;
      return d;
    });
    await toFile(loc.path, async () => newSrc);
    flash(`Archived to ${base(loc.path)}_archive`);
  }

  async function newTask(scheduled = "") {
    const files = (await call<string[]>("list_files")).map(rel);
    const inbox = cfg.config.inbox;
    await taskDialog!.open({ files: [inbox, ...files.filter((f) => f !== inbox)], scheduled }, async (spec) => {
      const entry: string = await call("task_entry", { title: spec.title, priority: spec.priority, tags: spec.tags, scheduled: spec.scheduled, deadline: spec.deadline });
      const path: string = await call("capture_path", { file: spec.file });
      await recorded(() => toFile(path, (text) => call("capture_insert", { text, heading: null, entry })));
      flash(`✓ Added “${spec.title}” to ${spec.file}`);
      reload++;
    });
  }

  type Tpl = { key: string; name: string; file: string; heading: string | null; body: string };
  const tplItems = (tpls: Tpl[]) => tpls.map((t) => ({ label: `${t.key}  ${t.name}`, detail: t.file || cfg.config.inbox, value: t }));

  /** Capture with a template (settings or templates folder): ask its prompts, file it, open at %?. */
  async function captureTemplate() {
    const tpls = await call<Tpl[]>("capture_templates");
    if (!tpls.length) return flash("No capture templates: add [[templates]] to config.toml (Space f C) or .org files in notes/templates/.");
    const t: Tpl | null = await pick({ prompt: "Capture with template", items: tplItems(tpls) });
    if (t) await fileTemplate(t);
  }

  /** File template T; HIDDEN (quick capture from a hidden window): don't open the file, just flash. */
  async function fileTemplate(t: Tpl, hidden = false) {
    const s = view.state.selection.main;
    const selection = !hidden && inFile() ? view.state.sliceDoc(s.from, s.to) : "";
    const answers: Record<string, string> = {};
    for (const p of await call<string[]>("template_prompts", { body: t.body })) {
      const a = await ask(p);
      if (a == null) return;
      answers[p] = a;
    }
    const path: string = await call("capture_path", { file: t.file || cfg.config.inbox });
    let at = [0, 0];
    await recorded(() =>
      toFile(path, async (text) => {
        const [next, line, col] = await call<[string, number, number]>("capture_template", { text, heading: t.heading ?? null, body: t.body, answers, selection });
        at = [line, col];
        return next;
      }),
    );
    if (!hidden) {
      await openFile(path, at[0]);
      view.dispatch({ selection: { anchor: view.state.doc.line(at[0] + 1).from + at[1] }, scrollIntoView: true });
    }
    flash(`✓ Captured “${t.name}” to ${rel(path)}`);
  }

  async function insertDate() {
    if (!inFile()) return;
    const input = await datePick("Insert date");
    if (input == null || input === "rm") return;
    const iso: string = await call("read_date", { input });
    const [y, m, d] = iso.split("-").map(Number);
    const time = input.match(/\d{1,2}:\d{2}/)?.[0];
    const day = new Date(y, m - 1, d).toLocaleDateString("en-US", { weekday: "short" });
    ed.insertText(view, `<${iso} ${day}${time ? " " + time : ""}>`);
    view.focus();
  }

  // ---------------------------------------------------------------- notes

  async function followLink(target = ed.linkAtCursor(view)) {
    if (!target) return flash("No link here.");
    if (target.startsWith("id:")) {
      const nodes: NoteNode[] = await call("notes_nodes");
      const n = nodes.find((n) => n.id === target.slice(3));
      return n ? openFile(n.path, n.line) : flash(`No note with ${target}`);
    }
    if (/^https?:\/\//.test(target)) return openUrl(target);
    const p = target.replace(/^file:/, "").replace(/::.*$/, "");
    const dir = tab?.path ? tab.path.replace(/[\\/][^\\/]*$/, "") : cfg.notes;
    await openFile(/^([\\/]|[A-Za-z]:)/.test(p) ? p : join(dir, p));
  }

  async function noteItems() {
    const [files, nodes, t] = await Promise.all([call<string[]>("list_files"), call<NoteNode[]>("notes_nodes"), call<Record<string, string>>("note_titles")]);
    const titles = new Map(Object.entries(t));
    const order = (p: string) => (recent.includes(p) ? recent.indexOf(p) : 1e9);
    return { files, nodes, titles, sorted: [...files].sort((a, b) => order(a) - order(b)) };
  }

  async function quickOpen() {
    const { files, nodes, titles, sorted } = await noteItems();
    const items = [
      ...sorted.map((p) => ({ label: titles.get(p) ?? niceName(p), detail: rel(p), value: p as unknown })),
      ...nodes.filter((n) => n.line > 0).map((n) => ({ label: n.title, detail: `heading in ${rel(n.path)}`, value: n as unknown })),
    ];
    const r = await pick({ prompt: "Open a note — or type a title to create one", items, allowCustom: true, hint: "Ctrl+↵ creates a note with the typed title" });
    if (r == null || r === "") return;
    if (typeof r === "object") return openFile(r.path, r.line);
    if (files.includes(r)) return openFile(r);
    await createNote(r);
  }

  async function createNote(title: string) {
    const n: NoteNode = await call("notes_new", { title });
    await openFile(n.path);
    view.dispatch({ selection: { anchor: view.state.doc.length } });
    ed.insertMode(view);
    return n;
  }

  async function newNote() {
    const title = await ask("New note title");
    if (title) await createNote(title);
  }

  /** Flash and return true when the tab's unsaved edits clash with the disk copy (rename/trash would lose them). */
  function refuseConflict(t: { conflict?: boolean; dirty?: boolean }): boolean {
    if (!(t.conflict && t.dirty)) return false;
    flash("Resolve the conflict first: :w keeps yours, Space f r loads theirs");
    return true;
  }

  /** Path of a note being renamed right now; `pruneMissing` skips it. */
  let renaming = "";

  /** Rename the open note in its folder; the backend fixes the [[file:]] links to it. */
  async function renameNote() {
    if (!inFile() || !tab.path!.startsWith(cfg.notes)) return flash("Open a note first.");
    const t = tab, from = t.path!;
    let name = (await ask("Rename note to", base(from)))?.trim();
    if (!name || name === base(from)) return;
    if (!/\.org(_archive)?$/.test(name)) name += ".org";
    await saveAll(); // links are rewritten on disk
    if (refuseConflict(t)) return; // saveAll skips conflict tabs, so the buffer is not what the disk holds
    renaming = from; // the watcher's sync must not prune FROM before the remaps below
    let to: string, changed: string[];
    try {
      [to, changed] = await call<[string, string[]]>("rename_note", { from, name });
    } finally {
      renaming = "";
    }
    // Re-key the tab: a fresh editor state, since autosave looks tabs up by key.
    const old = stateOf(t), text = old.doc.toString();
    // A sync during the move may have flagged the old path as missing; the trailing sync re-checks.
    Object.assign(t, { key: to, path: to, title: niceName(to), state: newState(to, text, to), saved: text, dirty: false, conflict: false, missing: false });
    if (t === tabs[cur]) {
      view.setState(t.state!);
      view.dispatch({ selection: old.selection });
      ed.onModeChange(view, (m) => (mode = m));
      mode = "normal";
    }
    recent = recent.map((p) => (p === from ? to : p));
    store("recent", JSON.stringify(recent));
    for (const c of closed) if ("path" in c && c.path === from) c.path = to; // so "Reopen closed tab" finds it
    syncFromDisk();
    const n = changed.filter((p) => p !== to).length;
    flash(`Renamed to ${base(to)}${n ? ` — updated links in ${n} note${n > 1 ? "s" : ""}` : ""}`);
  }

  /** Move the open note to <notes>/.trash/ after a confirm naming how many notes link to it (links are left as they are). */
  async function trashNote() {
    if (!inFile() || !tab.path!.startsWith(cfg.notes)) return flash("Open a note first.");
    const t = tab, path = t.path!;
    const n = new Set((await call<Hit[]>("notes_backlinks", { path })).map((h) => h.path)).size;
    const links = n ? ` ${n} note${n > 1 ? "s link" : " links"} to it; those links will break.` : "";
    const ok = await pick({ prompt: `Move “${t.title}” to the notes .trash folder?${links}`, items: [{ label: "Cancel", value: false }, { label: "Move to .trash", value: true }] });
    if (!ok) return;
    await saveTab(t);
    if (refuseConflict(t)) return; // the save can itself raise the conflict; trashing would drop the edits
    const to = await call<string>("trash_note", { path });
    t.dirty = false; // nothing left to save: closing must not recreate it
    await closeTab(tabs.indexOf(t));
    pruneRecent((p) => p !== path);
    flash(`🗑 Moved to ${rel(to)}`);
  }

  /** Pick a note from <notes>/.trash (newest first) and move it back to where it came from. */
  async function restoreNote() {
    const items = (await call<[string, number][]>("trashed_notes")).map(([p, secs]) => ({
      label: rel(p).replace(/^\.trash[\\/]/, "").replace(/\.org(\.\d+)?$/, ".org"),
      detail: `trashed ${new Date(secs * 1000).toLocaleString()}`,
      value: p,
    }));
    if (!items.length) return flash("Nothing in .trash");
    const from = await pick({ prompt: "Restore note from .trash", items });
    if (!from) return;
    const to = await call<string>("restore_note", { path: from });
    await openFile(to);
    flash(`Restored ${rel(to)}`);
  }

  async function insertLink() {
    if (!inFile()) return flash("Open a note first.");
    const loc = { path: tab.path!, pos: view.state.selection.main.head };
    const { nodes, sorted, titles } = await noteItems();
    const withId = new Set(nodes.filter((n) => n.line === 0).map((n) => n.path));
    const items = [
      ...nodes.map((n) => ({ label: n.title, detail: rel(n.path), value: n as unknown })),
      ...sorted.filter((p) => !withId.has(p) && p !== loc.path).map((p) => ({ label: titles.get(p) ?? niceName(p), detail: rel(p), value: p as unknown })),
    ];
    const r = await pick({ prompt: "Link to note — or type a title to create one", items, allowCustom: true, hint: "Ctrl+↵ creates a new note" });
    if (r == null || r === "") return;
    let link: string;
    if (typeof r === "object") link = `[[id:${r.id}][${r.title}]]`;
    else if (sorted.includes(r)) link = `[[file:${relativeTo(loc.path, r)}][${titles.get(r) ?? niceName(r)}]]`; // org resolves file: links from the note's folder
    else {
      const n: NoteNode = await call("notes_new", { title: r });
      link = `[[id:${n.id}][${n.title}]]`;
      flash(`Created note “${r}”`);
    }
    view.dispatch({ changes: { from: loc.pos, insert: link }, selection: { anchor: loc.pos + link.length } });
    view.focus();
  }

  async function search() {
    const h: Hit | null = await pick({
      prompt: "Search all notes",
      source: async (q) =>
        q.trim().length < 2 ? [] : (await call<Hit[]>("notes_search", { query: q })).map((h) => ({ label: h.text, detail: `${h.title} · line ${h.line + 1}`, value: h })),
    });
    if (h) await openFile(h.path, h.line);
  }

  async function refreshBacklinks() {
    const path = tab?.kind === "file" ? tab.path : null;
    [backlinks, mentions, graph] = await Promise.all([
      showBacklinks && path ? call("notes_backlinks", { path }) : [],
      showBacklinks && path ? call("notes_unlinked", { path }) : [],
      showGraph && path ? call("notes_graph", { path }) : null,
    ]);
  }

  /** Turn an unlinked mention of the current note into an [[id:]] link, giving the note an ID first if needed. Undoable. */
  async function linkMention(m: Mention) {
    if (!inFile()) return;
    const path = tab.path!;
    let id = (await call<NoteNode[]>("notes_nodes")).find((n) => n.path === path && n.line === 0)?.id;
    await recorded(async () => {
      if (!id) await toFile(path, async (text) => { const [t, i] = await call<[string, string]>("notes_ensure_id", { text }); id = i; return t; });
      await toFile(m.path, async (text) => {
        const lines = text.split("\n"), l = lines[m.line] ?? "";
        if (l.trim() !== m.text) throw new Error("That line changed — try again.");
        lines[m.line] = `${l.slice(0, m.col)}[[id:${id}][${l.slice(m.col, m.col + m.len)}]]${l.slice(m.col + m.len)}`;
        return lines.join("\n");
      });
    });
    flash(`Linked in ${m.title} — Space n u undoes it`);
    await refreshBacklinks();
  }

  async function pickBacklink() {
    if (!inFile()) return;
    const hits: Hit[] = await call("notes_backlinks", { path: tab.path });
    if (!hits.length) return flash("Nothing links here yet.");
    const h = await pick({ prompt: "Notes linking here", items: hits.map((h) => ({ label: h.title, detail: h.text, value: h })) });
    if (h) await openFile(h.path, h.line);
  }

  const journalFile = () => `${cfg.config.daily_dir}/%Y-%m-%d.org`;

  async function journal(dateInput?: string) {
    await openFile(await call("capture_path", { file: journalFile(), dateInput, journal: true }));
    view.dispatch({ selection: { anchor: view.state.doc.length }, scrollIntoView: true });
  }

  /** Open the previous/next existing journal entry, counted from the current journal tab's day, else today. */
  async function journalStep(dir: 1 | -1) {
    const daily: string = cfg.config.daily_dir, t = new Date();
    const from = tab?.path && journalNeighbour([tab.path], daily, "0000-00-00", 1) ? base(tab.path).slice(0, 10) : `${t.getFullYear()}-${pad(t.getMonth() + 1)}-${pad(t.getDate())}`;
    const f = journalNeighbour(await call<string[]>("list_files"), daily, from, dir);
    if (f) await openFile(f);
    else flash(`No ${dir < 0 ? "earlier" : "later"} journal entry`);
  }

  async function journalPick() {
    const d = await datePick("Open journal for");
    if (d && d !== "rm") await journal(d);
  }

  async function journalEntry() {
    const text = await ask("Add a line to today's journal");
    if (!text) return;
    const path: string = await call("capture_path", { file: journalFile(), journal: true });
    const t = new Date();
    await toFile(path, (txt) => call("capture_insert", { text: txt, heading: null, entry: `* ${pad(t.getHours())}:${pad(t.getMinutes())} ${text}` }));
    flash("Added to today's journal.");
  }

  /** Show PROMPT exactly; on YES, the local model's draft for it (null on NO or an error, which flashes). */
  async function aiDraft(prompt: string, yes: string, no: string, oneLine: boolean): Promise<string | null> {
    const model = cfg.config.ai_model;
    const go = await pick({ prompt: `${yes} with ${model}? It gets exactly this, on this machine:`, body: prompt, items: [{ label: yes, value: true }, { label: no, value: false }] });
    if (!go) return null;
    flash(`Drafting with ${model}…`);
    drafting++;
    try {
      const d = await call<string>("ai_draft", { prompt, oneLine });
      message = "";
      return d;
    } catch (e) {
      flash(`⚠ ${e}`);
      return null;
    } finally {
      drafting--;
    }
  }

  /** The part of the open note the chat sends (#211): selection, enclosing section or the note. */
  function chatPart() {
    const { from, to, head } = view.state.selection.main;
    return chatContext(view.state.doc.toString(), from, to, head, tab!.title);
  }

  const chatNote = () => { const c = chatPart(); return [c.label === tab!.title ? c.label : `${tab!.title}, ${c.label}`, c.text]; };

  /** The chat panel's answer to MESSAGES, about the open note's chatPart when WITH_NOTE (unsaved edits included). */
  async function chatAsk(messages: ChatMsg[], withNote: boolean, onPiece: (piece: string) => void): Promise<string> {
    const ch = new Channel<string>(onPiece);
    drafting++;
    chatting = true;
    try {
      return await call<string>("ai_chat", { messages, note: withNote && tab?.kind === "file" ? chatNote() : null, onPiece: ch });
    } finally {
      drafting--;
      chatting = false;
    }
  }

  /** Put a chat answer at the cursor (replacing the selection) in the open note. */
  function insertAtCursor(text: string) {
    view.dispatch(view.state.replaceSelection(text), { scrollIntoView: true });
    view.focus();
  }

  async function toggleChat() {
    showChat = !showChat;
    if (showChat) { await tick(); chat?.focus(); }
  }

  /** Drafts and chat answers in progress, for the status line's Cancel button (built-in models only; #149). */
  let drafting = $state(0);
  /** A chat answer is streaming in: cancellable over Ollama too (#210). */
  let chatting = $state(false);

  /** The model download in progress, for the status-line bar. */
  let download = $state<{ id: string; have: number; size: number } | null>(null);

  /** Settings for AI drafts: off, a model in Ollama, or one Margin downloads and runs (#132). */
  async function aiChoose() {
    const c = cfg.config;
    type Model = { id: string; name: string; size: number; ram_gb: number; licence: string; path: string; have: number; ready: boolean; runs: boolean };
    const models = await call<Model[]>("ai_models");
    const gb = (n: number) => `${(n / 1e9).toFixed(1)} GB`;
    const pct = (have: number, size: number) => `${Math.floor((have / size) * 100)}%`;
    const on = (b: string, m = "") => c.ai_model && c.ai_backend === b && (!m || c.ai_model === m) ? "✓ " : "";
    /** Save the choice, then flash MSG (plus the reload's warning, if any). */
    const set = async (backend: string, model: string, msg: string) => {
      const warn: string = await ownCfg(() => call("ai_set", { backend, model }));
      cfg = await call("config");
      flash(warn ? `⚠ ${msg} ${warn}` : msg);
    };
    const ollama = async () => {
      const model = (await ask("Ollama model", c.ai_backend === "ollama" && c.ai_model ? c.ai_model : "llama3.2:3b", "as in `ollama pull <model>`"))?.trim();
      if (!model) return;
      await set("ollama", model, `Drafts come from ${model} in Ollama.`);
    };
    const later = (m: Model) => m.runs ? `Drafts come from ${m.name}.` : "This build of Margin can't run it itself; use Ollama.";
    const embedded = async (m: Model) => {
      if (m.ready) return set("embedded", m.id, later(m));
      if (download) return flash(download.id === m.id ? `${m.name} is downloading.` : "Another model is downloading; cancel it first (AI drafts: choose model…).");
      const go = await pick({
        prompt: `Download ${m.name}?`,
        body: `${gb(m.size - m.have)} to download${m.have ? ` (${pct(m.have, m.size)} is already here)` : ""}, saved in\n${m.path}\n\nIt needs ~${m.ram_gb} GB of free RAM to run.\nLicence: ${m.licence}.`,
        items: [{ label: "Download", value: true }, { label: "Not now", value: false }],
      });
      if (!go) return;
      download = { id: m.id, have: m.have, size: m.size };
      const un = await listen<{ id: string; have: number; size: number }>("ai-download", (e) => (download = e.payload));
      try {
        await call("ai_download", { id: m.id });
        await set("embedded", m.id, `${m.name} is downloaded. ${later(m)}`);
      } finally {
        un();
        download = null;
      }
    };
    const run = await pick({
      prompt: "AI drafts",
      items: [
        { label: `${c.ai_model ? "" : "✓ "}Off`, value: () => set(c.ai_backend, "", "AI drafts are off.") },
        { label: `${on("ollama")}Ollama`, detail: c.ai_backend === "ollama" && c.ai_model ? c.ai_model : "a model you run in ollama.com", value: ollama },
        ...models.map((m) => ({
          label: `${on("embedded", m.id)}${m.name}`,
          detail: `${m.ready ? "downloaded" : download?.id === m.id ? `downloading, ${pct(download.have, m.size)}` : m.have ? `${pct(m.have, m.size)} downloaded` : `${gb(m.size)} download`} · needs ~${m.ram_gb} GB RAM · ${m.licence}`,
          value: () => embedded(m),
        })),
        ...(download ? [{ label: "Cancel download", detail: "what's here is kept, so it resumes next time", value: () => call("ai_download_cancel") }] : []),
        ...models.filter((m) => m.have && download?.id !== m.id).map((m) => ({
          label: `Delete ${m.name}`,
          detail: `frees ${gb(m.have)}`,
          value: () => call("ai_model_delete", { id: m.id }).then(() => flash(`Deleted ${m.name}.`)),
        })),
      ],
    });
    await run?.();
  }

  async function draftSummary() {
    if (!cfg.config.ai_model) return flash("Pick a model with AI drafts: choose model… (Space f a) to draft with a local model.");
    const draft = await aiDraft(await call("ai_day_prompt", { dateInput: "today" }), "Send", "Cancel", false);
    if (!draft) return draft === "" && flash("The model sent an empty draft.");
    const path: string = await call("capture_path", { file: journalFile(), journal: true });
    await recorded(() => toFile(path, (text) => call("capture_insert", { text, heading: null, entry: `* Summary\n${draft}` })));
    await journal();
    flash("Draft added under “Summary”: edit it, or Space n u to undo.");
  }

  async function openInbox() {
    await openFile(await call("capture_path", { file: cfg.config.inbox }));
  }

  // ---------------------------------------------------------------- time tracking

  async function refreshTc() {
    tc = await call("tc_status");
  }

  async function tcDo(cmd: string, args?: Record<string, unknown>) {
    flash(await call(cmd, args));
    await refreshTc();
    reload++;
  }

  /** Project picker; new projects also get an export code. Returns null on cancel. */
  async function pickProject(prompt: string, suggested?: string): Promise<{ project: string; code: string | null } | null> {
    const projects: Record<string, { export_code: string; active: boolean }> = await call("tc_projects");
    const names = Object.keys(projects).filter((p) => projects[p].active);
    const rest = names.filter((n) => n !== suggested);
    const items = !suggested ? names : names.includes(suggested) ? [suggested, ...rest] : [{ label: suggested, detail: "new project", value: suggested }, ...rest];
    const project = await pick({ prompt, items, allowCustom: true, hint: "type a name + Ctrl+↵ for a new project" });
    if (project == null) return null;
    let code: string | null = null;
    if (project && !projects[project]) {
      code = await ask(`Export code for the new project “${project}”`, "", "leave empty to use the name");
      if (code == null) return null;
    }
    return { project, code };
  }

  async function clockIn(suggested?: string) {
    if (suggested == null && tab?.kind === "file") {
      suggested = (await call("org_context", { path: tab.path, text: view.state.doc.toString(), line: curLine0() })).category;
    }
    const p = await pickProject("Track time on project", suggested);
    if (!p) return;
    const sugg: string[] = await call("tc_suggestions", { project: p.project });
    const task = (await pick({ prompt: `What are you working on? (optional)`, items: sugg, allowCustom: true, hint: "Esc to skip" })) ?? "";
    await tcDo("tc_in", { project: p.project, task, exportCode: p.code });
  }

  async function clockOut() {
    await refreshTc();
    if (tc.project == null) return flash("You're not tracking anything.");
    const note = await ask("What did you do?", tc.task ?? "", "goes into the work diary");
    if (note != null) await tcDo("tc_out", { note });
  }

  async function changeProject() {
    await refreshTc();
    if (tc.project == null) return clockIn();
    const note = await ask(`What did you do on “${tc.project}”?`, tc.task ?? "");
    if (note == null) return;
    const p = await pickProject("Switch to project");
    if (!p) return;
    await call("tc_out", { note });
    await tcDo("tc_in", { project: p.project, task: "", exportCode: p.code });
  }

  /** Back from SINCE–BACK away with a timer running: keep, discard, or discard and stop. */
  async function idleReturn({ since, back }: { since: string; back: string }) {
    await refreshTc();
    if (tc.project == null) return;
    const hm = (t: string) => t.slice(11, 16);
    const mins = Math.round((Date.parse(back) - Date.parse(since)) / 60000);
    const c = await pick({
      prompt: `You were away ${hm(since)}–${hm(back)} (${mins} min) while tracking “${tc.project || "Other"}”`,
      items: [
        { label: "Keep the time", value: "keep" },
        { label: "Discard idle time", detail: `clock back in from ${hm(back)}`, value: "discard" },
        { label: "Discard and stop", detail: `clock out at ${hm(since)}`, value: "stop" },
      ],
    });
    if (c == null || c === "keep") return;
    const note = c === "stop" ? await ask("What did you do?", tc.task ?? "", "goes into the work diary") : "";
    if (note != null) await tcDo("tc_idle", { since, back: c === "stop" ? null : back, note });
  }

  /** Ask WHAT happened (minutes ago, or a time of day) for the running clock; null if none or invalid. */
  async function askWhen(what: string) {
    await refreshTc();
    if (tc.project == null) return void flash("You're not tracking anything.");
    const m = await ask(`When did you actually ${what}?`, "", "minutes ago (25) or a time (14:05)");
    if (!m) return;
    const when = parseWhen(m, new Date());
    if (!when) flash("Enter minutes ago (25) or a time of day (14:05) that is not in the future.");
    return when;
  }

  async function adjustStart() {
    const when = await askWhen("start");
    if (when) await tcDo("tc_adjust", { minutes: Math.round((Date.parse(tc.since) - when.getTime()) / 60000) });
  }

  /** Forgot to stop: clock out at an earlier time. */
  async function stopEarlier() {
    const when = await askWhen("stop");
    if (!when) return;
    const note = await ask("What did you do?", tc.task ?? "", "goes into the work diary");
    if (note != null) await tcDo("tc_out_at", { at: naive(when), note });
  }

  async function report(kind: string, title: string, dateInput?: string | null) {
    await openReport(`report:${kind}`, title, await call("tc_report", { kind, dateInput }), { kind, dateInput });
  }

  async function dailyReport() {
    const d = await datePick("Day report for");
    if (d != null && d !== "rm") await report("daily", "Day report", d);
  }

  async function exportCsv() {
    const start = await datePick("Export from");
    if (start == null) return;
    const end = await datePick("Export to");
    if (end == null) return;
    const path = await ask("Save CSV as", join(cfg.export, `time_${tc.profile.toLowerCase()}.csv`));
    if (path) await tcDo("tc_csv", { start, end, path });
  }

  // ---------------------------------------------------------------- export

  /** Export the open note to the export folder; PDF opens the HTML in the browser to print from. */
  async function exportNote(format: "html" | "md" | "pdf") {
    if (!inFile()) return flash("Open a note first.");
    const path = await exportTo("export_note", { path: tab.path, text: view.state.doc.toString(), format });
    if (path) await exported(path, format === "pdf");
  }

  /** The open note plus the notes it links to by id:, as HTML/Markdown pages in a folder that link to each other, or one PDF. */
  async function exportLinked() {
    if (!inFile()) return flash("Open a note first.");
    const depth = await pick({ prompt: "Include notes", items: [{ label: "Linked from this note", value: 1 }, { label: "…and the notes they link to", value: 2 }] });
    if (depth == null) return;
    const format = await pick({
      prompt: "Export as",
      items: [{ label: "HTML", detail: "a page per note", value: "html" }, { label: "Markdown", detail: "a page per note", value: "md" }, { label: "PDF", detail: "one document, print from the browser", value: "pdf" }],
    });
    if (!format) return;
    const path = await exportTo("export_linked", { path: tab.path, text: view.state.doc.toString(), depth, format });
    if (path) await exported(path, format === "pdf");
  }

  /** Run export command CMD; if its target exists, ask to overwrite or keep both. Null on cancel. */
  async function exportTo(cmd: string, args: Record<string, unknown>): Promise<string | null> {
    try {
      return await call<string>(cmd, args);
    } catch (e) {
      const m = String(e).match(/^exists:(.*)$/s);
      if (!m) throw e;
      const overwrite = await pick({
        prompt: `${base(m[1])} already exists`,
        items: [{ label: "Overwrite", value: "y" }, { label: "Keep both", detail: "adds (2), (3)… to the name", value: "n" }, { label: "Cancel", value: "" }],
      });
      return overwrite ? call<string>(cmd, { ...args, overwrite: overwrite === "y" }) : null;
    }
  }

  async function exportReport() {
    const t = new Date();
    const day = (m: number, d: number) => new Date(t.getFullYear(), t.getMonth() + m, d).toLocaleDateString("sv-SE");
    let range: string[] | "custom" | null = await pick({
      prompt: "Time report for",
      items: [
        { label: "Last 7 days", value: [day(0, t.getDate() - 7), day(0, t.getDate())] },
        { label: "This month", value: [day(0, 1), day(0, t.getDate())] },
        { label: "Last month", value: [day(-1, 1), day(0, 0)] },
        { label: "Custom range…", value: "custom" },
      ],
    });
    if (range === "custom") {
      const start = await datePick("Report from");
      const end = start && (await datePick("Report to"));
      range = end ? [start, end] : null;
    }
    if (!range) return;
    const fmt = await pick({ prompt: "Export as", items: [{ label: "PDF (print from the browser)", value: "pdf" }, { label: "HTML", value: "html" }] });
    const path = fmt && (await exportTo("export_report", { start: range[0], end: range[1], print: fmt === "pdf" }));
    if (path) await exported(path, fmt === "pdf");
  }

  /** Flash where an export went; PRINT opens it right away, otherwise offer to. */
  async function exported(path: string, print: boolean) {
    if (print) {
      await call("export_open", { path });
      return flash(`Opened ${path} — print it to PDF from the browser`);
    }
    flash(`✓ Exported to ${path}`);
    const open = await pick({ prompt: `Exported to ${path}`, items: [{ label: "Open it", value: true }, { label: "Done", value: false }] });
    if (open) await call("export_open", { path });
  }

  async function switchProfile(name?: string) {
    name ??= await pick({ prompt: `Switch profile (now: ${tc.profile})`, items: cfg.config.profiles });
    if (!name) return;
    await tcDo("tc_switch_profile", { name });
    cfg = await call("config");
  }

  async function editSession(s?: Session) {
    if (!s) {
      const d = await datePick("Edit a session on");
      if (d == null) return;
      const ss: Session[] = await call("tc_sessions_on", { dateInput: d });
      if (!ss.length) return flash("No finished sessions that day.");
      s = await pick({
        prompt: "Which session?",
        items: ss.map((s) => ({ label: `${s.start.slice(0, 5)}–${s.end.slice(0, 5)}  ${s.project || "Other"}: ${s.desc || "(no description)"}`, detail: hm(s.hours), value: s })),
      });
      if (!s) return;
    }
    const note = await ask("Description", s.desc);
    if (note == null) return;
    const h = await ask(`Duration in hours (now ${s.hours.toFixed(2)})`, s.hours.toFixed(2));
    if (h == null || isNaN(+h)) return;
    await tcDo("tc_edit_session", { line: s.line, note, oldHours: s.hours, newHours: +h });
  }

  /** Log an ActivityWatch suggestion: confirm the project, add an optional note. */
  async function acceptSuggestion(s: Suggestion) {
    const p = await pickProject(`Log ${s.start.slice(11, 16)}–${s.end.slice(11, 16)} to project`, s.project ?? undefined);
    if (!p) return;
    // A meeting's window titles say little more than its name: no draft.
    const prompt = cfg.config.ai_model && !s.meeting && (await call<string>("ai_note_prompt", { project: p.project || null, task: s.task, apps: s.apps, titles: s.titles }));
    const draft = prompt ? await aiDraft(prompt, "Draft the note", "Write it myself", true) : null;
    const note = await ask("What did you do? (optional)", draft ?? (s.meeting ? `Meeting: ${s.meeting}` : s.task) ?? "", "goes into the work diary");
    if (note == null) return;
    await tcDo("tc_add_session", { start: s.start, end: s.end, project: p.project, exportCode: p.code, note });
    // Another project than guessed: remember it for blocks with this title. The session is logged either way.
    if (p.project && p.project !== s.project && s.titles[0])
      await call("activity_learn", { title: s.titles[0], project: p.project, editor: s.editor }).catch((e) => flash(`Logged, but couldn't remember the project: ${e}`));
  }

  /** Accept S after changing its start and end (HH:MM on its day). */
  async function editSuggestion(s: Suggestion) {
    const time = async (what: string, iso: string) => {
      const v = await ask(`${what} (HH:MM)`, iso.slice(11, 16));
      if (v == null) return null;
      const m = v.trim().match(/^(\d{1,2}):?(\d\d)$/);
      if (!m) throw new Error(`“${v}” isn't a time like 09:30`);
      return `${s.start.slice(0, 10)}T${m[1].padStart(2, "0")}:${m[2]}:00`;
    };
    const start = await time("Start", s.start);
    const end = start && (await time("End", s.end));
    if (end) await acceptSuggestion({ ...s, start, end });
  }

  async function importEmacs() {
    const dir = await ask(`Import Emacs timeclock files into “${tc.profile}” from`, "~/timeclock");
    if (dir) await tcDo("tc_import", { dir });
  }

  // ---------------------------------------------------------------- updates

  /** Look for a newer release. QUIET (at startup) only mentions it; otherwise offer to install and restart. */
  let version = $state("");

  async function checkUpdate(quiet = false) {
    const meta = await call<ConstructorParameters<typeof Update>[0] | null>("update_check"); // stable or nightly, per config.toml
    const u = meta && new Update(meta);
    if (!u) return quiet || flash("Margin is up to date.");
    if (quiet) return flash(`Margin ${u.version} is available — Space f u to install.`);
    const yes = await pick({ prompt: `Install Margin ${u.version}${version ? ` (you have ${version})` : ""} and restart?`, items: [{ label: "Install and restart", value: true }, { label: "Not now", value: false }] });
    if (!yes) return;
    flash(`Downloading Margin ${u.version}…`);
    await u.download();
    await saveAll(); // the Windows installer closes the app as soon as it starts
    saveSession();
    await u.install();
    await relaunch();
  }

  const timeActions: Record<string, (arg?: any) => unknown> = {
    start: (p?: string) => clockIn(p), stop: clockOut, pause: () => tcDo("tc_break"), resume: () => tcDo("tc_resume"),
    switch: changeProject, adjust: adjustStart, stopEarlier, editSession: (s) => editSession(s), export: exportCsv, exportReport, projects: () => openView("projects"),
    profile: (n) => switchProfile(n), holidays: () => report("holidays", "Holidays"), daily: dailyReport,
    weekly: () => report("weekly", "Week report"), doctor: () => report("doctor", "Log check"), backup: () => tcDo("backup_now"),
    backupLog: () => report("backup", "Backup log"), rawLog: () => openFile(cfg.log_path), open: (loc) => openFile(loc.path, loc.line), import: importEmacs, acceptSuggestion, editSuggestion,
    flex: async () => flash(await call("tc_report", { kind: "flex" })), leave: async () => flash(await call("tc_leave_time")), diary: () => openFile(cfg.diary_path),
    /** Open the Time view and focus its "Time off" form (the view may still be mounting, so retry briefly). */
    timeOff: async () => {
      await openView("time");
      for (let n = 0; n < 10; n++) {
        await tick();
        const el = document.querySelector<HTMLInputElement>('input[aria-label="Kind of time off"]');
        if (el) { el.scrollIntoView({ block: "center" }); return el.focus(); }
        await new Promise((r) => setTimeout(r, 30));
      }
    },
  };
  const timeApi: TimeApi = { act, run: async (name, arg) => { await timeActions[name](arg); } };
  const agendaApi: AgendaApi = {
    run: (op, it, arg) => recorded(async () => { await taskOps[op]({ path: it.path, line: it.line }, it, arg); }),
    newTask: (s) => act(() => newTask(s ?? "")),
    undo,
    act,
  };

  // ---------------------------------------------------------------- commands

  const t = (key: string) => () => timeActions[key]();
  const cmds: Cmd[] = [
    { label: "Command palette", keys: ["Ctrl+K"], leader: "SPC", run: () => palette() },
    { label: "Open note…", keys: ["Ctrl+P"], leader: ".", run: quickOpen },
    { label: "Search in all notes", keys: ["Ctrl+Shift+F"], leader: "/", run: search },
    { label: "New task", keys: ["Ctrl+N"], leader: "a", run: () => newTask() },
    { label: "Capture with template…", leader: "c", run: captureTemplate },
    { label: "New note", keys: ["Ctrl+Shift+N"], leader: "n n", run: newNote },
    { label: "Rename note…", leader: "n r", run: renameNote },
    { label: "Delete note… (move to .trash)", leader: "n D", run: trashNote },
    { label: "Restore note from .trash…", leader: "n R", run: restoreNote },
    { label: "Note: show in file manager", leader: "n f", run: () => act(() => (inFile() ? revealItemInDir(tab.path!) : flash("Open a note first."))) },
    { label: "Note: copy file path", leader: "n y", run: () => act(async () => { if (!inFile()) return flash("Open a note first."); await navigator.clipboard.writeText(tab.path!); flash(`Copied ${tab.path}`); }) },
    { label: "Today's journal", keys: ["Ctrl+J"], leader: "j", run: () => journal() },
    { label: "Journal for another day…", leader: "n J", run: journalPick },
    { label: "Journal: previous entry", leader: "n [", run: () => journalStep(-1) },
    { label: "Journal: next entry", leader: "n ]", run: () => journalStep(1) },
    { label: "Add a line to today's journal", leader: "n e", run: journalEntry },
    { label: "Journal: draft summary of today (local model)", leader: "n s", run: draftSummary },
    { label: "Insert link to a note…", keys: ["Ctrl+L"], leader: "n i", ctx: "editor", run: insertLink },
    { label: "Show notes linking here", leader: "n b", run: () => { showBacklinks = !showBacklinks; return refreshBacklinks(); } },
    { label: "Jump to a note linking here…", leader: "n l", run: pickBacklink },
    { label: "Show note graph", leader: "n g", run: () => { showGraph = !showGraph; return refreshBacklinks(); } },
    { label: "AI chat (local model)", leader: "n a", run: toggleChat },
    { label: "Undo last edit made from a task view or the links panel", leader: "n u", run: () => undo().then(refreshBacklinks) },
    { label: "Follow link at cursor (also gf, Ctrl+click)", keys: ["Enter"], ctx: "normal", leader: "n o", run: () => (ed.linkAtCursor(view) ? (act(() => followLink()), true) : false) },

    { label: "Go to Today", keys: ["Ctrl+1"], leader: "v t", run: () => openView("agenda") },
    { label: "Go to All tasks", keys: ["Ctrl+2"], leader: "v a", run: () => openView("todo") },
    { label: "Go to Inbox", keys: ["Ctrl+3"], leader: "v i", run: openInbox },
    { label: "Go to Time tracking", keys: ["Ctrl+4"], leader: "v c", run: () => openView("time") },
    { label: "Search tasks…", leader: "v f", run: searchTasks },
    { label: "Save this search as a view…", leader: "v v", run: saveView },
    { label: "Toggle top bar", keys: ["Ctrl+\\"], leader: "v s", run: () => { topbar = !topbar; store("sidebar", topbar ? "1" : "0"); } },
    { label: "Settings", keys: ["Ctrl+,"], leader: "f c", run: () => openView("settings") },
    { label: "Settings: edit config.toml", leader: "f C", run: editConfig },
    { label: "AI drafts: choose model…", leader: "f a", run: aiChoose },
    { label: "Save", keys: ["Ctrl+S"], leader: "f s", run: () => saveTab(tab, true) },
    { label: "Reload from disk (discard unsaved changes)", leader: "f r", run: reloadFromDisk },
    { label: "Save all", leader: "f S", run: saveAll },
    { label: "About Margin", run: async () => { await openView("settings"); await tick(); document.querySelector(".settings .about")?.scrollIntoView({ block: "nearest" }); } },
    { label: "Check for updates", leader: "f u", run: () => checkUpdate() },
    { label: "Report a bug (opens a prefilled GitHub issue)", run: () => openUrl(bugReportUrl({ version, os: osName(navigator.userAgent), nightly: !!cfg?.config?.nightly_updates, error: lastError })) },
    { label: "Run setup again…", run: setup },
    { label: "Help: tutorial", leader: "?", run: async () => openFile((await call<[string, boolean]>("tutorial"))[0]) },
    { label: "Switch tab…", leader: "b b", run: switchTab },
    { label: "Next tab (also gt)", keys: ["Ctrl+Tab"], leader: "b n", run: () => cycleTab(1) },
    { label: "Previous tab (also gT)", keys: ["Ctrl+Shift+Tab"], leader: "b p", run: () => cycleTab(-1) },
    { label: "Close tab (also :q)", leader: "b d", run: () => closeTab() },
    { label: "Close other tabs", leader: "b o", run: () => closeOthers() },
    { label: "Close all tabs", leader: "b D", run: () => closeOthers(true) },
    { label: "Reopen closed tab", keys: ["Ctrl+Shift+T"], leader: "b u", run: reopenTab },

    { label: "Task: mark done / reopen", leader: "x x", run: () => atCursor("done") },
    { label: "Task: set status…", leader: "x t", run: () => atCursor("state") },
    { label: "Task: next status / later date", keys: ["Shift+ArrowRight"], ctx: "normal", run: () => ed.shiftTimestamp(view, 1, false) || (ed.onHeading(view) && (act(() => atCursor("cycle", 1)), true)) },
    { label: "Task: previous status / earlier date", keys: ["Shift+ArrowLeft"], ctx: "normal", run: () => ed.shiftTimestamp(view, -1, false) || (ed.onHeading(view) && (act(() => atCursor("cycle", -1)), true)) },
    { label: "Task: raise priority / bump date part", keys: ["Shift+ArrowUp"], ctx: "normal", run: () => ed.shiftTimestamp(view, 1) || (ed.onHeading(view) && (act(() => atCursor("priority-up")), true)) },
    { label: "Task: lower priority / bump date part", keys: ["Shift+ArrowDown"], ctx: "normal", run: () => ed.shiftTimestamp(view, -1) || (ed.onHeading(view) && (act(() => atCursor("priority-down")), true)) },
    { label: "Task: set priority…", leader: "x p", run: () => atCursor("priority") },
    { label: "Task: schedule…", leader: "x s", run: () => atCursor("schedule") },
    { label: "Task: due date…", leader: "x d", run: () => atCursor("deadline") },
    { label: "Task: tags…", leader: "x g", run: () => atCursor("tags") },
    { label: "Task: move to…", leader: "x m", run: () => atCursor("move") },
    { label: "Task: archive", leader: "x A", run: () => atCursor("archive") },
    { label: "Task: track time on it", leader: "x i", run: () => atCursor("clockIn") },
    { label: "New heading / list item / table row below", keys: ["Alt+Enter"], ctx: "editor", leader: "x h", run: tableOr("rowBelow", () => ed.newItem(view)) },
    { label: "New task heading below", keys: ["Alt+Shift+Enter"], ctx: "editor", leader: "x n", run: () => ed.newHeading(view, kw().todo[0]) },
    { label: "Turn line into heading / back", leader: "x *", run: () => inFile() && ed.toggleHeading(view) },
    { label: "Toggle checkbox (or click it)", leader: "x c", run: () => inFile() && ed.toggleCheckbox(view) },
    { label: "Update progress cookies [2/5]", leader: "x u", run: () => inFile() && ed.updateCookies(view) },
    { label: "Go to heading…", keys: ["Ctrl+Shift+O"], leader: "n h", run: goToHeading },
    { label: "Promote heading / move table column left", keys: ["Alt+H"], ctx: "editor", run: tableOr("colLeft", () => ed.shiftHeading(view, -1)) },
    { label: "Demote heading / move table column right", keys: ["Alt+L"], ctx: "editor", run: tableOr("colRightMove", () => ed.shiftHeading(view, 1)) },
    { label: "Promote heading with children / delete table column", keys: ["Alt+Shift+H"], ctx: "editor", run: tableOr("delCol", () => ed.shiftHeading(view, -1, true)) },
    { label: "Demote heading with children / insert table column", keys: ["Alt+Shift+L"], ctx: "editor", run: tableOr("colRight", () => ed.shiftHeading(view, 1, true)) },
    { label: "Move heading / table row up", keys: ["Alt+K"], ctx: "editor", run: tableOr("rowUp", () => ed.moveSubtree(view, -1)) },
    { label: "Move heading / table row down", keys: ["Alt+J"], ctx: "editor", run: tableOr("rowDown", () => ed.moveSubtree(view, 1)) },
    { label: "Table: delete row", keys: ["Alt+Shift+K"], ctx: "editor", leader: "T d", run: tableCmd("delRow") },
    { label: "Table: insert row below", keys: ["Alt+Shift+J"], ctx: "editor", leader: "T j", run: tableCmd("rowBelow") },
    { label: "Table: insert row above", leader: "T k", run: tableCmd("rowAbove") },
    { label: "Table: insert column", leader: "T l", run: tableCmd("colRight") },
    { label: "Table: delete column", leader: "T h", run: tableCmd("delCol") },
    { label: "Table: add separator line below", leader: "T -", run: tableCmd("sep") },
    { label: "Table: align", leader: "T a", run: tableCmd("align") },
    { label: "Table: recalculate formulas (#+TBLFM)", leader: "T r", run: tableCmd("recalc") },
    { label: "Table: sort by this column", leader: "T s", run: tableCmd("sort") },
    { label: "Table: sort by this column, descending", leader: "T S", run: tableCmd("sortDesc") },
    { label: "Insert table…", leader: "i t", run: insertTable },
    { label: "Insert code block…", leader: "i c", run: insertCodeBlock },
    { label: "Insert quote block", leader: "i q", run: () => inFile() && ed.insertBlock(view, "quote") },
    { label: "Insert web link…", leader: "i w", run: insertWebLink },
    { label: "Insert link to a note…", leader: "i l", run: insertLink },
    { label: "Insert date…", leader: "i d", run: insertDate },
    { label: "Clean up unused attachments…", run: cleanAttachments },
    { label: "Insert horizontal rule", leader: "i -", run: () => inFile() && ed.insertText(view, "\n-----\n") },
    { label: "Toggle monospace font for notes", leader: "v m", run: () => { monoFont = !monoFont; store("mono-font", monoFont ? "1" : "0"); } },
    { label: "View: reset zoom (Ctrl/Cmd+= / - / 0 zoom in / out / reset)", leader: "v 0", run: resetZoom },
    { label: "Fold / unfold all headings (Tab folds one)", keys: ["Shift+Tab"], ctx: "normal", run: () => ed.orgShiftTab(view) },

    { label: "Time: start tracking", leader: "t i", run: t("start") },
    { label: "Time: stop", leader: "t o", run: t("stop") },
    { label: "Time: take a break", leader: "t b", run: t("pause") },
    { label: "Time: resume after break", leader: "t r", run: t("resume") },
    { label: "Time: switch project", leader: "t c", run: t("switch") },
    { label: "Time: I started earlier…", leader: "t a", run: t("adjust") },
    { label: "Time: I stopped earlier…", leader: "t O", run: t("stopEarlier") },
    { label: "Time: day report", leader: "t t", run: t("daily") },
    { label: "Time: week report", leader: "t s", run: t("weekly") },
    { label: "Time: show flex balance", leader: "t f", run: t("flex") },
    { label: "Time: when to leave today", leader: "t w", run: t("leave") },
    { label: "Time: public holidays", leader: "t h", run: t("holidays") },
    { label: "Time: export CSV…", leader: "t e", run: t("export") },
    { label: "Time: switch profile…", leader: "t p", run: () => switchProfile() },
    { label: "Time: project settings", leader: "t P", run: t("projects") },
    { label: "Time: open work diary", leader: "t d", run: t("diary") },
    { label: "Time: edit raw log", leader: "t E", run: t("rawLog") },
    { label: "Time: edit a session…", leader: "t S", run: () => editSession() },
    { label: "Time: back up now", leader: "t B", run: t("backup") },
    { label: "Time: check log for problems", leader: "t D", run: t("doctor") },
    { label: "Time: backup log", leader: "t L", run: t("backupLog") },
    { label: "Time: log time off…", leader: "t l", run: t("timeOff") },
    { label: "Time: import from Emacs…", leader: "t M", run: t("import") },

    { label: "Export: note as HTML", leader: "e h", run: () => exportNote("html") },
    { label: "Export: note as Markdown", leader: "e m", run: () => exportNote("md") },
    { label: "Export: note as PDF", leader: "e p", run: () => exportNote("pdf") },
    { label: "Export: note and linked notes…", leader: "e a", run: exportLinked },
    { label: "Export: time report (HTML/PDF)…", leader: "e t", run: exportReport },
  ];

  const GROUPS: Record<string, string> = { f: "Files & settings", n: "Notes & journal", x: "Task at cursor", i: "Insert", T: "Table", t: "Time tracking", e: "Export", v: "Go to & view", b: "Tabs" };
  const leader: MenuNode[] = (() => {
    const root: MenuNode[] = Object.entries(GROUPS).map(([key, label]) => ({ key, label, children: [] }));
    for (const c of cmds) {
      if (!c.leader) continue;
      const [a, b] = c.leader.split(" ");
      const node = { key: b ?? a, label: c.label.replace(/^(Task|Time|Export): /, ""), run: () => act(c.run) };
      if (b) root.find((g) => g.key === a)!.children!.push(node);
      else root.unshift(node);
    }
    return root;
  })();

  const pretty = (k: string) =>
    k.replace("ArrowRight", "→").replace("ArrowLeft", "←").replace("ArrowUp", "↑").replace("ArrowDown", "↓").replace("Ctrl", isMac ? "⌘" : "Ctrl");

  async function palette() {
    const c: Cmd | null = await pick({
      prompt: "Run a command",
      items: cmds.map((c) => ({ label: c.label, detail: [c.keys?.map(pretty).join(" / "), c.leader && `Space ${c.leader}`].filter(Boolean).join(" · "), value: c })),
    });
    if (c) act(c.run);
  }

  // ---------------------------------------------------------------- keys

  /** "ctrl+shift+f" style name for a key event (letters by physical key, so Alt works on macOS). */
  function combo(e: KeyboardEvent) {
    const k = e.code.startsWith("Key") ? e.code.slice(3) : e.code.startsWith("Digit") ? e.code.slice(5) : e.key;
    return `${e.ctrlKey || e.metaKey ? "ctrl+" : ""}${e.altKey ? "alt+" : ""}${e.shiftKey ? "shift+" : ""}${k}`.toLowerCase();
  }

  /** Under 1000 px the graph / links panels are drawers over the pane (see the CSS); Esc outside the editor or a click on the pane closes them. */
  const drawerOpen = () => (showGraph || showBacklinks || showChat) && matchMedia("(max-width: 999px)").matches;
  const closeDrawer = () => { showGraph = showBacklinks = showChat = false; };

  function onKey(e: KeyboardEvent) {
    if (picker?.isOpen() || menu?.isOpen() || taskDialog?.isOpen()) return;
    const inEditor = isText(tab) && view.contentDOM.contains(e.target as Node);
    if (e.key === "Escape" && !inEditor && drawerOpen()) { closeDrawer(); e.preventDefault(); return; }
    const idle = inEditor && ed.vimIdle(view);
    const c = combo(e);
    for (const cmd of cmds) {
      if (!cmd.keys?.some((k) => k.toLowerCase() === c)) continue;
      if (cmd.ctx && !(inEditor && tab.kind === "file" && (cmd.ctx === "editor" || idle))) continue;
      const r = cmd.run();
      if (r === false) return;
      e.preventDefault();
      e.stopPropagation();
      if (r instanceof Promise) r.catch((err) => flash(`⚠ ${err}`));
      return;
    }
    const typing = e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement || e.target instanceof HTMLSelectElement;
    if (e.key === " " && !e.ctrlKey && !e.altKey && !e.metaKey && (inEditor ? idle : !typing)) {
      e.preventDefault();
      e.stopPropagation();
      menu!.show();
    }
  }

  // ---------------------------------------------------------------- lifecycle

  // ---------------------------------------------------------------- live sync

  let syncing = false;
  const saveTimers = new Map<string, ReturnType<typeof setTimeout>>();

  /** Replace a tab's text with TEXT without marking it edited. */
  function replaceText(t: Tab, text: string) {
    const st = stateOf(t);
    const old = st.doc.toString();
    if (old === text) return;
    syncing = true;
    try {
      if (t === tabs[cur]) ed.applyText(view, text);
      else t.state = st.update({ changes: ed.diffChange(old, text) }).state;
    } finally {
      syncing = false;
    }
  }

  /** Bring open tabs, views, backlink lists and the timer up to date with the disk; PATHS (the watcher's changed paths) limits the re-reads.
   *  If every changed path is an open tab holding what it last saved (our own save coming back), skip the views, timer and backlinks: writeTab refreshed them. */
  async function doSync(paths?: string[]) {
    const echo = new Set<string>();
    for (const t of tabs) {
      if (t.kind !== "file" || !changed(t.path!, paths)) continue;
      const disk = ((await call("read_file", { path: t.path })) as string).replace(/\r\n?/g, "\n");
      // Deleted or moved away (reads as ""): keep the text and `saved`, flag it, don't autosave it back.
      t.missing = disk === "" && !!t.saved && !(await call<boolean>("file_exists", { path: t.path }));
      if (!t.missing && disk === t.saved) echo.add(t.path!);
      if (t.missing || disk === t.saved || disk === stateOf(t).doc.toString()) continue;
      if (t.dirty) mergeFromDisk(t, disk);
      else replaceText(t, disk);
      t.saved = disk;
    }
    // Reports even then: writeTab doesn't regenerate them after a save of the time log or the diary.
    for (const t of tabs) if (t.kind === "report" && t.report) replaceText(t, await call("tc_report", t.report));
    if (paths?.every((p) => echo.has(p))) return;
    await pruneMissing(paths);
    reload++;
    await refreshTc();
    await refreshBacklinks();
  }

  /** Drop notes that no longer exist from the recent list and the closed tabs (other files are kept); with PATHS, only checks those. */
  async function pruneMissing(paths?: string[]) {
    const ps = [...new Set([...recent, ...closed.flatMap((c) => ("path" in c ? [c.path] : []))])].filter((p) => p.startsWith(cfg.notes) && p.endsWith(".org") && p !== renaming && changed(p, paths));
    const have = await Promise.all(ps.map((p) => call<boolean>("file_exists", { path: p })));
    const gone = new Set(ps.filter((_, i) => !have[i]));
    if (gone.size) pruneRecent((p) => !gone.has(p));
  }

  /** The file changed on disk while T has unsaved edits: apply the disk change too if the two
   *  edits don't overlap (e.g. a diary line appended while you edit above it); otherwise flag it. */
  function mergeFromDisk(t: Tab, disk: string) {
    const st = stateOf(t);
    const change = ed.mergeEdits(t.saved!, st.doc.toString(), disk);
    if (!change) {
      t.conflict = true;
      flash(`⚠ “${t.title}” was changed elsewhere while you edited it. :w keeps yours, Space f r loads theirs.`);
      return;
    }
    syncing = true;
    try {
      if (t === tabs[cur]) view.dispatch({ changes: change });
      else t.state = st.update({ changes: change }).state;
    } finally {
      syncing = false;
    }
    // Your edits were undone back to the saved text: the tab now matches the disk.
    if (stateOf(t).doc.toString() === disk) Object.assign(t, { dirty: false, conflict: false });
  }

  async function reloadFromDisk() {
    if (!inFile()) return;
    if (tab.missing) return flash(`“${tab.title}” is gone from disk; :w recreates it.`);
    const disk = ((await call("read_file", { path: tab.path })) as string).replace(/\r\n?/g, "\n");
    replaceText(tab, disk);
    Object.assign(tab, { saved: disk, dirty: false, conflict: false });
    flash("Reloaded from disk.");
  }

  // Coalesce bursts of change events into one sync at a time.
  let syncChain = Promise.resolve();
  let syncQueued = false;
  let syncPaths: string[] | undefined;
  /** Sync with the disk; PATHS (the watcher's changed paths) narrows it, none checks everything. */
  function syncFromDisk(paths?: string[]) {
    if (!cfg) return; // still starting up (focus/fs event before config loads); startup prunes anyway
    if (syncQueued) return void (syncPaths = mergeChanged(syncPaths, paths));
    syncQueued = true;
    syncPaths = paths;
    syncChain = syncChain.then(async () => {
      syncQueued = false;
      await doSync(syncPaths).catch((e) => flash(`⚠ ${e}`));
    });
  }

  /** Store attachments for the open note (ADD returns each link target) and link them at POS. */
  async function attach(add: (note: string) => Promise<string>[], pos?: number) {
    if (!inFile()) return flash("Open a note first.");
    const links = await Promise.all(add(tab.path!));
    ed.insertText(view, links.map((l) => `[[file:${l}]]`).join("\n"), pos);
    flash(`📎 Attached ${links.length} file${links.length > 1 ? "s" : ""}`);
  }

  /** Pasted images are saved as pasted-YYYYMMDD-HHMMSS.png next to the note's other attachments. */
  function onPaste(e: ClipboardEvent) {
    const file = [...(e.clipboardData?.files ?? [])].find((f) => f.type.startsWith("image/"));
    if (!file || !inFile()) return;
    e.preventDefault();
    e.stopPropagation();
    const d = new Date();
    const ext = file.type.slice(6).replace("jpeg", "jpg").replace(/\+.*/, "");
    const name = `pasted-${d.getFullYear()}${pad(d.getMonth() + 1)}${pad(d.getDate())}-${pad(d.getHours())}${pad(d.getMinutes())}${pad(d.getSeconds())}.${ext}`;
    act(async () => {
      const bytes = new Uint8Array(await file.arrayBuffer());
      await attach((note) => [invoke<string>("attach_bytes", bytes, { headers: { note: encodeURIComponent(note), name: encodeURIComponent(name) } })]);
    });
  }

  /** Lists attachments no note links to; the picked ones (or all) move to <attachments_dir>/.trash/. */
  async function cleanAttachments() {
    const files = await call<string[]>("unused_attachments");
    if (!files.length) return flash("Every attachment is linked from a note.");
    const all = `Move all ${files.length} to the attachments .trash folder`;
    const v = await pick({ prompt: "Unused attachments — pick one to move to .trash, or all", items: [{ label: all, value: files }, ...files.map((f) => ({ label: rel(f), value: [f] }))] });
    if (!v) return;
    await call("trash_attachments", { files: v });
    flash(`🗑 Moved ${v.length} file${v.length > 1 ? "s" : ""} to the attachments .trash folder`);
  }

  onMount(() => {
    if (!("__TAURI_INTERNALS__" in window)) {
      message = "This page only works inside the app window — run `npm run tauri dev` instead of opening it in a browser.";
      return;
    }
    getVersion().then((v) => (version = v), () => {}); // show nothing if it fails
    act(async () => {
      cfg = await call("config");
      view = new EditorView({ parent: editorEl! });
      ed.hooks.save = () => act(() => saveTab(tab, true));
      ed.hooks.close = () => act(() => closeTab());
      ed.hooks.follow = (target) => act(() => followLink(target ?? ed.linkAtCursor(view)));
      ed.hooks.tab = (d) => act(() => cycleTab(d));
      await openView("agenda");
      requestAnimationFrame(() => (motion = true)); // nothing animates on first load
      await restoreSession();
      await pruneMissing();
      const [tut, first] = await call<[string, boolean]>("tutorial");
      if (first) {
        await setup().catch((e) => flash(`⚠ ${e}`));
        await openFile(tut);
      }
      await refreshTc();
      if (cfg.config_error) flash(`⚠ config.toml couldn't be loaded (${cfg.config_error}); Margin keeps running on the last good settings (defaults if it never loaded). The file is untouched.`);
      if (!import.meta.env.DEV) checkUpdate(true).catch(() => {}); // offline or no release yet: stay quiet
    });
    const timer = setInterval(() => act(refreshTc), 60_000);
    const blur = () => (saveSession(), act(saveAll));
    const focus = () => syncFromDisk();
    const unlisten = listen<string[]>("fs-changed", (e) => syncFromDisk(e.payload));
    // config.toml edited outside the app: apply it (a broken file keeps the current settings).
    const unlistenCfg = listen("config-changed", () => act(async () => {
      let warn = "";
      const err = await call<string>("reload_config").then((w) => ((warn = w), ""), String);
      cfg = await call("config");
      reload++;
      syncFromDisk();
      await refreshTc();
      if (!err && Date.now() < ownCfgUntil) return; // our own write: "recreated" stays
      flash(err ? `⚠ config.toml changed on disk: ${err}` : `config.toml changed on disk — settings reloaded.${warn ? ` ⚠ ${warn}` : ""}`);
    }));
    const unlistenTray = listen<string>("tray", (e) => act(() => timeActions[e.payload]()));
    const unlistenOpen = listen<{ path: string; line: number }>("open-entry", (e) => act(() => openFile(e.payload.path, e.payload.line)));
    const unlistenIdle = listen<{ since: string; back: string }>("idle", (e) => act(() => idleReturn(e.payload)));
    // Quick capture (global shortcut or `margin --capture`): a task, or a template if there are any.
    // The payload says the window was hidden before, so hide it again.
    const unlistenCapture = listen<boolean>("capture", (e) => act(async () => {
      const tpls = await call<Tpl[]>("capture_templates");
      const t: Tpl | "task" | null = tpls.length ? await pick({ prompt: "Capture", items: [{ label: "Task", detail: cfg.config.inbox, value: "task" }, ...tplItems(tpls)] }) : "task";
      if (t === "task") await newTask();
      else if (t) await fileTemplate(t, e.payload);
      if (e.payload) await getCurrentWindow().hide();
    }));
    // Dropped files (real paths from Tauri) are copied next to the note and linked where they land.
    const unlistenDrop = getCurrentWebview().onDragDropEvent((e) => {
      dropCue = (e.payload.type === "enter" || e.payload.type === "over") && inFile();
      if (e.payload.type !== "drop" || !e.payload.paths.length) return;
      const { paths, position } = e.payload;
      const at = { x: position.x / devicePixelRatio, y: position.y / devicePixelRatio };
      act(() => attach((note) => paths.map((src) => call<string>("attach_file", { note, src })), view.posAtCoords(at) ?? undefined));
    });
    editorEl!.addEventListener("paste", onPaste, true);
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("blur", blur);
    window.addEventListener("beforeunload", saveSession);
    window.addEventListener("focus", focus);
    return () => {
      clearInterval(timer);
      unlisten.then((f) => f());
      unlistenCfg.then((f) => f());
      unlistenTray.then((f) => f());
      unlistenOpen.then((f) => f());
      unlistenIdle.then((f) => f());
      unlistenCapture.then((f) => f());
      unlistenDrop.then((f) => f());
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("blur", blur);
      window.removeEventListener("beforeunload", saveSession);
      window.removeEventListener("focus", focus);
    };
  });

  const NAV: { label: string; icon: string; keys: string; on: (t?: Tab) => boolean; run: () => unknown }[] = [
    { label: "Today", icon: "◎", keys: "1", on: (t) => t?.kind === "agenda", run: () => openView("agenda") },
    { label: "Tasks", icon: "☑", keys: "2", on: (t) => t?.key === "todo", run: () => openView("todo") },
    { label: "Inbox", icon: "⇣", keys: "3", on: (t) => !!t?.path && cfg && base(t.path) === base(cfg.config.inbox), run: openInbox },
    { label: "Journal", icon: "✎", keys: "J", on: (t) => !!t?.path && cfg && t.path.includes(sep() + cfg.config.daily_dir + sep()), run: () => journal() },
    { label: "Time", icon: "◷", keys: "4", on: (t) => t?.kind === "time", run: () => openView("time") },
  ];
</script>

<div class="app" class:mono={monoFont}>
  {#if topbar && cfg}
    <header class="topnav">
      <span class="logo" aria-hidden="true">m</span>
      <nav class="views" class:has-ind={ind.on} bind:this={viewsEl}>
        <span class="ind" class:slide={motion} class:on={ind.on} style:transform="translate({ind.x}px, {ind.y}px)" style:width="{ind.w}px" style:height="{ind.h}px" aria-hidden="true"></span>
        {#each NAV as n}
          <button class:on={n.on(tab)} title="{n.label} ({pretty('Ctrl')}+{n.keys})" onclick={() => act(n.run)}><span class="ico">{n.icon}</span><span class="lbl">{n.label}</span><kbd>{n.keys}</kbd></button>
        {/each}
        <button class:on={tab?.kind === "file" && !NAV.some((n) => n.on(tab))} title="Open a note ({pretty('Ctrl')}+P)" onclick={() => act(quickOpen)}><span class="ico">▤</span><span class="lbl">Notes</span><kbd>P</kbd></button>
        {#each cfg.config.views ?? [] as v, i}
          <button class="saved" class:fold={i >= fit} class:on={tab?.key === `search:${v.query}`} onclick={() => act(() => openSearch(v.name, v.query))} title={v.query}><span class="ico">⌕</span><span class="lbl">{v.name}</span></button>
        {/each}
        <!-- always rendered, so fitViews can measure it -->
        <button class="more" class:fold={!folded.length} class:on={folded.some((v) => tab?.key === `search:${v.query}`)} title="More saved views" onclick={() => act(moreViews)}><span class="ico">⋯</span><span class="lbl">More</span></button>
      </nav>
      <span class="grow"></span>
      <button class="cmdk" title="Search or run a command ({pretty('Ctrl')}+K)" onclick={() => act(palette)}><span class="ico">⌕</span><span class="lbl">Search or run a command…</span><kbd>{pretty("Ctrl")} K</kbd></button>
      <button class="set" class:on={tab?.kind === "settings"} title="Settings ({pretty('Ctrl')}+,)" onclick={() => act(() => openView("settings"))}><span class="ico">⚙</span></button>
      <div class="clockchip" class:on={tc.project != null}>
        <button class="what" title="Time tracking ({pretty('Ctrl')}+4)" onclick={() => act(() => openView("time"))}>
          <span class="dot"></span>
          <b>{tc.project != null ? tc.project || "No project" : tc.on_break != null ? `Break · ${tc.on_break}` : "Not tracking"}</b>
          <span class="today">{tc.today}</span>
        </button>
        {#if tc.project != null}
          <button onclick={() => act(t("pause"))} title="Take a break">Pause</button>
          <button onclick={() => act(t("stop"))} title="Stop tracking">Stop</button>
        {:else if tc.on_break != null}
          <button onclick={() => act(t("resume"))}>Resume</button>
        {:else}
          <button onclick={() => act(t("start"))}>Start</button>
        {/if}
        {#if tc.backup_error}<span class="warn" title={tc.backup_error}>⚠ <span>Backup failed</span></span>{/if}
      </div>
    </header>
  {/if}

  <div class="maincol">
    <nav class="tabs">
      {#each tabs as t, i (t.key)}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="tab" class:cur={i === cur} onmousedown={(e) => e.button === 1 && e.preventDefault()} onauxclick={(e) => e.button === 1 && act(() => closeTab(i))}>
          <button title={t.path ? rel(t.path) : t.title} onclick={() => act(() => show(i))}>{t.title}{t.dirty ? " •" : ""}</button>
          <button class="x" title="Close" onclick={() => act(() => closeTab(i))}>×</button>
        </div>
      {/each}
    </nav>

    <main>
      <div class="pane" onpointerdowncapture={() => drawerOpen() && closeDrawer()}>
        <div class="editor" bind:this={editorEl} style:display={isText(tab) ? "block" : "none"}></div>
        {#if dropCue}<div class="drop-cue">Drop to attach to {tab.title}</div>{/if}
        {#if cfg}
          {#each tabs.filter((t) => t.kind === "agenda" || t.kind === "todo" || t.kind === "time" || t.kind === "projects" || t.kind === "settings") as v (v.key)}
            <div class="view" data-key={v.key} style:display={v === tab ? "block" : "none"}>
              {#if v.kind === "time"}
                <Time api={timeApi} active={v === tab} {reload} />
              {:else if v.kind === "projects"}
                <Projects api={timeApi} active={v === tab} {reload} />
              {:else if v.kind === "settings"}
                <Settings {version} active={v === tab} {reload} config={cfg.config} defaults={cfg.defaults} error={cfg.config_error} missing={cfg.config_missing} save={saveSettings} recreate={recreateConfig} edit={() => act(editConfig)} />
              {:else}
                <Agenda mode={v.kind as "agenda" | "todo"} api={agendaApi} active={v === tab} {reload} {...kw()} query={v.query} title={v.query != null ? v.title : undefined} time={timeApi} />
              {/if}
            </div>
          {/each}
        {/if}
      </div>
      {#if showGraph}
        <aside class="links">
          <h3>Graph <small>{graph?.nodes.length ?? 0}</small></h3>
          {#if graph?.nodes.length}<Graph {graph} open={(p) => act(() => openFile(p))} />{:else}<p>Open a note to see its links.</p>{/if}
        </aside>
      {/if}
      <!-- hidden, not removed, so the chat survives closing the panel -->
      <aside class="links chat" style:display={showChat ? null : "none"}><Chat bind:this={chat} ask={chatAsk} note={tab?.kind === "file" ? tab.title : null} noteLabel={() => chatPart().label} insert={tab?.kind === "file" ? insertAtCursor : undefined} /></aside>
      {#if showBacklinks}
        <aside class="links">
          <h3>Linked from <small>{backlinks.length}</small></h3>
          {#each backlinks as b}
            <button onclick={() => act(() => openFile(b.path, b.line))}><strong>{b.title}</strong><span>{b.text}</span></button>
          {:else}
            <p>No notes link here yet. Use {pretty("Ctrl")}+L in another note to add one.</p>
          {/each}
          {#if mentions.length}
            <h3>Unlinked mentions <small>{mentions.length}</small></h3>
            {#each mentions as m}
              <div class="mention">
                <button onclick={() => act(() => openFile(m.path, m.line))}><strong>{m.title}</strong><span>{m.text}</span></button>
                <button class="link-it" title="Turn into a link" onclick={() => act(() => linkMention(m))}>Link</button>
              </div>
            {/each}
          {/if}
        </aside>
      {/if}
    </main>

    <footer class="status">
      {#if isText(tab)}<span class="mode">{mode.toUpperCase()}</span>{/if}
      <span class="file">{tab?.path ? rel(tab.path) : ""}{tab?.missing ? " • deleted on disk (:w recreates it)" : tab?.conflict ? " • changed on disk (:w keeps yours, Space f r reloads)" : tab?.dirty ? " • unsaved" : ""}</span>
      <span class="msg">{message}</span>
      {#if download}<progress class="dl" max={download.size} value={download.have} title="Downloading the AI model ({Math.floor((download.have / download.size) * 100)}%); cancel it in AI drafts: choose model…"></progress>{/if}
      {#if drafting && (cfg.config.ai_backend === "embedded" || chatting)}<button class="hint cancel" onclick={() => act(() => call("ai_draft_cancel"))}>Drafting… cancel</button>{/if}
      <button class="hint" onclick={() => act(palette)}>Space menu · {pretty("Ctrl")}+K commands</button>
    </footer>
  </div>
</div>

<Picker bind:this={picker} />
<Menu bind:this={menu} root={leader} />
<TaskDialog bind:this={taskDialog} />

<style>
  :global(html, body) { margin: 0; height: 100%; background: var(--bg); color: var(--fg); font: var(--fs-base) var(--sans); overflow: hidden; }
  :global(button) { font-family: inherit; }
  .app { display: flex; flex-direction: column; height: 100vh; }
  .app.mono { --editor-font: var(--mono); }
  :global(.cm-plain) { font-family: var(--mono); }
  .maincol { flex: 1; display: flex; flex-direction: column; min-width: 0; min-height: 0; background: var(--canvas); }

  /* top bar (round-5 mockups, #251): logo, views (the open one a raised card), search, settings, clock chip */
  .topnav { display: flex; align-items: center; gap: var(--s2); padding: var(--s2) var(--s4); background: var(--canvas); flex: none; min-width: 0; }
  .topnav button { background: none; border: 0; color: var(--dim); cursor: pointer; font-size: var(--fs-md); display: flex; align-items: center; gap: 7px; padding: 6px 10px; border-radius: 10px; white-space: nowrap; transition: background-color 120ms var(--ease), color 120ms var(--ease), box-shadow 120ms var(--ease); }
  .topnav button:hover { color: var(--fg); }
  .topnav button:focus-visible { outline: none; box-shadow: var(--ring); }
  .logo { width: 28px; height: 28px; border-radius: 9px; background: var(--accent); color: var(--bg); font: 700 15px/28px var(--sans); text-align: center; margin-right: var(--s2); flex: none; }
  .views { display: flex; gap: 2px; min-width: 0; overflow-x: auto; scrollbar-width: none; position: relative; }
  .views button { position: relative; } /* above the indicator */
  .views .fold { position: absolute; left: 0; top: 0; visibility: hidden; pointer-events: none; }
  .ind { position: absolute; left: 0; top: 0; border-radius: 10px; background: var(--card); box-shadow: var(--soft); opacity: 0; pointer-events: none; }
  .ind.on { opacity: 1; }
  .ind.slide { transition: transform 180ms var(--ease), width 180ms var(--ease), opacity 120ms var(--ease); }
  .views.has-ind button.on { background: none; box-shadow: none; }
  @media (prefers-reduced-motion: reduce) { .ind.slide { transition: none; } }
  .views button.on, .set.on { background: var(--card); color: var(--fg); box-shadow: var(--soft); font-weight: 500; }
  .views button.on .ico, .set.on .ico { color: var(--link); }
  .views button.on kbd { display: none; }
  .grow { flex: 1; }
  .ico { width: 16px; text-align: center; line-height: 1; font-size: var(--fs-lg); }
  .topnav .cmdk { width: 250px; flex: 0 1000 250px; min-width: 100px; background: var(--card); box-shadow: var(--soft); padding: 6px var(--s3); }
  .cmdk .lbl { flex: 1; text-align: left; overflow: hidden; text-overflow: ellipsis; }
  kbd { font: 10px var(--mono); color: var(--dim); border: 1px solid var(--border); border-radius: var(--radius-sm); padding: 0 var(--s1); background: var(--bg); }
  .clockchip { display: flex; align-items: center; gap: 2px; background: var(--card); box-shadow: var(--soft); border-radius: 20px; padding: 2px var(--s1); flex: none; }
  .clockchip button { padding: 3px var(--s2); border-radius: 16px; font-size: var(--fs-sm); }
  .clockchip button:not(.what):hover { background: var(--active); }
  .clockchip .what b { color: var(--fg); font-weight: 600; max-width: 160px; overflow: hidden; text-overflow: ellipsis; }
  .clockchip .today { font-family: var(--mono); }
  .clockchip.on .today { color: var(--done); }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--dim); flex: none; }
  .clockchip.on .dot { background: var(--done); }
  .warn { color: var(--todo); font-size: var(--fs-xs); padding-right: var(--s2); white-space: nowrap; }

  /* open tabs as pills on the canvas, the current one a raised card (round-5 mockups, #251) */
  .tabs { display: flex; gap: var(--s1); padding: var(--s1) var(--s4) var(--s2); overflow-x: auto; scrollbar-width: none; flex: none; min-height: 26px; }
  .tab { display: flex; align-items: center; border-radius: 20px; flex: none; transition: background-color 120ms var(--ease), box-shadow 120ms var(--ease); }
  .tab:hover { background: var(--active); }
  .tab button { background: none; border: 0; color: var(--dim); padding: var(--s1) 2px var(--s1) var(--s3); font-size: var(--fs-sm); cursor: pointer; white-space: nowrap; display: flex; align-items: center; width: 100%; border-radius: 20px; }
  .tab button:focus-visible { outline: none; box-shadow: var(--ring); }
  .tab .x { padding: 2px var(--s2) 2px var(--s1); opacity: 0; font-size: var(--fs-base); }
  .tab:hover .x, .tab.cur .x { opacity: 0.7; }
  .tab.cur { background: var(--card); box-shadow: var(--soft); }
  .tab.cur button { color: var(--fg); font-weight: 500; }
  main { flex: 1; display: flex; min-height: 0; }
  .pane { flex: 1; min-width: 0; position: relative; }
  .drop-cue { position: absolute; inset: 6px; border: 2px dashed var(--accent); border-radius: 8px; pointer-events: none; display: flex; align-items: flex-end; justify-content: center; padding-bottom: var(--s5); color: var(--accent); font-weight: 600; background: color-mix(in srgb, var(--accent) 6%, transparent); }
  .editor, .view { height: 100%; }
  /* the editor as a card on the canvas: border-box keeps CodeMirror (own scroller, Vim ex-line) inside the padding */
  .editor { box-sizing: border-box; padding: 0 var(--s4) var(--s3); }
  .editor :global(.cm-editor) { background: var(--card); border-radius: var(--r); box-shadow: var(--soft); overflow: hidden; }
  /* Graph, Chat and Linked from: each panel its own card */
  .links { width: min(280px, 35vw); background: var(--card); border-radius: var(--r); box-shadow: var(--soft); margin: 0 var(--s4) var(--s3) 0; overflow-y: auto; padding: 10px var(--s2); flex: none; }
  .links h3 { margin: var(--s1) 6px var(--s2); font-size: var(--fs-sm); color: var(--dim); text-transform: uppercase; letter-spacing: 0.05em; }
  .links button { display: block; width: 100%; text-align: left; background: none; border: 0; color: var(--fg); padding: 6px; border-radius: var(--radius); cursor: pointer; }
  .links button:hover { background: var(--active); }
  .links button span { display: block; color: var(--dim); font-size: var(--fs-sm); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .links p { color: var(--dim); margin: 6px; font-size: var(--fs-sm); }
  .links.chat { width: min(360px, 40vw); overflow: hidden; }
  .mention { display: flex; align-items: center; }
  .mention button:first-child { min-width: 0; }
  .links .link-it { width: auto; flex: none; color: var(--link); font-size: var(--fs-sm); }
  .status { display: flex; gap: var(--s3); align-items: center; padding: 0 var(--s4) var(--s2); color: var(--dim); font: var(--fs-xs) var(--mono); flex: none; min-height: 20px; }
  .mode { color: var(--bg); background: var(--accent); padding: 0 6px; border-radius: var(--radius-sm); font-weight: 700; }
  .file { color: var(--dim); }
  .msg { color: var(--fg); flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .dl { width: 120px; height: 8px; accent-color: var(--accent); }
  .hint { background: none; border: 0; color: var(--dim); font: var(--fs-xs) var(--sans); cursor: pointer; }
  /* narrow windows: compact top bar < 1200, links drawer < 1000, icon-only top bar < 760, short status bar < 560 */
  @media (max-width: 999px) {
    main { position: relative; }
    .links { position: absolute; right: 0; top: 0; bottom: 0; width: min(320px, 85vw); z-index: 10; box-shadow: var(--shadow); box-sizing: border-box; margin: 0; border-radius: var(--r) 0 0 var(--r); }
  }
  @media (max-width: 1199px) {
    .views kbd, .cmdk .lbl, .clockchip .warn span { display: none; }
    .topnav .cmdk { width: auto; flex: none; min-width: 0; }
  }
  @media (max-width: 759px) {
    .topnav { gap: 2px; padding: var(--s1) var(--s2); }
    .logo { margin-right: var(--s1); }
    .views button, .topnav .cmdk { padding: 6px 8px; }
    .topnav .lbl, .topnav kbd, .clockchip b, .clockchip .warn, .clockchip button:not(.what) { display: none; }
  }
  @media (max-width: 559px) {
    .status .file, .hint:not(.cancel) { display: none; }
  }
</style>
