<script lang="ts">
  // Journal metrics trend (#721): a sparkline per metric, long days vs the rest, and a day table; read-only.
  import { tick } from "svelte";
  import { hm } from "./clock";
  import { num, sparkline, type Trend } from "./metrics";

  const spans = [30, 90, 365], W = 240, H = 28;
  let active = $state(false);
  let span = $state(30);
  let trend = $state<Trend>({ stats: [], days: [] });
  let sel = $state(0);
  let box = $state<HTMLElement>();
  let prevFocus: HTMLElement | null = null;
  let load: (days: number) => Promise<Trend> = async () => ({ stats: [], days: [] });
  let openDay: (date: string) => void = () => {};
  const rows = $derived([...trend.days].reverse()); // newest first
  const today = () => new Date().toLocaleDateString("sv-SE");

  export const isOpen = () => active;

  /** Show the trend; LOAD fetches DAYS days of it, OPEN_DAY opens a journal day (after closing). */
  export async function open(l: typeof load, o: typeof openDay) {
    prevFocus = document.activeElement as HTMLElement | null;
    [load, openDay, sel] = [l, o, 0];
    trend = await load(span);
    active = true;
    await tick();
    box?.focus();
  }

  function close(refocus = true) {
    active = false;
    if (refocus) prevFocus?.focus();
  }

  async function setSpan(s: number) {
    [span, sel] = [s, 0];
    trend = await load(s);
  }

  async function key(e: KeyboardEvent) {
    e.stopPropagation();
    if (e.key === "Escape") close();
    else if (e.key === "j" || e.key === "ArrowDown") sel = Math.min(sel + 1, rows.length - 1);
    else if (e.key === "k" || e.key === "ArrowUp") sel = Math.max(sel - 1, 0);
    else if (e.key === "r") await setSpan(spans[(spans.indexOf(span) + 1) % spans.length]);
    else if (e.key === "Enter" && rows[sel]) (close(false), openDay(rows[sel].date));
    else return;
    e.preventDefault();
    await tick();
    box?.querySelector("tr.sel")?.scrollIntoView({ block: "nearest" });
  }
</script>

{#if active}
  <div class="backdrop" onmousedown={() => close()} role="presentation"></div>
  <div class="dialog metrics" bind:this={box} onkeydown={key} role="dialog" aria-label="Journal metrics" tabindex="-1">
    <header>
      <h2>Journal metrics</h2>
      {#each spans as s}<button type="button" class:on={s === span} onclick={() => setSpan(s)}>{s} days</button>{/each}
    </header>
    {#if !trend.stats.length}
      <p class="dim">No metrics configured (Settings &gt; Daily metrics).</p>
    {/if}
    {#each trend.stats as m, i}
      {@const pts = sparkline(trend.days, i, today(), span, W, H)}
      <div class="stat">
        <span class="name">{m.name}</span>
        <svg viewBox="-3 -3 {W + 6} {H + 6}" width={W} height={H + 6} role="img" aria-label="{m.name} over {span} days">
          {#if pts.length > 1}<polyline points={pts.map((p) => p.join(",")).join(" ")} />{/if}
          {#if pts.length}<circle cx={pts[pts.length - 1][0]} cy={pts[pts.length - 1][1]} r="2.5" />{/if}
        </svg>
        <span class="nums">{#if m.n}latest <b class="latest">{num(m.latest)}</b> · mean {num(m.mean)} · {num(m.min)}–{num(m.max)} · {m.n} day{m.n === 1 ? "" : "s"}{:else}not logged{/if}</span>
        {#if m.long != null || m.rest != null}<span class="split" title="Mean on workdays with at least the expected hours clocked vs the other workdays (days off left out)">≥ expected hours {num(m.long)} · shorter days {num(m.rest)}</span>{/if}
      </div>
    {/each}
    <div class="table">
      <table>
        <thead><tr><th>Day</th>{#each trend.stats as m}<th>{m.name}</th>{/each}<th>Clocked</th></tr></thead>
        <tbody>
          {#each rows as r, i}
            <tr class:sel={i === sel} onclick={() => { close(false); openDay(r.date); }}>
              <td>{r.date}</td>{#each r.values as v}<td>{num(v)}</td>{/each}<td>{r.hours ? hm(r.hours) : "–"}</td>
            </tr>
          {:else}
            <tr><td class="dim" colspan={trend.stats.length + 2}>No journal entries or clocked time in the last {span} days.</td></tr>
          {/each}
        </tbody>
      </table>
    </div>
    <footer>j/k move · ↵ opens the journal day · r: 30/90/365 days · Esc closes</footer>
  </div>
{/if}

<style>
  .backdrop { position: fixed; inset: 0; z-index: 499; background: var(--scrim); }
  .dialog {
    position: fixed; top: 8%; left: 50%; transform: translateX(-50%); z-index: 500; max-height: 84vh; display: flex; flex-direction: column;
    width: min(720px, calc(100vw - 32px)); background: var(--card); border: 1px solid var(--border);
    border-radius: var(--r); box-shadow: var(--pshadow); padding: 18px 20px; outline: none; box-sizing: border-box;
  }
  header { display: flex; gap: var(--s2); align-items: center; margin-bottom: var(--s3); }
  h2 { margin: 0; flex: 1; font-size: var(--fs-md); color: var(--dim); font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; }
  header button { background: var(--bg); border: 1px solid var(--border); color: var(--dim); border-radius: var(--radius); padding: 3px 10px; font: var(--fs-xs) var(--sans); cursor: pointer; }
  header button.on { color: var(--fg); border-color: var(--accent); }
  .stat { display: grid; grid-template-columns: 7em auto 1fr; align-items: center; gap: 2px var(--s3); padding: 4px 0; }
  .name { font-weight: 600; }
  svg { overflow: visible; }
  polyline { fill: none; stroke: var(--link); stroke-width: 1.5; stroke-linejoin: round; vector-effect: non-scaling-stroke; }
  circle { fill: var(--accent); }
  .nums, .split, .dim, footer { color: var(--dim); font-size: var(--fs-xs); }
  .nums b { color: var(--fg); }
  .split { grid-column: 2 / 4; }
  .table { overflow: auto; margin-top: var(--s3); border-top: 1px solid var(--border); min-height: 0; }
  table { width: 100%; border-collapse: collapse; font: var(--fs-xs) var(--mono); }
  th { text-align: left; color: var(--dim); font-weight: 600; position: sticky; top: 0; background: var(--card); }
  th, td { padding: 3px 8px; }
  tr.sel td { background: var(--sel); }
  tbody tr { cursor: pointer; }
  footer { margin-top: var(--s3); }
</style>
