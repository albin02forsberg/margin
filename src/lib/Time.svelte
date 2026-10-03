<script lang="ts" module>
  export type Suggestion = { start: string; end: string; apps: string[]; titles: string[]; project: string | null };
  export type Session = { date: string; project: string; desc: string; hours: number; start: string; end: string; line: number | null };
  export type TimeApi = {
    act: (f: () => unknown) => void;
    /** Named actions: start, stop, pause, resume, switch, adjust, editSession, export, exportReport, projects,
     *  profile, holidays, daily, weekly, doctor, backup, backupLog, rawLog, import, acceptSuggestion, editSuggestion. */
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
  // ActivityWatch suggestions for DAY: null when off; errors show as a hint, not a flash.
  let day = $state(new Date().toLocaleDateString("sv-SE"));
  let sugg = $state<Suggestion[] | null>(null);
  let awError = $state("");
  async function loadSuggestions() {
    try {
      [sugg, awError] = [await invoke("activity_suggestions", { dateInput: day }), ""];
    } catch (e) {
      [sugg, awError] = [[], String(e)];
    }
  }
  $effect(() => {
    void [reload, day];
    if (active) loadSuggestions();
  });
  const shiftDay = (n: number) => {
    const t = new Date(+day.slice(0, 4), +day.slice(5, 7) - 1, +day.slice(8, 10) + n);
    day = t.toLocaleDateString("sv-SE");
  };
  const accept = (s: Suggestion, name = "acceptSuggestion") => api.act(async () => { await api.run(name, s); await refresh(); await loadSuggestions(); root?.focus(); });
  // Hidden at once; the backend keeps it hidden (by overlap) across restarts.
  const dismiss = (s: Suggestion) => api.act(async () => { sugg = sugg!.filter((x) => x !== s); await invoke("activity_dismiss", { start: s.start, end: s.end }); });

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

  const KEYS: Record<string, string> = { i: "start", o: "stop", p: "pause", r: "resume", c: "switch", a: "adjust", e: "export", E: "exportReport", t: "daily", w: "weekly" };
  function key(e: KeyboardEvent) {
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    if (sugg && (e.key === "h" || e.key === "l")) return shiftDay(e.key === "h" ? -1 : 1);
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

    {#if sugg}
      <section class="suggested">
        <h3>Suggested <span class="dim">from ActivityWatch · {day === todayIso ? "today" : `${dayName(day)} ${day}`} · <kbd>h</kbd><kbd>l</kbd> day</span></h3>
        {#if awError}
          <p class="dim">{awError}. Is <a href="https://activitywatch.net/" target="_blank" rel="noreferrer">ActivityWatch</a> running?</p>
        {:else if sugg.length}
          <table>
            <tbody>
              {#each sugg as s (s.start)}
                <tr>
                  <td class="mono">{clock(s.start)}–{clock(s.end)}</td>
                  <td>{s.apps.join(", ")} <span class="dim">{s.titles.map((t) => `“${t}”`).join(" ")}</span>{#if s.project} → <strong>{s.project}</strong>?{/if}</td>
                  <td class="num">{hm((Date.parse(s.end) - Date.parse(s.start)) / 3_600_000)}</td>
                  <td class="actions"><button onclick={() => accept(s)}>Accept</button><button class="icon" title="Change start and end, then accept" onclick={() => accept(s, "editSuggestion")}>✎</button><button class="icon" title="Dismiss" onclick={() => dismiss(s)}>✕</button></td>
                </tr>
              {/each}
            </tbody>
          </table>
        {:else}
          <p class="dim">Nothing to suggest.</p>
        {/if}
      </section>
    {/if}

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
        <button onclick={() => run("exportReport")}><kbd>E</kbd>Export report (HTML/PDF)…</button>
        <button onclick={() => run("projects")}>Project settings</button>
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
  .profile { color: var(--dim); font-size: var(--fs-md); display: flex; gap: var(--s2); align-items: center; }
  select { background: var(--panel); color: var(--fg); border: 1px solid var(--border); border-radius: var(--radius); padding: var(--s1) var(--s2); font: var(--fs-md) var(--sans); }
  section { margin-top: 22px; }
  h3 { font-size: var(--fs-md); font-weight: 650; margin: 0 0 10px; color: var(--h2); display: flex; gap: 10px; align-items: baseline; }
  .status { display: grid; grid-template-columns: 1fr auto; gap: 14px; background: var(--panel); border: 1px solid var(--border); border-radius: 12px; padding: 18px 20px; }
  .state { display: flex; gap: var(--s3); align-items: flex-start; }
  .state small, .total small, .flex small { color: var(--dim); font-size: var(--fs-sm); display: block; }
  .state h2 { margin: 2px 0; font-size: 20px; }
  .state p { margin: 0; color: var(--dim); }
  .dot { width: 10px; height: 10px; border-radius: 50%; background: var(--done); margin-top: 6px; animation: pulse 2s infinite; flex: none; }
  .paused .dot { background: var(--h5); animation: none; }
  @keyframes pulse { 50% { opacity: 0.35; } }
  .total { text-align: right; }
  .total strong { font: 600 28px var(--mono); }
  .buttons { grid-column: 1 / -1; display: flex; flex-wrap: wrap; gap: var(--s2); }
  button { background: var(--bg); border: 1px solid var(--border); color: var(--fg); border-radius: var(--radius); padding: 6px var(--s3); font: var(--fs-md) var(--sans); cursor: pointer; }
  button:hover { border-color: var(--dim); }
  button.primary { background: var(--accent); border-color: var(--accent); color: var(--bg); font-weight: 600; }
  button.primary kbd { background: rgb(0 0 0 / 0.15); color: inherit; border-color: transparent; }
  .icon { padding: 2px var(--s2); border: 0; background: none; color: var(--dim); }
  kbd { font: var(--fs-xs) var(--mono); background: var(--active); border: 1px solid var(--border); border-radius: var(--radius-sm); padding: 0 var(--s1); margin-right: 6px; }
  .cols { display: grid; grid-template-columns: repeat(auto-fit, minmax(320px, 1fr)); gap: 0 var(--s6); }
  table { width: 100%; border-collapse: collapse; font-size: var(--fs-base); }
  td, th { padding: 6px var(--s2); border-bottom: 1px solid var(--border); text-align: left; }
  th { color: var(--dim); font-weight: 500; font-size: var(--fs-sm); }
  .num { text-align: right; font-family: var(--mono); white-space: nowrap; }
  .mono { font-family: var(--mono); font-size: var(--fs-md); white-space: nowrap; }
  .dim { color: var(--dim); font-weight: 400; }
  .chart { display: grid; grid-template-columns: repeat(7, 1fr); gap: var(--s2); height: 150px; }
  .col { display: flex; flex-direction: column; align-items: center; gap: var(--s1); min-width: 0; }
  .plot { flex: 1; width: 100%; position: relative; border-bottom: 1px solid var(--border); }
  .bar { position: absolute; bottom: 0; left: 50%; transform: translateX(-50%); width: min(26px, 70%); background: var(--link); border-radius: var(--radius-sm) 4px 0 0; min-height: 0; }
  .col:hover .bar { filter: brightness(1.15); }
  .expected { position: absolute; left: 8%; right: 8%; border-top: 1.5px dashed var(--dim); }
  .val { font: var(--fs-xs) var(--mono); color: var(--fg); white-space: nowrap; }
  .day { font-size: var(--fs-xs); color: var(--dim); }
  .col.today .day { color: var(--accent); font-weight: 700; }
  .legend { color: var(--dim); font-size: var(--fs-xs); margin: 6px 0 0; display: flex; align-items: center; gap: 6px; }
  .dash { width: 16px; border-top: 1.5px dashed var(--dim); }
  .flex { display: flex; gap: 28px; margin-top: 14px; }
  .flex strong { font: 600 18px var(--mono); }
  .flex strong.neg { color: var(--todo); }
  .tools div { display: flex; flex-wrap: wrap; gap: var(--s2); }
  .suggested td:nth-child(2) { overflow-wrap: anywhere; }
  .actions { white-space: nowrap; text-align: right; }
  .error { color: var(--todo); font-size: var(--fs-md); }
  @media (max-width: 759px) { .status { grid-template-columns: 1fr; } .total { text-align: left; } }
</style>
