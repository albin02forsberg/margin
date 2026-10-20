<script lang="ts" module>
  export type TableSpec = { cols: number; rows: number; header: boolean };
</script>

<script lang="ts">
  // "Insert table" form: pick a size on the grid (or type it) and whether it gets a header row.
  import { tick } from "svelte";

  const N = 8;
  let active = $state(false);
  let spec = $state<TableSpec>({ cols: 3, rows: 2, header: true });
  let hover = $state<[number, number] | null>(null);
  let first = $state<HTMLInputElement>();
  let prevFocus: HTMLElement | null = null;
  let closed: (s: TableSpec | null) => void = () => {};

  export const isOpen = () => active;

  /** Resolves with the chosen size, or null when cancelled. */
  export async function open(): Promise<TableSpec | null> {
    prevFocus = document.activeElement as HTMLElement | null;
    hover = null;
    active = true;
    await tick();
    first?.select();
    return new Promise((r) => (closed = r));
  }

  function close(s: TableSpec | null) {
    active = false;
    prevFocus?.focus();
    closed(s && { cols: Math.max(1, Math.min(99, Math.round(s.cols) || 1)), rows: Math.max(1, Math.min(999, Math.round(s.rows) || 1)), header: s.header });
  }

  function key(e: KeyboardEvent) {
    e.stopPropagation();
    if (e.key === "Escape") close(null);
    else if (e.key === "Enter" && !(e.target instanceof HTMLButtonElement)) close(spec);
    else return;
    e.preventDefault();
  }

  const [hc, hr] = $derived(hover ?? [spec.cols, spec.rows]);
</script>

{#if active}
  <div class="backdrop" onmousedown={() => close(null)} role="presentation"></div>
  <div class="dialog" onkeydown={key} role="dialog" aria-label="Insert table" tabindex="-1">
    <h2>Insert table</h2>
    <div class="grid" role="grid" tabindex="-1" onmouseleave={() => (hover = null)}>
      {#each { length: N } as _, r}
        <div class="row" role="row">
          {#each { length: N } as _, c}
            <button type="button" role="gridcell" aria-label="{c + 1} × {r + 1}" class:on={c < hc && r < hr} class:head={spec.header && r === 0}
              onmouseenter={() => (hover = [c + 1, r + 1])} onclick={() => close({ ...spec, cols: c + 1, rows: r + 1 })}></button>
          {/each}
        </div>
      {/each}
    </div>
    <div class="fields">
      <label>Columns <input type="number" min="1" max="99" bind:this={first} bind:value={spec.cols} /></label>
      <label>Rows <input type="number" min="1" max="999" bind:value={spec.rows} /></label>
      <label class="check"><input type="checkbox" bind:checked={spec.header} /> Header row</label>
    </div>
    <footer>
      <span>{hc} × {hr}{spec.header ? " + header" : ""} · ↵ insert · Esc cancel · then Tab moves between cells</span>
      <button type="button" onclick={() => close(null)}>Cancel</button>
      <button type="button" class="primary" onclick={() => close(spec)}>Insert</button>
    </footer>
  </div>
{/if}

<style>
  .backdrop { position: fixed; inset: 0; z-index: 499; background: var(--scrim); }
  .dialog {
    position: fixed; top: 10%; left: 50%; transform: translateX(-50%); z-index: 500;
    width: min(420px, calc(100vw - 32px)); background: var(--card); border: 1px solid var(--border);
    border-radius: var(--r); box-shadow: var(--pshadow); padding: 18px 20px; outline: none; box-sizing: border-box;
  }
  h2 { margin: 0 0 var(--s3); font-size: var(--fs-md); color: var(--dim); font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; }
  .grid { display: flex; flex-direction: column; gap: 3px; outline: none; }
  .row { display: flex; gap: 3px; }
  .grid button { width: 22px; height: 22px; padding: 0; background: var(--bg); border: 1px solid var(--border); border-radius: var(--radius-sm); cursor: pointer; }
  .grid button.head { border-bottom-width: 2px; }
  .grid button.on { background: color-mix(in srgb, var(--accent) 30%, var(--bg)); border-color: var(--accent); }
  .fields { display: flex; flex-wrap: wrap; gap: var(--s3); align-items: center; margin-top: 14px; }
  label { color: var(--dim); font-size: var(--fs-md); display: flex; gap: 6px; align-items: center; }
  input[type="number"] { width: 64px; background: var(--bg); border: 1px solid var(--border); border-radius: var(--radius); color: var(--fg); font: var(--fs-base) var(--sans); padding: 5px 7px; box-sizing: border-box; }
  input:focus { outline: 2px solid var(--accent); outline-offset: -1px; }
  footer { display: flex; gap: var(--s2); align-items: center; margin-top: 18px; }
  footer span { flex: 1; color: var(--dim); font-size: var(--fs-xs); }
  footer button { background: var(--bg); border: 1px solid var(--border); color: var(--fg); border-radius: var(--radius); padding: 7px 14px; font: var(--fs-md) var(--sans); cursor: pointer; }
  footer .primary { background: var(--accent); border-color: var(--accent); color: var(--bg); font-weight: 600; }
</style>
