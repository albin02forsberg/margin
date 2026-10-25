<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { tick } from "svelte";
  import { money, monthName, shiftMonth } from "$lib/ledger";
  import type { TimeApi } from "$lib/Time.svelte";

  type Total = { currency: string; income: number; expense: number; net: number; categories: [string | null, number][]; payees: [string, number][] };
  type Row = { line: number; date: string; cents: number; currency: string; text: string; category: string | null; payee: string | null };
  /** A month of the ledger file (#789): totals per currency, category and payee, then the entries (click: open at its line).
   *  `h` / `l` or the arrows switch months. Reloads on RELOAD. */
  let { api, active, reload }: { api: TimeApi; active: boolean; reload: number } = $props();
  const thisMonth = new Date().toLocaleDateString("sv-SE").slice(0, 7);
  let month = $state(thisMonth);
  let path = $state<string | null>(null);
  let totals = $state.raw<Total[]>([]);
  let rows = $state.raw<Row[]>([]);
  let loaded = $state(false);
  let root = $state<HTMLDivElement>();

  $effect(() => {
    void [reload, month];
    if (active) api.act(async () => { [path, totals, rows] = await invoke<[string | null, Total[], Row[]]>("ledger_month", { month }); loaded = true; });
  });
  $effect(() => {
    if (active) tick().then(() => root?.focus());
  });
  const shift = (n: number) => (month = shiftMonth(month, n));
  function key(e: KeyboardEvent) {
    if (e.ctrlKey || e.metaKey || e.altKey || !(e.key === "h" || e.key === "l")) return;
    e.preventDefault();
    shift(e.key === "h" ? -1 : 1);
  }
  const open = (line: number) => api.act(() => api.run("open", { path, line }));
</script>

<div class="ledger" bind:this={root} tabindex="0" onkeydown={key} role="toolbar" aria-label="Ledger">
  <header>
    <h1>Ledger</h1>
    <div class="switch">
      <button title="Previous month (h)" onclick={() => shift(-1)}>‹</button>
      <strong class="month">{monthName(month)}</strong>
      <button title="Next month (l)" onclick={() => shift(1)}>›</button>
      {#if month !== thisMonth}<button class="today" onclick={() => (month = thisMonth)}>This month</button>{/if}
    </div>
  </header>
  {#if loaded && !path}
    <p class="dim">No ledger yet: <kbd>Space</kbd> <kbd>f</kbd> <kbd>l</kbd> adds the first entry.</p>
  {:else if loaded && !rows.length}
    <p class="dim empty">No entries in {monthName(month)}.</p>
  {:else if loaded}
    <div class="cards">
      {#each totals as t (t.currency)}
        <section class="card total">
          <h3>{t.currency}</h3>
          <div class="sums">
            <div><small>Income</small><strong>{money(t.income, t.currency, false)}</strong></div>
            <div><small>Expenses</small><strong>{money(-t.expense, t.currency, false)}</strong></div>
            <div><small>Net</small><strong class:neg={t.net < 0}>{money(t.net, t.currency)}</strong></div>
          </div>
          <table class="cats">
            <thead><tr><th>Category</th><th></th></tr></thead>
            <tbody>
              {#each t.categories as [c, n]}<tr><td class:dim={!c}>{c ? `#${c}` : "uncategorised"}</td><td class="num">{money(n, t.currency)}</td></tr>{/each}
            </tbody>
          </table>
          {#if t.payees.length}
            <table class="payees">
              <thead><tr><th>Payee</th><th></th></tr></thead>
              <tbody>
                {#each t.payees as [p, n]}<tr><td>@{p}</td><td class="num">{money(n, t.currency)}</td></tr>{/each}
              </tbody>
            </table>
          {/if}
        </section>
      {/each}
    </div>
    <h2>Entries</h2>
    <div class="card list"><table>
      <tbody>
        {#each rows as r (r.line)}
          <tr class="entry">
            <td class="mono">{r.date}</td>
            <td class="num" class:neg={r.cents < 0}>{money(r.cents, r.currency)}</td>
            <td><button class="link" title="Open in the ledger file" onclick={() => open(r.line)}>{r.text || "(no text)"}</button></td>
          </tr>
        {/each}
      </tbody>
    </table></div>
  {/if}
</div>

<style>
  .ledger { height: 100%; overflow-y: auto; background: var(--canvas); padding: 20px 28px 40px; box-sizing: border-box; font-family: var(--sans); outline: none; }
  header { display: flex; align-items: baseline; gap: var(--s4); flex-wrap: wrap; margin: 0 0 18px; }
  h1 { margin: 0; font-size: 22px; font-weight: 650; }
  h2 { margin: 28px 0 12px; font-size: 17px; font-weight: 650; }
  h3 { font-size: var(--fs-md); font-weight: 600; margin: 0 0 var(--s3); }
  .switch { display: flex; gap: var(--s2); align-items: center; }
  .month { min-width: 9em; text-align: center; font-weight: 500; }
  button { background: var(--active); border: 1px solid transparent; color: var(--fg); border-radius: 8px; padding: 2px var(--s3); font: var(--fs-md) var(--sans); cursor: pointer; }
  button:hover { border-color: var(--border); }
  .today { font-size: var(--fs-sm); }
  .cards { display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); gap: var(--s4); align-items: start; }
  .card { padding: var(--s4); }
  .list { padding: var(--s2) var(--s4); }
  .sums { display: grid; grid-template-columns: repeat(3, 1fr); gap: var(--s2); margin-bottom: var(--s3); }
  .sums > div { background: var(--canvas); border-radius: 8px; padding: var(--s2) var(--s3); }
  .sums small { color: var(--dim); font-size: var(--fs-sm); display: block; }
  .sums strong { font: 500 16px var(--mono); white-space: nowrap; }
  .neg { color: var(--todo); }
  table { width: 100%; border-collapse: collapse; font-size: var(--fs-base); }
  table + table { margin-top: var(--s3); }
  td, th { padding: 6px var(--s2); border-bottom: 1px solid var(--border); text-align: left; }
  td:first-child, th:first-child { padding-left: 0; }
  td:last-child, th:last-child { padding-right: 0; }
  th { color: var(--dim); font-weight: 500; font-size: var(--fs-sm); }
  tr:last-child td { border-bottom: 0; }
  .num { text-align: right; font-family: var(--mono); white-space: nowrap; }
  .mono { font-family: var(--mono); font-size: var(--fs-md); white-space: nowrap; }
  .entry:hover td { background: var(--active); }
  .entry .num { width: 1%; }
  button.link { background: none; border: 0; padding: 0; text-align: left; overflow-wrap: anywhere; width: 100%; }
  .dim { color: var(--dim); font-size: var(--fs-md); }
  kbd { font: var(--fs-xs) var(--mono); color: var(--dim); background: var(--card); border: 1px solid var(--border); border-radius: var(--radius-sm); padding: 0 var(--s1); }
</style>
