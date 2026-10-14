<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { cards, merge, ROUNDINGS, toProject, type Card, type Project, type Row, type Task, type Tracked } from "$lib/projects";
  import { hm } from "$lib/clock";
  import type { TimeApi } from "$lib/Time.svelte";

  /** Project cards (this week's and month's hours, open tasks, Track), then every project's settings at once, saved per
   *  row. Reloads on RELOAD, keeping unsaved rows. */
  let { api, active, reload }: { api: TimeApi; active: boolean; reload: number } = $props();
  const act = (f: () => unknown) => api.act(f);
  let rows = $state<Row[]>([]);
  let list = $state<Card[]>([]);
  let running = $state<string | null>(null); // the project being tracked ("Other" for none), null when not tracking
  let off = $state<{ kind: string; code: string; dirty: boolean }[]>([]);
  let error = $state("");

  async function load() {
    try {
      const projects = await invoke<Record<string, Project>>("tc_projects");
      rows = merge(rows, projects);
      const d = await invoke<{ projects: Tracked[]; project: string | null }>("tc_dashboard");
      list = cards(projects, d.projects, await invoke<Task[]>("todos"));
      running = d.project == null ? null : d.project || "Other";
      const codes = await invoke<Record<string, { export_code: string }>>("tc_leave_codes");
      off = Object.entries(codes).map(([kind, c]) => off.find((o) => o.kind === kind && o.dirty) ?? { kind, code: c.export_code, dirty: false });
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
  const saveOff = (o: { kind: string; code: string; dirty: boolean }) => act(async () => {
    await invoke("tc_save_leave_code", { kind: o.kind, code: o.code });
    o.dirty = false;
  });
  const run = (name: string, arg?: unknown) => act(async () => { await api.run(name, arg); await load(); });
  const week = $derived(list.reduce((s, c) => [s[0] + c.worked, s[1] + c.billable], [0, 0]));
  const base = (p: string) => p.split(/[\\/]/).pop();
  const options =(r: Row) => (ROUNDINGS.some(([v]) => v === r.rounding) ? ROUNDINGS : [...ROUNDINGS, [r.rounding, `${r.rounding} h`]]);
</script>

<div class="projects">
  <h1>Projects</h1>
  {#if !error && list.length}
    <p class="sub dim">This week · {hm(week[0])} tracked · {week[1].toFixed(1)} h billable</p>
    <div class="cards">
      {#each list as c (c.project)}
        {@const on = running === c.project}
        <section class="card pcard" class:on>
          <h3>{c.project} <span class="code mono">{c.code}</span></h3>
          <strong class="big">{hm(c.worked)}</strong>
          <small>this week · {hm(c.month)} this month · {c.billable.toFixed(2)} h billable</small>
          <div class="progress" title="Share of this week's tracked time"><i style:width="{c.share * 100}%"></i></div>
          <div class="foot">
            <small>{c.tasks.length} open task{c.tasks.length === 1 ? "" : "s"}</small>
            {#if c.tasks.length}<button class="link" onclick={() => run("open", { path: c.tasks[0].path })}>{base(c.tasks[0].path)}</button>{/if}
            <span class="grow"></span>
            {#if on}<small>Tracking now</small>{:else}<button onclick={() => run("start", c.project === "Other" ? undefined : c.project)}>Track</button>{/if}
          </div>
          {#if c.tasks.length}
            <ul>
              {#each c.tasks.slice(0, 3) as t (t.path + t.line)}
                <li><button class="task" onclick={() => run("open", t)}>{#if t.keyword}<b>{t.keyword}</b> {/if}{t.title}</button></li>
              {/each}
            </ul>
          {/if}
        </section>
      {/each}
    </div>
    <h2>Settings</h2>
  {/if}
  {#if error}
    <p class="error">⚠ {error}. Fix the file (Time → Check log), then come back.</p>
  {:else if !rows.length}
    <p class="dim">No projects yet: they're added the first time you track time on one.</p>
  {:else}
    <div class="card form"><table>
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
    </table></div>
    <p class="dim">Enter in an export code saves its row. Hidden projects keep their time; they just aren't offered when you start tracking.</p>
  {/if}
  {#if !error}
    <h2>Time off</h2>
    <div class="card form"><table>
      <thead><tr><th>Kind</th><th>Export code</th><th></th></tr></thead>
      <tbody>
        {#each off as o, i (o.kind)}
          <tr>
            <td>{o.kind}</td>
            <td><form id="o{i}" onsubmit={(e) => { e.preventDefault(); saveOff(o); }}><input aria-label="Export code for time off {o.kind}" bind:value={o.code} oninput={() => (o.dirty = true)} placeholder={o.kind} /></form></td>
            <td><button form="o{i}" type="submit" disabled={!o.dirty}>Save</button></td>
          </tr>
        {/each}
      </tbody>
    </table></div>
    <p class="dim">Time off goes into the CSV export as its own rows under this code, kept apart from the time log.</p>
  {/if}
</div>

<style>
  .projects { height: 100%; overflow-y: auto; background: var(--canvas); padding: 20px 28px 40px; box-sizing: border-box; font-family: var(--sans); }
  h1 { margin: 0 0 18px; font-size: 22px; font-weight: 650; }
  h2 { margin: 28px 0 12px; font-size: 17px; font-weight: 650; }
  .form:hover { transform: none; box-shadow: var(--soft); } /* inputs inside: no hover lift */
  .form { padding: var(--s2) var(--s4); }
  tr:last-child td { border-bottom: 0; }
  table { width: 100%; border-collapse: collapse; font-size: var(--fs-base); }
  td, th { padding: 6px var(--s2); border-bottom: 1px solid var(--border); text-align: left; }
  th { color: var(--dim); font-weight: 500; font-size: var(--fs-sm); }
  form { margin: 0; }
  input:not([type]), select { background: var(--canvas); color: var(--fg); border: 1px solid var(--border); border-radius: var(--radius); padding: var(--s1) var(--s2); font: var(--fs-md) var(--sans); }
  input:not([type]) { width: 100%; box-sizing: border-box; font-family: var(--mono); }
  button { background: var(--accent); border: 1px solid var(--accent); color: var(--bg); border-radius: var(--radius); padding: var(--s1) var(--s3); font: 600 var(--fs-md) var(--sans); cursor: pointer; }
  button:disabled { background: var(--canvas); border-color: var(--border); color: var(--dim); cursor: default; font-weight: 400; }
  .inactive td:first-child { color: var(--dim); }
  .dim { color: var(--dim); font-size: var(--fs-md); }
  .error { color: var(--todo); }
  .sub { margin: -12px 0 18px; }
  /* The Time view's project cards (#373), with the open tasks under them (round-5 mockup, #357) */
  .cards { display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: var(--s4); align-items: start; }
  .pcard { padding: var(--s4); }
  .pcard.on { box-shadow: inset 3px 0 0 var(--done), var(--soft); }
  h3 { font-size: var(--fs-md); font-weight: 600; margin: 0 0 var(--s3); display: flex; flex-wrap: wrap; gap: 0 10px; align-items: baseline; }
  .code { font-size: var(--fs-xs); color: var(--dim); background: var(--active); border-radius: var(--radius-sm); padding: 0 var(--s1); }
  .mono { font-family: var(--mono); }
  .big { display: block; font: 500 28px var(--mono); }
  .pcard small { color: var(--dim); font-size: var(--fs-sm); }
  .progress { height: 8px; border-radius: 999px; background: var(--active); overflow: hidden; margin: var(--s2) 0; }
  .progress i { display: block; height: 100%; background: var(--accent); border-radius: inherit; }
  .foot { display: flex; gap: var(--s2); align-items: center; min-height: 1.6em; }
  .grow { flex: 1; }
  .pcard button { background: var(--active); border: 1px solid transparent; color: var(--fg); border-radius: 8px; padding: 4px var(--s3); font: 400 var(--fs-md) var(--sans); }
  .pcard button:hover { border-color: var(--border); }
  .pcard button.link, .pcard button.task { background: none; border: 0; padding: 0; text-align: left; }
  .pcard button.link { color: var(--accent); text-decoration: underline; font-size: var(--fs-sm); }
  .pcard ul { list-style: none; margin: var(--s3) 0 0; padding: 0; border-top: 1px solid var(--border); }
  .pcard li { padding: 6px 0; border-bottom: 1px solid var(--border); }
  .pcard li:last-child { border-bottom: 0; padding-bottom: 0; }
  .task b { font: 600 var(--fs-xs) var(--mono); color: var(--todo); }
</style>
