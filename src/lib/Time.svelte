<script lang="ts" module>
  export type Session = { date: string; project: string; desc: string; hours: number; start: string; end: string; line: number | null };
  export type TimeApi = {
    act: (f: () => unknown) => void;
    /** Named actions: start, stop, pause, resume, switch, adjust, editSession, export, projects,
     *  profile, holidays, daily, weekly, doctor, backup, backupLog, rawLog, import. */
    run: (name: string, arg?: unknown) => Promise<void>;
  };
  export const hm = (h: number) => {
    const m = Math.round(h * 60);
    return m >= 60 ? `${Math.floor(m / 60)}h ${String(m % 60).padStart(2, "0")}m` : `${m}m`;
  };
</script>

<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { tick } from "svelte";

  let { api, active, reload }: { api: TimeApi; active: boolean; reload: number } = $props();
  let d = $state<any>(null);
  let root = $state<HTMLDivElement>();

  async function refresh() {
    d = await invoke("tc_dashboard");
  }
  $effect(() => {
    void reload;
    if (!active) return;
    api.act(refresh);
    const t = setInterval(() => api.act(refresh), 30_000);
    return () => clearInterval(t);
  });
  $effect(() => {
    if (active) tick().then(() => root?.focus());
  });

  const run = (name: string, arg?: unknown) => api.act(async () => { await api.run(name, arg); await refresh(); root?.focus(); });
  const clock = (iso: string) => iso?.slice(11, 16);
  const dayName = (s: string) => new Date(+s.slice(0, 4), +s.slice(5, 7) - 1, +s.slice(8, 10)).toLocaleDateString(undefined, { weekday: "short" });
  const max = $derived(d ? Math.max(1, ...d.week.map((w: any) => Math.max(w.hours, w.expected))) : 1);
  const weekTotal = $derived(d ? d.week.reduce((s: number, w: any) => s + w.hours, 0) : 0);
  const weekExpected = $derived(d ? d.week.reduce((s: number, w: any) => s + w.expected, 0) : 0);
  const todayIso = $derived(new Date().toLocaleDateString("sv-SE"));
  const signed = (h: number) => `${h >= 0 ? "+" : "−"}${hm(Math.abs(h))}`;

  const KEYS: Record<string, string> = { i: "start", o: "stop", p: "pause", r: "resume", c: "switch", a: "adjust", e: "export", t: "daily", w: "weekly" };
  function key(e: KeyboardEvent) {
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    const name = KEYS[e.key];
    if (!name) return;
    e.preventDefault();
    run(name);
  }
</script>

<div class="time" bind:this={root} tabindex="0" onkeydown={key} role="toolbar" aria-label="Time tracking">
  {#if d}
    <header>
      <h1>Time</h1>
      <label class="profile">
        Profile
        <select value={d.profile} onchange={(e) => run("profile", (e.target as HTMLSelectElement).value)}>
          {#each d.profiles as p}<option>{p}</option>{/each}
        </select>
      </label>
    </header>

    <section class="status" class:on={d.project != null} class:paused={d.on_break != null}>
      <div class="state">
        {#if d.project != null}
          <span class="dot"></span>
          <div>
            <small>Tracking since {clock(d.since)}</small>
            <h2>{d.project || "No project"}</h2>
            {#if d.task}<p>{d.task}</p>{/if}
          </div>
        {:else if d.on_break != null}
          <span class="dot"></span>
          <div><small>On a break</small><h2>{d.on_break}</h2></div>
        {:else}
          <div><small>Not tracking</small><h2>Ready when you are</h2></div>
        {/if}
      </div>
      <div class="total"><strong>{hm(d.today_hours)}</strong><small>today</small></div>
      <div class="buttons">
        {#if d.project != null}
          <button onclick={() => run("pause")}><kbd>p</kbd>Pause</button>
          <button onclick={() => run("switch")}><kbd>c</kbd>Switch project</button>
          <button onclick={() => run("adjust")}><kbd>a</kbd>Started earlier…</button>
          <button class="primary" onclick={() => run("stop")}><kbd>o</kbd>Stop</button>
        {:else if d.on_break != null}
          <button onclick={() => run("start")}><kbd>i</kbd>Start something else</button>
          <button class="primary" onclick={() => run("resume")}><kbd>r</kbd>Resume</button>
        {:else}
          <button class="primary" onclick={() => run("start")}><kbd>i</kbd>Start tracking</button>
        {/if}
      </div>
    </section>

    <div class="cols">
      <section>
        <h3>Today</h3>
        {#if d.today.length}
          <table>
            <tbody>
              {#each d.today as s}
                <tr>
                  <td class="mono">{s.start.slice(0, 5)}–{s.end.slice(0, 5)}</td>
                  <td><strong>{s.project || "Other"}</strong> <span class="dim">{s.desc}</span></td>
                  <td class="num">{hm(s.hours)}</td>
                  <td>{#if s.line != null}<button class="icon" title="Edit session" onclick={() => run("editSession", s)}>✎</button>{/if}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        {:else}
          <p class="dim">No finished sessions today yet.</p>
        {/if}
      </section>

      <section>
        <h3>This week <span class="dim">{hm(weekTotal)} of {hm(weekExpected)}</span></h3>
        <div class="chart" role="img" aria-label="Hours worked per day this week">
          {#each d.week as w}
            <div class="col" class:today={w.date === todayIso} title="{dayName(w.date)} {w.date}: {hm(w.hours)} worked, {hm(w.expected)} expected">
              <div class="plot">
                {#if w.expected > 0}<div class="expected" style:bottom="{(w.expected / max) * 100}%"></div>{/if}
                <div class="bar" style:height="{(w.hours / max) * 100}%"></div>
              </div>
              <span class="val">{w.hours ? hm(w.hours) : "–"}</span>
              <span class="day">{dayName(w.date)}</span>
            </div>
          {/each}
        </div>
        <p class="legend"><span class="dash"></span> expected hours</p>
        <div class="flex">
          <div><small>Flex balance</small><strong class:neg={d.flex_total < 0}>{signed(d.flex_total)}</strong></div>
          <div><small>This week</small><strong class:neg={d.flex_week < 0}>{signed(d.flex_week)}</strong></div>
        </div>
      </section>
    </div>

    <section>
      <h3>Projects this week</h3>
      {#if d.projects.length}
        <table class="projects">
          <thead><tr><th>Project</th><th>Code</th><th class="num">Worked</th><th class="num">Billable</th></tr></thead>
          <tbody>
            {#each d.projects as p}
              <tr><td>{p.project}</td><td class="dim mono">{p.code}</td><td class="num">{hm(p.worked)}</td><td class="num">{p.billable.toFixed(2)} h</td></tr>
            {/each}
          </tbody>
        </table>
      {:else}
        <p class="dim">Nothing tracked this week.</p>
      {/if}
    </section>

    <section class="tools">
      <h3>More</h3>
      <div>
        <button onclick={() => run("daily")}><kbd>t</kbd>Day report</button>
        <button onclick={() => run("weekly")}><kbd>w</kbd>Week report</button>
        <button onclick={() => run("export")}><kbd>e</kbd>Export CSV…</button>
        <button onclick={() => run("projects")}>Project settings…</button>
        <button onclick={() => run("holidays")}>Public holidays</button>
        <button onclick={() => run("doctor")}>Check log for problems</button>
        <button onclick={() => run("rawLog")}>Edit raw log</button>
        <button onclick={() => run("backup")}>Back up now</button>
        <button onclick={() => run("backupLog")}>Backup log</button>
        <button onclick={() => run("import")}>Import from Emacs…</button>
      </div>
      {#if d.backup_error}<p class="error">⚠ Last backup failed: {d.backup_error}</p>{/if}
    </section>
  {/if}
</div>

<style>
  .time { height: 100%; overflow-y: auto; padding: 20px 28px 40px; box-sizing: border-box; outline: none; font-family: var(--sans); }
  header { display: flex; justify-content: space-between; align-items: center; }
  h1 { margin: 0; font-size: 22px; font-weight: 650; }
  .profile { color: var(--dim); font-size: 13px; display: flex; gap: 8px; align-items: center; }
  select { background: var(--panel); color: var(--fg); border: 1px solid var(--border); border-radius: 6px; padding: 4px 8px; font: 13px var(--sans); }
  section { margin-top: 22px; }
  h3 { font-size: 13px; font-weight: 650; margin: 0 0 10px; color: var(--h2); display: flex; gap: 10px; align-items: baseline; }
  .status { display: grid; grid-template-columns: 1fr auto; gap: 14px; background: var(--panel); border: 1px solid var(--border); border-radius: 12px; padding: 18px 20px; }
  .state { display: flex; gap: 12px; align-items: flex-start; }
  .state small, .total small, .flex small { color: var(--dim); font-size: 12px; display: block; }
  .state h2 { margin: 2px 0; font-size: 20px; }
  .state p { margin: 0; color: var(--dim); }
  .dot { width: 10px; height: 10px; border-radius: 50%; background: var(--done); margin-top: 6px; animation: pulse 2s infinite; flex: none; }
  .paused .dot { background: var(--h5); animation: none; }
  @keyframes pulse { 50% { opacity: 0.35; } }
  .total { text-align: right; }
  .total strong { font: 600 28px var(--mono); }
  .buttons { grid-column: 1 / -1; display: flex; flex-wrap: wrap; gap: 8px; }
  button { background: var(--bg); border: 1px solid var(--border); color: var(--fg); border-radius: 6px; padding: 6px 12px; font: 13px var(--sans); cursor: pointer; }
  button:hover { border-color: var(--dim); }
  button.primary { background: var(--accent); border-color: var(--accent); color: var(--bg); font-weight: 600; }
  button.primary kbd { background: rgb(0 0 0 / 0.15); color: inherit; border-color: transparent; }
  .icon { padding: 2px 8px; border: 0; background: none; color: var(--dim); }
  kbd { font: 11px var(--mono); background: var(--active); border: 1px solid var(--border); border-radius: 4px; padding: 0 4px; margin-right: 6px; }
  .cols { display: grid; grid-template-columns: repeat(auto-fit, minmax(320px, 1fr)); gap: 0 32px; }
  table { width: 100%; border-collapse: collapse; font-size: 14px; }
  td, th { padding: 6px 8px; border-bottom: 1px solid var(--border); text-align: left; }
  th { color: var(--dim); font-weight: 500; font-size: 12px; }
  .num { text-align: right; font-family: var(--mono); white-space: nowrap; }
  .mono { font-family: var(--mono); font-size: 13px; white-space: nowrap; }
  .dim { color: var(--dim); font-weight: 400; }
  .chart { display: grid; grid-template-columns: repeat(7, 1fr); gap: 8px; height: 150px; }
  .col { display: flex; flex-direction: column; align-items: center; gap: 4px; min-width: 0; }
  .plot { flex: 1; width: 100%; position: relative; border-bottom: 1px solid var(--border); }
  .bar { position: absolute; bottom: 0; left: 50%; transform: translateX(-50%); width: min(26px, 70%); background: var(--link); border-radius: 4px 4px 0 0; min-height: 0; }
  .col:hover .bar { filter: brightness(1.15); }
  .expected { position: absolute; left: 8%; right: 8%; border-top: 1.5px dashed var(--dim); }
  .val { font: 11px var(--mono); color: var(--fg); white-space: nowrap; }
  .day { font-size: 11px; color: var(--dim); }
  .col.today .day { color: var(--accent); font-weight: 700; }
  .legend { color: var(--dim); font-size: 11px; margin: 6px 0 0; display: flex; align-items: center; gap: 6px; }
  .dash { width: 16px; border-top: 1.5px dashed var(--dim); }
  .flex { display: flex; gap: 28px; margin-top: 14px; }
  .flex strong { font: 600 18px var(--mono); }
  .flex strong.neg { color: var(--todo); }
  .tools div { display: flex; flex-wrap: wrap; gap: 8px; }
  .error { color: var(--todo); font-size: 13px; }
</style>
