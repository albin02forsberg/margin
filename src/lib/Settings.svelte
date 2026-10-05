<script lang="ts">
  import { tick, untrack } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { changes, checkTemplates, checkViews, GROUPS, refresh, refreshRows, toForm, type Field, type Form, type Row } from "$lib/settings";

  type List = "views" | "templates";
  /** The config.toml settings as a form, saved per group; unsaved fields survive a reload of CONFIG. */
  /** ERROR: why config.toml couldn't be loaded; MISSING: it's gone, so RECREATE can write it from the current settings. */
  let { config, defaults, error, missing, save, edit, recreate }: { config: Record<string, unknown>; defaults: Record<string, unknown>; error: string | null; missing: boolean; save: (values: Record<string, unknown>, reset?: string[], tables?: Partial<Record<List, Row[]>>) => Promise<void>; edit: () => void; recreate: () => Promise<void> } = $props();
  let form = $state<Form>({});
  let base = $state<Form>({});
  let errors = $state<Record<string, string>>({});
  /** Fields with unsaved edits here that config.toml also changed underneath. */
  let clash = $state<string[]>([]);
  /** Saved views and capture templates (`[[views]]`, `[[templates]]`) as edited here, as last
   *  loaded, and whether config.toml changed them under unsaved edits. */
  let rows = $state<Record<List, Row[]>>({ views: [], templates: [] });
  let rowsBase = $state<Record<List, Row[]>>({ views: [], templates: [] });
  let rowsClash = $state<Record<List, boolean>>({ views: false, templates: false });
  /** Keys of the templates folder's files, which capture_templates lists after config.toml's. */
  let fileKeys = $state<string[]>([]);
  $effect(() => {
    const c = config;
    untrack(() => {
      let more: string[];
      [form, base, more] = refresh(form, base, c);
      clash = [...new Set([...more, ...clash.filter((k) => form[k] !== base[k])])];
      for (const k of ["views", "templates"] as const) {
        let changed: boolean;
        [rows[k], rowsBase[k], changed] = refreshRows(rows[k], rowsBase[k], c[k]);
        rowsClash[k] = changed || (rowsClash[k] && dirtyRows(k));
      }
      const n = ((c.templates ?? []) as Row[]).length;
      invoke<Row[]>("capture_templates").then((t) => (fileKeys = t.slice(n).map((t) => t.key)), () => {});
    });
  });
  const dirtyRows = (k: List) => JSON.stringify(rows[k]) !== JSON.stringify(rowsBase[k]);
  const fileClash = $derived([...new Set(rows.templates.map((t) => t.key.trim()).filter((k) => fileKeys.includes(k)))]);
  async function saveRows(k: List) {
    try {
      await save({}, [], { [k]: k === "views" ? checkViews(rows.views) : checkTemplates(rows.templates) });
      await tick();
      rows[k] = rowsBase[k].map((v) => ({ ...v }));
      rowsClash[k] = false;
      errors[k] = "";
    } catch (e) {
      errors[k] = String(e);
    }
  }
  /** Swap rows I and I+D of LIST. */
  function move(list: Row[], i: number, d: number) {
    [list[i], list[i + d]] = [list[i + d], list[i]];
  }
  const FIELDS = GROUPS.flatMap((g) => g.fields);
  const clashing = $derived(FIELDS.filter((f) => clash.includes(f.key) && form[f.key] !== base[f.key]));

  const dirty = (fields: Field[]) => fields.some((f) => form[f.key] !== base[f.key]);
  async function submit(name: string, fields: Field[]) {
    try {
      await save(changes(fields, form, config));
      await tick();
      for (const f of fields) form[f.key] = base[f.key];
      errors[name] = "";
    } catch (e) {
      errors[name] = String(e);
    }
  }
  /** Drop F's line from config.toml (and any unsaved edit of it), so the default applies. */
  async function reset(name: string, f: Field) {
    try {
      await save({}, [f.key]);
      await tick();
      form[f.key] = base[f.key];
      errors[name] = "";
    } catch (e) {
      errors[name] = String(e);
    }
  }
  const text = (e: Event) => (e.currentTarget as HTMLInputElement).value;
  async function restore() {
    try {
      await recreate();
      errors.config = "";
    } catch (e) {
      errors.config = String(e);
    }
  }
</script>

<div class="settings">
  <h1>Settings</h1>
  <p class="dim">Saved to config.toml; comments and everything not shown here stay as written.{#if !missing} <button class="link" onclick={edit}>Edit config.toml</button> by hand.{/if}</p>
  {#if error}
    <p class="error">⚠ config.toml couldn't be loaded ({error}); Margin keeps running on the last good settings (defaults if it never loaded).{missing ? "" : " Fix the file first."}</p>
    {#if missing}<button onclick={restore}>Recreate config.toml from the current settings</button>{/if}
    {#if errors.config}<p class="error">⚠ {errors.config}</p>{/if}
  {/if}
  {#if clashing.length}
    <p class="error">⚠ config.toml changed on disk; your unsaved {clashing.map((f) => f.label).join(", ")} would overwrite it.</p>
  {/if}
  {#each GROUPS as g (g.name)}
    <form onsubmit={(e) => { e.preventDefault(); submit(g.name, g.fields); }}>
      <fieldset disabled={!!error}>
        <legend>{g.name}</legend>
        {#each g.fields as f (f.key)}
          <label class:check={f.kind === "bool"}>
            {#if f.kind === "bool"}
              <input type="checkbox" checked={form[f.key] as boolean} onchange={(e) => (form[f.key] = (e.currentTarget as HTMLInputElement).checked)} />
              <span>{f.label}</span>
            {:else}
              <span>{f.label}</span>
              {#if f.kind === "list"}
                <textarea rows={Math.max(2, String(form[f.key] ?? "").split("\n").length)} value={form[f.key] as string} oninput={(e) => (form[f.key] = text(e))}></textarea>
              {:else}
                <input type={f.kind === "number" ? "number" : "text"} step={f.step} value={form[f.key] as string} oninput={(e) => (form[f.key] = text(e))} />
              {/if}
            {/if}
            {#if f.hint}<small>{f.hint}</small>{/if}
            {#if base[f.key] !== toForm(f, defaults[f.key])}
              <button type="button" class="link reset" title="Default: {String(toForm(f, defaults[f.key])).replaceAll("\n", ", ") || "empty"}" onclick={() => reset(g.name, f)}>Reset to default</button>
            {/if}
          </label>
        {/each}
        {#if errors[g.name]}<p class="error">⚠ {errors[g.name]}</p>{/if}
        <button type="submit" disabled={!dirty(g.fields)}>Save {g.name.toLowerCase()}</button>
      </fieldset>
    </form>
  {/each}
  {#snippet buttons(list: Row[], i: number)}
    <button type="button" class="link" title="Move up" disabled={i === 0} onclick={() => move(list, i, -1)}>↑</button>
    <button type="button" class="link" title="Move down" disabled={i === list.length - 1} onclick={() => move(list, i, 1)}>↓</button>
    <button type="button" class="link" title="Remove" onclick={() => list.splice(i, 1)}>✕</button>
  {/snippet}
  {#snippet foot(k: List, what: string)}
    {#if rowsClash[k] && dirtyRows(k)}<p class="error">⚠ config.toml changed on disk; saving your unsaved {what} would overwrite it.</p>{/if}
    {#if errors[k]}<p class="error">⚠ {errors[k]}</p>{/if}
    <button type="submit" disabled={!dirtyRows(k)}>Save {what}</button>
  {/snippet}
  <form onsubmit={(e) => { e.preventDefault(); saveRows("views"); }}>
    <fieldset disabled={!!error} class="list">
      <legend>Saved views</legend>
      <small>Task searches in the sidebar. Saving rewrites the [[views]] tables of config.toml; comments inside them are not kept.</small>
      {#each rows.views as v, i}
        <div class="row">
          <input aria-label="View name" placeholder="Name" bind:value={v.name} />
          <input aria-label="View query" placeholder="Query, e.g. todo:NEXT tag:work" bind:value={v.query} />
          {@render buttons(rows.views, i)}
        </div>
      {/each}
      <button type="button" class="link" onclick={() => rows.views.push({ name: "", query: "" })}>Add view</button>
      {@render foot("views", "views")}
    </fieldset>
  </form>
  <form onsubmit={(e) => { e.preventDefault(); saveRows("templates"); }}>
    <fieldset disabled={!!error} class="list">
      <legend>Capture templates</legend>
      <small>Picked with Space c. File: strftime pattern relative to the notes folder, empty = the inbox. Heading: filed under it (created if missing). Body: {"%^{Prompt}"} asks, %U is now, %i the selection, %? the cursor. Templates folder files aren't listed here. Saving rewrites the [[templates]] tables of config.toml; comments inside them are not kept.</small>
      {#each rows.templates as t, i}
        <div class="tpl">
          <div class="row key">
            <input aria-label="Template key" placeholder="Key" bind:value={t.key} />
            <input aria-label="Template name" placeholder="Name" bind:value={t.name} />
            {@render buttons(rows.templates, i)}
          </div>
          <div class="pair">
            <input aria-label="Template file" placeholder="File (empty = inbox)" bind:value={t.file} />
            <input aria-label="Template heading" placeholder="Under heading (optional)" bind:value={t.heading} />
          </div>
          <textarea aria-label="Template body" rows={Math.max(2, t.body.split("\n").length)} placeholder={"* %^{Title}\n%U\n%?"} bind:value={t.body}></textarea>
        </div>
      {/each}
      <button type="button" class="link" onclick={() => rows.templates.push({ key: "", name: "", file: "", heading: "", body: "" })}>Add template</button>
      {#if fileClash.length}<p class="error">⚠ Key {fileClash.join(", ")} is also used by a file in the templates folder.</p>{/if}
      {@render foot("templates", "templates")}
    </fieldset>
  </form>
</div>

<style>
  .settings { height: 100%; overflow-y: auto; padding: 20px 28px 40px; box-sizing: border-box; font-family: var(--sans); max-width: 760px; }
  h1 { margin: 0 0 8px; font-size: 22px; font-weight: 650; }
  form { margin: 18px 0 0; }
  fieldset { border: 1px solid var(--border); border-radius: var(--radius); padding: var(--s2) var(--s3) var(--s3); margin: 0; }
  legend { color: var(--dim); font-size: var(--fs-sm); padding: 0 var(--s1); }
  label { display: grid; gap: 2px; margin: var(--s2) 0; font-size: var(--fs-base); }
  label.check { grid-template-columns: auto 1fr; align-items: center; column-gap: var(--s2); }
  label.check small { grid-column: 2; }
  small { color: var(--dim); font-size: var(--fs-sm); }
  input:not([type="checkbox"]), textarea { background: var(--panel); color: var(--fg); border: 1px solid var(--border); border-radius: var(--radius); padding: var(--s1) var(--s2); font: var(--fs-md) var(--mono); box-sizing: border-box; width: 100%; resize: vertical; }
  button { background: var(--accent); border: 1px solid var(--accent); color: var(--bg); border-radius: var(--radius); padding: var(--s1) var(--s3); font: 600 var(--fs-md) var(--sans); cursor: pointer; margin-top: var(--s1); }
  button:disabled { background: var(--bg); border-color: var(--border); color: var(--dim); cursor: default; font-weight: 400; }
  button.link { background: none; border: none; color: var(--accent); padding: 0; margin: 0; font: inherit; text-decoration: underline; }
  button.reset { justify-self: start; font-size: var(--fs-sm); }
  label.check button.reset { grid-column: 2; }
  .list { display: grid; gap: var(--s1); justify-items: start; }
  .row { display: grid; grid-template-columns: 1fr 2fr auto auto auto; gap: var(--s1); align-items: center; width: 100%; }
  .row.key { grid-template-columns: 4em 1fr auto auto auto; }
  .tpl { display: grid; gap: var(--s1); width: 100%; padding-bottom: var(--s2); border-bottom: 1px solid var(--border); }
  .pair { display: grid; grid-template-columns: 1fr 1fr; gap: var(--s1); }
  .row button.link { text-decoration: none; padding: 0 var(--s1); }
  .row button.link:disabled { background: none; color: var(--dim); }
  .dim { color: var(--dim); font-size: var(--fs-md); }
  .error { color: var(--todo); }
</style>
