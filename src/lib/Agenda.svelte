<script lang="ts" module>
  export type AgendaItem = {
    date: string; kind: string; label: string; time: string | null; path: string; line: number;
    keyword: string | null; priority: string | null; title: string; tags: string[]; category: string;
    /** Habits on today: last 21 days ('x' done, '!' due, '.' not due) and streak. */
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

  /** QUERY (todo mode): show only open tasks matching it, under TITLE. */
  let { mode, api, active, reload, todo, done, query, title }: {
    mode: "agenda" | "todo"; api: AgendaApi; active: boolean; reload: number; todo: string[]; done: string[]; query?: string; title?: string;
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

  async function refresh() {
    now = new Date();
    items = mode === "agenda" ? await invoke("agenda", { start: iso(start), days: span })
      : query != null ? await invoke("search_todos", { query }) : await invoke("todos");
    sel = Math.min(sel, Math.max(0, shown.length - 1));
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
    const t = setInterval(() => (now = new Date()), 60_000);
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
      <span class="habit" title="Last {it.habit.bar.length} days: green done, red due, grey not due">
        {#each it.habit.bar as c}<i class:x={c === "x"} class:due={c === "!"}></i>{/each}
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

  <div class="list">
    {#each groups as g (g.key)}
      {#if mode === "todo" || span === 7}
        <h2 class:today={g.today}>{g.title} <small>{g.sub}</small>{#if mode === "todo"}<small>{g.rows.length}</small>{/if}</h2>
      {/if}
      {@const ni = g.today ? nowIndex(g.rows) : -1}
      {#each g.rows as { it, i }, j (i)}
        {#if j === ni}<div class="now"><span>now {String(now.getHours()).padStart(2, "0")}:{String(now.getMinutes()).padStart(2, "0")}</span></div>{/if}
        {@render row(it, i)}
      {/each}
      {#if ni === g.rows.length}<div class="now"><span>now</span></div>{/if}
      {#if span === 7 && mode === "agenda" && !g.rows.length}<p class="none">—</p>{/if}
    {/each}
    {#if !shown.length}
      <div class="empty">
        <p>{filter ? `Nothing matches “${filter}”.` : query != null ? "No tasks match." : mode === "todo" ? "No open tasks. 🎉" : "Nothing planned."}</p>
        <p>Press <kbd>n</kbd> to add a task{mode === "agenda" ? ", or h / l to look at other days" : ""}.</p>
      </div>
    {/if}
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
  .agenda { height: 100%; display: flex; flex-direction: column; outline: none; font-family: var(--sans); position: relative; }
  header { display: flex; flex-wrap: wrap; align-items: end; justify-content: space-between; gap: 12px; padding: 20px 28px 12px; }
  .titles h1 { margin: 0; font-size: 22px; font-weight: 650; }
  .titles span { color: var(--dim); font-size: 13px; }
  .controls { display: flex; gap: 6px; align-items: center; flex-wrap: wrap; }
  .controls button, .seg button { background: var(--panel); border: 1px solid var(--border); color: var(--fg); border-radius: 6px; padding: 5px 10px; font: 13px var(--sans); cursor: pointer; }
  .controls .primary { background: var(--accent); border-color: var(--accent); color: var(--bg); font-weight: 600; }
  .seg { display: flex; }
  .seg button { border-radius: 0; }
  .seg button:first-child { border-radius: 6px 0 0 6px; }
  .seg button:last-child { border-radius: 0 6px 6px 0; border-left: 0; }
  .seg button.on { background: var(--sel); }
  .controls input { background: var(--panel); border: 1px solid var(--border); border-radius: 6px; color: var(--fg); padding: 5px 9px; font: 13px var(--sans); width: 140px; }
  .list { flex: 1; overflow-y: auto; padding: 0 28px 24px; }
  h2 { font-size: 13px; font-weight: 650; margin: 16px 0 4px; color: var(--h2); display: flex; gap: 8px; align-items: baseline; }
  h2.today { color: var(--accent); }
  h2 small { color: var(--dim); font-weight: 400; }
  .row { display: flex; align-items: center; gap: 8px; padding: 5px 8px; border-radius: 6px; cursor: default; font-size: 14px; min-height: 22px; }
  .row:hover { background: var(--active); }
  .row.sel { background: var(--sel); }
  .row.done .title { text-decoration: line-through; color: var(--dim); }
  .check { width: 15px; height: 15px; border: 1.5px solid var(--dim); border-radius: 4px; flex: none; display: grid; place-items: center; font-size: 11px; color: var(--done); cursor: pointer; }
  .row.done .check { border-color: var(--done); }
  .check:hover { border-color: var(--accent); }
  .prio { font-size: 11px; font-weight: 700; border-radius: 4px; padding: 0 5px; background: var(--active); color: var(--dim); }
  .prio.pA { color: var(--todo); }
  .prio.pB { color: var(--h5); }
  .kw { font-size: 11px; font-weight: 700; color: var(--link); letter-spacing: 0.03em; }
  .title { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; }
  .tag { font-size: 11px; color: var(--dim); background: var(--active); border-radius: 999px; padding: 0 7px; white-space: nowrap; }
  .spacer { flex: 1; }
  .habit { display: inline-flex; gap: 1px; }
  .habit i { width: 4px; height: 12px; border-radius: 1px; background: var(--active); }
  .habit i.x { background: var(--done); }
  .habit i.due { background: var(--todo); }
  .time { font: 12px var(--mono); color: var(--date); }
  .meta { font-size: 12px; color: var(--dim); white-space: nowrap; }
  .meta.warn { color: var(--todo); }
  .cat { font-size: 12px; color: var(--dim); max-width: 16ch; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .now { display: flex; align-items: center; gap: 8px; color: var(--accent); font-size: 11px; margin: 2px 0; }
  .now::after { content: ""; flex: 1; border-top: 1px dashed var(--accent); }
  .none { color: var(--dim); margin: 0 0 0 8px; font-size: 12px; }
  .empty { color: var(--dim); text-align: center; margin-top: 15vh; }
  kbd { font: 11px var(--mono); background: var(--active); border: 1px solid var(--border); border-radius: 4px; padding: 0 4px; margin-right: 5px; color: var(--fg); }
  .actions { display: flex; flex-wrap: wrap; gap: 4px; padding: 8px 20px; border-top: 1px solid var(--border); background: var(--panel); }
  .actions button { background: none; border: 0; color: var(--fg); font: 12px var(--sans); padding: 3px 6px; border-radius: 4px; cursor: pointer; }
  .actions button:hover { background: var(--active); }
  .help { position: absolute; right: 20px; bottom: 52px; background: var(--panel); border: 1px solid var(--border); border-radius: 10px; padding: 14px 18px; box-shadow: 0 12px 32px rgb(0 0 0 / 0.35); }
  .help h3 { margin: 0 0 8px; font-size: 13px; }
  .help h3 small { color: var(--dim); font-weight: 400; }
  .keys { display: grid; grid-template-columns: repeat(2, minmax(200px, auto)); gap: 4px 20px; font-size: 13px; }
</style>
