<script lang="ts">
  import { tick, untrack } from "svelte";
  import { changes, GROUPS, refresh, toForm, type Field, type Form } from "$lib/settings";

  /** The config.toml settings as a form, saved per group; unsaved fields survive a reload of CONFIG. */
  let { config, defaults, error, save, edit }: { config: Record<string, unknown>; defaults: Record<string, unknown>; error: string | null; save: (values: Record<string, unknown>, reset?: string[]) => Promise<void>; edit: () => void } = $props();
  let form = $state<Form>({});
  let base = $state<Form>({});
  let errors = $state<Record<string, string>>({});
  /** Fields with unsaved edits here that config.toml also changed underneath. */
  let clash = $state<string[]>([]);
  $effect(() => {
    const c = config;
    untrack(() => {
      let more: string[];
      [form, base, more] = refresh(form, base, c);
      clash = [...new Set([...more, ...clash.filter((k) => form[k] !== base[k])])];
    });
  });
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
</script>

<div class="settings">
  <h1>Settings</h1>
  <p class="dim">Saved to config.toml; comments and everything not shown here stay as written. <button class="link" onclick={edit}>Edit config.toml</button> for saved views and templates.</p>
  {#if error}
    <p class="error">⚠ config.toml was ignored ({error}), so these are the defaults. Fix the file first.</p>
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
  .dim { color: var(--dim); font-size: var(--fs-md); }
  .error { color: var(--todo); }
</style>
