<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { tick } from "svelte";
  import type { AgendaItem } from "./Agenda.svelte";
  import type { TimeApi } from "./Time.svelte";
  import { fmtEffort, overbooked, plannedLabel } from "./effort";
  import { monthName, shiftMonth } from "./ledger";
  import { addDays, bucket, monthGrid, type Day } from "./monthcal";

  /** The month's planned tasks (#1039): a Monday-first grid with each day's scheduled tasks, deadlines and planned
   *  effort, tinted when it overbooks the day's expected hours. h/j/k/l or arrows move, [ / ] change month, t today,
   *  Enter or a click opens the day in Today (OPEN). DONE: the done keywords. Reloads on RELOAD. */
  let { api, active, reload, done, open }: { api: TimeApi; active: boolean; reload: number; done: string[]; open: (day: string) => void } = $props();
  const today = new Date().toLocaleDateString("sv-SE");
  let sel = $state(today);
  const month = $derived(sel.slice(0, 7));
  const grid = $derived(monthGrid(month));
  let days = $state.raw<Record<string, Day>>({});
  let expected = $state.raw<Record<string, number>>({});
  let root = $state<HTMLDivElement>();
  const WEEK = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

  $effect(() => {
    void reload;
    const ds = grid;
    if (!active) return;
    api.act(async () => {
      const items = await invoke<AgendaItem[]>("agenda", { start: ds[0], days: ds.length });
      days = bucket(items, (it) => !!it.keyword && done.includes(it.keyword));
      const hs = await invoke<number[]>("expected_hours_for", { dates: ds }).catch(() => [] as number[]);
      expected = Object.fromEntries(ds.map((d, i) => [d, hs[i]]));
    });
  });
  $effect(() => {
    if (active) tick().then(() => root?.focus());
  });
  /** Move the selection to the same day N months on (clamped to that month's end). */
  function shift(n: number) {
    const m = shiftMonth(month, n), last = monthGrid(m).filter((d) => d.startsWith(m)).at(-1)!;
    sel = `${m}-${sel.slice(8)}` > last ? last : `${m}-${sel.slice(8)}`;
  }
  const MOVES: Record<string, number> = { h: -1, ArrowLeft: -1, l: 1, ArrowRight: 1, k: -7, ArrowUp: -7, j: 7, ArrowDown: 7 };
  function key(e: KeyboardEvent) {
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    if (e.key in MOVES) sel = addDays(sel, MOVES[e.key]);
    else if (e.key === "[" || e.key === "]") shift(e.key === "[" ? -1 : 1);
    else if (e.key === "t") sel = today;
    else if (e.key === "Enter") open(sel);
    else return;
    e.preventDefault();
  }
  const label = (d: string, x?: Day) => [x?.titles.join(", "), x?.effort ? plannedLabel(x.effort, expected[d]) : ""].filter(Boolean).join(" · ") || "Nothing planned";
</script>

<div class="monthcal" bind:this={root} tabindex="0" onkeydown={key} role="grid" aria-label="Month calendar">
  <header>
    <h1>Calendar</h1>
    <div class="switch">
      <button title="Previous month ([)" onclick={() => shift(-1)}>‹</button>
      <strong class="month">{monthName(month)}</strong>
      <button title="Next month (])" onclick={() => shift(1)}>›</button>
      {#if month !== today.slice(0, 7)}<button class="today" onclick={() => (sel = today)}>Today (t)</button>{/if}
    </div>
    <span class="dim">h j k l move · Enter opens the day in Today</span>
  </header>
  <div class="grid">
    {#each WEEK as w}<div class="wd">{w}</div>{/each}
    {#each grid as d (d)}
      {@const x = days[d]}
      <button class="day" class:out={!d.startsWith(month)} class:sel={d === sel} class:now={d === today}
        class:over={!!x?.effort && overbooked(x.effort, expected[d])} data-day={d} title={label(d, x)}
        onclick={() => { sel = d; open(d); }}>
        <span class="n">{+d.slice(8)}</span>
        {#if x}
          <span class="counts">
            {#if x.scheduled}<span>{x.scheduled} task{x.scheduled === 1 ? "" : "s"}</span>{/if}
            {#if x.deadline}<span class="due">{x.deadline} due</span>{/if}
            {#if x.effort}<span class="eff">~{fmtEffort(x.effort)}</span>{/if}
          </span>
          {#each x.titles.slice(0, 3) as t}<span class="t">{t}</span>{/each}
          {#if x.titles.length > 3}<span class="t dim">+{x.titles.length - 3} more</span>{/if}
        {/if}
      </button>
    {/each}
  </div>
</div>

<style>
  .monthcal { height: 100%; overflow-y: auto; background: var(--canvas); padding: 20px 28px 40px; box-sizing: border-box; font-family: var(--sans); outline: none; }
  header { display: flex; align-items: baseline; gap: var(--s4); flex-wrap: wrap; margin: 0 0 18px; }
  h1 { margin: 0; font-size: 22px; font-weight: 650; }
  .switch { display: flex; gap: var(--s2); align-items: center; }
  .month { min-width: 9em; text-align: center; font-weight: 500; }
  button { background: var(--active); border: 1px solid transparent; color: var(--fg); border-radius: 8px; padding: 2px var(--s3); font: var(--fs-md) var(--sans); cursor: pointer; }
  button:hover { border-color: var(--border); }
  .today { font-size: var(--fs-sm); }
  .dim { color: var(--dim); font-size: var(--fs-sm); }
  .grid { display: grid; grid-template-columns: repeat(7, minmax(0, 1fr)); gap: var(--s1); }
  .wd { color: var(--dim); font-size: var(--fs-sm); padding: 0 var(--s2); }
  .day { display: flex; flex-direction: column; align-items: stretch; gap: 2px; min-height: 92px; padding: var(--s1) var(--s2); text-align: left; background: var(--card); border: 1px solid var(--border); overflow: hidden; }
  .day.out { opacity: 0.45; }
  .day.over { background: color-mix(in srgb, var(--todo) 14%, var(--card)); }
  .day.sel { outline: 2px solid var(--accent); outline-offset: -1px; }
  .n { font: 500 var(--fs-md) var(--mono); }
  .now .n { color: var(--accent); font-weight: 700; }
  .counts { display: flex; gap: var(--s2); flex-wrap: wrap; font-size: var(--fs-xs); color: var(--dim); }
  .due { color: var(--todo); }
  .t { font-size: var(--fs-xs); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
</style>
