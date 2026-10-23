<script lang="ts" module>
  export type Suggestion = { start: string; end: string; apps: string[]; titles: string[]; project: string | null; task: string | null; meeting: string | null; editor: boolean };
  export type Session = { date: string; project: string; desc: string; hours: number; start: string; end: string; line: number | null };
  export type TimeApi = {
    act: (f: () => unknown) => void;
    /** Named actions: start, stop, pause, resume, switch, adjust, editSession, export, exportReport, projects, timeOff,
     *  open ({path, line?}), profile, holidays, daily, weekly, doctor, backup, backupLog, rawLog, import, acceptSuggestion, editSuggestion, logSession, estimates. */
    run: (name: string, arg?: unknown) => Promise<void>;
  };
  import { hm } from "./clock";
  export { hm };
</script>

<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { tick } from "svelte";
  import WeekBars from "./WeekBars.svelte";
  import BudgetBar from "./BudgetBar.svelte";
  import { worked } from "./clock";

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
  // Suggestions for DAY from ActivityWatch or the built-in watcher (SOURCE): null when off; errors show as a hint, not a flash.
  let day = $state(new Date().toLocaleDateString("sv-SE"));
  let sugg = $state<Suggestion[] | null>(null);
  let source = $state("ActivityWatch");
  let awError = $state("");
  let warnings = $state<string[]>([]);
  async function loadSuggestions() {
    try {
      const r = await invoke<{ source: string; items: Suggestion[]; warnings: string[] } | null>("activity_suggestions", { dateInput: day });
      [sugg, source, awError, warnings] = [r?.items ?? null, r?.source ?? source, "", r?.warnings ?? []];
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
  const accept = (s: Suggestion, name = "acceptSuggestion") => api.act(async () => { await api.run(name, s); await refresh(); await loadSuggestions(); refocus(); });
  // Hidden at once; the backend keeps it hidden (by overlap) across restarts.
  const dismiss = (s: Suggestion) => api.act(async () => { sugg = sugg!.filter((x) => x !== s); await invoke("activity_dismiss", { start: s.start, end: s.end }); });

  $effect(() => {
    if (active) tick().then(() => root?.focus());
  });

  /** Back to the view after a flow, unless focus moved on (a picker the next key opened would blur and cancel). */
  const refocus = () => { const a = document.activeElement; if (!a || a === document.body || root?.contains(a)) root?.focus(); };
  const run = (name: string, arg?: unknown) => api.act(async () => { await api.run(name, arg); refocus(); await refresh(); refocus(); });
  const clock = (iso: string) => iso?.slice(11, 16);
  const dayName = (s: string) => new Date(+s.slice(0, 4), +s.slice(5, 7) - 1, +s.slice(8, 10)).toLocaleDateString(undefined, { weekday: "short" });
  const weekTotal = $derived(d ? d.week.reduce((s: number, w: any) => s + w.hours, 0) : 0);
  const weekExpected = $derived(d ? d.week.reduce((s: number, w: any) => s + w.expected, 0) : 0);
  const todayIso = $derived(new Date().toLocaleDateString("sv-SE"));
  const work = $derived(d ? worked(d.week, todayIso) : null);
  const projTotal = $derived(d ? d.projects.reduce((s: number, p: any) => s + p.worked, 0) : 0);
  const signed = (h: number) => `${h >= 0 ? "+" : "−"}${hm(Math.abs(h))}`;

  // Time off (logged through the timeOff flow), newest first.
  const allOff = $derived(d ? [...d.leave].sort((a: any, b: any) => (a[1].from < b[1].from ? 1 : -1)) : []);
  const removeOff = (line: number) => api.act(async () => { await invoke("tc_leave_remove", { line }); await refresh(); });
  const offHours = (l: any) => (l.rest ? "rest of day" : l.hours ? `${hm(l.hours)}/day` : "full day");

  const KEYS: Record<string, string> = { i: "start", o: "stop", p: "pause", r: "resume", c: "switch", a: "adjust", n: "logSession", e: "export", E: "exportReport", O: "stopEarlier", S: "editSession", L: "timeOff", t: "daily", w: "weekly" };
  function key(e: KeyboardEvent) {
    if (e.ctrlKey || e.metaKey || e.altKey || (e.target as HTMLElement).closest("input, select")) return;
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

    <div class="cols">
      <div class="lane">
        <h2>Clock</h2>
        <section class="card status" class:on={d.project != null} class:paused={d.on_break != null}>
          <div class="state">
            {#if d.project != null}
              <span class="dot"></span>
              <div>
                <small>Tracking since {clock(d.since)}</small>
                <h2>{d.project || "No project"}</h2>
                {#if d.task}<p title={d.task}>{d.task}</p>{/if}
              </div>
            {:else if d.on_break != null}
              <span class="dot"></span>
              <div><small>On a break</small><h2>{d.on_break}</h2></div>
            {:else}
              <div><small>Not tracking</small><h2>Ready when you are</h2></div>
            {/if}
          </div>
          <div class="total"><strong>{hm(d.today_hours)}</strong><small>today</small><small class="leave">{d.leave_time}</small></div>
          <div class="buttons">
            {#if d.project != null}
              <button onclick={() => run("pause")}><kbd>p</kbd>Pause</button>
              <button onclick={() => run("switch")}><kbd>c</kbd>Switch project</button>
              <button onclick={() => run("adjust")}><kbd>a</kbd>Started earlier…</button>
              <button class="primary" onclick={() => run("stop")}><kbd>o</kbd>Stop</button>
              <button onclick={() => run("stopEarlier")}><kbd>O</kbd>Stopped earlier…</button>
            {:else if d.on_break != null}
              <button onclick={() => run("start")}><kbd>i</kbd>Start something else</button>
              <button class="primary" onclick={() => run("resume")}><kbd>r</kbd>Resume</button>
            {:else}
              <button class="primary" onclick={() => run("start")}><kbd>i</kbd>Start tracking</button>
            {/if}
            <button onclick={() => run("logSession")}><kbd>n</kbd>Log a past session…</button>
          </div>
        </section>
        {#if work}
          <section class="card worked">
            <h3>Worked today</h3>
            <strong class="big">{hm(work.hours)}</strong>
            <div class="progress" role="progressbar" aria-label="Worked today" aria-valuenow={Math.round(work.progress * 100)}><i style:width="{work.progress * 100}%"></i></div>
            <small>{work.expected > 0 ? `of ${hm(work.expected)} · ${work.left > 0 ? `${hm(work.left)} to go` : "done for today"}` : "no expected hours today"}</small>
          </section>
        {/if}
        <section class="card tools">
          <h3>Balance</h3>
          <div class="flex">
            <div><small>Flex balance</small><strong class:neg={d.flex_total < 0}>{signed(d.flex_total)}</strong></div>
            <div><small>This week</small><strong class:neg={d.flex_week < 0}>{signed(d.flex_week)}</strong></div>
          </div>
          <div>
            <button onclick={() => run("daily")}><kbd>t</kbd>Day report</button>
            <button onclick={() => run("weekly")}><kbd>w</kbd>Week report</button>
            <button onclick={() => run("export")}><kbd>e</kbd>Export CSV…</button>
            <button onclick={() => run("exportReport")}><kbd>E</kbd>Export report (HTML/PDF)…</button>
            <button onclick={() => run("estimates")}>Estimates vs actuals…</button>
          </div>
        </section>
        <section class="card tools">
          <h3>More</h3>
          <div>
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
      </div>

      <div class="lane">
        <h2>Sessions</h2>
        <section class="card">
          <h3>Today <button class="link" onclick={() => run("editSession")}><kbd>S</kbd>Edit a session…</button></h3>
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

        <section class="card">
          <h3>This week <span class="dim">{hm(weekTotal)} of {hm(weekExpected)}</span></h3>
          <WeekBars week={d.week} />
        </section>

        {#if sugg}
          <section class="card suggested">
            <h3>Suggested <span class="dim">from {source === "Margin" ? "this computer's activity" : source} · {day === todayIso ? "today" : `${dayName(day)} ${day}`} · <kbd>h</kbd><kbd>l</kbd> day</span></h3>
            {#each warnings as w}
              <p class="dim">{w}</p>
            {/each}
            {#if awError}
              <p class="dim">{awError}.{#if /activitywatch/i.test(awError)} Is <a href="https://activitywatch.net/" target="_blank" rel="noreferrer">ActivityWatch</a> running?{/if}</p>
            {:else if sugg.length}
              <table>
                <tbody>
                  {#each sugg as s (s.start)}
                    <tr>
                      <td class="mono">{clock(s.start)}–{clock(s.end)}</td>
                      <td>{#if s.meeting}<span class="badge">Meeting</span> {/if}{s.apps.join(", ")} <span class="dim">{s.titles.map((t) => `“${t}”`).join(" ")}</span>{#if s.project || s.task} → <strong>{[s.project, s.task].filter(Boolean).join(" / ")}</strong>?{/if}</td>
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
      </div>

      <div class="lane">
        <h2>Projects</h2>
        {#each d.projects.filter((p: any) => p.worked > 0) as p (p.project)}
          {@const on = d.project != null && (d.project || "Other") === p.project}
          <section class="card pcard" class:on>
            <h3>{p.project} <span class="code mono">{p.code}</span></h3>
            <strong class="big">{hm(p.worked)}</strong>
            <small>this week · {hm(p.month)} this month · {p.billable.toFixed(2)} h billable</small>
            <div class="progress" title="Share of this week's tracked time"><i style:width="{projTotal > 0 ? (p.worked / projTotal) * 100 : 0}%"></i></div>
            <BudgetBar project={p} worked={p.worked} month={p.month} />
            <div class="foot">
              {#if on}<small>Tracking now</small>{:else}<button onclick={() => run("start", p.project === "Other" ? undefined : p.project)}>Track</button>{/if}
            </div>
          </section>
        {:else}
          <section class="card"><p class="dim">Nothing tracked this week.</p></section>
        {/each}

        <section class="card timeoff">
          <h3>Time off <span class="dim">lowers the day's expected hours; work on those days is flex</span></h3>
          <div><button onclick={() => run("timeOff")}><kbd>L</kbd>Log time off…</button></div>
          {#if allOff.length}
            <table>
              <tbody>
                {#each allOff as [line, l] (line)}
                  <tr>
                    <td><strong>{l.kind}</strong> <span class="dim">{l.note ?? ""}</span></td>
                    <td class="mono">{l.from}{l.to !== l.from ? ` – ${l.to}` : ""}</td>
                    <td class="num">{offHours(l)}</td>
                    <td><button class="icon" title="Remove time off" onclick={() => removeOff(line)}>✕</button></td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {/if}
        </section>

      </div>
    </div>
  {/if}
</div>

<style>
  .time { height: 100%; overflow-y: auto; padding: 20px 28px 40px; box-sizing: border-box; outline: none; font-family: var(--sans); background: var(--canvas); }
  header { display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--s4); }
  h1 { margin: 0; font-size: 22px; font-weight: 650; }
  .profile { color: var(--dim); font-size: var(--fs-md); display: flex; gap: var(--s2); align-items: center; }
  select { background: var(--card); color: var(--fg); border: 1px solid var(--border); border-radius: var(--radius); padding: var(--s1) var(--s2); font: var(--fs-md) var(--sans); }
  /* B5 lanes of B2 cards (round-5 mockup, #251) */
  .cols { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: var(--s4); align-items: start; }
  .lane { background: color-mix(in srgb, var(--active) 60%, transparent); border-radius: var(--r); padding: var(--s3); display: flex; flex-direction: column; gap: var(--s3); min-width: 0; }
  .lane > h2 { margin: 0; padding: 0 var(--s1); font-size: var(--fs-md); font-weight: 600; }
  .card { padding: var(--s4); }
  h3 button { margin-left: auto; font-size: var(--fs-sm); }
  h3 { font-size: var(--fs-md); font-weight: 600; margin: 0 0 var(--s3); display: flex; flex-wrap: wrap; gap: 0 10px; align-items: baseline; }
  .status { display: grid; grid-template-columns: minmax(0, 1fr) auto; gap: 14px; }
  .status.on { background: linear-gradient(160deg, color-mix(in srgb, var(--done) 12%, var(--card)), var(--card) 60%); }
  .status.paused { background: linear-gradient(160deg, color-mix(in srgb, var(--h5) 12%, var(--card)), var(--card) 60%); }
  .state { display: flex; gap: var(--s3); align-items: flex-start; min-width: 0; }
  .state small, .total small, .flex small { color: var(--dim); font-size: var(--fs-sm); display: block; }
  .state > div { min-width: 0; }
  .state small { white-space: nowrap; }
  .state h2 { margin: 2px 0; font-size: 20px; overflow-wrap: anywhere; }
  .state p { margin: 0; color: var(--dim); overflow: hidden; display: -webkit-box; -webkit-box-orient: vertical; -webkit-line-clamp: 2; line-clamp: 2; }
  .dot { width: 10px; height: 10px; border-radius: 50%; background: var(--done); box-shadow: 0 0 0 3px color-mix(in srgb, var(--done) 25%, transparent); margin-top: 6px; animation: pulse 2s infinite; flex: none; }
  .paused .dot { background: var(--h5); box-shadow: 0 0 0 3px color-mix(in srgb, var(--h5) 25%, transparent); animation: none; }
  @keyframes pulse { 50% { opacity: 0.35; } }
  .total { text-align: right; max-width: 11em; }
  .total strong { font: 500 28px var(--mono); }
  .buttons { grid-column: 1 / -1; display: flex; flex-wrap: wrap; gap: var(--s2); }
  button { background: var(--active); border: 1px solid transparent; color: var(--fg); border-radius: 8px; padding: 6px var(--s3); font: var(--fs-md) var(--sans); cursor: pointer; transition: background-color 120ms var(--ease), border-color 120ms var(--ease); }
  button:hover { border-color: var(--border); background: color-mix(in srgb, var(--active) 70%, var(--border)); }
  button:focus-visible { outline: none; box-shadow: var(--ring); }
  button.primary { background: var(--accent); border-color: var(--accent); color: var(--bg); font-weight: 600; }
  button.primary kbd { background: rgb(0 0 0 / 0.15); color: inherit; border-color: transparent; }
  .icon { padding: 2px var(--s2); border: 0; background: none; color: var(--dim); }
  .icon:hover { color: var(--fg); }
  kbd { font: var(--fs-xs) var(--mono); color: var(--dim); background: var(--card); border: 1px solid var(--border); border-radius: var(--radius-sm); padding: 0 var(--s1); margin-right: 6px; }
  table { width: 100%; border-collapse: collapse; font-size: var(--fs-base); }
  td { padding: 6px var(--s2); border-bottom: 1px solid var(--border); text-align: left; }
  td:first-child { padding-left: 0; }
  td:last-child { padding-right: 0; }
  .num { text-align: right; font-family: var(--mono); white-space: nowrap; }
  .mono { font-family: var(--mono); font-size: var(--fs-md); white-space: nowrap; }
  .dim { color: var(--dim); font-weight: 400; }
  .timeoff > div { margin-bottom: var(--s2); }
  .flex { display: grid; grid-template-columns: 1fr 1fr; gap: var(--s2); }
  .flex > div { background: var(--canvas); border-radius: 8px; padding: var(--s2) var(--s3); }
  .flex strong { font: 500 18px var(--mono); }
  .flex strong.neg { color: var(--todo); }
  .tools > div:not(.flex) { display: flex; flex-wrap: wrap; gap: var(--s2); }
  .tools .flex { margin-bottom: var(--s3); }
  .worked small, .pcard small { color: var(--dim); font-size: var(--fs-sm); }
  .big { display: block; font: 500 28px var(--mono); }
  .progress { height: 8px; border-radius: 999px; background: var(--active); overflow: hidden; margin: var(--s2) 0; }
  .progress i { display: block; height: 100%; background: var(--done); border-radius: inherit; }
  .pcard .progress i { background: var(--accent); }
  .pcard.on { box-shadow: inset 3px 0 0 var(--done); }
  .code { font-size: var(--fs-xs); color: var(--dim); background: var(--active); border-radius: var(--radius-sm); padding: 0 var(--s1); }
  .foot { display: flex; justify-content: flex-end; min-height: 1.6em; align-items: center; }
  .suggested td:nth-child(2) { overflow-wrap: anywhere; }
  .badge { font-size: var(--fs-xs); background: var(--active); border: 1px solid var(--border); border-radius: var(--radius-sm); padding: 0 var(--s1); }
  .actions { white-space: nowrap; text-align: right; }
  .error { color: var(--todo); font-size: var(--fs-md); }
  tr:last-child td { border-bottom: 0; }
  @media (max-width: 759px) {
    .status { grid-template-columns: 1fr; }
    .total { text-align: left; max-width: none; }
    .lane { background: none; padding: 0; }
  }
</style>
