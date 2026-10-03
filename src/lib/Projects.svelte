<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { merge, ROUNDINGS, toProject, type Project, type Row } from "$lib/projects";

  /** Project settings: every project's fields at once, saved per row. Reloads on RELOAD, keeping unsaved rows. */
  let { act, active, reload }: { act: (f: () => unknown) => void; active: boolean; reload: number } = $props();
  let rows = $state<Row[]>([]);
  let error = $state("");

  async function load() {
    try {
      rows = merge(rows, await invoke<Record<string, Project>>("tc_projects"));
      error = "";
    } catch (e) {
      error = String(e); // a malformed projects.toml: say so rather than show an empty list
    }
  }
  $effect(() => {
    void reload;
    if (active) act(load);
  });

  const save = (r: Row) => act(async () => {
    await invoke("tc_save_project", { name: r.name, project: toProject(r) });
    r.dirty = false;
  });
  const options = (r: Row) => (ROUNDINGS.some(([v]) => v === r.rounding) ? ROUNDINGS : [...ROUNDINGS, [r.rounding, `${r.rounding} h`]]);
</script>

<div class="projects">
  <h1>Projects</h1>
  {#if error}
    <p class="error">⚠ {error}. Fix the file (Time → Check log), then come back.</p>
  {:else if !rows.length}
    <p class="dim">No projects yet: they're added the first time you track time on one.</p>
  {:else}
    <table>
      <thead><tr><th>Project</th><th>Export code</th><th>Round billable hours to</th><th>Always round up</th><th>Show in project list</th><th></th></tr></thead>
      <tbody>
        {#each rows as r, i (r.name)}
          <tr class:inactive={!r.active}>
            <td>{r.name}</td>
            <td><form id="p{i}" onsubmit={(e) => { e.preventDefault(); save(r); }}><input aria-label="Export code for {r.name}" bind:value={r.code} oninput={() => (r.dirty = true)} placeholder={r.name} /></form></td>
            <td>
              <select form="p{i}" aria-label="Rounding for {r.name}" bind:value={r.rounding} onchange={() => (r.dirty = true)}>
                {#each options(r) as [v, l]}<option value={v}>{l}</option>{/each}
              </select>
            </td>
            <td><input form="p{i}" type="checkbox" aria-label="Always round up {r.name}" bind:checked={r.up} disabled={!r.rounding} onchange={() => (r.dirty = true)} /></td>
            <td><input form="p{i}" type="checkbox" aria-label="Show {r.name} in the project list" bind:checked={r.active} onchange={() => (r.dirty = true)} /></td>
            <td><button form="p{i}" type="submit" disabled={!r.dirty}>Save</button></td>
          </tr>
        {/each}
      </tbody>
    </table>
    <p class="dim">Enter in an export code saves its row. Hidden projects keep their time; they just aren't offered when you start tracking.</p>
  {/if}
</div>

<style>
  .projects { height: 100%; overflow-y: auto; padding: 20px 28px 40px; box-sizing: border-box; font-family: var(--sans); }
  h1 { margin: 0 0 18px; font-size: 22px; font-weight: 650; }
  table { width: 100%; border-collapse: collapse; font-size: var(--fs-base); }
  td, th { padding: 6px var(--s2); border-bottom: 1px solid var(--border); text-align: left; }
  th { color: var(--dim); font-weight: 500; font-size: var(--fs-sm); }
  form { margin: 0; }
  input:not([type]), select { background: var(--panel); color: var(--fg); border: 1px solid var(--border); border-radius: var(--radius); padding: var(--s1) var(--s2); font: var(--fs-md) var(--sans); }
  input:not([type]) { width: 100%; box-sizing: border-box; font-family: var(--mono); }
  button { background: var(--accent); border: 1px solid var(--accent); color: var(--bg); border-radius: var(--radius); padding: var(--s1) var(--s3); font: 600 var(--fs-md) var(--sans); cursor: pointer; }
  button:disabled { background: var(--bg); border-color: var(--border); color: var(--dim); cursor: default; font-weight: 400; }
  .inactive td:first-child { color: var(--dim); }
  .dim { color: var(--dim); font-size: var(--fs-md); }
  .error { color: var(--todo); }
</style>
