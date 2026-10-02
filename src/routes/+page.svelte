<script lang="ts">
  import "$lib/theme.css";
  import { onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getVersion } from "@tauri-apps/api/app";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { check } from "@tauri-apps/plugin-updater";
  import { relaunch } from "@tauri-apps/plugin-process";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { EditorView } from "@codemirror/view";
  import type { EditorState } from "@codemirror/state";
  import * as ed from "$lib/editor";
  import { relativeTo } from "$lib/paths";
  import Picker, { type PickOpts } from "$lib/Picker.svelte";
  import Menu, { type MenuNode } from "$lib/Menu.svelte";
  import Agenda, { type AgendaApi, type AgendaItem } from "$lib/Agenda.svelte";
  import Time, { hm, type Session, type Suggestion, type TimeApi } from "$lib/Time.svelte";
  import TaskDialog from "$lib/TaskDialog.svelte";
  import Graph, { type NoteGraph } from "$lib/Graph.svelte";

  type View = "agenda" | "todo" | "time";
  /** SAVED: the text last read from / written to disk, to tell our own writes from outside changes.
   *  REPORT: how to regenerate a report tab when the data changes. */
  type Tab = {
    key: string; title: string; kind: "file" | "report" | View; path?: string; state?: EditorState; dirty: boolean;
    saved?: string; report?: { kind: string; dateInput?: string | null }; conflict?: boolean; query?: string;
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
  let reload = $state(0);
  const stored = (k: string, d: string) => { try { return localStorage.getItem(k) ?? d; } catch { return d; } };
  const store = (k: string, v: string) => { try { localStorage.setItem(k, v); } catch {} };
  let sidebar = $state(stored("sidebar", "1") === "1");
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
  const isMac = typeof navigator !== "undefined" && /Mac/.test(navigator.platform);

  let msgTimer: ReturnType<typeof setTimeout>;
  function flash(m: string) {
    message = m;
    clearTimeout(msgTimer);
    msgTimer = setTimeout(() => (message = ""), 6000);
  }

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
    if (t?.state) {
      view.setState(t.state);
      ed.onModeChange(view, (m) => (mode = m));
      mode = "normal";
      await tick();
      view.focus();
    }
    refreshBacklinks();
  }

  /** FORCE (from :w / Ctrl+S) also overwrites a file that changed on disk in a conflicting way. */
  async function saveTab(t?: Tab, force = false) {
    if (!t || t.kind !== "file" || !t.dirty || (t.conflict && !force)) return;
    t.conflict = false;
    clearTimeout(saveTimers.get(t.key));
    const text = stateOf(t).doc.toString();
    await call("write_file", { path: t.path, text });
    t.saved = text;
    // Keystrokes typed while the write was in flight keep the tab dirty.
    if (stateOf(t).doc.toString() === text) t.dirty = false;
    if (t.path === cfg.log_path) {
      const r: string = await call("tc_report", { kind: "doctor" });
      if (!r.includes("No issues")) flash("Saved — the time log has problems, see Time → Check log");
    } else if (t.path === cfg.config_path) {
      cfg = await call("config");
      flash("Settings reloaded.");
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
      tabs.push({ key: kind, title: { agenda: "Today", todo: "Tasks", time: "Time" }[kind], kind, dirty: false });
      i = tabs.length - 1;
    }
    await show(i);
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
    await call("save_view", { name: name.trim(), query, rename: rename === "rename" });
    cfg = await call("config");
    flash(`Saved “${name.trim()}” to the sidebar.`);
  }

  // Search tabs take the name of the view with their query, or the query itself once no view has it.
  $effect(() => {
    for (const t of tabs) {
      if (t.query != null) t.title = cfg?.config.views?.find((v: { query: string }) => v.query === t.query)?.name ?? t.query;
    }
  });

  async function closeTab(i = cur) {
    const t = tabs[i];
    if (!t) return;
    if (i === cur && isText(t)) t.state = view.state;
    await saveTab(t);
    tabs.splice(i, 1);
    if (!tabs.length) {
      cur = -1;
      return openView("agenda");
    }
    const next = Math.min(i <= cur ? Math.max(cur - 1, 0) : cur, tabs.length - 1);
    cur = -1;
    await show(next);
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
      await call("write_file", { path, text: next });
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

  const taskOps: Record<string, (loc: Loc, it?: AgendaItem, arg?: number) => Promise<unknown>> = {
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
    if (!tpls.length) return flash("No capture templates: add [[templates]] in settings (Ctrl+,) or .org files in notes/templates/.");
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
    await openFile(await call("capture_path", { file: journalFile(), dateInput }));
    view.dispatch({ selection: { anchor: view.state.doc.length }, scrollIntoView: true });
  }

  async function journalPick() {
    const d = await datePick("Open journal for");
    if (d && d !== "rm") await journal(d);
  }

  async function journalEntry() {
    const text = await ask("Add a line to today's journal");
    if (!text) return;
    const path: string = await call("capture_path", { file: journalFile() });
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
    try {
      const d = await call<string>("ai_draft", { prompt, oneLine });
      message = "";
      return d;
    } catch (e) {
      flash(`⚠ ${e}`);
      return null;
    }
  }

  async function draftSummary() {
    if (!cfg.config.ai_model) return flash(`Set ai_model in settings (Ctrl+,), e.g. "llama3.2:3b", to draft with a local model.`);
    const draft = await aiDraft(await call("ai_day_prompt", { dateInput: "today" }), "Send", "Cancel", false);
    if (!draft) return draft === "" && flash("The model sent an empty draft.");
    const path: string = await call("capture_path", { file: journalFile() });
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

  async function adjustStart() {
    const m = await ask("How many minutes ago did you actually start?");
    if (m && !isNaN(+m)) await tcDo("tc_adjust", { minutes: Math.round(+m) });
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

  /** The open note plus the notes it links to by id:, as HTML pages in a folder that link to each other. */
  async function exportLinked() {
    if (!inFile()) return flash("Open a note first.");
    const depth = await pick({ prompt: "Include notes", items: [{ label: "Linked from this note", value: 1 }, { label: "…and the notes they link to", value: 2 }] });
    if (depth == null) return;
    const path = await exportTo("export_linked", { path: tab.path, text: view.state.doc.toString(), depth });
    if (path) await exported(path, false);
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

  async function editProject() {
    const projects: Record<string, any> = await call("tc_projects");
    const name = await pick({ prompt: "Project settings", items: Object.keys(projects) });
    if (!name) return;
    const p = projects[name];
    const code = await ask(`Export code for “${name}”`, p.export_code);
    if (code == null) return;
    const curR = p.rounding == null ? "none" : String(p.rounding);
    const r = await pick({ prompt: "Round billable hours to", items: [["0.5", "Half hour"], ["0.25", "Quarter hour"], ["1.0", "Whole hour"], ["none", "Don't round"]].map(([v, l]) => ({ label: l, detail: v === curR ? "current" : "", value: v })) });
    if (r == null) return;
    const rounding = r === "none" ? null : parseFloat(r);
    const up = rounding != null && (await pick({ prompt: "Always round up?", items: p.round_up ? ["Yes", "No"] : ["No", "Yes"] })) === "Yes";
    const active = await pick({ prompt: "Show in the project list?", items: p.active ? ["Yes", "No"] : ["No", "Yes"] });
    if (active == null) return;
    await call("tc_save_project", { name, project: { export_code: code, rounding, round_up: up, active: active === "Yes" } });
    flash(`✓ Saved “${name}”`);
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
    const prompt = cfg.config.ai_model && (await call<string>("ai_note_prompt", { project: p.project || null, apps: s.apps, titles: s.titles }));
    const draft = prompt ? await aiDraft(prompt, "Draft the note", "Write it myself", true) : null;
    const note = await ask("What did you do? (optional)", draft ?? "", "goes into the work diary");
    if (note != null) await tcDo("tc_add_session", { start: s.start, end: s.end, project: p.project, exportCode: p.code, note });
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
    const u = await check();
    if (!u) return quiet || flash("Margin is up to date.");
    if (quiet) return flash(`Margin ${u.version} is available — Space f u to install.`);
    const yes = await pick({ prompt: `Install Margin ${u.version}${version ? ` (you have ${version})` : ""} and restart?`, items: [{ label: "Install and restart", value: true }, { label: "Not now", value: false }] });
    if (!yes) return;
    flash(`Downloading Margin ${u.version}…`);
    await u.download();
    await saveAll(); // the Windows installer closes the app as soon as it starts
    await u.install();
    await relaunch();
  }

  const timeActions: Record<string, (arg?: any) => unknown> = {
    start: () => clockIn(), stop: clockOut, pause: () => tcDo("tc_break"), resume: () => tcDo("tc_resume"),
    switch: changeProject, adjust: adjustStart, editSession: (s) => editSession(s), export: exportCsv, exportReport, projects: editProject,
    profile: (n) => switchProfile(n), holidays: () => report("holidays", "Holidays"), daily: dailyReport,
    weekly: () => report("weekly", "Week report"), doctor: () => report("doctor", "Log check"), backup: () => tcDo("backup_now"),
    backupLog: () => report("backup", "Backup log"), rawLog: () => openFile(cfg.log_path), import: importEmacs, acceptSuggestion, editSuggestion,
    flex: async () => flash(await call("tc_report", { kind: "flex" })), diary: () => openFile(cfg.diary_path),
  };
  const timeApi: TimeApi = { act, run: async (name, arg) => { await timeActions[name](arg); } };
  const agendaApi: AgendaApi = {
    run: (op, it) => recorded(async () => { await taskOps[op]({ path: it.path, line: it.line }, it); }),
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
    { label: "Today's journal", keys: ["Ctrl+J"], leader: "j", run: () => journal() },
    { label: "Journal for another day…", leader: "n J", run: journalPick },
    { label: "Add a line to today's journal", leader: "n e", run: journalEntry },
    { label: "Journal: draft summary of today (local model)", leader: "n s", run: draftSummary },
    { label: "Insert link to a note…", keys: ["Ctrl+L"], leader: "n i", ctx: "editor", run: insertLink },
    { label: "Show notes linking here", leader: "n b", run: () => { showBacklinks = !showBacklinks; return refreshBacklinks(); } },
    { label: "Jump to a note linking here…", leader: "n l", run: pickBacklink },
    { label: "Show note graph", leader: "n g", run: () => { showGraph = !showGraph; return refreshBacklinks(); } },
    { label: "Undo last edit made from a task view or the links panel", leader: "n u", run: () => undo().then(refreshBacklinks) },
    { label: "Follow link at cursor (also gf, Ctrl+click)", keys: ["Enter"], ctx: "normal", leader: "n o", run: () => (ed.linkAtCursor(view) ? (act(() => followLink()), true) : false) },

    { label: "Go to Today", keys: ["Ctrl+1"], leader: "v t", run: () => openView("agenda") },
    { label: "Go to All tasks", keys: ["Ctrl+2"], leader: "v a", run: () => openView("todo") },
    { label: "Go to Inbox", keys: ["Ctrl+3"], leader: "v i", run: openInbox },
    { label: "Go to Time tracking", keys: ["Ctrl+4"], leader: "v c", run: () => openView("time") },
    { label: "Search tasks…", leader: "v f", run: searchTasks },
    { label: "Save this search as a view…", leader: "v v", run: saveView },
    { label: "Toggle sidebar", keys: ["Ctrl+\\"], leader: "v s", run: () => { sidebar = !sidebar; store("sidebar", sidebar ? "1" : "0"); } },
    { label: "Settings (config file)", keys: ["Ctrl+,"], leader: "f c", run: () => openFile(cfg.config_path) },
    { label: "Save", keys: ["Ctrl+S"], leader: "f s", run: () => saveTab(tab, true) },
    { label: "Reload from disk (discard unsaved changes)", leader: "f r", run: reloadFromDisk },
    { label: "Save all", leader: "f S", run: saveAll },
    { label: "About Margin", run: () => flash(`Margin ${version}`) },
    { label: "Check for updates", leader: "f u", run: () => checkUpdate() },
    { label: "Help: tutorial", leader: "?", run: async () => openFile((await call<[string, boolean]>("tutorial"))[0]) },
    { label: "Switch tab…", leader: "b b", run: switchTab },
    { label: "Next tab (also gt)", keys: ["Ctrl+Tab"], leader: "b n", run: () => cycleTab(1) },
    { label: "Previous tab (also gT)", keys: ["Ctrl+Shift+Tab"], leader: "b p", run: () => cycleTab(-1) },
    { label: "Close tab (also :q)", leader: "b d", run: () => closeTab() },

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
    { label: "Fold / unfold all headings (Tab folds one)", keys: ["Shift+Tab"], ctx: "normal", run: () => ed.orgShiftTab(view) },

    { label: "Time: start tracking", leader: "t i", run: t("start") },
    { label: "Time: stop", leader: "t o", run: t("stop") },
    { label: "Time: take a break", leader: "t b", run: t("pause") },
    { label: "Time: resume after break", leader: "t r", run: t("resume") },
    { label: "Time: switch project", leader: "t c", run: t("switch") },
    { label: "Time: I started earlier…", leader: "t a", run: t("adjust") },
    { label: "Time: day report", leader: "t t", run: t("daily") },
    { label: "Time: week report", leader: "t s", run: t("weekly") },
    { label: "Time: show flex balance", leader: "t f", run: t("flex") },
    { label: "Time: public holidays", leader: "t h", run: t("holidays") },
    { label: "Time: export CSV…", leader: "t e", run: t("export") },
    { label: "Time: switch profile…", leader: "t p", run: () => switchProfile() },
    { label: "Time: project settings…", leader: "t P", run: t("projects") },
    { label: "Time: open work diary", leader: "t d", run: t("diary") },
    { label: "Time: edit raw log", leader: "t E", run: t("rawLog") },
    { label: "Time: edit a session…", leader: "t S", run: () => editSession() },
    { label: "Time: back up now", leader: "t B", run: t("backup") },
    { label: "Time: check log for problems", leader: "t D", run: t("doctor") },
    { label: "Time: backup log", leader: "t L", run: t("backupLog") },
    { label: "Time: import from Emacs…", leader: "t M", run: t("import") },

    { label: "Export: note as HTML", leader: "e h", run: () => exportNote("html") },
    { label: "Export: note as Markdown", leader: "e m", run: () => exportNote("md") },
    { label: "Export: note as PDF", leader: "e p", run: () => exportNote("pdf") },
    { label: "Export: note and linked notes as HTML…", leader: "e a", run: exportLinked },
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
  const drawerOpen = () => (showGraph || showBacklinks) && matchMedia("(max-width: 999px)").matches;
  const closeDrawer = () => { showGraph = showBacklinks = false; };

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

  /** Bring every open tab, view, backlink list and the timer up to date with the disk. */
  async function doSync() {
    for (const t of tabs) {
      if (t.kind === "report" && t.report) {
        replaceText(t, await call("tc_report", t.report));
      } else if (t.kind === "file") {
        const disk = ((await call("read_file", { path: t.path })) as string).replace(/\r\n?/g, "\n");
        if (disk === t.saved || disk === stateOf(t).doc.toString()) continue;
        if (t.dirty) mergeFromDisk(t, disk);
        else replaceText(t, disk);
        t.saved = disk;
      }
    }
    reload++;
    await refreshTc();
    await refreshBacklinks();
  }

  /** The file changed on disk while T has unsaved edits: apply the disk change too if the two
   *  edits don't overlap (e.g. a diary line appended while you edit above it); otherwise flag it. */
  function mergeFromDisk(t: Tab, disk: string) {
    const st = stateOf(t);
    const mine = ed.diffChange(t.saved!, st.doc.toString());
    const theirs = ed.diffChange(t.saved!, disk);
    const shift = mine.insert.length - (mine.to - mine.from);
    let at: { from: number; to: number } | null = null;
    if (theirs.from > mine.to) at = { from: theirs.from + shift, to: theirs.to + shift };
    else if (theirs.to < mine.from) at = { from: theirs.from, to: theirs.to };
    if (!at) {
      t.conflict = true;
      flash(`⚠ “${t.title}” was changed elsewhere while you edited it. :w keeps yours, Space f r loads theirs.`);
      return;
    }
    const change = { ...at, insert: theirs.insert };
    syncing = true;
    try {
      if (t === tabs[cur]) view.dispatch({ changes: change });
      else t.state = st.update({ changes: change }).state;
    } finally {
      syncing = false;
    }
  }

  async function reloadFromDisk() {
    if (!inFile()) return;
    const disk = ((await call("read_file", { path: tab.path })) as string).replace(/\r\n?/g, "\n");
    replaceText(tab, disk);
    Object.assign(tab, { saved: disk, dirty: false, conflict: false });
    flash("Reloaded from disk.");
  }

  // Coalesce bursts of change events into one sync at a time.
  let syncChain = Promise.resolve();
  let syncQueued = false;
  function syncFromDisk() {
    if (syncQueued) return;
    syncQueued = true;
    syncChain = syncChain.then(async () => {
      syncQueued = false;
      await doSync().catch((e) => flash(`⚠ ${e}`));
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
      const [tut, first] = await call<[string, boolean]>("tutorial");
      if (first) await openFile(tut);
      await refreshTc();
      if (!import.meta.env.DEV) checkUpdate(true).catch(() => {}); // offline or no release yet: stay quiet
    });
    const timer = setInterval(() => act(refreshTc), 60_000);
    const blur = () => act(saveAll);
    const focus = () => syncFromDisk();
    const unlisten = listen("fs-changed", () => syncFromDisk());
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
    window.addEventListener("focus", focus);
    return () => {
      clearInterval(timer);
      unlisten.then((f) => f());
      unlistenTray.then((f) => f());
      unlistenOpen.then((f) => f());
      unlistenIdle.then((f) => f());
      unlistenCapture.then((f) => f());
      unlistenDrop.then((f) => f());
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("blur", blur);
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
  {#if sidebar && cfg}
    <aside class="side">
      <button class="cmdk" title="Search or run a command" onclick={() => act(palette)}><span class="ico">⌕</span><span>Search or run a command…</span><kbd>{pretty("Ctrl")} K</kbd></button>
      <nav>
        {#each NAV as n}
          <button class:on={n.on(tab)} title={n.label} onclick={() => act(n.run)}><span class="ico">{n.icon}</span><span class="lbl">{n.label}</span><kbd>{pretty("Ctrl")} {n.keys}</kbd></button>
        {/each}
        {#each cfg.config.views ?? [] as v}
          <button class:on={tab?.key === `search:${v.query}`} onclick={() => act(() => openSearch(v.name, v.query))} title={v.query}><span class="ico">⌕</span><span class="lbl">{v.name}</span></button>
        {/each}
      </nav>
      <div class="sec">
        <span>Notes</span>
        <button title="Open note ({pretty('Ctrl')}+P)" onclick={() => act(quickOpen)}>⌕</button>
        <button title="New note" onclick={() => act(newNote)}>+</button>
      </div>
      <div class="notes">
        {#each recent as p (p)}
          <button class:on={tab?.path === p} onclick={() => act(() => openFile(p))} title={rel(p)}>{niceName(p)}</button>
        {:else}
          <p>No notes opened yet. Press {pretty("Ctrl")}+P to find or create one.</p>
        {/each}
      </div>
      <div class="timer" class:on={tc.project != null}>
        <button class="what" onclick={() => act(() => openView("time"))}>
          <span class="dot"></span>
          <span>{tc.project != null ? tc.project || "No project" : tc.on_break != null ? `Break · ${tc.on_break}` : "Not tracking"}</span>
          <small>{tc.today} today</small>
        </button>
        <div class="tbtns">
          {#if tc.project != null}
            <button onclick={() => act(t("pause"))} title="Take a break">Pause</button>
            <button onclick={() => act(t("stop"))} title="Stop tracking">Stop</button>
          {:else if tc.on_break != null}
            <button onclick={() => act(t("resume"))}>Resume</button>
          {:else}
            <button onclick={() => act(t("start"))}>Start</button>
          {/if}
        </div>
        {#if tc.backup_error}<p class="warn" title={tc.backup_error}>⚠ Backup failed</p>{/if}
      </div>
      {#if version}<div class="version">v{version}</div>{/if}
    </aside>
  {/if}

  <div class="maincol">
    <nav class="tabs">
      {#each tabs as t, i (t.key)}
        <div class="tab" class:cur={i === cur}>
          <button onclick={() => act(() => show(i))}>{t.title}{t.dirty ? " •" : ""}</button>
          <button class="x" title="Close" onclick={() => act(() => closeTab(i))}>×</button>
        </div>
      {/each}
    </nav>

    <main>
      <div class="pane" onpointerdowncapture={() => drawerOpen() && closeDrawer()}>
        <div class="editor" bind:this={editorEl} style:display={isText(tab) ? "block" : "none"}></div>
        {#if dropCue}<div class="drop-cue">Drop to attach to {tab.title}</div>{/if}
        {#if cfg}
          {#each tabs.filter((t) => t.kind === "agenda" || t.kind === "todo" || t.kind === "time") as v (v.key)}
            <div class="view" style:display={v === tab ? "block" : "none"}>
              {#if v.kind === "time"}
                <Time api={timeApi} active={v === tab} {reload} />
              {:else}
                <Agenda mode={v.kind as "agenda" | "todo"} api={agendaApi} active={v === tab} {reload} {...kw()} query={v.query} title={v.query != null ? v.title : undefined} />
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
      <span class="file">{tab?.path ? rel(tab.path) : ""}{tab?.conflict ? " • changed on disk (:w keeps yours, Space f r reloads)" : tab?.dirty ? " • unsaved" : ""}</span>
      <span class="msg">{message}</span>
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
  .app { display: flex; height: 100vh; }
  .app.mono { --editor-font: var(--mono); }
  :global(.cm-plain) { font-family: var(--mono); }
  .maincol { flex: 1; display: flex; flex-direction: column; min-width: 0; }

  .side { width: 232px; flex: none; background: var(--panel); border-right: 1px solid var(--border); display: flex; flex-direction: column; padding: 10px var(--s2); gap: var(--s1); box-sizing: border-box; }
  .side button { background: none; border: 0; color: var(--fg); text-align: left; cursor: pointer; border-radius: var(--radius); font-size: var(--fs-md); }
  .cmdk { white-space: nowrap; overflow: hidden; display: flex; gap: 6px; justify-content: space-between; align-items: center; border: 1px solid var(--border) !important; padding: 7px 9px; color: var(--dim) !important; margin-bottom: var(--s2); }
  nav button { display: flex; align-items: center; gap: 9px; width: 100%; padding: 6px 9px; }
  nav button:hover, .notes button:hover { background: var(--active); }
  nav button.on, .notes button.on { background: var(--sel); }
  nav button kbd { margin-left: auto; }
  .cmdk .ico { display: none; }
  .ico { width: 16px; text-align: center; color: var(--dim); }
  kbd { font: 10px var(--mono); color: var(--dim); border: 1px solid var(--border); border-radius: var(--radius-sm); padding: 0 var(--s1); background: var(--bg); }
  .sec { display: flex; align-items: center; gap: 2px; margin: 14px var(--s1) 2px 9px; color: var(--dim); font-size: var(--fs-xs); text-transform: uppercase; letter-spacing: 0.05em; }
  .sec span { flex: 1; }
  .sec button { color: var(--dim); font-size: var(--fs-lg); padding: 0 6px; }
  .notes { flex: 1; overflow-y: auto; min-height: 0; }
  .notes button { display: block; width: 100%; padding: var(--s1) 9px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .notes p { color: var(--dim); font-size: var(--fs-sm); margin: var(--s1) 9px; }
  .timer { border-top: 1px solid var(--border); padding-top: var(--s2); }
  .timer .what { display: grid; grid-template-columns: auto 1fr; gap: 0 var(--s2); width: 100%; padding: 6px 9px; align-items: center; }
  .timer .what small { grid-column: 2; color: var(--dim); font-size: var(--fs-xs); }
  .timer .what span:nth-child(2) { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--dim); }
  .version { color: var(--dim); font-size: var(--fs-xs); padding: 0 9px; }
  .timer.on .dot { background: var(--done); }
  .tbtns { display: flex; gap: 6px; padding: var(--s1) 9px; }
  .tbtns button { border: 1px solid var(--border) !important; padding: 3px 10px; font-size: var(--fs-sm); }
  .warn { color: var(--todo); font-size: var(--fs-xs); margin: var(--s1) 9px; }

  .tabs { display: flex; background: var(--panel); border-bottom: 1px solid var(--border); overflow-x: auto; flex: none; min-height: 33px; }
  .tab { display: flex; align-items: center; border-right: 1px solid var(--border); }
  .tab button { background: none; border: 0; color: var(--dim); padding: var(--s2) var(--s1) var(--s2) 14px; font-size: var(--fs-sm); cursor: pointer; white-space: nowrap; }
  .tab .x { padding: var(--s1) var(--s2); opacity: 0; font-size: var(--fs-base); }
  .tab:hover .x, .tab.cur .x { opacity: 0.7; }
  .tab.cur { background: var(--bg); }
  .tab.cur button { color: var(--fg); }
  main { flex: 1; display: flex; min-height: 0; }
  .pane { flex: 1; min-width: 0; position: relative; }
  .drop-cue { position: absolute; inset: 6px; border: 2px dashed var(--accent); border-radius: 8px; pointer-events: none; display: flex; align-items: flex-end; justify-content: center; padding-bottom: var(--s5); color: var(--accent); font-weight: 600; background: color-mix(in srgb, var(--accent) 6%, transparent); }
  .editor, .view { height: 100%; }
  .links { width: min(280px, 35vw); border-left: 1px solid var(--border); background: var(--panel); overflow-y: auto; padding: 10px var(--s2); flex: none; }
  .links h3 { margin: var(--s1) 6px var(--s2); font-size: var(--fs-sm); color: var(--dim); text-transform: uppercase; letter-spacing: 0.05em; }
  .links button { display: block; width: 100%; text-align: left; background: none; border: 0; color: var(--fg); padding: 6px; border-radius: var(--radius); cursor: pointer; }
  .links button:hover { background: var(--active); }
  .links button span { display: block; color: var(--dim); font-size: var(--fs-sm); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .links p { color: var(--dim); margin: 6px; font-size: var(--fs-sm); }
  .mention { display: flex; align-items: center; }
  .mention button:first-child { min-width: 0; }
  .links .link-it { width: auto; flex: none; color: var(--link); font-size: var(--fs-sm); }
  .status { display: flex; gap: var(--s3); align-items: center; padding: 3px 10px; background: var(--panel); border-top: 1px solid var(--border); font: var(--fs-xs) var(--mono); flex: none; min-height: 20px; }
  .mode { color: var(--bg); background: var(--accent); padding: 0 6px; border-radius: var(--radius-sm); font-weight: 700; }
  .file { color: var(--dim); }
  .msg { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .hint { background: none; border: 0; color: var(--dim); font: var(--fs-xs) var(--sans); cursor: pointer; }
  /* narrow windows: links drawer < 1000, sidebar icon rail < 760, short status bar < 560 */
  @media (max-width: 999px) {
    main { position: relative; }
    .links { position: absolute; right: 0; top: 0; bottom: 0; width: min(320px, 85vw); z-index: 10; box-shadow: var(--shadow); box-sizing: border-box; }
  }
  @media (max-width: 759px) {
    .side { width: 44px; padding: 10px var(--s1); align-items: stretch; }
    .side .lbl, .side kbd, .side .sec, .side .notes, .side .tbtns, .side .warn, .version, .timer small, .timer .what span:nth-child(2), .cmdk > span:not(.ico) { display: none; }
    .cmdk .ico { display: block; }
    .cmdk, nav button, .timer .what { justify-content: center; padding-left: 0; padding-right: 0; }
    .cmdk { margin-bottom: var(--s2); }
    .timer .what { grid-template-columns: auto; }
  }
  @media (max-width: 559px) {
    .status .file, .hint { display: none; }
  }
</style>
