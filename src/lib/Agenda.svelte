<script lang="ts" module>
  export type AgendaItem = {
    date: string; kind: string; label: string; time: string | null; path: string; line: number;
    keyword: string | null; priority: string | null; title: string; tags: string[]; category: string;
    /** Habits (today's entry, Tasks): last 21 days ('x' done, '-' due within `.+2d/3d` max, '!' overdue, '.' not due) and streak. */
    habit?: { bar: string; streak: number } | null;
  };
  export type AgendaApi = {
    /** Run a task operation on IT (see ACTIONS); resolves when the files are written. */
    run: (op: string, it: AgendaItem) => Promise<void>;
    newTask: (scheduled?: string) => void;
    undo: () => Promise<void>;
    act: (f: () => unknown) => void;
  };

  export const iso = (d: Date) =>
    `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
  const parse = (s: string) => new Date(+s.slice(0, 4), +s.slice(5, 7) - 1, +s.slice(8, 10));
  const monday = (d: Date) => new Date(d.getFullYear(), d.getMonth(), d.getDate() - ((d.getDay() + 6) % 7));
  const addDays = (d: Date, n: number) => new Date(d.getFullYear(), d.getMonth(), d.getDate() + n);
  const week = (d: Date) => {
    const t = new Date(Date.UTC(d.getFullYear(), d.getMonth(), d.getDate()));
    t.setUTCDate(t.getUTCDate() + 4 - (t.getUTCDay() || 7));
    return Math.ceil(((t.getTime() - Date.UTC(t.getUTCFullYear(), 0, 1)) / 86400000 + 1) / 7);
  };
  const fmtDay = (d: Date, o: Intl.DateTimeFormatOptions = { weekday: "long", day: "numeric", month: "long" }) => d.toLocaleDateString(undefined, o);

  /** Item actions: key, label, op. Shown in the action bar and help. */
  export const ACTIONS: [string, string, string][] = [
    ["x", "Done", "done"],
    ["s", "Schedule", "schedule"],
    ["d", "Due date", "deadline"],
    ["p", "Priority", "priority"],
    ["t", "Status", "state"],
    ["#", "Tags", "tags"],
    ["m", "Move to…", "move"],
    ["A", "Archive", "archive"],
    ["I", "Track time", "clockIn"],
    ["↵", "Open", "open"],
  ];
  const MORE: [string, string][] = [
    ["j / k", "Move selection"], ["< / >", "Reschedule a day earlier / later"], ["+ / -", "Raise / lower priority"],
    ["n", "New task"], ["u", "Undo last change"], ["v", "Day / week view"], ["h / l", "Previous / next period"],
    [".", "Jump to today"], ["/", "Filter"], ["r", "Refresh"], ["?", "Toggle this help"],
  ];
</script>

<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { tick } from "svelte";
  import type { TimeApi } from "./Time.svelte";
  import WeekBars from "./WeekBars.svelte";
  import { hm, worked } from "./clock";

  /** QUERY (todo mode): show only open tasks matching it, under TITLE. TIME (agenda mode): the Clock column's actions. */
  let { mode, api, active, reload, todo, done, query, title, time }: {
    mode: "agenda" | "todo"; api: AgendaApi; active: boolean; reload: number; todo: string[]; done: string[]; query?: string; title?: string; time?: TimeApi;
  } = $props();

  const stored = (k: string, d: string) => { try { return localStorage.getItem(k) ?? d; } catch { return d; } };
  const initialSpan = +stored("agenda-span", "1");
  let span = $state(initialSpan);
  let start = $state(initialSpan === 7 ? monday(new Date()) : new Date());
  let items = $state.raw<AgendaItem[]>([]);
  let filter = $state("");
  let sel = $state(0);
  let help = $state(false);
  let busy = $state(false);
  let root = $state<HTMLDivElement>();
  let filterInput = $state<HTMLInputElement>();
  let now = $state(new Date());

  const today = $derived(iso(now));
  const isDone = (it: AgendaItem) => !!it.keyword && done.includes(it.keyword);
  const days = $derived(Array.from({ length: span }, (_, i) => addDays(start, i)));
  const shown = $derived.by(() => {
    const f = filter.toLowerCase();
    let xs = f ? items.filter((i) => `${i.keyword ?? ""} ${i.title} ${i.tags.join(" ")} ${i.category}`.toLowerCase().includes(f)) : items;
    if (mode === "todo") {
      const rank = (k: string | null) => { const r = todo.indexOf(k ?? ""); return r < 0 ? 99 : r; };
      xs = [...xs].sort((a, b) => rank(a.keyword) - rank(b.keyword));
    }
    return xs;
  });
  const selected = $derived(shown[sel]);
  const groups = $derived.by(() => {
    if (mode === "todo") {
      const ks = [...new Set(shown.map((i) => i.keyword ?? ""))];
      return ks.map((k) => ({ key: k, title: k, sub: "", today: false, rows: rowsOf((i) => (i.keyword ?? "") === k) }));
    }
    return days.map((d) => ({
      key: iso(d),
      title: iso(d) === today ? "Today" : fmtDay(d, { weekday: "long" }),
      sub: fmtDay(d, { day: "numeric", month: "long" }),
      today: iso(d) === today,
      rows: rowsOf((i) => i.date === iso(d)),
    }));
  });
  function rowsOf(pred: (i: AgendaItem) => boolean) {
    return shown.map((it, i) => ({ it, i })).filter((x) => pred(x.it));
  }
  /** Where the "now" line goes among today's timed items. */
  function nowIndex(rows: { it: AgendaItem }[]) {
    const t = `${String(now.getHours()).padStart(2, "0")}:${String(now.getMinutes()).padStart(2, "0")}`;
    const timed = rows.filter((r) => r.it.time);
    if (!timed.length) return -1;
    const i = rows.findIndex((r) => r.it.time && r.it.time.padStart(5, "0") > t);
    return i < 0 ? rows.indexOf(timed[timed.length - 1]) + 1 : i;
  }

  const heading = $derived(
    mode === "todo" ? (title ?? "Tasks")
    : span === 1 ? (iso(start) === today ? "Today" : fmtDay(start))
    : `Week ${week(start)}`,
  );
  const subheading = $derived(
    mode === "todo" ? `${items.length} ${query != null ? "found" : "open"}${query ? ` · ${query}` : ""}`
    : span === 1 ? (iso(start) === today ? fmtDay(start) : "")
    : `${fmtDay(start, { day: "numeric", month: "short" })} – ${fmtDay(days[6], { day: "numeric", month: "short", year: "numeric" })}`,
  );

  /** The Clock column: `tc_dashboard` (running clock, today's hours vs the target, the week). */
  let tc = $state<any>(null);
  const clockOn = $derived(mode === "agenda" && !!time);
  const day = $derived(tc ? worked(tc.week, today) : null);
  const loadClock = async () => { if (clockOn) tc = await invoke("tc_dashboard"); };
  /** A Time view action; it bumps `reload`, which reloads the column. */
  const tcRun = (name: string) => api.act(async () => { await time!.run(name); root?.focus(); });
  const elapsed = (since: string) => hm(Math.max(0, now.getTime() - Date.parse(since)) / 3_600_000);

  async function refresh() {
    now = new Date();
    items = mode === "agenda" ? await invoke("agenda", { start: iso(start), days: span })
      : query != null ? await invoke("search_todos", { query }) : await invoke("todos");
    sel = Math.min(sel, Math.max(0, shown.length - 1));
    await loadClock();
  }

  $effect(() => {
    void reload, start, span;
    if (active) api.act(refresh);
  });
  $effect(() => {
    if (active) tick().then(() => root?.focus());
  });
  $effect(() => {
    if (!active) return;
    const t = setInterval(() => { now = new Date(); api.act(loadClock); }, 60_000);
    return () => clearInterval(t);
  });
  $effect(() => {
    root?.querySelector(`[data-i="${sel}"]`)?.scrollIntoView({ block: "nearest" });
  });

  function setSpan(n: number) {
    span = n;
    start = n === 7 ? monday(start) : (iso(monday(new Date())) === iso(start) ? new Date() : start);
    try { localStorage.setItem("agenda-span", String(n)); } catch {}
  }
  const move = (d: number) => (start = addDays(start, d * span));
  const goToday = () => (start = span === 7 ? monday(new Date()) : new Date());

  /** Run an op on the selected item, then reload. */
  function op(name: string, it = selected) {
    if (!it || busy) return;
    api.act(async () => {
      busy = true;
      try {
        await api.run(name, it);
      } finally {
        busy = false;
      }
      if (name !== "open") await refresh();
      root?.focus();
    });
  }

  function key(e: KeyboardEvent) {
    if (e.target === filterInput) {
      if (e.key === "Escape" || e.key === "Enter" || e.key === "ArrowDown") {
        if (e.key === "Escape") filter = "";
        root?.focus();
        e.preventDefault();
      }
      return;
    }
    if (e.ctrlKey || e.metaKey) return;
    const k = e.shiftKey && e.key === "ArrowRight" ? ">" : e.shiftKey && e.key === "ArrowLeft" ? "<"
      : e.shiftKey && e.key === "ArrowUp" ? "+" : e.shiftKey && e.key === "ArrowDown" ? "-" : e.key;
    const action = ACTIONS.find((a) => a[0] === k || (k === "Enter" && a[0] === "↵") || (k === ":" && a[2] === "tags") || (k === "$" && a[2] === "archive"));
    if (action) op(action[2]);
    else switch (k) {
      case "j": case "ArrowDown": sel = Math.min(sel + 1, shown.length - 1); break;
      case "k": case "ArrowUp": sel = Math.max(sel - 1, 0); break;
      case "g": case "Home": sel = 0; break;
      case "G": case "End": sel = shown.length - 1; break;
      case ">": op("later"); break;
      case "<": op("earlier"); break;
      case "+": op("priority-up"); break;
      case "-": op("priority-down"); break;
      case "n": api.newTask(dayInput()); break;
      case "u": api.act(async () => { await api.undo(); await refresh(); }); break;
      case "v": if (mode === "agenda") setSpan(span === 1 ? 7 : 1); break;
      case "l": case "ArrowRight": case "f": if (mode === "agenda") move(1); break;
      case "h": case "ArrowLeft": case "b": if (mode === "agenda") move(-1); break;
      case ".": goToday(); sel = 0; break;
      case "/": filterInput?.focus(); break;
      case "r": api.act(refresh); break;
      case "?": help = !help; break;
      case "Escape": help = false; filter = ""; break;
      default: return;
    }
    e.preventDefault();
  }

  function pointer(e: MouseEvent) {
    const el = (e.target as HTMLElement).closest<HTMLElement>("[data-i]");
    if (!el) return;
    sel = +el.dataset.i!;
    if ((e.target as HTMLElement).closest(".check")) op("done");
    else if (e.type === "dblclick") op("open");
  }

  /** Default "When" for a new task: the day being looked at. */
  function dayInput() {
    if (mode !== "agenda") return "";
    const d = span === 1 ? iso(start) : (selected?.date ?? today);
    return d === today ? "today" : d;
  }

  const dateChip = (s: string) => (s.length === 10 ? fmtDay(parse(s), { weekday: "short", day: "numeric", month: "short" }) : "");
</script>

{#snippet row(it: AgendaItem, i: number)}
  <div class="row" class:sel={i === sel} class:done={isDone(it)} data-i={i} role="option" aria-selected={i === sel}>
    <span class="check" title="Mark done (x)">{isDone(it) ? "✓" : ""}</span>
    {#if it.priority}<span class="prio p{it.priority}">{it.priority}</span>{/if}
    {#if it.keyword && it.keyword !== todo[0] && !isDone(it)}<span class="kw">{it.keyword}</span>{/if}
    <span class="title">{it.title}</span>
    {#each it.tags as t}<span class="tag">{t}</span>{/each}
    <span class="spacer"></span>
    {#if it.habit}
      <span class="habit" title="Last {it.habit.bar.length} days: green done, amber due (not yet overdue), red overdue, grey not due">
        {#each it.habit.bar as c}<i class:x={c === "x"} class:ok={c === "-"} class:due={c === "!"}></i>{/each}
      </span>
      <span class="meta" title="Completions in a row">{it.habit.streak} in a row</span>
    {/if}
    {#if mode === "todo" && dateChip(it.date)}<span class="meta">{dateChip(it.date)}</span>{/if}
    {#if it.time}<span class="time">{it.time}</span>{/if}
    {#if it.label && mode === "agenda"}<span class="meta" class:warn={/Overdue|ago|Due/.test(it.label)}>{it.label}</span>{/if}
    <span class="cat">{it.category}</span>
  </div>
{/snippet}

<div class="agenda" bind:this={root} tabindex="0" onkeydown={key} onclick={pointer} ondblclick={pointer} role="listbox">
  <header>
    <div class="titles">
      <h1>{heading}</h1>
      {#if subheading}<span>{subheading}</span>{/if}
    </div>
    <div class="controls">
      {#if mode === "agenda"}
        <div class="seg">
          <button class:on={span === 1} onclick={() => setSpan(1)}>Day</button>
          <button class:on={span === 7} onclick={() => setSpan(7)}>Week</button>
        </div>
        <button onclick={() => move(-1)} title="Previous (h)">‹</button>
        <button onclick={goToday} title="Today (.)">Today</button>
        <button onclick={() => move(1)} title="Next (l)">›</button>
      {/if}
      <input bind:this={filterInput} bind:value={filter} placeholder="Filter  /" spellcheck="false" />
      <button class="primary" onclick={() => api.newTask(dayInput())} title="New task (n)">+ New task</button>
    </div>
  </header>

  <div class="cols" class:clocked={clockOn}>
  {#if clockOn && tc}
    <aside class="clock" aria-label="Clock">
      <h2>Clock</h2>
      <section class="card run" class:on={tc.project != null} class:paused={tc.on_break != null}>
        {#if tc.project != null}
          <div class="state"><span class="dot"></span><small>Tracking since {tc.since?.slice(11, 16)}</small><span class="mono">{elapsed(tc.since)}</span></div>
          <b>{tc.project || "No project"}</b>
          {#if tc.task}<p title={tc.task}>{tc.task}</p>{/if}
          <div class="buttons">
            <button onclick={() => tcRun("pause")}>Pause</button>
            <button onclick={() => tcRun("switch")}>Switch</button>
            <button class="primary" onclick={() => tcRun("stop")}>Stop</button>
          </div>
        {:else if tc.on_break != null}
          <div class="state"><span class="dot"></span><small>On a break</small></div>
          <b>{tc.on_break}</b>
          <div class="buttons"><button class="primary" onclick={() => tcRun("resume")}>Resume</button></div>
        {:else}
          <div class="state"><small>Not tracking</small></div>
          <div class="buttons"><button class="primary" onclick={() => tcRun("start")}>Start tracking</button></div>
        {/if}
      </section>
      {#if day}
        <section class="card worked">
          <h3>Worked today</h3>
          <div class="big mono">{hm(day.hours)}</div>
          <div class="progress" role="progressbar" aria-label="Worked today" aria-valuenow={Math.round(day.progress * 100)}><i style:width="{day.progress * 100}%"></i></div>
          <small>{day.expected > 0 ? `of ${hm(day.expected)} · ${day.left > 0 ? `${hm(day.left)} to go` : "done for today"}` : "no expected hours today"}</small>
        </section>
      {/if}
      <section class="card">
        <h3>This week</h3>
        <WeekBars week={tc.week} height={110} />
      </section>
    </aside>
  {/if}
  <div class="list">
    {#each groups as g (g.key)}
      {@const ni = g.today ? nowIndex(g.rows) : -1}
      {#if g.rows.length || (span === 7 && mode === "agenda")}
        <section class="card" class:today={g.today}>
          {#if mode === "todo" || span === 7}
            <h2 class:today={g.today}>{g.title} <small>{g.sub}</small>{#if g.rows.length}<small class="n">{g.rows.length}</small>{/if}</h2>
          {/if}
          {#each g.rows as { it, i }, j (i)}
            {#if j === ni}<div class="now"><span>now {String(now.getHours()).padStart(2, "0")}:{String(now.getMinutes()).padStart(2, "0")}</span></div>{/if}
            {@render row(it, i)}
          {/each}
          {#if ni === g.rows.length}<div class="now"><span>now</span></div>{/if}
          {#if !g.rows.length}<p class="none">—</p>{/if}
        </section>
      {/if}
    {/each}
    {#if !shown.length}
      <div class="empty">
        <p>{filter ? `Nothing matches “${filter}”.` : query != null ? "No tasks match." : mode === "todo" ? "No open tasks. 🎉" : "Nothing planned."}</p>
        <p>Press <kbd>n</kbd> to add a task{mode === "agenda" ? ", or h / l to look at other days" : ""}.</p>
      </div>
    {/if}
  </div>
  </div>

  {#if help}
    <div class="help">
      <h3>Keys <small>? to close</small></h3>
      <div class="keys">
        {#each ACTIONS as [k, l]}<div><kbd>{k}</kbd>{l}</div>{/each}
        {#each MORE as [k, l]}<div><kbd>{k}</kbd>{l}</div>{/each}
      </div>
    </div>
  {/if}

  <footer class="actions">
    {#if selected}
      {#each ACTIONS as [k, l, name]}
        <button onclick={() => op(name)} disabled={busy}><kbd>{k}</kbd>{name === "done" && isDone(selected) ? "Reopen" : l}</button>
      {/each}
    {/if}
    <span class="spacer"></span>
    <button onclick={() => (help = !help)}><kbd>?</kbd>More</button>
  </footer>
</div>

<style>
  .agenda { height: 100%; display: flex; flex-direction: column; outline: none; font-family: var(--sans); position: relative; background: var(--canvas); }
  header { display: flex; flex-wrap: wrap; align-items: end; justify-content: space-between; gap: var(--s3); padding: 20px var(--s5) var(--s4); }
  .titles h1 { margin: 0; font-size: var(--fs-h1); font-weight: 700; letter-spacing: -0.01em; }
  .titles span { color: var(--dim); font-size: var(--fs-md); }
  .controls { display: flex; gap: var(--s2); align-items: center; flex-wrap: wrap; }
  /* chip-style controls: card surface, soft shadow, no border */
  .controls button, .controls input { background: var(--card); border: 0; box-shadow: var(--soft); color: var(--fg); border-radius: 8px; padding: 6px 12px; font: var(--fs-md) var(--sans); cursor: pointer;
    transition: transform 140ms var(--ease), box-shadow 140ms var(--ease), background-color 120ms var(--ease); }
  .controls button:hover { transform: translateY(-1px); box-shadow: var(--lift); }
  .controls button:focus-visible, .controls input:focus { outline: none; box-shadow: var(--soft), var(--ring); }
  .controls .primary { background: var(--accent); color: var(--bg); font-weight: 600; }
  .seg { display: flex; gap: 2px; padding: 2px; background: var(--card); border-radius: 10px; box-shadow: var(--soft); }
  .controls .seg button { background: none; box-shadow: none; color: var(--dim); }
  .controls .seg button:hover { transform: none; color: var(--fg); }
  .controls .seg button.on { background: var(--sel); color: var(--fg); font-weight: 500; }
  .controls input { cursor: text; width: 140px; }
  /* Clock column (round-5 mockup, #357): left of the list; under it on narrow windows */
  .cols { flex: 1; min-height: 0; display: flex; }
  .clock { flex: 0 0 300px; overflow-y: auto; padding: 2px 0 var(--s5) var(--s5); display: flex; flex-direction: column; gap: var(--s3); }
  .clock > h2 { margin: 0; }
  .clock .card { flex: none; padding: var(--s3) var(--s4); }
  .clock h3 { font-size: var(--fs-md); font-weight: 600; margin: 0 0 var(--s2); }
  .clock small { color: var(--dim); font-size: var(--fs-sm); }
  .mono { font-family: var(--mono); }
  .run { display: flex; flex-direction: column; gap: var(--s1); }
  .run.on { background: linear-gradient(160deg, color-mix(in srgb, var(--done) 12%, var(--card)), var(--card) 60%); }
  .run.paused { background: linear-gradient(160deg, color-mix(in srgb, var(--h5) 12%, var(--card)), var(--card) 60%); }
  .run .state { display: flex; align-items: center; gap: var(--s2); }
  .run .state .mono { margin-left: auto; font-size: 20px; }
  .run b { font-size: 18px; overflow-wrap: anywhere; }
  .run p { margin: 0; color: var(--dim); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--done); box-shadow: 0 0 0 3px color-mix(in srgb, var(--done) 25%, transparent); flex: none; }
  .paused .dot { background: var(--h5); box-shadow: 0 0 0 3px color-mix(in srgb, var(--h5) 25%, transparent); }
  .buttons { display: flex; flex-wrap: wrap; gap: var(--s2); margin-top: var(--s2); }
  .buttons button { background: var(--active); border: 0; color: var(--fg); border-radius: 8px; padding: 5px var(--s3); font: var(--fs-md) var(--sans); cursor: pointer; transition: background-color 120ms var(--ease); }
  .buttons button:hover { background: color-mix(in srgb, var(--active) 70%, var(--border)); }
  .buttons button:focus-visible { outline: none; box-shadow: var(--ring); }
  .buttons .primary { background: var(--accent); color: var(--bg); font-weight: 600; }
  .worked .big { font-size: 28px; font-weight: 500; }
  .progress { height: 8px; border-radius: 999px; background: var(--active); overflow: hidden; margin: var(--s2) 0; }
  .progress i { display: block; height: 100%; background: var(--done); border-radius: inherit; }
  @media (max-width: 759px) {
    .cols.clocked { flex-direction: column; overflow-y: auto; }
    .cols.clocked .list { flex: none; overflow: visible; }
    .cols.clocked .clock { order: 1; flex: none; overflow: visible; padding: 0 var(--s5) var(--s5); }
  }
  .list { flex: 1; min-width: 0; overflow-y: auto; padding: 2px var(--s5) var(--s5); display: flex; flex-direction: column; gap: var(--s3); }
  .list > .card { flex: none; padding: var(--s3); }
  .list > .card.today { box-shadow: var(--soft), inset 3px 0 0 var(--accent); }
  /* long scrolling cards: no hover lift (the card moves under the pointer and flickers) */
  .list > .card:hover { transform: none; box-shadow: var(--soft); }
  .list > .card.today:hover { box-shadow: var(--soft), inset 3px 0 0 var(--accent); }
  h2 { font-size: var(--fs-md); font-weight: 650; margin: 0 0 var(--s2); padding: 0 var(--s2); color: var(--fg); display: flex; gap: var(--s2); align-items: baseline; }
  h2.today { color: var(--accent); }
  h2 small { color: var(--dim); font-weight: 400; }
  h2 .n { font-size: var(--fs-xs); background: var(--active); border-radius: 999px; padding: 0 7px; }
  .row { display: flex; align-items: center; gap: var(--s2); padding: 6px var(--s2); border-radius: 8px; cursor: default; font-size: var(--fs-base); min-height: 22px; transition: background-color 120ms var(--ease); }
  .row:hover { background: var(--active); }
  .row.sel { background: var(--sel); }
  .row.done .title { text-decoration: line-through; color: var(--dim); }
  .check { width: 15px; height: 15px; border: 1.5px solid var(--dim); border-radius: 50%; flex: none; display: grid; place-items: center; font-size: var(--fs-xs); color: var(--done); cursor: pointer; transition: border-color 120ms var(--ease); }
  .row.done .check { border-color: var(--done); }
  .check:hover { border-color: var(--accent); }
  .prio { font-size: var(--fs-xs); font-weight: 700; border-radius: var(--radius-sm); padding: 0 5px; background: var(--active); color: var(--dim); }
  .prio.pA { color: var(--todo); }
  .prio.pB { color: var(--h5); }
  .kw { font-size: var(--fs-xs); font-weight: 700; color: var(--link); letter-spacing: 0.03em; }
  .title { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; }
  .tag { font-size: var(--fs-xs); color: var(--dim); background: var(--active); border-radius: 999px; padding: 0 7px; white-space: nowrap; }
  .spacer { flex: 1; }
  .habit { display: inline-flex; gap: 1px; }
  .habit i { width: 4px; height: 12px; border-radius: 1px; background: var(--active); }
  .habit i.x { background: var(--done); }
  .habit i.ok { background: var(--accent); }
  .habit i.due { background: var(--todo); }
  .time { font: var(--fs-sm) var(--mono); color: var(--date); }
  .meta { font-size: var(--fs-sm); color: var(--dim); white-space: nowrap; }
  .meta.warn { color: var(--todo); background: color-mix(in srgb, var(--todo) 14%, transparent); border-radius: var(--radius-sm); padding: 0 6px; }
  .cat { font-size: var(--fs-sm); color: var(--dim); max-width: 16ch; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .now { display: flex; align-items: center; gap: var(--s2); color: var(--accent); font-size: var(--fs-xs); margin: 2px 0; }
  .now::after { content: ""; flex: 1; border-top: 1px dashed var(--accent); }
  .none { color: var(--dim); margin: 0 0 0 var(--s2); font-size: var(--fs-sm); }
  .empty { color: var(--dim); text-align: center; margin-top: 15vh; }
  kbd { font: var(--fs-xs) var(--mono); background: var(--active); border: 1px solid var(--border); border-radius: var(--radius-sm); padding: 0 var(--s1); margin-right: 5px; color: var(--fg); }
  /* key-hint pill: in flow (never covers rows), centred, wraps on narrow windows */
  .actions { display: flex; flex-wrap: wrap; justify-content: center; gap: 2px; align-self: center; max-width: calc(100% - 2 * var(--s4)); margin: 0 var(--s4) var(--s3); padding: var(--s1) var(--s3); background: var(--card); border-radius: 20px; box-shadow: var(--lift); }
  .actions .spacer { flex: 0 0 var(--s2); }
  .actions button { background: none; border: 0; color: var(--fg); font: var(--fs-sm) var(--sans); padding: 3px 6px; border-radius: 8px; cursor: pointer; transition: background-color 120ms var(--ease); }
  .actions button:hover { background: var(--active); }
  .actions button:focus-visible { outline: none; box-shadow: var(--ring); }
  /* in flow above the action pill, so a pill wrapped to two lines on a narrow window never hides under it */
  .help { align-self: flex-end; flex: none; max-width: calc(100% - 2 * var(--s5)); max-height: 50%; overflow-y: auto; box-sizing: border-box; margin: 0 var(--s5) var(--s2); background: var(--card); border-radius: var(--r); padding: 14px 18px; box-shadow: var(--pshadow); z-index: 1; }
  .help h3 { margin: 0 0 var(--s2); font-size: var(--fs-md); }
  .help h3 small { color: var(--dim); font-weight: 400; }
  @media (prefers-reduced-motion: reduce) {
    .controls button, .controls input, .row, .check, .actions button { transition: none; }
    .controls button:hover { transform: none; }
  }
  .keys { display: grid; grid-template-columns: repeat(2, minmax(200px, auto)); gap: var(--s1) 20px; font-size: var(--fs-md); }
  @media (max-width: 559px) { .keys { grid-template-columns: 1fr; } }
</style>
