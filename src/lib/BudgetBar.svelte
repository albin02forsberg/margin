<script lang="ts">
  import { budgetUsage, type Project } from "$lib/projects";

  /** A project card's budget line, "12.5 / 20 h this month", with a bar that turns the warning colour past 100%. Nothing
   *  without a budget. */
  let { project, worked, month }: { project: Pick<Project, "budget_hours" | "budget_period"> | undefined; worked: number; month: number } = $props();
  const u = $derived(budgetUsage(project, { worked, month }));
  const n = (h: number) => +h.toFixed(2);
</script>

{#if u}
  <div class="budget" class:over={u.over}>
    <small>{n(u.used)} / {n(u.total)} h this {u.period}{u.over ? ` · ${n(u.used - u.total)} h over` : ` · ${n(u.total - u.used)} h left`}</small>
    <div class="bar" role="progressbar" aria-label="Budget used" aria-valuenow={Math.round(u.pct)}><i style:width="{Math.min(u.pct, 100)}%"></i></div>
  </div>
{/if}

<style>
  .budget { margin: var(--s2) 0; }
  small { color: var(--dim); font-size: var(--fs-sm); }
  .over small { color: var(--todo); }
  .bar { height: 4px; border-radius: 999px; background: var(--active); overflow: hidden; margin-top: var(--s1); }
  .bar i { display: block; height: 100%; background: var(--done); border-radius: inherit; }
  .over .bar i { background: var(--todo); }
</style>
