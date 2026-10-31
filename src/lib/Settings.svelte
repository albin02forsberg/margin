<script lang="ts">
  import { tick, untrack } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { CATEGORIES, changes, checkFeeds, checkSnippets, checkTemplates, checkViews, GROUPS, hide, needsMacPrompt, refresh, refreshRows, shows, toForm, type Field, type Form, type Row } from "$lib/settings";

  type List = "views" | "templates" | "snippets" | "feeds";
  /** The config.toml settings as a form, saved per group; unsaved fields survive a reload of CONFIG. */
  /** ERROR: why config.toml couldn't be loaded; MISSING: it's gone, so RECREATE can write it from the current settings. */
  let { version, config, defaults, error, missing, save, edit, recreate, active, reload }: { version: string; config: Record<string, unknown>; defaults: Record<string, unknown>; error: string | null; missing: boolean; save: (values: Record<string, unknown>, reset?: string[], tables?: Partial<Record<List, Row[]>>) => Promise<void>; edit: () => void; recreate: () => Promise<void>; active: boolean; reload: number } = $props();
  let form = $state<Form>({});
  let base = $state<Form>({});
  let errors = $state<Record<string, string>>({});
  /** Fields with unsaved edits here that config.toml also changed underneath. */
  let clash = $state<string[]>([]);
  /** Saved views, capture templates, snippets and feeds (`[[views]]`, `[[templates]]`, `[[snippets]]`, `[[feeds]]`) as
   *  edited here, as last loaded, and whether config.toml changed them under unsaved edits. */
  let rows = $state<Record<List, Row[]>>({ views: [], templates: [], snippets: [], feeds: [] });
  let rowsBase = $state<Record<List, Row[]>>({ views: [], templates: [], snippets: [], feeds: [] });
  let rowsClash = $state<Record<List, boolean>>({ views: false, templates: false, snippets: false, feeds: false });
  /** Keys of the templates folder's files, which capture_templates lists after config.toml's. */
  let fileKeys = $state<string[]>([]);
  $effect(() => {
    const c = config;
    untrack(() => {
      let more: string[];
      [form, base, more] = refresh(form, base, c);
      clash = [...new Set([...more, ...clash.filter((k) => form[k] !== base[k])])];
      for (const k of ["views", "templates", "snippets", "feeds"] as const) {
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
      await save({}, [], { [k]: ({ views: checkViews, templates: checkTemplates, snippets: checkSnippets, feeds: checkFeeds })[k](rows[k]) });
      await tick();
      rows[k] = rowsBase[k].map((v) => ({ ...v }));
      rowsClash[k] = false;
      errors[k] = "";
    } catch (e) {
      errors[k] = String(e);
    }
  }
  let opmlPath = $state("");
  /** Add the feeds of the OPML file at OPMLPATH that aren't listed yet; they stay unsaved until Save. */
  async function importOpml() {
    try {
      const have = new Set(rows.feeds.map((f) => f.url.trim()));
      const add = (await invoke<[string, string][]>("feeds_opml", { path: opmlPath })).filter(([, u]) => !have.has(u));
      rows.feeds.push(...add.map(([name, url]) => ({ name, url, file: "", heading: "" })));
      errors.feeds = add.length ? "" : "Every feed in that file is already listed";
      if (add.length) opmlPath = "";
    } catch (e) {
      errors.feeds = String(e);
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
  /** On GNOME Wayland the activity watcher's Shell extension: "running", "installed" or "missing"; null elsewhere. */
  let gnome = $state<string | null>(null);
  async function gnomeExtension(install = false) {
    try {
      gnome = await invoke<string | null>("gnome_extension", { install });
      errors.gnome = "";
    } catch (e) {
      errors.gnome = String(e);
    }
  }
  gnomeExtension();
  /** The profile's learned activity rules (activity_rules.toml), reloaded when the page is shown. */
  type Rule = { pattern: string; project: string; missing: boolean };
  let rules = $state<Rule[]>([]);
  async function loadRules(f: () => Promise<unknown> = async () => {}) {
    try {
      await f();
      rules = await invoke<Rule[]>("activity_rules_list");
      errors.rules = "";
    } catch (e) {
      errors.rules = String(e);
    }
  }
  $effect(() => {
    void reload;
    if (active) untrack(() => loadRules());
  });
  /** macOS: whether Margin may read window titles (the Screen Recording permission); null elsewhere. */
  let granted = $state<boolean | null>(null);
  invoke<boolean | null>("screen_recording", { request: false }).then((g) => (granted = g), () => {});
  /** The `claude mcp add …` line that registers this binary's read-only MCP server (#929). */
  let mcp = $state("");
  invoke<string>("mcp_command").then((c) => (mcp = c), () => {});
  /** The semantic index (#965): size on disk, what's in it, whether it's updating, the last error. */
  type IndexStatus = { bytes: number; notes: number; sections: number; running: boolean; error: string };
  let index = $state<IndexStatus | null>(null);
  async function loadIndex(f: () => Promise<unknown> = async () => {}) {
    try {
      await f();
      index = await invoke<IndexStatus>("ai_index_status");
      errors.index = "";
    } catch (e) {
      errors.index = String(e);
    }
  }
  $effect(() => {
    void reload;
    if (active && config.ai_semantic) untrack(() => loadIndex());
  });
  $effect(() => {
    const off = listen("ai-index", () => loadIndex());
    return () => void off.then((f) => f());
  });
  let macDialog: HTMLDialogElement;
  const MAC_OK = "activity-watcher-macos-ok";
  function check(f: Field, box: HTMLInputElement) {
    let ok = false;
    try { ok = localStorage.getItem(MAC_OK) === "1"; } catch {}
    if (f.key === "activity_watcher" && needsMacPrompt(granted, base[f.key] as boolean, box.checked, ok)) {
      box.checked = false;
      macDialog.showModal();
    } else form[f.key] = box.checked;
  }
  async function macContinue() {
    try { localStorage.setItem(MAC_OK, "1"); } catch {}
    macDialog.close();
    form.activity_watcher = true;
    granted = await invoke<boolean | null>("screen_recording", { request: true }).catch(() => granted);
  }
  /** Category chip and filter text: they only hide cards and fields; Save still sends a group's hidden edits. */
  let cat = $state("All");
  let q = $state("");
  let filterInput = $state<HTMLInputElement>();
  const fieldsOf = (g: (typeof GROUPS)[number]) => g.fields.filter((f) => (!f.when || f.when(config)) && shows(cat, q, g.name, f.label, f.hint, f.key));
  const VIEWS = "task searches query", TPLS = "capture key file heading body", SNIPS = "editor tab key text insert", FEEDS = "rss atom url reading subscribe";
  const showRules = $derived(!!(rules.length || errors.rules) && shows(cat, q, "Integrations", "Learned activity rules"));
  const none = $derived(!GROUPS.some((g) => fieldsOf(g).length) && !shows(cat, q, "Saved views", VIEWS) && !shows(cat, q, "Capture templates", TPLS) && !shows(cat, q, "Snippets", SNIPS) && !shows(cat, q, "Feeds", FEEDS) && !showRules);
  function key(e: KeyboardEvent) {
    if (e.target === filterInput) {
      if (e.key === "Escape") { q = ""; filterInput?.blur(); e.preventDefault(); }
    } else if (e.key === "/" && !(e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement)) {
      filterInput?.focus();
      e.preventDefault();
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

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="settings" onkeydown={key}>
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
  <div class="bar">
    <div class="chips" role="group" aria-label="Categories">
      {#each CATEGORIES as c (c)}<button type="button" class:on={cat === c} onclick={() => (cat = c)}>{c}</button>{/each}
    </div>
    <input class="filter" type="search" aria-label="Filter settings" placeholder="Filter settings  /" spellcheck="false" bind:this={filterInput} bind:value={q} />
  </div>
  {#if none}<p class="dim">No settings match.</p>{/if}
  {#each GROUPS as g (g.name)}
    {@const shown = fieldsOf(g)}
    <form class="card" hidden={!shown.length} onsubmit={(e) => { e.preventDefault(); submit(g.name, g.fields); }}>
      <fieldset disabled={!!error}>
        <legend>{g.name}</legend>
        {#each shown as f (f.key)}
          <svelte:element this={f.options ? "div" : "label"} class="field" class:check={f.kind === "bool"}>
            {#if f.kind === "bool"}
              <input type="checkbox" checked={form[f.key] as boolean} onchange={(e) => check(f, e.currentTarget)} />
              <span>{f.label}</span>
            {:else if f.options}
              <span>{f.label}</span>
              <span class="opts">{#each f.options as o (o)}<label class="check"><input type="checkbox" checked={!String(form[f.key]).split("\n").includes(o)} onchange={(e) => (form[f.key] = hide(String(form[f.key]), o, !e.currentTarget.checked))} /><span>{o[0].toUpperCase() + o.slice(1)}</span></label>{/each}</span>
            {:else}
              <span>{f.label}</span>
              {#if f.kind === "list" || f.kind === "multiline"}
                <textarea rows={Math.max(2, String(form[f.key] ?? "").split("\n").length)} value={form[f.key] as string} oninput={(e) => (form[f.key] = text(e))}></textarea>
              {:else}
                <input type={f.kind === "number" ? "number" : "text"} step={f.step} value={form[f.key] as string} oninput={(e) => (form[f.key] = text(e))} />
              {/if}
            {/if}
            {#if f.hint}<small>{f.hint}</small>{/if}
            {#if base[f.key] !== toForm(f, defaults[f.key])}
              <button type="button" class="link reset" title="Default: {String(toForm(f, defaults[f.key])).replaceAll("\n", ", ") || "empty"}" onclick={() => reset(g.name, f)}>Reset to default</button>
            {/if}
          </svelte:element>
          {#if f.key === "ai_semantic" && base.ai_semantic}
            <p class="dim">
              {#if index}Index: {index.sections} sections of {index.notes} notes, {(index.bytes / 1048576).toFixed(1)} MB{index.running ? " (updating…)" : ""}.{/if}
              <button type="button" class="link" disabled={index?.running} onclick={() => loadIndex(() => invoke("ai_index_rebuild"))}>Rebuild</button>
            </p>
            {#if index?.error}<p class="error">⚠ {index.error}</p>{/if}
            {#if errors.index}<p class="error">⚠ {errors.index}</p>{/if}
          {/if}
        {/each}
        {#if g.name === "Integrations" && base.activity_watcher && gnome && gnome !== "running"}
          {#if gnome === "missing"}
            <p class="dim">On GNOME Wayland the activity watcher needs a small Shell extension to see windows. <button type="button" class="link" onclick={() => gnomeExtension(true)}>Install the extension</button></p>
          {:else}
            <p class="dim">GNOME extension installed: log out and in once, then turn on “Margin active window” in the Extensions app.</p>
          {/if}
          {#if errors.gnome}<p class="error">⚠ {errors.gnome}</p>{/if}
        {/if}
        {#if g.name === "Integrations" && base.activity_watcher && granted === false}
          <p class="dim">macOS hides window titles until Margin has the Screen Recording permission: System Settings → Privacy & Security → Screen & System Audio Recording, turn on Margin, then restart it.</p>
        {/if}
        {#if g.name === "AI chat" && mcp}
          <div class="field mcp">
            <span>Claude (MCP)</span>
            <code>{mcp}</code>
            <small>Lets Claude Code search and read your notes, tasks and time log (read-only unless <b>Let Claude (MCP) edit notes</b> is on; <code>ai_exclude</code> applies). Run this in a terminal; for Claude Desktop add a server with this command and the argument <code>--mcp</code>. <button type="button" class="link" onclick={() => navigator.clipboard.writeText(mcp)}>Copy</button></small>
          </div>
        {/if}
        {#if errors[g.name]}<p class="error">⚠ {errors[g.name]}</p>{/if}
        <button type="submit" disabled={!dirty(g.fields)}>Save {g.name.toLowerCase()}</button>
      </fieldset>
    </form>
  {/each}
  {#if showRules}
    <fieldset class="list rules card">
      <legend>Learned activity rules</legend>
      <small>Suggested blocks whose title matches go to the project; learned when you log a suggestion to another project than guessed. Kept in this profile's activity_rules.toml.</small>
      {#each rules as r (r.pattern)}
        <div class="row" class:gone={r.missing}>
          <code>{r.pattern}</code>
          <span>→ {r.project}{#if r.missing} <small>(no such active project, so it's skipped)</small>{/if}</span>
          <button type="button" class="link" title="Delete rule" onclick={() => loadRules(() => invoke("activity_rules_delete", { pattern: r.pattern }))}>✕</button>
        </div>
      {/each}
      {#if errors.rules}<p class="error">⚠ {errors.rules}</p>{/if}
    </fieldset>
  {/if}
  <dialog bind:this={macDialog}>
    <p>The activity watcher notes the active app and window title every 5 seconds in the time-tracking folder, for suggestions in the Time view. It stays on this Mac and is never uploaded.</p>
    <p>macOS only shows window titles to apps with the <b>Screen Recording</b> permission, so it asks for it next. Margin never captures the screen itself.</p>
    <button onclick={macContinue}>Continue</button>
    <button class="link" onclick={() => macDialog.close()}>Cancel</button>
  </dialog>
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
  <form class="card wide" hidden={!shows(cat, q, "Saved views", VIEWS)} onsubmit={(e) => { e.preventDefault(); saveRows("views"); }}>
    <fieldset disabled={!!error} class="list">
      <legend>Saved views</legend>
      <small>Task searches in the top bar. Saving rewrites the [[views]] tables of config.toml; comments inside them are not kept.</small>
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
  <form class="card wide" hidden={!shows(cat, q, "Capture templates", TPLS)} onsubmit={(e) => { e.preventDefault(); saveRows("templates"); }}>
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
  <form class="card wide" hidden={!shows(cat, q, "Snippets", SNIPS)} onsubmit={(e) => { e.preventDefault(); saveRows("snippets"); }}>
    <fieldset disabled={!!error} class="list">
      <legend>Snippets</legend>
      <small>In a note, type {"<key"} alone on a line and press Tab (or pick one with "Insert snippet…" in the command palette). Yours win over the built-in blocks ({"<s <q <e <v <c <C <l <h <a <el"}). Text: %t/%T today's date/time, %u/%U inactive ones, {"%^{Prompt}"} asks, %? the cursor; later lines get the line's indent. Saving rewrites the [[snippets]] tables of config.toml; comments inside them are not kept.</small>
      {#each rows.snippets as s, i}
        <div class="tpl">
          <div class="row key">
            <input aria-label="Snippet key" placeholder="Key, e.g. mtg" bind:value={s.key} />
            {@render buttons(rows.snippets, i)}
          </div>
          <textarea aria-label="Snippet text" rows={Math.max(2, s.text.split("\n").length)} placeholder={"* Meeting %u\n- %?"} bind:value={s.text}></textarea>
        </div>
      {/each}
      <button type="button" class="link" onclick={() => rows.snippets.push({ key: "", text: "" })}>Add snippet</button>
      {@render foot("snippets", "snippets")}
    </fieldset>
  </form>
  <form class="card wide" hidden={!shows(cat, q, "Feeds", FEEDS)} onsubmit={(e) => { e.preventDefault(); saveRows("feeds"); }}>
    <fieldset disabled={!!error} class="list">
      <legend>Feeds</legend>
      <small>RSS and Atom feeds. "Fetch feeds" in the command palette files their new items as TODO Read entries, each once; a new feed brings its latest 20. File: strftime pattern relative to the notes folder, empty = the inbox. Heading: empty = Reading. Saving rewrites the [[feeds]] tables of config.toml; comments inside them are not kept.</small>
      {#each rows.feeds as f, i}
        <div class="tpl">
          <div class="row">
            <input aria-label="Feed name" placeholder="Name" bind:value={f.name} />
            <input aria-label="Feed URL" placeholder="https://example.com/feed.xml" bind:value={f.url} />
            {@render buttons(rows.feeds, i)}
          </div>
          <div class="pair">
            <input aria-label="Feed file" placeholder="File (empty = inbox)" bind:value={f.file} />
            <input aria-label="Feed heading" placeholder="Under heading (empty = Reading)" bind:value={f.heading} />
          </div>
        </div>
      {/each}
      <button type="button" class="link" onclick={() => rows.feeds.push({ name: "", url: "", file: "", heading: "" })}>Add feed</button>
      <div class="row">
        <input aria-label="OPML file" placeholder="Absolute path to an OPML file, e.g. ~/feeds.opml" bind:value={opmlPath} />
        <button type="button" class="link" disabled={!opmlPath.trim()} onclick={importOpml}>Import OPML</button>
      </div>
      {@render foot("feeds", "feeds")}
    </fieldset>
  </form>
  <section class="card wide about" hidden={cat !== "All" || !!q.trim()}>
    <h2>About</h2>
    <p>Margin {version ? `v${version}` : ""}</p>
  </section>
</div>

<style>
  .settings { height: 100%; overflow-y: auto; padding: 20px 28px 40px; box-sizing: border-box; font-family: var(--sans); display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: var(--s3); align-content: start; align-items: start; }
  /* heading, hint and banners span the full grid; cards sit on the form/fieldset, never inside a label */
  .settings > :not(.card) { grid-column: 1 / -1; margin: 0; }
  .card.wide, .rules { grid-column: 1 / -1; }
  .card[hidden] { display: none; }
  /* Category chips and filter (round-5 mockup, #357) */
  .bar { display: flex; flex-wrap: wrap; gap: var(--s2); align-items: center; justify-content: space-between; }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; }
  .chips button { background: var(--bg); border: 1px solid var(--border); color: var(--fg); border-radius: 999px; padding: 3px 10px; margin: 0; font: var(--fs-sm) var(--sans); }
  .chips button.on { border-color: var(--accent); color: var(--accent); }
  input.filter { width: 220px; font-family: var(--sans); }
  .card:hover { transform: none; box-shadow: var(--soft); }
  @media (max-width: 700px) { .settings { grid-template-columns: minmax(0, 1fr); } }
  .about h2 { margin: 0 0 var(--s1); color: var(--dim); font-size: var(--fs-sm); font-weight: 400; }
  .about p { margin: 0; }
  h1 { margin: 0 0 8px; font-size: 22px; font-weight: 650; }
  form > fieldset { border: 0; padding: 0; margin: 0; min-width: 0; }
  .rules { border: 0; margin: 0; min-width: 0; }
  legend { color: var(--dim); font-size: var(--fs-sm); padding: 0 var(--s1); }
  label, .field { display: grid; gap: 2px; margin: var(--s2) 0; font-size: var(--fs-base); }
  .opts { display: flex; flex-wrap: wrap; column-gap: var(--s4); }
  .opts label { margin: 0; }
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
  .rules .row { grid-template-columns: 1fr 1fr auto; }
  .rules code { font: var(--fs-md) var(--mono); overflow-wrap: anywhere; }
  .gone { color: var(--dim); }
  .mcp > code { font: var(--fs-sm) var(--mono); overflow-wrap: anywhere; user-select: all; }
  .dim { color: var(--dim); font-size: var(--fs-md); }
  dialog { background: var(--bg); color: var(--fg); border: 1px solid var(--border); border-radius: var(--radius); max-width: 440px; font-size: var(--fs-base); }
  .error { color: var(--todo); }
</style>
