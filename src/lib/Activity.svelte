<script lang="ts">
  // Activity calendar (#916): clocked hours per day over a year, GitHub-style; read-only, Enter opens a day's timeline.
  import { tick } from "svelte";
  import { addDays, hours, levels, streaks, weeks } from "./activity";

  const DAYS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"], MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
  let active = $state(false);
  let year = $state(0);
  let today = $state("");
  let days = $state<Record<string, number>>({});
  let sel = $state("");
  let box = $state<HTMLElement>();
  let prevFocus: HTMLElement | null = null;
  let load: (year: number) => Promise<Record<string, number>> = async () => ({});
  let onDay: (date: string) => unknown = () => {};
  const cols = $derived(weeks(year));
  const inYear = $derived(cols.flat().filter((d): d is string => !!d));
  const shade = $derived.by(() => { const l = levels(inYear.map((d) => days[d] ?? 0)); return Object.fromEntries(inYear.map((d, i) => [d, l[i]])); });
  const total = $derived(inYear.reduce((s, d) => s + (days[d] ?? 0), 0));
  const nActive = $derived(inYear.filter((d) => (days[d] ?? 0) > 0).length);
  const st = $derived(streaks(days, year, today));
  const longDay = (d: string) => `${DAYS[(new Date(`${d}T12:00`).getDay() + 6) % 7]} ${d}`;

  export const isOpen = () => active;

  /** Show the calendar; LOAD fetches a year's seconds per date, ON_DAY opens a date's timeline. */
  export async function open(l: typeof load, o: typeof onDay, now: string) {
    prevFocus = document.activeElement as HTMLElement | null;
    [load, onDay, today] = [l, o, now];
    await show(+now.slice(0, 4), now);
    active = true;
    await tick();
    box?.focus();
  }

  async function show(y: number, at: string) {
    days = await load(y);
    [year, sel] = [y, at];
  }

  function close() {
    active = false;
    prevFocus?.focus();
  }

  async function key(e: KeyboardEvent) {
    e.stopPropagation();
    const mv: Record<string, number> = { h: -7, ArrowLeft: -7, l: 7, ArrowRight: 7, k: -1, ArrowUp: -1, j: 1, ArrowDown: 1 };
    if (e.key === "Escape") close();
    else if (mv[e.key]) { const d = addDays(sel, mv[e.key]); if (d.startsWith(`${year}-`)) sel = d; }
    else if (e.key === "[" || e.key === "]") { const y = year + (e.key === "[" ? -1 : 1); await show(y, `${y}${sel.slice(4, 10) === "-02-29" ? "-02-28" : sel.slice(4)}`); }
    else if (e.key === "Enter") { close(); onDay(sel); }
    else return;
    e.preventDefault();
  }
</script>

{#if active}
  <div class="backdrop" onmousedown={close} role="presentation"></div>
  <div class="dialog activity" bind:this={box} onkeydown={key} role="dialog" aria-label="Activity calendar" tabindex="-1">
    <header><h2>Activity {year}</h2><span class="dim">clocked hours</span></header>
    <p class="summary">{hours(total)} in total · {nActive} active day{nActive === 1 ? "" : "s"} · longest streak {st.longest} d{year === +today.slice(0, 4) ? ` · current streak ${st.current} d` : ""}</p>
    <div class="grid" role="grid" aria-label="Clocked hours per day">
      <span></span>
      {#each cols as c}{@const first = c.find((d) => d?.endsWith("-01"))}<span class="mm">{first ? MONTHS[+first.slice(5, 7) - 1] : ""}</span>{/each}
      {#each DAYS as day, r}
        <span class="dd">{r % 2 ? "" : day}</span>
        {#each cols as c}
          {@const d = c[r]}
          {#if d}
            <button type="button" class="cell s{shade[d] ?? 0}" class:future={d > today} class:sel={d === sel} title="{d}: {hours(days[d] ?? 0)}" aria-label="{d}: {hours(days[d] ?? 0)}" onclick={() => (sel = d)} ondblclick={() => (close(), onDay(d))}></button>
          {:else}<i></i>{/if}
        {/each}
      {/each}
    </div>
    <div class="legend" aria-hidden="true">less {#each [0, 1, 2, 3, 4] as s}<i class="cell s{s}"></i>{/each} more</div>
    <h3>{longDay(sel)} · {sel > today ? "in the future" : (days[sel] ?? 0) > 0 ? `${hours(days[sel])} clocked` : "nothing clocked"}</h3>
    <footer>h/l week · j/k day · [ / ] year · Enter opens the day's timeline · Esc closes</footer>
  </div>
{/if}

<style>
  .backdrop { position: fixed; inset: 0; z-index: 499; background: var(--scrim); }
  .dialog {
    position: fixed; top: 8%; left: 50%; transform: translateX(-50%); z-index: 500; max-height: 84vh; display: flex; flex-direction: column;
    width: min(920px, calc(100vw - 32px)); background: var(--card); border: 1px solid var(--border);
    border-radius: var(--r); box-shadow: var(--pshadow); padding: 18px 20px; outline: none; box-sizing: border-box;
  }
  header { display: flex; gap: var(--s2); align-items: baseline; }
  h2 { margin: 0; flex: 1; font-size: var(--fs-md); color: var(--dim); font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; }
  .summary { margin: var(--s3) 0; }
  .grid { display: grid; grid-template-rows: auto repeat(7, auto); grid-auto-flow: column; grid-template-columns: 2.5em; grid-auto-columns: 1fr; gap: 2px; font: var(--fs-xs) var(--mono); color: var(--dim); overflow-x: auto; }
  .mm { white-space: nowrap; overflow: visible; width: 0; }
  .dd { align-self: center; }
  .cell { aspect-ratio: 1; min-width: 8px; padding: 0; border: 1px solid transparent; border-radius: 2px; cursor: pointer; }
  /* Same single-hue ramp as Rhythm.svelte, so it reads in light and dark themes. */
  .s0 { background: color-mix(in srgb, var(--fg) 6%, var(--card)); }
  .s1 { background: color-mix(in srgb, var(--accent) 25%, var(--card)); }
  .s2 { background: color-mix(in srgb, var(--accent) 50%, var(--card)); }
  .s3 { background: color-mix(in srgb, var(--accent) 75%, var(--card)); }
  .s4 { background: var(--accent); }
  .cell.future { background: transparent; border-color: color-mix(in srgb, var(--fg) 6%, var(--card)); }
  .cell.sel { border-color: var(--fg); }
  .legend { display: flex; gap: 3px; align-items: center; margin-top: var(--s2); color: var(--dim); font-size: var(--fs-xs); }
  .legend i { display: inline-block; width: 12px; height: 12px; border-radius: 2px; }
  h3 { margin: var(--s3) 0 var(--s1); font-size: var(--fs-sm); }
  .dim, footer { color: var(--dim); font-size: var(--fs-xs); }
  footer { margin-top: var(--s3); }
</style>
