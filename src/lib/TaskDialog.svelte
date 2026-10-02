<script lang="ts" module>
  export type TaskSpec = { title: string; scheduled: string; deadline: string; priority: string; tags: string[]; file: string };
</script>

<script lang="ts">
  // "New task" form. SUBMIT throws to keep the dialog open with an error.
  import { invoke } from "@tauri-apps/api/core";
  import { tick } from "svelte";

  let active = $state(false);
  let spec = $state<TaskSpec>({ title: "", scheduled: "", deadline: "", priority: "", tags: [], file: "" });
  let tagText = $state("");
  let files = $state.raw<string[]>([]);
  let error = $state("");
  let schedPreview = $state("");
  let duePreview = $state("");
  let titleInput = $state<HTMLInputElement>();
  let submit: (s: TaskSpec) => Promise<void> = async () => {};
  let prevFocus: HTMLElement | null = null;

  export const isOpen = () => active;

  export async function open(init: Partial<TaskSpec> & { files: string[] }, onSubmit: (s: TaskSpec) => Promise<void>) {
    prevFocus = document.activeElement as HTMLElement | null;
    files = init.files;
    spec = { title: "", scheduled: "", deadline: "", priority: "", tags: [], file: init.files[0] ?? "", ...init };
    tagText = spec.tags.join(" ");
    error = "";
    submit = onSubmit;
    active = true;
    await tick();
    titleInput?.focus();
  }

  function close() {
    active = false;
    prevFocus?.focus();
  }

  async function save() {
    try {
      await submit({ ...spec, tags: tagText.split(/[\s,:]+/).filter(Boolean) });
      close();
    } catch (e) {
      error = String(e);
    }
  }

  const preview = (s: string) => invoke<string>("date_preview", { input: s }).catch(() => "");
  $effect(() => {
    const s = spec.scheduled;
    const t = setTimeout(async () => (schedPreview = await preview(s)), 100);
    return () => clearTimeout(t);
  });
  $effect(() => {
    const s = spec.deadline;
    const t = setTimeout(async () => (duePreview = await preview(s)), 100);
    return () => clearTimeout(t);
  });

  function key(e: KeyboardEvent) {
    e.stopPropagation();
    if (e.key === "Escape") close();
    else if (e.key === "Enter" && !(e.target instanceof HTMLSelectElement)) save();
    else if (e.altKey && "0123".includes(e.key)) spec.priority = ["", "A", "B", "C"][+e.key];
    else return;
    e.preventDefault();
  }

  const quick = (field: "scheduled" | "deadline", v: string) => (spec[field] = v);
</script>

{#if active}
  <div class="backdrop" onmousedown={close} role="presentation"></div>
  <div class="dialog" onkeydown={key} role="dialog" aria-label="New task" tabindex="-1">
    <h2>New task</h2>
    <input class="title" bind:this={titleInput} bind:value={spec.title} placeholder="What needs to be done?" spellcheck="false" />

    <div class="grid">
      <label for="sched">When</label>
      <div>
        <input id="sched" bind:value={spec.scheduled} placeholder="today, fri, +3d, 12-24 14:00" spellcheck="false" />
        <div class="chips">
          {#each [["Today", "today"], ["Tomorrow", "tomorrow"], ["Next week", "mon"], ["No date", ""]] as [l, v]}
            <button type="button" class:on={spec.scheduled === v} onclick={() => quick("scheduled", v)}>{l}</button>
          {/each}
          <span class="pv" class:bad={schedPreview.startsWith("✗")}>{schedPreview}</span>
        </div>
      </div>

      <label for="due">Due</label>
      <div>
        <input id="due" bind:value={spec.deadline} placeholder="optional deadline" spellcheck="false" />
        <span class="pv" class:bad={duePreview.startsWith("✗")}>{duePreview}</span>
      </div>

      <span class="lbl">Priority</span>
      <div class="chips">
        {#each [["", "None"], ["A", "High"], ["B", "Medium"], ["C", "Low"]] as [v, l], i}
          <button type="button" class="prio p{v}" class:on={spec.priority === v} onclick={() => (spec.priority = v)} title="Alt+{i}">{l}</button>
        {/each}
      </div>

      <label for="tags">Tags</label>
      <input id="tags" bind:value={tagText} placeholder="work home" spellcheck="false" />

      <label for="file">Save in</label>
      <select id="file" bind:value={spec.file}>
        {#each files as f}<option value={f}>{f}</option>{/each}
      </select>
    </div>

    {#if error}<p class="error">{error}</p>{/if}
    <footer>
      <span>↵ add · Esc cancel · Alt+0–3 priority</span>
      <button type="button" onclick={close}>Cancel</button>
      <button type="button" class="primary" onclick={save}>Add task</button>
    </footer>
  </div>
{/if}

<style>
  .backdrop { position: fixed; inset: 0; z-index: 499; background: rgb(0 0 0 / 0.35); }
  .dialog {
    position: fixed; top: 10%; left: 50%; transform: translateX(-50%); z-index: 500;
    width: min(560px, calc(100vw - 32px)); background: var(--panel); border: 1px solid var(--border);
    border-radius: 12px; box-shadow: 0 16px 48px rgb(0 0 0 / 0.4); padding: 18px 20px; outline: none;
  }
  h2 { margin: 0 0 12px; font-size: 13px; color: var(--dim); font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; }
  input, select { background: var(--bg); border: 1px solid var(--border); border-radius: 6px; color: var(--fg); font: 14px var(--sans); padding: 7px 9px; width: 100%; box-sizing: border-box; }
  input:focus, select:focus { outline: 2px solid var(--accent); outline-offset: -1px; }
  .title { font-size: 17px; padding: 10px 12px; margin-bottom: 14px; }
  .grid { display: grid; grid-template-columns: 70px 1fr; gap: 10px 12px; align-items: start; }
  .grid label, .lbl { color: var(--dim); font-size: 13px; padding-top: 8px; }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; margin-top: 6px; }
  .grid > .chips { margin-top: 2px; }
  .chips button { background: var(--bg); border: 1px solid var(--border); color: var(--fg); border-radius: 999px; padding: 3px 10px; font: 12px var(--sans); cursor: pointer; }
  .chips button.on { border-color: var(--accent); color: var(--accent); }
  .prio.pA.on { border-color: var(--todo); color: var(--todo); }
  .pv { font-size: 12px; color: var(--done); margin-left: 4px; }
  .pv.bad { color: var(--todo); }
  .error { color: var(--todo); font-size: 13px; margin: 12px 0 0; }
  footer { display: flex; gap: 8px; align-items: center; margin-top: 18px; }
  footer span { flex: 1; color: var(--dim); font-size: 11px; }
  footer button { background: var(--bg); border: 1px solid var(--border); color: var(--fg); border-radius: 6px; padding: 7px 14px; font: 13px var(--sans); cursor: pointer; }
  footer .primary { background: var(--accent); border-color: var(--accent); color: var(--bg); font-weight: 600; }
</style>
