<script lang="ts" module>
  export type Item = { label: string; value: unknown; detail?: string };
  export type PickOpts = {
    prompt: string;
    items?: (string | Item)[];
    /** Live results computed from the query (replaces fuzzy filtering of `items`). */
    source?: (query: string) => Promise<Item[]>;
    /** One-line feedback for the typed text, e.g. a parsed date. */
    preview?: (query: string) => Promise<string>;
    hint?: string;
    /** Text shown in full above the list, e.g. what is about to be sent somewhere. */
    body?: string;
    initial?: string;
    allowCustom?: boolean;
  };

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
  let o = $state.raw<PickOpts>({ prompt: "" });
  let query = $state("");
  let items = $state.raw<Item[]>([]);
  let previewText = $state("");
  let sel = $state(0);
  let input = $state<HTMLInputElement>();
  let list = $state<HTMLUListElement>();
  let resolve: ((v: unknown) => void) | null = null;
  let prevFocus: HTMLElement | null = null;

  const toItems = (xs: (string | Item)[]) => xs.map((i) => (typeof i === "string" ? { label: i, value: i } : i));
  const hasList = $derived(!!o.items?.length || !!o.source);
  const filtered = $derived.by(() => {
    if (o.source || !query.trim()) return items.slice(0, 300);
    return items
      .map((it) => [score(it.label + " " + (it.detail ?? ""), query), it] as const)
      .filter(([s]) => s >= 0)
      .sort((a, b) => b[0] - a[0])
      .slice(0, 300)
      .map(([, it]) => it);
  });

  export const isOpen = () => active;

  /** Resolves to the chosen item's value, the typed text (custom/plain prompt), or null on cancel. */
  export async function open(opts: PickOpts): Promise<any> {
    resolve?.(null);
    if (!active) prevFocus = document.activeElement as HTMLElement | null;
    o = opts;
    items = toItems(opts.items ?? []);
    query = opts.initial ?? "";
    previewText = "";
    sel = 0;
    active = true;
    await tick();
    input?.focus();
    input?.select();
    return new Promise((r) => (resolve = r));
  }

  // Live source / preview, debounced.
  $effect(() => {
    const q = query;
    if (!active || (!o.source && !o.preview)) return;
    const t = setTimeout(async () => {
      if (o.source) {
        items = await o.source(q).catch(() => []);
        sel = 0;
      }
      if (o.preview) previewText = q.trim() ? await o.preview(q).catch(() => "") : "";
    }, 120);
    return () => clearTimeout(t);
  });

  function done(v: unknown) {
    if (!active) return;
    active = false;
    prevFocus?.focus();
    const r = resolve;
    resolve = null;
    r?.(v);
  }

  function key(e: KeyboardEvent) {
    const c = e.ctrlKey || e.metaKey;
    if (e.key === "Escape") done(null);
    else if (e.key === "Enter") {
      const custom = o.allowCustom ?? !hasList;
      if (!hasList || ((c || e.shiftKey) && custom)) done(query);
      else if (filtered.length) done(filtered[sel].value);
      else if (custom) done(query);
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
  <div class="backdrop" onmousedown={() => done(null)} role="presentation"></div>
  <div class="picker">
    <label>
      <span>{o.prompt}</span>
      <input bind:this={input} bind:value={query} oninput={() => (sel = 0)} onkeydown={key} onblur={() => document.hasFocus() && done(null)} spellcheck="false" />
    </label>
    {#if o.body}<pre class="body">{o.body}</pre>{/if}
    {#if previewText}<div class="preview" class:bad={previewText.startsWith("✗")}>{previewText}</div>{/if}
    {#if hasList}
      <ul bind:this={list} role="listbox">
        {#each filtered as it, i}
          <li role="option" aria-selected={i === sel} class:sel={i === sel} onmousedown={(e) => { e.preventDefault(); done(it.value); }}>
            <span>{it.label}</span>{#if it.detail}<small>{it.detail}</small>{/if}
          </li>
        {/each}
        {#if !filtered.length}
          <li class="empty">{o.source && !query ? "Start typing…" : (o.allowCustom ? `↵ use “${query}”` : "No matches")}</li>
        {/if}
      </ul>
    {/if}
    <footer>{o.hint ?? ""}{o.hint ? " · " : ""}↵ choose{o.allowCustom && hasList ? " · Ctrl+↵ use typed text" : ""} · ↑↓ move · Esc cancel</footer>
  </div>
{/if}

<style>
  .backdrop { position: fixed; inset: 0; z-index: 499; background: rgb(0 0 0 / 0.25); }
  .picker {
    position: fixed; top: 12%; left: 50%; transform: translateX(-50%);
    width: min(680px, calc(100vw - 32px)); background: var(--panel); border: 1px solid var(--border);
    border-radius: 10px; box-shadow: 0 16px 48px rgb(0 0 0 / 0.4); z-index: 500; overflow: hidden;
  }
  label { display: flex; flex-direction: column; gap: 6px; padding: 12px 14px; border-bottom: 1px solid var(--border); }
  label span { color: var(--dim); font-size: 12px; }
  input { background: none; border: 0; outline: 0; color: var(--fg); font: 16px var(--sans); }
  .body { margin: 0; padding: 8px 14px; max-height: 40vh; overflow-y: auto; white-space: pre-wrap; font: 12px var(--mono); color: var(--dim); border-bottom: 1px solid var(--border); }
  .preview { padding: 6px 14px; color: var(--done); font-size: 13px; border-bottom: 1px solid var(--border); }
  .preview.bad { color: var(--todo); }
  ul { list-style: none; margin: 0; padding: 4px; max-height: 50vh; overflow-y: auto; }
  li { padding: 6px 10px; display: flex; justify-content: space-between; gap: 12px; cursor: pointer; border-radius: 6px; font-size: 14px; }
  li span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  li.sel { background: var(--sel); }
  li small { color: var(--dim); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; flex-shrink: 1; max-width: 50%; }
  li.empty { color: var(--dim); cursor: default; }
  footer { padding: 6px 14px; font-size: 11px; color: var(--dim); border-top: 1px solid var(--border); }
</style>
