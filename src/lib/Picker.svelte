<script lang="ts" module>
  export type Item = { label: string; value: unknown; detail?: string };
  export type PickOpts = { prompt: string; items?: (string | Item)[]; initial?: string; allowCustom?: boolean };

  /** Orderless fuzzy match: every space-separated word must match as a subsequence. */
  function score(label: string, query: string): number {
    const l = label.toLowerCase();
    let total = 0;
    for (const w of query.toLowerCase().split(/\s+/).filter(Boolean)) {
      if (l.includes(w)) { total += 10 + (l.startsWith(w) ? 5 : 0); continue; }
      let qi = 0, last = -2, s = 0;
      for (let i = 0; i < l.length && qi < w.length; i++) {
        if (l[i] === w[qi]) { s += last === i - 1 ? 2 : 1; last = i; qi++; }
      }
      if (qi < w.length) return -1;
      total += s;
    }
    return total - l.length * 0.01;
  }
</script>

<script lang="ts">
  import { tick } from "svelte";

  let active = $state(false);
  let prompt = $state("");
  let query = $state("");
  let items = $state.raw<Item[]>([]);
  let allowCustom = $state(false);
  let sel = $state(0);
  let input = $state<HTMLInputElement>();
  let list = $state<HTMLUListElement>();
  let resolve: ((v: unknown) => void) | null = null;
  let prevFocus: HTMLElement | null = null;

  const filtered = $derived.by(() => {
    if (!query.trim()) return items.slice(0, 300);
    return items
      .map((it) => [score(it.label + " " + (it.detail ?? ""), query), it] as const)
      .filter(([s]) => s >= 0)
      .sort((a, b) => b[0] - a[0])
      .slice(0, 300)
      .map(([, it]) => it);
  });

  export const isOpen = () => active;

  /** Resolves to the chosen item's value, the typed text (custom/plain prompt), or null on cancel. */
  export async function open(o: PickOpts): Promise<any> {
    resolve?.(null);
    if (!active) prevFocus = document.activeElement as HTMLElement | null;
    prompt = o.prompt;
    items = (o.items ?? []).map((i) => (typeof i === "string" ? { label: i, value: i } : i));
    allowCustom = o.allowCustom ?? !o.items;
    query = o.initial ?? "";
    sel = 0;
    active = true;
    await tick();
    input?.focus();
    input?.select();
    return new Promise((r) => (resolve = r));
  }

  function done(v: unknown) {
    if (!active) return;
    active = false;
    prevFocus?.focus();
    const r = resolve;
    resolve = null;
    r?.(v);
  }

  function key(e: KeyboardEvent) {
    const c = e.ctrlKey;
    if (e.key === "Escape" || (c && e.key === "g")) done(null);
    else if (e.key === "Enter") {
      if (!items.length || ((c || e.shiftKey) && allowCustom)) done(query);
      else if (filtered.length) done(filtered[sel].value);
      else if (allowCustom) done(query);
    } else if (e.key === "ArrowDown" || (c && (e.key === "n" || e.key === "j"))) sel = Math.min(sel + 1, filtered.length - 1);
    else if (e.key === "ArrowUp" || (c && (e.key === "p" || e.key === "k"))) sel = Math.max(sel - 1, 0);
    else if (e.key === "Tab" && filtered.length) query = filtered[sel].label;
    else return;
    e.preventDefault();
    e.stopPropagation();
  }

  $effect(() => {
    list?.children[sel]?.scrollIntoView({ block: "nearest" });
  });
</script>

{#if active}
  <div class="picker">
    <label>
      <span>{prompt}</span>
      <input bind:this={input} bind:value={query} oninput={() => (sel = 0)} onkeydown={key} onblur={() => done(null)} spellcheck="false" />
    </label>
    {#if items.length}
      <ul bind:this={list} role="listbox">
        {#each filtered as it, i}
          <li role="option" aria-selected={i === sel} class:sel={i === sel} onmousedown={(e) => { e.preventDefault(); done(it.value); }}>
            <span>{it.label}</span>{#if it.detail}<small>{it.detail}</small>{/if}
          </li>
        {/each}
        {#if !filtered.length}<li class="empty">{allowCustom ? "↵ use typed text" : "no match"}</li>{/if}
      </ul>
    {/if}
    <footer>↵ select · C-↵ use typed text · C-n/C-p move · Tab complete · Esc cancel</footer>
  </div>
{/if}

<style>
  .picker {
    position: fixed; top: 12%; left: 50%; transform: translateX(-50%);
    width: min(720px, calc(100vw - 32px)); background: var(--panel); border: 1px solid var(--border);
    border-radius: 8px; box-shadow: 0 12px 40px rgb(0 0 0 / 0.35); z-index: 20; overflow: hidden;
  }
  label { display: flex; gap: 8px; align-items: center; padding: 10px 12px; border-bottom: 1px solid var(--border); }
  label span { color: var(--accent); white-space: nowrap; font-size: 13px; }
  input { flex: 1; background: none; border: 0; outline: 0; color: var(--fg); font: 15px var(--mono); }
  ul { list-style: none; margin: 0; padding: 4px 0; max-height: 50vh; overflow-y: auto; }
  li { padding: 4px 12px; display: flex; justify-content: space-between; gap: 12px; cursor: pointer; font: 14px var(--mono); }
  li.sel { background: var(--sel); }
  li small { color: var(--dim); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  li.empty { color: var(--dim); cursor: default; }
  footer { padding: 4px 12px; font-size: 11px; color: var(--dim); border-top: 1px solid var(--border); }
</style>
