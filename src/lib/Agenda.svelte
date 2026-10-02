<script lang="ts" module>
  export type AgendaItem = {
    date: string; kind: string; label: string; time: string | null; path: string; line: number;
    keyword: string | null; priority: string | null; title: string; tags: string[]; category: string;
  };
  export type AgendaApi = {
    open: (path: string, line: number) => void;
    cycle: (it: AgendaItem, dir: number) => Promise<void>;
    plan: (it: AgendaItem, kind: "SCHEDULED" | "DEADLINE") => Promise<void>;
    clockIn: (it: AgendaItem) => void;
    clockOut: () => void;
    pick: (o: { prompt: string; allowCustom?: boolean }) => Promise<string | null>;
    close: () => void;
    act: (f: () => Promise<void>) => void;
  };

  export const iso = (d: Date) =>
    `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
  const monday = (d: Date) => new Date(d.getFullYear(), d.getMonth(), d.getDate() - ((d.getDay() + 6) % 7));
  const addDays = (d: Date, n: number) => new Date(d.getFullYear(), d.getMonth(), d.getDate() + n);
  // ISO week number.
  const week = (d: Date) => {
    const t = new Date(Date.UTC(d.getFullYear(), d.getMonth(), d.getDate()));
    t.setUTCDate(t.getUTCDate() + 4 - (t.getUTCDay() || 7));
    return Math.ceil(((t.getTime() - Date.UTC(t.getUTCFullYear(), 0, 1)) / 86400000 + 1) / 7);
  };
</script>

<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { tick } from "svelte";

  let { mode, api, active, reload }: { mode: "agenda" | "todo"; api: AgendaApi; active: boolean; reload: number } = $props();

  let start = $state(monday(new Date()));
  let items = $state.raw<AgendaItem[]>([]);
  let filter = $state("");
  let sel = $state(0);
  let root = $state<HTMLDivElement>();

  const today = $derived(iso(new Date()));
  const shown = $derived(
    filter ? items.filter((i) => `${i.keyword} ${i.title} :${i.tags.join(":")}: ${i.category}`.toLowerCase().includes(filter.toLowerCase())) : items,
  );
  const days = $derived(Array.from({ length: 7 }, (_, i) => addDays(start, i)));

  async function refresh() {
    items = mode === "agenda" ? await invoke("agenda", { start: iso(start), days: 7 }) : await invoke("todos");
    sel = Math.min(sel, Math.max(0, shown.length - 1));
  }

  $effect(() => {
    void reload;
    void start;
    if (active) api.act(refresh);
  });
  $effect(() => {
    if (active) tick().then(() => root?.focus());
  });
  $effect(() => {
    root?.querySelector(`[data-i="${sel}"]`)?.scrollIntoView({ block: "nearest" });
  });

  async function key(e: KeyboardEvent) {
    const it = shown[sel];
    const k = (e.shiftKey && e.key === "ArrowRight") ? "t" : (e.shiftKey && e.key === "ArrowLeft") ? "T" : e.key;
    const run = (f: () => Promise<void>) => api.act(async () => { await f(); await refresh(); });
    switch (k) {
      case "j": case "ArrowDown": sel = Math.min(sel + 1, shown.length - 1); break;
      case "k": case "ArrowUp": sel = Math.max(sel - 1, 0); break;
      case "g": sel = 0; break;
      case "G": sel = shown.length - 1; break;
      case "Enter": case "Tab": if (it) api.open(it.path, it.line); break;
      case "t": if (it) run(() => api.cycle(it, 1)); break;
      case "T": if (it) run(() => api.cycle(it, -1)); break;
      case "s": if (it) run(() => api.plan(it, "SCHEDULED")); break;
      case "d": if (it) run(() => api.plan(it, "DEADLINE")); break;
      case "I": if (it) api.clockIn(it); break;
      case "O": api.clockOut(); break;
      case "r": api.act(refresh); break;
      case "f": if (mode === "agenda") start = addDays(start, 7); break;
      case "b": if (mode === "agenda") start = addDays(start, -7); break;
      case ".": start = monday(new Date()); sel = 0; break;
      case "/": filter = (await api.pick({ prompt: "Filter (text, tag, keyword)", allowCustom: true })) ?? filter; sel = 0; root?.focus(); break;
      case "q": api.close(); break;
      default: return;
    }
    e.preventDefault();
  }

  function pointer(e: MouseEvent) {
    const i = (e.target as HTMLElement).closest<HTMLElement>("[data-i]")?.dataset.i;
    if (i == null) return;
    sel = +i;
    if (e.type === "dblclick") api.open(shown[sel].path, shown[sel].line);
  }

  // Flat index of each item in `shown`, so day groups can share one selection.
  const indexed = $derived(shown.map((it, i) => ({ it, i })));
</script>

{#snippet row(it: AgendaItem, i: number)}
  <div class="row" class:sel={i === sel} data-i={i} role="option" aria-selected={i === sel} tabindex="-1">
    <span class="cat">{it.category}:</span>
    <span class="lbl" class:warn={it.kind === "overdue" || it.kind === "warning" || it.kind === "deadline"}>{it.time ?? ""} {mode === "agenda" ? it.label : ""}</span>
    {#if it.keyword}<span class="kw" class:done={!!it.keyword && /DONE|CANCEL/.test(it.keyword)}>{it.keyword}</span>{/if}
    {#if it.priority}<span class="prio">[#{it.priority}]</span>{/if}
    <span class="title">{it.title}</span>
    {#if it.tags.length}<span class="tags">:{it.tags.join(":")}:</span>{/if}
  </div>
{/snippet}

<div class="agenda" bind:this={root} tabindex="0" onkeydown={key} onclick={pointer} ondblclick={pointer} role="listbox">
  {#if mode === "agenda"}
    <h2>Week {week(start)} <small>{iso(start)} – {iso(days[6])}</small></h2>
    {#each days as d}
      <div class="day" class:today={iso(d) === today}>
        {d.toLocaleDateString(undefined, { weekday: "long", day: "numeric", month: "long", year: "numeric" })}
      </div>
      {#each indexed.filter((x) => x.it.date === iso(d)) as { it, i } (i)}
        {@render row(it, i)}
      {/each}
    {/each}
  {:else}
    <h2>TODOs <small>{filter ? `filter: ${filter}` : `${items.length} open`}</small></h2>
    {#each indexed as { it, i } (i)}
      {@render row(it, i)}
    {/each}
  {/if}
  <footer>
    j/k move · ↵ open · t/T cycle · s schedule · d deadline · I clock in · O clock out
    {mode === "agenda" ? "· f/b week · . today" : "· / filter"} · r refresh · q close
  </footer>
</div>

<style>
  .agenda { height: 100%; overflow-y: auto; padding: 12px 24px; font: 14px/1.6 var(--mono); outline: none; box-sizing: border-box; }
  h2 { font-size: 16px; color: var(--h1); margin: 0 0 8px; }
  h2 small { color: var(--dim); font-weight: normal; margin-left: 8px; }
  .day { color: var(--h2); font-weight: 600; margin-top: 8px; }
  .day.today { color: var(--accent); text-decoration: underline; }
  .row { display: flex; gap: 8px; padding-left: 16px; cursor: default; white-space: nowrap; }
  .row.sel { background: var(--sel); }
  .cat { color: var(--dim); min-width: 14ch; overflow: hidden; text-overflow: ellipsis; }
  .lbl { color: var(--dim); min-width: 13ch; }
  .lbl.warn { color: var(--todo); }
  .kw { color: var(--todo); font-weight: 700; }
  .kw.done { color: var(--done); }
  .prio { color: var(--todo); }
  .title { overflow: hidden; text-overflow: ellipsis; }
  .tags { color: var(--dim); margin-left: auto; font-style: italic; }
  footer { color: var(--dim); font-size: 12px; margin-top: 24px; }
</style>
