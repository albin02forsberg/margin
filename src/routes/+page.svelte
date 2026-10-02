<script lang="ts">
  import { onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { EditorView } from "@codemirror/view";
  import type { EditorState } from "@codemirror/state";
  import * as ed from "$lib/editor";
  import Picker, { type PickOpts } from "$lib/Picker.svelte";
  import Menu, { type MenuNode } from "$lib/Menu.svelte";
  import Agenda, { type AgendaApi, type AgendaItem } from "$lib/Agenda.svelte";

  type Tab = { key: string; title: string; kind: "file" | "report" | "agenda" | "todo"; path?: string; state?: EditorState; dirty: boolean };
  type NoteNode = { id: string; title: string; path: string; line: number };
  type Hit = { path: string; title: string; line: number; text: string };
  type Session = { date: string; project: string; desc: string; hours: number; start: string; end: string; line: number };

  let cfg = $state<any>(null);
  let tabs = $state<Tab[]>([]);
  let cur = $state(-1);
  let editorEl = $state<HTMLDivElement>();
  let view: EditorView;
  let picker = $state<Picker>();
  let menu = $state<Menu>();
  let mode = $state("normal");
  let message = $state("");
  let tc = $state<any>({});
  let backlinks = $state.raw<Hit[]>([]);
  let showBacklinks = $state(false);
  let reload = $state(0);

  const tab = $derived(tabs[cur]);
  const isText = (t?: Tab) => t?.kind === "file" || t?.kind === "report";
  const call = <T = any,>(cmd: string, args?: Record<string, unknown>) => invoke<T>(cmd, args);
  const pick = (o: PickOpts) => picker!.open(o);
  const ask = (prompt: string, initial = "") => pick({ prompt, initial }) as Promise<string | null>;
  const sep = () => (cfg.notes.includes("\\") ? "\\" : "/");
  const join = (a: string, b: string) => a.replace(/[\\/]$/, "") + sep() + b;
  const base = (p: string) => p.split(/[\\/]/).pop()!;
  const rel = (p: string) => (p.startsWith(cfg.notes) ? p.slice(cfg.notes.length + 1) : p);
  const hm = (h: number) => (h >= 1 ? `${Math.trunc(h)}h ${String(Math.trunc((h % 1) * 60)).padStart(2, "0")}m` : `${Math.trunc(h * 60)}m`);

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

  // ---------------------------------------------------------------- tabs

  const stateOf = (t: Tab) => (t === tabs[cur] && isText(t) ? view.state : t.state!);

  function newState(key: string, text: string, path?: string, readOnly = false) {
    return ed.createState(text, {
      org: !path || path.endsWith(".org"),
      readOnly,
      todo: cfg.config.todo_keywords,
      done: cfg.config.done_keywords,
      onChange: () => {
        const t = tabs.find((x) => x.key === key);
        if (t?.kind === "file") t.dirty = true;
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

  async function saveTab(t?: Tab) {
    if (!t || t.kind !== "file" || !t.dirty) return;
    await call("write_file", { path: t.path, text: stateOf(t).doc.toString() });
    t.dirty = false;
    if (t.path === cfg.log_path) {
      const r: string = await call("tc_report", { kind: "doctor" });
      flash(r.includes("No issues") ? "Saved. Timelog OK." : "Saved — timelog has issues, see SPC t D");
    } else if (t.path === cfg.config_path) {
      cfg = await call("config");
      flash("Config reloaded.");
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
      tabs.push({ key: path, title: base(path), kind: "file", path, dirty: false, state: newState(path, text, path) });
      i = tabs.length - 1;
    }
    await show(i);
    if (line != null) {
      const pos = view.state.doc.line(Math.min(line + 1, view.state.doc.lines)).from;
      view.dispatch({ selection: { anchor: pos }, effects: EditorView.scrollIntoView(pos, { y: "center" }) });
    }
  }

  async function openReport(key: string, title: string, text: string) {
    const state = newState(key, text, undefined, true);
    let i = tabs.findIndex((t) => t.key === key);
    if (i < 0) {
      tabs.push({ key, title, kind: "report", dirty: false, state });
      i = tabs.length - 1;
    } else {
      tabs[i].state = state;
      if (i === cur) cur = -1; // force reload of the new state
    }
    await show(i);
  }

  async function openView(kind: "agenda" | "todo") {
    let i = tabs.findIndex((t) => t.kind === kind);
    if (i < 0) {
      tabs.push({ key: kind, title: kind === "agenda" ? "Agenda" : "TODOs", kind, dirty: false });
      i = tabs.length - 1;
    }
    await show(i);
  }

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

  /** Apply an async text transform to a file, through its open tab if there is one. */
  async function toFile(path: string, f: (text: string) => Promise<string>) {
    const t = tabs.find((x) => x.kind === "file" && x.path === path);
    if (!t) {
      const text: string = await call("read_file", { path });
      await call("write_file", { path, text: await f(text) });
      return;
    }
    const st = stateOf(t);
    const old = st.doc.toString();
    const next = await f(old);
    if (t === tabs[cur]) ed.applyText(view, next);
    else t.state = st.update({ changes: ed.diffChange(old, next) }).state;
    t.dirty = true;
    await saveTab(t);
  }

  // ---------------------------------------------------------------- org editing

  const curLine0 = () => view.state.doc.lineAt(view.state.selection.main.head).number - 1;
  const inFile = () => tab?.kind === "file";

  async function cycleTodo(dir: number) {
    if (!inFile()) return;
    ed.applyText(view, await call("org_cycle", { text: view.state.doc.toString(), line: curLine0(), dir }));
  }

  async function planning(kind: "SCHEDULED" | "DEADLINE") {
    if (!inFile()) return;
    const input = await ask(`${kind} (e.g. today, +3d, fri, 12-24 14:00, rm)`);
    if (input == null) return;
    ed.applyText(view, await call("org_planning", { text: view.state.doc.toString(), line: curLine0(), kind, input }));
  }

  async function followLink() {
    const target = ed.linkAtCursor(view);
    if (!target) return flash("No link at point");
    if (target.startsWith("id:")) {
      const nodes: NoteNode[] = await call("notes_nodes");
      const n = nodes.find((n) => n.id === target.slice(3));
      return n ? openFile(n.path, n.line) : flash(`No node with ${target}`);
    }
    if (/^https?:\/\//.test(target)) return openUrl(target);
    const p = target.replace(/^file:/, "").replace(/::.*$/, "");
    const dir = tab?.path ? tab.path.replace(/[\\/][^\\/]*$/, "") : cfg.notes;
    await openFile(/^([\\/]|[A-Za-z]:)/.test(p) ? p : join(dir, p));
  }

  // ---------------------------------------------------------------- notes

  async function findFile() {
    const files: string[] = await call("list_files");
    const r = await pick({ prompt: "Find file", items: files.map((p) => ({ label: rel(p), value: p })), allowCustom: true });
    if (r == null || r === "") return;
    await openFile(files.includes(r) ? r : join(cfg.notes, r.endsWith(".org") ? r : `${r}.org`));
  }

  /** Pick a node; typing a new title creates a note. */
  async function pickNode(prompt: string): Promise<NoteNode | null> {
    const nodes: NoteNode[] = await call("notes_nodes");
    const r = await pick({ prompt, items: nodes.map((n) => ({ label: n.title, detail: rel(n.path), value: n })), allowCustom: true });
    if (r == null || r === "") return null;
    if (typeof r !== "string") return r;
    const n: NoteNode = await call("notes_new", { title: r });
    flash(`Created ${base(n.path)}`);
    return n;
  }

  async function findNode() {
    const n = await pickNode("Find or create node");
    if (n) await openFile(n.path, n.line);
  }

  async function insertLink() {
    if (!inFile()) return;
    const n = await pickNode("Insert link to");
    if (!n) return;
    const at = view.state.selection.main.head;
    const insert = `[[id:${n.id}][${n.title}]]`;
    view.dispatch({ changes: { from: at, insert }, selection: { anchor: at + insert.length } });
    view.focus();
  }

  async function newNote() {
    const title = await ask("New note title");
    if (!title) return;
    const n: NoteNode = await call("notes_new", { title });
    await openFile(n.path);
    view.dispatch({ selection: { anchor: view.state.doc.length } });
  }

  async function search() {
    const q = await ask("Search notes");
    if (!q) return;
    const hits: Hit[] = await call("notes_search", { query: q });
    if (!hits.length) return flash(`No matches for "${q}"`);
    const h = await pick({ prompt: `${hits.length} matches`, items: hits.map((h) => ({ label: h.text, detail: `${h.title} · ${rel(h.path)}:${h.line + 1}`, value: h })) });
    if (h) await openFile(h.path, h.line);
  }

  async function refreshBacklinks() {
    backlinks = showBacklinks && tab?.kind === "file" ? await call("notes_backlinks", { path: tab.path }) : [];
  }

  async function pickBacklink() {
    if (!inFile()) return;
    const hits: Hit[] = await call("notes_backlinks", { path: tab.path });
    if (!hits.length) return flash("No backlinks");
    const h = await pick({ prompt: "Backlinks", items: hits.map((h) => ({ label: h.title, detail: h.text, value: h })) });
    if (h) await openFile(h.path, h.line);
  }

  // ---------------------------------------------------------------- timeclock

  async function refreshTc() {
    tc = await call("tc_status");
  }

  async function tcDo(cmd: string, args?: Record<string, unknown>) {
    flash(await call(cmd, args));
    await refreshTc();
  }

  /** Project picker; new projects also get an export code. Returns null on cancel. */
  async function pickProject(prompt: string, suggested?: string): Promise<{ project: string; code: string | null } | null> {
    const projects: Record<string, { export_code: string; active: boolean }> = await call("tc_projects");
    const names = Object.keys(projects).filter((p) => projects[p].active);
    const rest = names.filter((n) => n !== suggested);
    const items = !suggested ? names : names.includes(suggested) ? [suggested, ...rest] : [{ label: suggested, detail: "suggested · new", value: suggested }, ...rest];
    const project = await pick({ prompt, items, allowCustom: true });
    if (project == null) return null;
    let code: string | null = null;
    if (project && !projects[project]) {
      code = await ask(`Export code for NEW project '${project}' (Enter for name)`);
      if (code == null) return null;
    }
    return { project, code };
  }

  async function clockIn(suggested?: string) {
    if (suggested == null && tab?.kind === "file") {
      suggested = (await call("org_context", { path: tab.path, text: view.state.doc.toString(), line: curLine0() })).category;
    }
    const p = await pickProject(suggested ? `Clock in on project (suggested: ${suggested})` : "Clock in on project", suggested);
    if (!p) return;
    const sugg: string[] = await call("tc_suggestions", { project: p.project });
    const task = sugg.length ? ((await pick({ prompt: `Task for '${p.project}' (Esc to skip)`, items: sugg, allowCustom: true })) ?? "") : "";
    await tcDo("tc_in", { project: p.project, task, exportCode: p.code });
  }

  async function clockOut() {
    await refreshTc();
    if (tc.project == null) return flash("Not clocked in.");
    const note = await ask("Done for now! What did you do under this session?", tc.task ?? "");
    if (note != null) await tcDo("tc_out", { note });
  }

  async function changeProject() {
    await refreshTc();
    if (tc.project == null) return clockIn();
    const note = await ask(`🔄 Switching from '${tc.project}'. What did you do until now?`, tc.task ?? "");
    if (note == null) return;
    const p = await pickProject("Clock in on new project");
    if (!p) return;
    await call("tc_out", { note });
    await tcDo("tc_in", { project: p.project, task: "", exportCode: p.code });
  }

  async function adjustStart() {
    const m = await ask("Oops, forgot to clock in! How many minutes ago did you start?");
    if (m && !isNaN(+m)) await tcDo("tc_adjust", { minutes: Math.round(+m) });
  }

  async function report(kind: string, title: string, dateInput?: string | null) {
    await openReport(`report:${kind}`, title, await call("tc_report", { kind, dateInput }));
  }

  async function dailyReport() {
    const d = await ask("Daily summary for date (Enter = today)");
    if (d != null) await report("daily", "Day report", d);
  }

  async function exportCsv() {
    const start = await ask("Export from (date)", "-1m");
    if (start == null) return;
    const end = await ask("Export to (date)", "today");
    if (end == null) return;
    const path = await ask("Save CSV to", join(cfg.export, `time_${tc.profile.toLowerCase()}.csv`));
    if (path) await tcDo("tc_csv", { start, end, path });
  }

  async function switchProfile() {
    const name = await pick({ prompt: `Switch profile (current: ${tc.profile})`, items: cfg.config.profiles });
    if (!name) return;
    await tcDo("tc_switch_profile", { name });
    cfg = await call("config");
  }

  async function editProject() {
    const projects: Record<string, any> = await call("tc_projects");
    const name = await pick({ prompt: "Edit project config", items: Object.keys(projects) });
    if (!name) return;
    const p = projects[name];
    const code = await ask(`Export code (${p.export_code})`, p.export_code);
    if (code == null) return;
    const opts = ["0.5", "0.25", "1.0", "None"];
    const curR = p.rounding == null ? "None" : String(p.rounding);
    const r = await pick({ prompt: "Rounding resolution", items: [curR, ...opts.filter((o) => o !== curR)] });
    if (r == null) return;
    const rounding = r === "None" ? null : parseFloat(r);
    const up = rounding != null && (await pick({ prompt: "Always round UP?", items: p.round_up ? ["yes", "no"] : ["no", "yes"] })) === "yes";
    const active = await pick({ prompt: "Is project ACTIVE?", items: p.active ? ["yes", "no"] : ["no", "yes"] });
    if (active == null) return;
    await call("tc_save_project", { name, project: { export_code: code, rounding, round_up: up, active: active === "yes" } });
    flash(`✅ Project '${name}' updated!`);
  }

  async function editSession() {
    const d = await ask("Edit session on date (Enter = today)");
    if (d == null) return;
    const ss: Session[] = await call("tc_sessions_on", { dateInput: d });
    if (!ss.length) return flash("No completed sessions on that date.");
    const s: Session | null = await pick({
      prompt: "Edit which session",
      items: ss.map((s) => ({ label: `[${s.start}-${s.end}] ${s.project || "Other"}: ${s.desc || "(no description)"} (${hm(s.hours)})`, value: s })),
    });
    if (!s) return;
    const note = await ask("New description", s.desc);
    if (note == null) return;
    const h = await ask(`New duration in hours (currently ${s.hours.toFixed(2)})`, s.hours.toFixed(2));
    if (h == null || isNaN(+h)) return;
    await tcDo("tc_edit_session", { line: s.line, note, oldHours: s.hours, newHours: +h });
  }

  async function importEmacs() {
    const dir = await ask(`Import Emacs timeclock files for ${tc.profile} from`, "~/timeclock");
    if (dir) await tcDo("tc_import", { dir });
  }

  // ---------------------------------------------------------------- leader menu

  const m = (key: string, label: string, run: () => unknown): MenuNode => ({ key, label, run: () => act(run) });
  const g = (key: string, label: string, children: MenuNode[]): MenuNode => ({ key, label, children });
  const leader: MenuNode[] = [
    m("SPC", "find file", findFile),
    m("/", "search notes", search),
    m(".", "agenda", () => openView("agenda")),
    g("f", "file", [m("f", "find file", findFile), m("s", "save", () => saveTab(tab)), m("S", "save all", saveAll), m("c", "open config", () => openFile(cfg.config_path))]),
    g("b", "buffer", [
      m("b", "switch tab", async () => { const i = await pick({ prompt: "Switch tab", items: tabs.map((t, i) => ({ label: t.title, detail: t.path ? rel(t.path) : t.kind, value: i })) }); if (i != null) await show(i); }),
      m("n", "next tab", () => cycleTab(1)),
      m("p", "previous tab", () => cycleTab(-1)),
      m("d", "close tab", () => closeTab()),
    ]),
    g("a", "agenda", [m("a", "week agenda", () => openView("agenda")), m("t", "TODO list", () => openView("todo"))]),
    g("n", "notes", [
      m("f", "find node", findNode),
      m("i", "insert link", insertLink),
      m("n", "new note", newNote),
      m("b", "toggle backlinks panel", () => { showBacklinks = !showBacklinks; return refreshBacklinks(); }),
      m("l", "jump to backlink", pickBacklink),
      m("d", "open diary", () => openFile(cfg.diary_path)),
    ]),
    g("m", "org", [
      m("t", "cycle TODO", () => cycleTodo(1)),
      m("T", "cycle TODO back", () => cycleTodo(-1)),
      m("s", "schedule", () => planning("SCHEDULED")),
      m("d", "deadline", () => planning("DEADLINE")),
      m("x", "toggle checkbox", () => inFile() && ed.toggleCheckbox(view)),
      m("h", "new heading", () => inFile() && ed.newHeading(view)),
      m("o", "follow link", followLink),
    ]),
    g("t", "timeclock", [
      m("i", "clock IN", () => clockIn()),
      m("o", "clock OUT", clockOut),
      m("b", "take BREAK", () => tcDo("tc_break")),
      m("r", "resume", () => tcDo("tc_resume")),
      m("c", "switch project", changeProject),
      m("a", "adjust start time", adjustStart),
      m("t", "daily summary", dailyReport),
      m("s", "weekly summary", () => report("weekly", "Week report")),
      m("f", "show flex", async () => flash(await call("tc_report", { kind: "flex" }))),
      m("h", "public holidays", () => report("holidays", "Holidays")),
      m("e", "export CSV", exportCsv),
      m("p", "switch profile", switchProfile),
      m("P", "project settings", editProject),
      m("d", "open diary", () => openFile(cfg.diary_path)),
      m("E", "edit raw log", () => openFile(cfg.log_path)),
      m("S", "edit session", editSession),
      m("B", "git backup", () => tcDo("backup_now")),
      m("D", "doctor (check log)", () => report("doctor", "Doctor")),
      m("L", "backup log", () => report("backup", "Backup log")),
      m("M", "import from Emacs", importEmacs),
    ]),
  ];

  const agendaApi: AgendaApi = {
    open: (path, line) => act(() => openFile(path, line)),
    cycle: (it, dir) => toFile(it.path, (text) => call("org_cycle", { text, line: it.line, dir })),
    plan: async (it, kind) => {
      const input = await ask(`${kind} (e.g. today, +3d, fri, 12-24 14:00, rm)`);
      if (input != null) await toFile(it.path, (text) => call("org_planning", { text, line: it.line, kind, input }));
    },
    clockIn: (it: AgendaItem) => act(() => clockIn(it.category)),
    clockOut: () => act(clockOut),
    pick: (o) => pick(o),
    close: () => act(() => closeTab()),
    act,
  };

  // ---------------------------------------------------------------- keys

  function onKey(e: KeyboardEvent) {
    if (picker?.isOpen() || menu?.isOpen()) return;
    if (e.ctrlKey && e.key === "Tab") return stop(e, () => cycleTab(e.shiftKey ? -1 : 1));
    const inEditor = isText(tab) && view.contentDOM.contains(e.target as Node);
    if (!inEditor) {
      if (e.key === " " && !(e.target instanceof HTMLInputElement)) stop(e, () => menu!.show());
      return;
    }
    const idle = ed.vimIdle(view);
    const k = e.key;
    if (k === " " && idle && !e.ctrlKey && !e.altKey) return stop(e, () => menu!.show());
    if (tab.kind !== "file") return;
    if (e.altKey && !e.ctrlKey) {
      const ops: Record<string, () => unknown> = {
        h: () => ed.shiftHeading(view, -1), l: () => ed.shiftHeading(view, 1),
        k: () => ed.moveSubtree(view, -1), j: () => ed.moveSubtree(view, 1),
        Enter: () => ed.newHeading(view),
      };
      if (ops[k]) return stop(e, ops[k]);
    }
    if (k === "Tab" && e.shiftKey) return stop(e, () => ed.orgShiftTab(view));
    if (e.shiftKey && idle && (k === "ArrowRight" || k === "ArrowLeft") && ed.level(view.state.doc.lineAt(view.state.selection.main.head).text))
      return stop(e, () => cycleTodo(k === "ArrowRight" ? 1 : -1));
    if (k === "Enter" && idle && !e.shiftKey && ed.linkAtCursor(view)) return stop(e, followLink);
    if (e.ctrlKey && k === "c" && idle) return stop(e, () => ed.toggleCheckbox(view));
  }

  function stop(e: KeyboardEvent, f: () => unknown) {
    e.preventDefault();
    e.stopPropagation();
    act(f);
  }

  // ---------------------------------------------------------------- lifecycle

  async function onFocus() {
    // Pick up edits made outside the app (e.g. in Emacs) in clean tabs.
    for (const t of tabs) {
      if (t.kind !== "file" || t.dirty) continue;
      const disk = ((await call("read_file", { path: t.path })) as string).replace(/\r\n?/g, "\n");
      const st = stateOf(t);
      if (disk === st.doc.toString()) continue;
      if (t === tabs[cur]) {
        ed.applyText(view, disk);
        t.dirty = false;
      } else t.state = st.update({ changes: ed.diffChange(st.doc.toString(), disk) }).state;
    }
    reload++;
    await refreshTc();
  }

  onMount(() => {
    act(async () => {
      cfg = await call("config");
      view = new EditorView({ parent: editorEl! });
      ed.hooks.save = () => act(() => saveTab(tab));
      ed.hooks.close = () => act(() => closeTab());
      ed.hooks.follow = () => act(followLink);
      ed.hooks.tab = (d) => act(() => cycleTab(d));
      await openView("agenda");
      await refreshTc();
    });
    const timer = setInterval(() => act(refreshTc), 60_000);
    const blur = () => act(saveAll);
    const focus = () => act(onFocus);
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("blur", blur);
    window.addEventListener("focus", focus);
    return () => {
      clearInterval(timer);
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("blur", blur);
      window.removeEventListener("focus", focus);
    };
  });

  const clock = $derived(
    tc.project != null ? `[${tc.project || "—"}]` : tc.on_break != null ? `[Break: ${tc.on_break}]` : "[Paused]",
  );
</script>

<div class="app">
  <nav class="tabs">
    {#each tabs as t, i (t.key)}
      <button class:cur={i === cur} onclick={() => act(() => show(i))}>{t.title}{t.dirty ? " ●" : ""}</button>
    {/each}
  </nav>

  <main>
    <div class="pane">
      <div class="editor" bind:this={editorEl} style:display={isText(tab) ? "block" : "none"}></div>
      {#each tabs.filter((t) => t.kind === "agenda" || t.kind === "todo") as t (t.key)}
        <div class="view" style:display={t === tab ? "block" : "none"}>
          <Agenda mode={t.kind as "agenda" | "todo"} api={agendaApi} active={t === tab} {reload} />
        </div>
      {/each}
    </div>
    {#if showBacklinks}
      <aside>
        <h3>Backlinks <small>{backlinks.length}</small></h3>
        {#each backlinks as b}
          <button onclick={() => act(() => openFile(b.path, b.line))}><strong>{b.title}</strong><span>{b.text}</span></button>
        {:else}
          <p>No backlinks.</p>
        {/each}
      </aside>
    {/if}
  </main>

  <footer class="status">
    <span class="mode">{isText(tab) ? mode.toUpperCase() : tab?.kind.toUpperCase() ?? ""}</span>
    <span class="file">{tab?.path ? rel(tab.path) : (tab?.title ?? "")}{tab?.dirty ? " [+]" : ""}</span>
    <span class="msg">{message}</span>
    {#if tc.backup_error}<span class="warn" title={tc.backup_error}>⚠ backup failed</span>{/if}
    <span class="clock" class:on={tc.project != null}>{tc.profile} {clock} {tc.today}</span>
  </footer>
</div>

<Picker bind:this={picker} />
<Menu bind:this={menu} root={leader} />

<style>
  :global(:root) {
    --mono: "JetBrains Mono", "Cascadia Code", ui-monospace, Menlo, Consolas, monospace;
    --bg: #1d1f21; --fg: #d6d6d4; --panel: #26282b; --border: #3a3d41; --dim: #7c8186;
    --active: #24272a; --sel: #34495e; --accent: #e6a23c;
    --h1: #81a2be; --h2: #b294bb; --h3: #8abeb7; --h4: #b5bd68; --h5: #f0c674; --h6: #de935f;
    --todo: #e06c75; --done: #98c379; --date: #8abeb7; --link: #61afef; --code: #b5bd68;
    color-scheme: dark;
  }
  @media (prefers-color-scheme: light) {
    :global(:root) {
      --bg: #fafafa; --fg: #2b2b2b; --panel: #f0f0f0; --border: #d4d4d4; --dim: #8a8a8a;
      --active: #f2f2f2; --sel: #cfe3f7; --accent: #c4720a;
      --h1: #2d5f9a; --h2: #7a3e9d; --h3: #1f7a73; --h4: #5a7a12; --h5: #9a6a00; --h6: #b4501a;
      --todo: #c0392b; --done: #2e8b3e; --date: #1f7a73; --link: #1a66c2; --code: #5a7a12;
      color-scheme: light;
    }
  }
  :global(html, body) { margin: 0; height: 100%; background: var(--bg); color: var(--fg); font: 14px system-ui, sans-serif; overflow: hidden; }
  .app { display: flex; flex-direction: column; height: 100vh; }
  .tabs { display: flex; background: var(--panel); border-bottom: 1px solid var(--border); overflow-x: auto; flex: none; }
  .tabs button {
    background: none; border: 0; border-right: 1px solid var(--border); color: var(--dim);
    padding: 6px 14px; font: 13px var(--mono); cursor: pointer; white-space: nowrap;
  }
  .tabs button.cur { color: var(--fg); background: var(--bg); }
  main { flex: 1; display: flex; min-height: 0; }
  .pane { flex: 1; min-width: 0; position: relative; }
  .editor, .view { height: 100%; }
  aside { width: 300px; border-left: 1px solid var(--border); background: var(--panel); overflow-y: auto; padding: 8px; flex: none; }
  aside h3 { margin: 4px 4px 8px; font-size: 13px; color: var(--accent); }
  aside h3 small { color: var(--dim); }
  aside button { display: block; width: 100%; text-align: left; background: none; border: 0; color: var(--fg); padding: 6px; border-radius: 4px; cursor: pointer; }
  aside button:hover { background: var(--sel); }
  aside button span { display: block; color: var(--dim); font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  aside p { color: var(--dim); margin: 4px; }
  .status { display: flex; gap: 12px; align-items: center; padding: 3px 10px; background: var(--panel); border-top: 1px solid var(--border); font: 12px var(--mono); flex: none; }
  .mode { color: var(--bg); background: var(--accent); padding: 0 6px; border-radius: 3px; font-weight: 700; }
  .file { color: var(--dim); }
  .msg { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .warn { color: var(--todo); }
  .clock { color: var(--dim); }
  .clock.on { color: var(--done); }
</style>
