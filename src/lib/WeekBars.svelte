<script lang="ts">
  import { hm } from "./Time.svelte";
  import type { WeekDay } from "./clock";

  /** Hours worked per day of WEEK (from `tc_dashboard`), with expected hours and time off; HEIGHT of the plot in px. */
  let { week, height = 150 }: { week: WeekDay[]; height?: number } = $props();
  const max = $derived(Math.max(1, ...week.map((w) => Math.max(w.hours, w.expected, w.leave_hours))));
  const todayIso = $derived(new Date().toLocaleDateString("sv-SE"));
  const dayName = (s: string) => new Date(+s.slice(0, 4), +s.slice(5, 7) - 1, +s.slice(8, 10)).toLocaleDateString(undefined, { weekday: "short" });
</script>

<div class="chart" style:height="{height}px" role="img" aria-label="Hours worked per day this week">
  {#each week as w}
    <div class="col" class:today={w.date === todayIso} title="{dayName(w.date)} {w.date}: {hm(w.hours)} worked, {hm(w.expected)} expected{w.leave ? `, ${w.leave} ${hm(w.leave_hours)}` : ""}">
      <div class="plot">
        {#if w.leave_hours > 0}<div class="off" style:height="{(w.leave_hours / max) * 100}%"></div>{/if}
        {#if w.expected > 0}<div class="expected" style:bottom="{(w.expected / max) * 100}%"></div>{/if}
        <div class="bar" style:height="{(w.hours / max) * 100}%"></div>
      </div>
      <span class="val">{w.hours ? hm(w.hours) : "–"}</span>
      <span class="day">{dayName(w.date)}</span>
    </div>
  {/each}
</div>
<p class="legend"><span class="dash"></span> expected hours{#if week.some((w) => w.leave_hours > 0)} <span class="hatch"></span> time off{/if}</p>

<style>
  .chart { display: grid; grid-template-columns: repeat(7, 1fr); gap: var(--s2); }
  .col { display: flex; flex-direction: column; align-items: center; gap: var(--s1); min-width: 0; }
  .plot { flex: 1; width: 100%; position: relative; border-bottom: 1px solid var(--border); }
  .bar { position: absolute; bottom: 0; left: 50%; transform: translateX(-50%); width: min(26px, 70%); background: var(--link); border-radius: var(--radius-sm) 4px 0 0; min-height: 0; }
  .col.today .bar { background: var(--accent); }
  .col:hover .bar { filter: brightness(1.15); }
  .off { position: absolute; bottom: 0; left: 50%; transform: translateX(-50%); width: min(26px, 70%); background: repeating-linear-gradient(135deg, var(--border) 0 3px, transparent 3px 6px); border: 1px solid var(--border); border-bottom: 0; box-sizing: border-box; }
  .hatch { width: 12px; height: 10px; margin-left: var(--s3); background: repeating-linear-gradient(135deg, var(--dim) 0 2px, transparent 2px 4px); }
  .expected { position: absolute; left: 8%; right: 8%; border-top: 1.5px dashed var(--dim); }
  .val { font: var(--fs-xs) var(--mono); color: var(--fg); text-align: center; line-height: 1.15; }
  .day { font-size: var(--fs-xs); color: var(--dim); }
  .col.today .day { color: var(--accent); font-weight: 700; }
  .legend { color: var(--dim); font-size: var(--fs-xs); margin: 6px 0 0; display: flex; align-items: center; gap: 6px; }
  .dash { width: 16px; border-top: 1.5px dashed var(--dim); }
</style>
