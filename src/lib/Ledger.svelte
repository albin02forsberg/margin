<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { tick } from "svelte";
  import { budgetBar, filterRows, money, monthName, savingsRate, shiftMonth, sumByCurrency } from "$lib/ledger";
  import type { TimeApi } from "$lib/Time.svelte";

  type Total = { currency: string; income: number; expense: number; net: number; categories: [string | null, number][]; payees: [string, number][] };
  type Row = { line: number; date: string; cents: number; currency: string; text: string; category: string | null; payee: string | null };
  /** A month of the ledger file (#789): totals per currency, category and payee, then the entries (click: open at its line).
   *  `h` / `l` or the arrows switch months; `y` toggles the year view (#1196). Reloads on RELOAD. */
  let { api, active, reload }: { api: TimeApi; active: boolean; reload: number } = $props();
  const thisMonth = new Date().toLocaleDateString("sv-SE").slice(0, 7);
  let month = $state(thisMonth);
  let path = $state<string | null>(null);
  let totals = $state.raw<Total[]>([]);
  let rows = $state.raw<Row[]>([]);
  type Budget = { category: string; currency: string; budget: number; used: number };
  let budgets = $state.raw<Budget[]>([]);
  let loaded = $state(false);
  let root = $state<HTMLDivElement>();
  let filter = $state("");
  const shown = $derived(filterRows(rows, filter));
  const toggle = (q: string) => (filter = filter === q ? "" : q);

  $effect(() => {
    void [reload, month];
    if (active && !yearView) api.act(async () => { [path, totals, rows, budgets] = await invoke<[string | null, Total[], Row[], Budget[]]>("ledger_month", { month }); loaded = true; });
  });
  $effect(() => {
    if (active) tick().then(() => root?.focus());
  });
  type Year = { currency: string; income: number[]; expense: number[]; categories: [string | null, number[]][] };
  let yearView = $state(false);
  let years = $state.raw<Year[]>([]);
  const year = $derived(month.slice(0, 4));
  $effect(() => {
    void [reload, year];
    if (active && yearView) api.act(async () => { [path, years] = await invoke<[string | null, Year[]]>("ledger_year", { year }); loaded = true; });
  });
  const shortMonths = Array.from({ length: 12 }, (_, i) => new Date(2000, i, 1).toLocaleDateString(undefined, { month: "short" }));
  const sum = (m: number[]) => m.reduce((a, n) => a + n, 0);
  const cell = (n: number) => (n ? money(n, "", false).trim() : "");
  /** Back or forward one month, or one year in the year view. */
  const shift = (n: number) => ((month = shiftMonth(month, yearView ? 12 * n : n)), (filter = ""));
  function key(e: KeyboardEvent) {
    if ((e.target as HTMLElement).tagName === "INPUT") return;
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    const n = ({ h: -1, l: 1, ...(yearView ? { "[": -1, "]": 1 } : {}) } as Record<string, number>)[e.key];
    if (!n && e.key !== "y") return;
    e.preventDefault();
    if (n) shift(n);
    else yearView = !yearView;
  }
  const open = (line: number) => api.act(() => api.run("open", { path, line }));
</script>

<div class="ledger" bind:this={root} tabindex="0" onkeydown={key} role="toolbar" aria-label="Ledger">
  <header>
    <h1>Ledger</h1>
    <div class="switch">
      <button title={yearView ? "Previous year (h)" : "Previous month (h)"} onclick={() => shift(-1)}>‹</button>
      <strong class="month">{yearView ? year : monthName(month)}</strong>
      <button title={yearView ? "Next year (l)" : "Next month (l)"} onclick={() => shift(1)}>›</button>
      {#if yearView ? year !== thisMonth.slice(0, 4) : month !== thisMonth}<button class="today" onclick={() => ((month = thisMonth), (filter = ""))}>This {yearView ? "year" : "month"}</button>{/if}
      <button class="today" title="Toggle the year view (y)" onclick={() => (yearView = !yearView)}>{yearView ? "Month" : "Year"}</button>
    </div>
  </header>
  {#if loaded && !path}
    <p class="dim">No ledger yet: <kbd>Space</kbd> <kbd>f</kbd> <kbd>l</kbd> adds the first entry.</p>
  {:else if loaded && yearView}
    {#each years as y (y.currency)}
      {@const net = y.income.map((n, i) => n + y.expense[i])}
      {@const rate = savingsRate(sum(y.income), sum(net))}
      {@const max = Math.max(1, ...y.categories.flatMap(([, m]) => m.map(Math.abs)))}
      <section class="card year">
        <h3>{y.currency}{#if rate !== null}<small class="dim"> · savings rate {rate}%</small>{/if}</h3>
        <div class="scroll"><table>
          <thead><tr><th></th>{#each shortMonths as name, i}<th class="num"><button class="link" title="Open this month" onclick={() => { month = `${year}-${String(i + 1).padStart(2, "0")}`; yearView = false; }}>{name}</button></th>{/each}<th class="num">Total</th></tr></thead>
          <tbody>
            {#each [{ label: "Income", m: y.income }, { label: "Expenses", m: y.expense }, { label: "Net", m: net }] as r (r.label)}
              <tr class="sum"><td>{r.label}</td>{#each r.m as n}<td class="num" class:neg={n < 0}>{cell(n)}</td>{/each}<td class="num" class:neg={sum(r.m) < 0}>{cell(sum(r.m))}</td></tr>
            {/each}
            {#each y.categories as [c, m] (c)}
              <tr><td class:dim={!c}>{c ? `#${c}` : "uncategorised"}</td>{#each m as n}<td class="num heat" style:--a={Math.abs(n) / max}>{cell(n)}</td>{/each}<td class="num">{cell(sum(m))}</td></tr>
            {/each}
          </tbody>
        </table></div>
      </section>
    {:else}
      <p class="dim empty">No entries in {year}.</p>
    {/each}
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
              {#each t.categories as [c, n]}<tr><td class:dim={!c}><button class="link" class:on={filter === (c ? `#${c}` : "#-")} onclick={() => toggle(c ? `#${c}` : "#-")}>{c ? `#${c}` : "uncategorised"}</button></td><td class="num">{money(n, t.currency)}</td></tr>{/each}
            </tbody>
          </table>
          {#each budgets.filter((b) => b.currency === t.currency) as b (b.category)}
            {@const bar = budgetBar(b.used, b.budget)}
            <div class="budget" class:over={bar.over}>
              <span>#{b.category}</span><span class="num">{money(b.used, t.currency, false)} / {money(b.budget, t.currency, false)}</span>
              <progress max="100" value={bar.pct}></progress>
            </div>
          {/each}
          {#if t.payees.length}
            <table class="payees">
              <thead><tr><th>Payee</th><th></th></tr></thead>
              <tbody>
                {#each t.payees as [p, n]}<tr><td><button class="link" class:on={filter === `@${p}`} onclick={() => toggle(`@${p}`)}>@{p}</button></td><td class="num">{money(n, t.currency)}</td></tr>{/each}
              </tbody>
            </table>
          {/if}
        </section>
      {/each}
    </div>
    <h2>Entries</h2>
    <div class="filter">
      <input type="search" placeholder="Filter: text, #category, @payee" bind:value={filter} onkeydown={(e) => { if (e.key === "Escape" && filter) { e.stopPropagation(); filter = ""; } }} />
      {#if filter.trim()}<span class="dim">{shown.length} {shown.length === 1 ? "entry" : "entries"}{#each sumByCurrency(shown) as [c, n]}, {money(n, c)}{/each}</span>{/if}
    </div>
    <div class="card list"><table>
      <tbody>
        {#each shown as r (r.line)}
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
  .budget { display: grid; grid-template-columns: 1fr auto; gap: 2px var(--s2); margin-top: var(--s3); font-size: var(--fs-base); }
  .budget progress { grid-column: 1 / -1; width: 100%; height: 6px; accent-color: currentColor; }
  .budget.over { color: var(--todo); }
  .entry:hover td { background: var(--active); }
  .entry .num { width: 1%; }
  button.link { background: none; border: 0; padding: 0; text-align: left; overflow-wrap: anywhere; width: 100%; }
  .filter { display: flex; gap: var(--s3); align-items: center; margin-bottom: var(--s3); }
  .filter input { background: var(--card); border: 1px solid var(--border); color: var(--fg); border-radius: 8px; padding: 3px var(--s3); font: var(--fs-md) var(--sans); width: 18em; }
  button.on { font-weight: 600; text-decoration: underline; }
  .dim { color: var(--dim); font-size: var(--fs-md); }
  .year { margin-bottom: var(--s4); }
  .year .scroll { overflow-x: auto; }
  .year td.num, .year th.num { padding-left: var(--s3); }
  .year .sum td { font-weight: 500; }
  .year tr:nth-child(3) td { border-bottom-width: 2px; }
  .heat { background: color-mix(in srgb, var(--accent) calc(var(--a) * 30%), transparent); }
  kbd { font: var(--fs-xs) var(--mono); color: var(--dim); background: var(--card); border: 1px solid var(--border); border-radius: var(--radius-sm); padding: 0 var(--s1); }
</style>
