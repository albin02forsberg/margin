<script lang="ts">
  // When do I work (#738): average clocked minutes per weekday x hour of day over a range; read-only.
  import { tick } from "svelte";
  import { DAYS, hourSpan, mins, shade, summary, type Rhythm } from "./rhythm";

  const ranges = [["4w", "4 weeks"], ["12w", "12 weeks"], ["month", "This month"]], HOURS = [...Array(24).keys()];
  const empty = (): Rhythm => ({ minutes: DAYS.map(() => HOURS.map(() => 0)), peak: null, evening: 0, hours: 0, pieces: [] });
  let active = $state(false);
  let range = $state("4w");
  let project = $state("");
  let projects = $state<string[]>([]);
  let r = $state<Rhythm>(empty());
  let sel = $state<[number, number]>([0, 9]);
  let box = $state<HTMLElement>();
  let prevFocus: HTMLElement | null = null;
  let load: (range: string, project: string | null) => Promise<Rhythm> = async () => empty();
  const max = $derived(Math.max(0, ...r.minutes.flat()));
  const behind = $derived(r.pieces.filter((p) => p.day === sel[0] && p.hour === sel[1]));
  const hm = (t: string) => t.slice(11, 16);

  export const isOpen = () => active;

  /** Show the heatmap; LOAD fetches it for a range and project, PROJECTS fill the filter. */
  export async function open(l: typeof load, ps: string[]) {
    prevFocus = document.activeElement as HTMLElement | null;
    [load, projects] = [l, ps];
    if (!ps.includes(project)) project = "";
    await reload();
    active = true;
    await tick();
    box?.focus();
  }

  async function reload() {
    r = await load(range, project || null);
    if (r.peak) sel = r.peak;
  }

  function close() {
    active = false;
    prevFocus?.focus();
  }

  async function key(e: KeyboardEvent) {
    if ((e.target as HTMLElement).closest("select")) return;
    e.stopPropagation();
    const [d, h] = sel, mv: Record<string, [number, number]> = { h: [0, -1], ArrowLeft: [0, -1], l: [0, 1], ArrowRight: [0, 1], k: [-1, 0], ArrowUp: [-1, 0], j: [1, 0], ArrowDown: [1, 0] };
    if (e.key === "Escape") close();
    else if (mv[e.key]) sel = [Math.min(6, Math.max(0, d + mv[e.key][0])), Math.min(23, Math.max(0, h + mv[e.key][1]))];
    else if (e.key === "r") (range = ranges[(ranges.findIndex((x) => x[0] === range) + 1) % ranges.length][0]), await reload();
    else return;
    e.preventDefault();
  }
</script>

{#if active}
  <div class="backdrop" onmousedown={close} role="presentation"></div>
  <div class="dialog rhythm" bind:this={box} onkeydown={key} role="dialog" aria-label="When do I work" tabindex="-1">
    <header>
      <h2>When do I work</h2>
      {#each ranges as [k, label]}<button type="button" class:on={k === range} onclick={() => ((range = k), reload())}>{label}</button>{/each}
      <select bind:value={project} onchange={reload} aria-label="Project" onkeydown={(e) => e.key === "Escape" && box?.focus()}>
        <option value="">All projects</option>
        {#each projects as p}<option value={p}>{p}</option>{/each}
      </select>
    </header>
    <p class="summary">{summary(r)}</p>
    <div class="grid" role="grid" aria-label="Average clocked minutes by weekday and hour">
      <span></span>{#each HOURS as h}<span class="hh">{h % 3 ? "" : h}</span>{/each}
      {#each DAYS as day, d}
        <span class="dd">{day}</span>
        {#each HOURS as h}
          {@const m = r.minutes[d][h]}
          <button type="button" class="cell s{shade(m, max)}" class:sel={sel[0] === d && sel[1] === h} title="{day} {hourSpan(h)}: {mins(m)} avg" aria-label="{day} {hourSpan(h)}: {mins(m)} average" onclick={() => (sel = [d, h])}></button>
        {/each}
      {/each}
    </div>
    <div class="legend" aria-hidden="true">less {#each [0, 1, 2, 3, 4] as s}<i class="cell s{s}"></i>{/each} more · busiest cell {mins(max)} avg</div>
    <h3>{DAYS[sel[0]]} {hourSpan(sel[1])} · {mins(r.minutes[sel[0]][sel[1]])} avg</h3>
    <ul class="pieces">
      {#each behind as p}
        <li><span>{p.start.slice(0, 10)}</span> {hm(p.start)}–{hm(p.end)}{p.end.slice(0, 10) !== p.start.slice(0, 10) ? " (+1)" : ""} · {p.project || "Other"} · <b>{mins(p.minutes)}</b> in this hour</li>
      {:else}
        <li class="dim">Nothing clocked in this hour.</li>
      {/each}
    </ul>
    <footer>h/j/k/l move · r: 4 weeks / 12 weeks / this month · Esc closes</footer>
  </div>
{/if}

<style>
  .backdrop { position: fixed; inset: 0; z-index: 499; background: var(--scrim); }
  .dialog {
    position: fixed; top: 8%; left: 50%; transform: translateX(-50%); z-index: 500; max-height: 84vh; display: flex; flex-direction: column;
    width: min(760px, calc(100vw - 32px)); background: var(--card); border: 1px solid var(--border);
    border-radius: var(--r); box-shadow: var(--pshadow); padding: 18px 20px; outline: none; box-sizing: border-box;
  }
  header { display: flex; gap: var(--s2); align-items: center; }
  h2 { margin: 0; flex: 1; font-size: var(--fs-md); color: var(--dim); font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; }
  header button, select { background: var(--bg); border: 1px solid var(--border); color: var(--dim); border-radius: var(--radius); padding: 3px 10px; font: var(--fs-xs) var(--sans); cursor: pointer; }
  header button.on { color: var(--fg); border-color: var(--accent); }
  .summary { margin: var(--s3) 0; }
  .grid { display: grid; grid-template-columns: 3em repeat(24, 1fr); gap: 2px; font: var(--fs-xs) var(--mono); color: var(--dim); }
  .hh { text-align: left; }
  .dd { align-self: center; }
  .cell { aspect-ratio: 1; min-width: 0; padding: 0; border: 1px solid transparent; border-radius: 2px; cursor: pointer; }
  /* One hue (the accent) mixed into the card colour, so the ramp reads in light and dark themes. */
  .s0 { background: color-mix(in srgb, var(--fg) 6%, var(--card)); }
  .s1 { background: color-mix(in srgb, var(--accent) 25%, var(--card)); }
  .s2 { background: color-mix(in srgb, var(--accent) 50%, var(--card)); }
  .s3 { background: color-mix(in srgb, var(--accent) 75%, var(--card)); }
  .s4 { background: var(--accent); }
  .cell.sel { border-color: var(--fg); }
  .legend { display: flex; gap: 3px; align-items: center; margin-top: var(--s2); color: var(--dim); font-size: var(--fs-xs); }
  .legend i { display: inline-block; width: 12px; height: 12px; border-radius: 2px; }
  h3 { margin: var(--s3) 0 var(--s1); font-size: var(--fs-sm); }
  .pieces { list-style: none; margin: 0; padding: 0; overflow: auto; min-height: 0; font: var(--fs-xs) var(--mono); }
  .pieces li { padding: 2px 0; }
  .pieces span, .dim, footer { color: var(--dim); font-size: var(--fs-xs); }
  footer { margin-top: var(--s3); }
</style>
