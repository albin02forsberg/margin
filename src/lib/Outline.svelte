<script lang="ts">
  // Outline panel (#915): the note's headings; ACTIVE is the line of the heading holding the cursor.
  import { visibleOutline, type OutlineItem } from "$lib/outline";

  let { items, active, jump }: { items: OutlineItem[]; active: number; jump: (line: number) => void } = $props();
  let filter = $state("");
  let hideDone = $state(false);
  let list = $state<HTMLElement>();
  const shown = $derived(visibleOutline(items, filter, hideDone));

  $effect(() => { void active; void shown; list?.querySelector(".cur")?.scrollIntoView({ block: "nearest" }); });
</script>

<div class="bar">
  <input type="search" placeholder="Filter headings" bind:value={filter} />
  <label title="Hide done headings and their subtrees"><input type="checkbox" bind:checked={hideDone} /> Hide done</label>
</div>
<div class="list" bind:this={list}>
  {#each shown as it (it.line)}
    <button class:cur={it.line === active} class:done={it.done} style:padding-left="{6 + (it.level - 1) * 12}px" title="Line {it.line + 1}" onclick={() => jump(it.line)}>
      {#if it.keyword}<em class="kw">{it.keyword}</em>{/if}
      <span class="t">{it.title || "(untitled)"}</span>
      {#if it.cookie}<small>{it.cookie}</small>{/if}
      {#each it.tags as tag}<small class="tag">:{tag}:</small>{/each}
    </button>
  {:else}
    <p>{items.length ? "No matching headings." : "This note has no headings."}</p>
  {/each}
</div>

<style>
  .bar { display: flex; gap: var(--s2); align-items: center; margin: 0 6px var(--s2); position: sticky; top: -10px; padding: 4px 0; background: var(--card); z-index: 1; }
  .bar input[type="search"] { flex: 1; min-width: 0; background: var(--bg); color: var(--fg); border: 1px solid var(--border, transparent); border-radius: var(--radius); padding: 4px 6px; font: inherit; font-size: var(--fs-sm); }
  .bar label { color: var(--dim); font-size: var(--fs-xs); white-space: nowrap; }
  button { display: flex; gap: 6px; align-items: baseline; width: 100%; text-align: left; background: none; border: 0; color: var(--fg); padding: 4px 6px; border-radius: var(--radius); cursor: pointer; font: inherit; }
  button:hover { background: var(--active); }
  button.cur { background: var(--active); box-shadow: inset 2px 0 0 var(--accent); }
  button.done .t { color: var(--dim); text-decoration: line-through; }
  .t { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .kw { font: 700 var(--fs-xs) var(--mono); font-style: normal; color: var(--todo); background: color-mix(in srgb, var(--todo) 18%, transparent); padding: 0 4px; border-radius: var(--radius-sm); }
  .done .kw { color: var(--done); background: color-mix(in srgb, var(--done) 18%, transparent); }
  small { color: var(--dim); font: var(--fs-xs) var(--mono); }
  p { color: var(--dim); margin: 6px; font-size: var(--fs-sm); }
</style>
