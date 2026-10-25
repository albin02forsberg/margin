<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { tick } from "svelte";
  import { spark } from "$lib/habits";
  import type { AgendaItem } from "$lib/Agenda.svelte";
  import type { TimeApi } from "$lib/Time.svelte";

  type Row = AgendaItem & { habit: { bar: string; streak: number; best: number; pct: number } };
  /** Every open habit over the last 30 or 90 days (#802): consistency bar, weekly sparkline, current and best streak,
   *  on-time rate per due occurrence; worst first. `r` toggles the window, a click opens the heading. Reloads on RELOAD. */
  let { api, active, reload }: { api: TimeApi; active: boolean; reload: number } = $props();
  let days = $state(30);
  let rows = $state.raw<Row[]>([]);
  let loaded = $state(false);
  let root = $state<HTMLDivElement>();

  $effect(() => {
    void [reload, days];
    if (active) api.act(async () => { rows = await invoke<Row[]>("habit_overview", { days }); loaded = true; });
  });
  $effect(() => {
    if (active) tick().then(() => root?.focus());
  });
  const toggle = () => (days = days === 30 ? 90 : 30);
  function key(e: KeyboardEvent) {
    if (e.ctrlKey || e.metaKey || e.altKey || e.key !== "r") return;
    e.preventDefault();
    toggle();
  }
  const open = (r: Row) => api.act(() => api.run("open", { path: r.path, line: r.line }));
</script>

<div class="habits" bind:this={root} tabindex="0" onkeydown={key} role="toolbar" aria-label="Habits">
  <header>
    <h1>Habits</h1>
    <button class="days" title="Switch between 30 and 90 days (r)" onclick={toggle}>Last {days} days</button>
  </header>
  {#if loaded && !rows.length}
    <p class="dim">No habits yet: give a repeating task <code>:STYLE: habit</code> in its properties.</p>
  {:else if loaded}
    <div class="card"><table>
      <thead><tr><th>Habit</th><th>Last {days} days</th><th>Weeks</th><th class="num">Streak</th><th class="num">Best</th><th class="num">On time</th></tr></thead>
      <tbody>
        {#each rows as r (`${r.path}:${r.line}`)}
          <tr class="habit">
            <td><button class="link" title="Open the heading" onclick={() => open(r)}>{r.title}</button></td>
            <td><span class="bar" title="Green done, amber due (not yet overdue), red overdue, grey not due">
              {#each r.habit.bar as c}<i class:x={c === "x"} class:ok={c === "-"} class:due={c === "!"}></i>{/each}
            </span></td>
            <td class="spark" title="Completions per week">{spark(r.habit.bar)}</td>
            <td class="num">{r.habit.streak}</td>
            <td class="num">{r.habit.best}</td>
            <td class="num pct" class:low={r.habit.pct < 50}>{r.habit.pct}%</td>
          </tr>
        {/each}
      </tbody>
    </table></div>
  {/if}
</div>

<style>
  .habits { height: 100%; overflow-y: auto; background: var(--canvas); padding: 20px 28px 40px; box-sizing: border-box; font-family: var(--sans); outline: none; }
  header { display: flex; align-items: baseline; gap: var(--s4); margin: 0 0 18px; }
  h1 { margin: 0; font-size: 22px; font-weight: 650; }
  button { background: var(--active); border: 1px solid transparent; color: var(--fg); border-radius: 8px; padding: 2px var(--s3); font: var(--fs-md) var(--sans); cursor: pointer; }
  button:hover { border-color: var(--border); }
  .card { padding: var(--s2) var(--s4); }
  table { width: 100%; border-collapse: collapse; font-size: var(--fs-base); }
  td, th { padding: 6px var(--s2); border-bottom: 1px solid var(--border); text-align: left; }
  td:first-child, th:first-child { padding-left: 0; }
  td:last-child, th:last-child { padding-right: 0; }
  th { color: var(--dim); font-weight: 500; font-size: var(--fs-sm); white-space: nowrap; }
  tr:last-child td { border-bottom: 0; }
  .habit:hover td { background: var(--active); }
  .num { text-align: right; font-family: var(--mono); white-space: nowrap; width: 1%; }
  .low { color: var(--todo); }
  .spark { font-family: var(--mono); color: var(--done); white-space: nowrap; letter-spacing: 1px; }
  .bar { display: inline-flex; gap: 1px; }
  .bar i { width: 4px; height: 12px; border-radius: 1px; background: var(--active); }
  .bar i.x { background: var(--done); }
  .bar i.ok { background: var(--accent); }
  .bar i.due { background: var(--todo); }
  button.link { background: none; border: 0; padding: 0; text-align: left; overflow-wrap: anywhere; width: 100%; }
  .dim { color: var(--dim); font-size: var(--fs-md); }
</style>
