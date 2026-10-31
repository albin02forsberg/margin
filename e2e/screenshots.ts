// README media: start the e2e build through tauri-driver (e2e/setup.ts) on a demo dataset in a
// temp dir, save PNGs to docs/screenshots/ (dark), then record GIFs to docs/gifs/ (light; frames
// assembled by ffmpeg). `npm run screenshots` (under `xvfb-run` without a display; CI: the
// manual "Screenshots" workflow). All content below is made up.
import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { browser, close, find, Key, launch, texts, type Dirs } from "./setup.ts";
import { GIFS } from "../scripts/media-size.ts";
import { selected } from "../scripts/pr-media.ts";

/** SCREENSHOTS_ONLY=a,b: save just those PNGs/GIFs (the flows still run); a feature's shot is one more shot("name") in pngs(). */
const only = process.env.SCREENSHOTS_ONLY ?? "";
const want = (name: string) => selected([name], only).length > 0;

const out = resolve(process.env.SCREENSHOTS_DIR ?? "docs/screenshots");
const gifOut = resolve(process.env.GIFS_DIR ?? "docs/gifs");

const pad = (n: number) => String(n).padStart(2, "0");
const day = (offset: number) => { const d = new Date(); d.setHours(12, 0, 0, 0); d.setDate(d.getDate() + offset); return d; };
const iso = (d: Date) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
const wd = (d: Date) => d.toLocaleDateString("en-US", { weekday: "short" });
/** Org timestamp OFFSET days from today: <2026-10-02 Fri>, with optional time and repeater. */
const ts = (offset: number, extra = "") => `<${iso(day(offset))} ${wd(day(offset))}${extra ? " " + extra : ""}>`;
const its = (offset: number) => `[${iso(day(offset))} ${wd(day(offset))}]`;

function write(path: string, text: string) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, text.replace(/^\n/, ""));
}

/** config.toml lines on top of setup.ts's. */
const config = [`templates = []`, ``, `[[views]]`, `name = "Work this week"`, `query = "tag:work due:<=+7d"`, ``, `[[feeds]]`, `name = "Orchard blog"`, `url = "https://blog.example/feed.xml"`, ``, `[[feeds]]`, `name = "Release notes"`, `url = "https://releases.example/atom.xml"`, `heading = "Upgrades"`];

/** A small Markdown vault outside the notes folder, for the import preview (#900). */
let vault = "";

function fixtures({ root, notes, data }: Dirs) {
  vault = join(root, "Obsidian vault");
  write(join(vault, "Garden.md"), "---\ntags: [home]\n---\n# Garden\n- [ ] Plant tulips, see [[Bulbs]]\n![](img/bed.png)\n");
  write(join(vault, "plants/Bulbs.md"), "Order from the **spring** catalogue.\n");
  write(join(vault, "Reading.md"), "## Next\n- *Braiding Sweetgrass*\n");
  write(join(notes, "orchard.org"), `
:PROPERTIES:
:ID: orchard
:END:
#+title: Orchard platform

Backend for the [[id:harbor][Harbor app]]. See [[id:adr][Decisions]] and [[id:sync][Team sync]].

* Goals for Q4 [1/2]
- [X] Move image uploads to the new storage bucket
- [ ] p95 API latency under 200 ms

* Budget
| Item              | Qty | Unit cost | Total |
|-------------------+-----+-----------+-------|
| Build minutes     |  12 |        40 |   480 |
| Staging instances |   3 |       110 |   330 |
|-------------------+-----+-----------+-------|
| Sum               |     |           |   810 |
#+TBLFM: $4=$2*$3::@>$4=vsum(@I..@II)

* Rate limiter
#+begin_src rust
fn allow(bucket: &mut Bucket, now: Instant) -> bool {
    bucket.refill(now);
    if bucket.tokens < 1.0 { return false; }
    bucket.tokens -= 1.0;
    true
}
#+end_src

* TODO [#A] Load test the rate limiter :work:
DEADLINE: ${ts(2)}
* NEXT Write the partner onboarding guide :work:docs:
SCHEDULED: ${ts(0)}
`);

  write(join(notes, "harbor.org"), `
:PROPERTIES:
:ID: harbor
:END:
#+title: Harbor app

Mobile client for field crews. Talks to [[id:orchard][Orchard]]; offline sync design in
[[id:sync-design][Offline sync]].

* TODO [#B] Fix the crash when a photo upload is cancelled :work:bug:
SCHEDULED: ${ts(-1)}
* TODO Review the release checklist :work:
SCHEDULED: ${ts(0, "14:00")}
* WAIT App store review for 3.2 :work:
${[-1, -2, -2, -5, -8, -9, -9, -9, -15, -23, -30, -44].map((o, i) => `* DONE Release task ${i + 1} :work:\nCLOSED: ${its(o)}\n`).join("")}`);

  write(join(notes, "sync-design.org"), `
:PROPERTIES:
:ID: sync-design
:END:
#+title: Offline sync

Last-writer-wins per field, with a vector clock per record. See [[id:adr][ADR 7]].
Server side lives in [[id:orchard][Orchard]].

#+begin_src mermaid
sequenceDiagram
  Phone->>Server: push edits + vector clock
  Server->>Server: merge per field
  Server-->>Phone: merged record
#+end_src
`);

  write(join(notes, "adr.org"), `
:PROPERTIES:
:ID: adr
:END:
#+title: Decisions

* ADR 7: Field-level merge for offline edits
Accepted. Discussed in [[id:sync][Team sync]].
* ADR 8: One region until partner traffic doubles
Proposed.
`);

  write(join(notes, "team-sync.org"), `
:PROPERTIES:
:ID: sync
:END:
#+title: Team sync

* ${its(-7)}
- Sync design review went well; open question on tombstones.
* ${its(0)}
- Rate limiter ready for load testing.
- [[id:reading][Reading list]] has the paper on CRDTs.
`);

  write(join(notes, "reading.org"), `
:PROPERTIES:
:ID: reading
:END:
#+title: Reading list

* TODO Read "Local-first software" :reading:
* DONE Designing Data-Intensive Applications, ch. 5 :reading:
`);

  write(join(notes, "home.org"), `
#+title: Home

* TODO [#A] Renew the bike insurance :home:
DEADLINE: ${ts(5)}
* TODO Book a dentist appointment :home:
SCHEDULED: ${ts(0)}
* TODO Stretch for ten minutes
SCHEDULED: ${ts(0, ".+1d")}
:PROPERTIES:
:STYLE: habit
:LAST_REPEAT: ${its(-1)}
:END:
:LOGBOOK:
${[-1, -2, -3, -5, -6, -7, -8, -9, -10, -12, -13, -14, -15, -16, -17, -18, -19, -20].map((o) => `- State "DONE"       from "TODO"       ${its(o)}`).join("\n")}
:END:
`);

  write(join(notes, "inbox.org"), `
* TODO Ask about the conference budget :work:
* TODO Order new keyboard keycaps
`);

  write(join(notes, "daily", `${iso(day(0))}.org`), `
#+title: ${iso(day(0))}

- Morning: planning for the [[id:orchard][Orchard]] load test.
- Lunch walk by the river, first cold day.
- Idea: show sync conflicts inline instead of in a dialog.
`);

  // Time log: this week and last week, plus a timer running now.
  write(join(data, "work", "projects.toml"), `
[Orchard]
export_code = "ORC-100"
rounding = 0.25

[Harbor]
export_code = "HRB-210"
rounding = 0.25

[Internal]
export_code = "INT"
`);
  const ev: object[] = [];
  const at = (d: Date, h: number, m = 0) => `${iso(d)}T${pad(h)}:${pad(m)}:00`;
  const plan: [string, string, number, number, number, number][] = [ // project, note, from h:m, to h:m
    ["Orchard", "Rate limiter", 8, 30, 11, 45],
    ["Internal", "Team sync", 12, 30, 13, 15],
    ["Harbor", "Photo upload fixes", 13, 15, 17, 30],
  ];
  const alt: typeof plan = [
    ["Harbor", "Offline sync review", 8, 0, 10, 30],
    ["Orchard", "Partner API keys", 10, 45, 15, 0],
    ["Internal", "Interviews", 15, 0, 16, 20],
  ];
  for (let o = -10; o < 0; o++) {
    const d = day(o);
    if (d.getDay() === 0 || d.getDay() === 6) continue;
    for (const [project, note, h1, m1, h2, m2] of o % 2 ? plan : alt) {
      ev.push({ ev: "in", t: at(d, h1, m1), project }, { ev: "out", t: at(d, h2, m2), note });
    }
  }
  // Today, relative to now so it never runs into the future.
  const ago = (min: number) => { const d = new Date(Date.now() - min * 60_000); return `${iso(d)}T${pad(d.getHours())}:${pad(d.getMinutes())}:00`; };
  ev.push(
    { ev: "in", t: ago(390), project: "Orchard" }, { ev: "out", t: ago(225), note: "Load test plan" },
    { ev: "in", t: ago(180), project: "Internal" }, { ev: "out", t: ago(150), note: "Team sync" },
    { ev: "in", t: ago(140), project: "Harbor", task: "Fix the crash when a photo upload is cancelled" },
  );
  write(join(data, "work", "timelog.jsonl"), ev.map((e) => JSON.stringify(e)).join("\n") + "\n");
}

const pause = (ms = 600) => browser.pause(ms);
const type = async (s: string) => { for (const c of s) await browser.keys(c); };
const size = (h = 800) => browser.setWindowSize(1280, h).catch((e) => console.warn("setWindowSize:", e.message));

/** Start the app on fresh demo data in GTK_THEME, run FN, close it. */
async function session(theme: string, fn: () => Promise<void>) {
  process.env.GTK_THEME = theme; // launch() passes process.env on
  // The app's git backup of the demo dirs needs an identity (HOME is a temp dir), or the top bar says it failed.
  for (const k of ["AUTHOR", "COMMITTER"]) Object.assign(process.env, { [`GIT_${k}_NAME`]: "Margin demo", [`GIT_${k}_EMAIL`]: "demo@example.invalid" });
  await launch({ config, files: fixtures });
  try {
    await size();
    await browser.$(".agenda h1").waitForDisplayed();
    await fn();
  } finally {
    await close();
  }
}

/** Press KEYS until the picker labelled PROMPT shows (a keypress can be lost while the page settles). */
async function picker(keys: string | string[], prompt: string) {
  const shown = async () => (await texts(".picker label span")).some((t) => t.includes(prompt));
  await browser.waitUntil(async () => {
    if (await shown()) return true;
    await browser.keys(keys);
    await pause(400);
    return shown();
  }, { timeoutMsg: `no “${prompt}” picker` });
}

/** Wait until the open tab is FILE. */
const opened = (file: string) => browser.waitUntil(async () => (await browser.$(".status .file").getText()) === file, { timeoutMsg: `${file} didn't open` });

async function pngs() {
  const shot = async (name: string) => { if (!want(name)) return; await pause(); await browser.saveScreenshot(join(out, `${name}.png`)); console.log(`saved ${name}.png`); };
  const open = async (q: string, file: string) => {
    await picker([Key.Ctrl, "p"], "Open a note");
    await type(q);
    await pause(300);
    await browser.keys(Key.Enter);
    await opened(file); // an unmatched title would create a new empty note instead
    await pause();
  };
  const command = async (q: string) => {
    await picker([Key.Ctrl, "k"], "Run a command");
    await type(q);
    await pause(300);
    await browser.keys(Key.Enter);
    await pause();
  };

  // Open a few notes first so the tabs and Ctrl P look lived in.
  for (const [q, file] of [["Reading list", "reading.org"], ["Offline sync", "sync-design.org"], ["Harbor app", "harbor.org"], ["Orchard platform", "orchard.org"]]) await open(q, file);
  await browser.keys(Key.Escape);
  await browser.keys("}"); // cursor off the title line, so it renders
  await shot("note");

  if (want("mermaid")) { // #931
    await open("Offline sync", "sync-design.org");
    await browser.keys(Key.Escape);
    await browser.$(".cm-diagram svg").waitForDisplayed({ timeout: 20_000 });
    await shot("mermaid");
    await open("Orchard platform", "orchard.org");
  }

  await command("Show note graph");
  await shot("graph");
  await command("Show note graph"); // toggle off again

  await command("View: toggle outline");
  await shot("outline");
  await command("View: toggle outline");

  await browser.keys([Key.Ctrl, "1"]);
  await browser.$(".agenda h1").waitForDisplayed();
  await shot("today");

  await browser.keys([Key.Ctrl, "4"]);
  await browser.$(".time h1").waitForDisplayed();
  await size(900); // taller, so the Clock / Balance / More lane fits without its last button row clipped
  await shot("time");
  await size();
  if (want("activity") || want("activity-closed")) {
    await command("Time: activity calendar");
    await browser.$(".activity").waitForDisplayed();
    await shot("activity");
    await browser.keys("r"); // tasks closed (#919)
    await browser.$(".activity header .dim*=tasks closed").waitForDisplayed();
    await shot("activity-closed");
    await browser.keys(Key.Escape);
  }

  await browser.keys([Key.Ctrl, "1"]);
  await picker([Key.Ctrl, "k"], "Run a command");
  await type("task");
  await shot("palette");
  await browser.keys(Key.Escape);

  if (want("search-regex")) { // #968
    await command("Search in all notes");
    await type('re:"harbou?r|orchard" -todo');
    await pause(500);
    await shot("search-regex");
    await browser.keys(Key.Escape);
  }

  await command("Notes: replace text in all notes"); // the preview only: Esc writes nothing
  await type("Harbor");
  await browser.keys(Key.Enter);
  await pause(300);
  await browser.keys([Key.Ctrl, "a"]);
  await type("Harbour");
  await browser.keys(Key.Enter);
  await find(".picker label span", "Replace “Harbor” with “Harbour”");
  await shot("replace");
  await browser.keys(Key.Escape);

  await command("Notes: import a Markdown"); // the preview only: Esc writes nothing
  await type(vault);
  await browser.keys(Key.Enter);
  await find(".picker label span", "Import 3 notes");
  await shot("md-import");
  await browser.keys(Key.Escape);

  await open("Harbor app", "harbor.org");
  await browser.keys(Key.Escape);
  await type("Go"); // inline [[ completion (#899)
  await type("Mobile side: [[Orch");
  await find(".cm-tooltip-autocomplete li", "Orchard");
  await shot("complete");
  await browser.keys(Key.Escape); // closes the popup
  await browser.keys(Key.Escape);
  await type("u");

  await command("Split view: open a note");
  await type("Orchard platform");
  await pause(300);
  await browser.keys(Key.Enter);
  await opened("orchard.org"); // the right pane has focus
  await browser.keys(Key.Escape);
  await shot("split");

  await browser.keys([Key.Ctrl, ","]); // the MCP snippet (#929)
  await (await find(".settings .chips button", "AI chat")).click();
  await browser.$(".settings .mcp code").waitForDisplayed();
  await shot("mcp");
  if (want("ai-tools")) { // #963: the switch, shown as the backend is Ollama (the default)
    await (await browser.$(`//div[contains(@class, "settings")]//label[span[contains(., "Let the model search notes itself")]]`)).scrollIntoView({ block: "center" });
    await shot("ai-tools");
  }

  if (want("feeds")) { // #981: the feeds list in Settings
    await (await find(".settings .chips button", "Feeds")).click();
    await shot("feeds");
  }

  if (want("long-session")) { // #970: the demo clock has run 2h 20m, past a 2 h limit; last, as it tints the chip
    await (await find(".settings .chips button", "Time")).click();
    await (await browser.$(`//div[contains(@class, "settings")]//label[span[contains(., "Ask if still working after")]]/input`)).click();
    await browser.keys([Key.Ctrl, "a"]);
    await type("2");
    await (await find(".settings button", "Save time")).click();
    await find(".status .msg", "Settings saved");
    await browser.keys([Key.Ctrl, "4"]);
    await browser.$(".clockchip.long").waitForDisplayed();
    await shot("long-session");
  }
}

// GIFs: each frame is a screenshot shown for a fixed time, so a clip doesn't depend on how fast
// the machine took it; ffmpeg turns the list into an 800 px wide GIF.
let frames: string[] = [], frameDir = "";

/** A frame shown for HOLD ms, taken WAIT ms after the last action. */
async function frame(hold = 120, wait = 100) {
  await pause(wait);
  const f = join(frameDir, `${String(frames.length / 2).padStart(4, "0")}.png`);
  await browser.saveScreenshot(f);
  frames.push(`file '${f}'`, `duration ${hold / 1000}`);
}

/** Type S as key presses, with a frame every EVERY of them. */
async function typed(s: string, every = 2) {
  for (let i = 0; i < s.length; i++) {
    await browser.keys(s[i]);
    if (i % every === every - 1 || i === s.length - 1) await frame(90, 0);
  }
}

/** Run FN, recording the frames it takes, and save them as NAME.gif. */
async function clip(name: string, fn: () => Promise<void>) {
  frameDir = mkdtempSync(join(tmpdir(), `margin-gif-${name}-`));
  frames = [];
  try {
    await fn();
    await frame(2500, 600); // the end, held before the loop restarts
    const n = frames.length / 2;
    frames.push(frames.at(-2)!); // the concat demuxer drops the last file's duration unless it's listed again
    writeFileSync(join(frameDir, "list.txt"), frames.join("\n") + "\n");
    const r = spawnSync("ffmpeg", ["-y", "-loglevel", "error", "-f", "concat", "-safe", "0", "-i", join(frameDir, "list.txt"),
      "-vf", "scale=800:-1:flags=lanczos,split[a][b];[a]palettegen[p];[b][p]paletteuse=dither=none:diff_mode=rectangle",
      "-fps_mode", "vfr", "-loop", "0", join(gifOut, `${name}.gif`)], { stdio: "inherit" });
    if (r.status !== 0) throw new Error(`ffmpeg failed on ${name}.gif${r.error ? `: ${r.error.message}` : ""}`);
    console.log(`saved ${name}.gif (${n} frames)`);
  } finally {
    rmSync(frameDir, { recursive: true, force: true });
  }
}

/** Click the Today row titled TITLE. */
const row = async (title: string) => { await (await find(".agenda .row .title", title)).click(); await frame(500, 300); };
/** Click the Time view button labelled LABEL. */
const button = async (label: string) => { await (await find(".time .buttons button", label)).click(); };

const clips: Record<string, () => Promise<void>> = {
  capture: async () => {
    await frame(900);
    await picker([Key.Ctrl, "k"], "Run a command");
    await frame(300);
    await typed("new task");
    await frame(600);
    await browser.keys(Key.Enter);
    await browser.$(".dialog input.title").waitForDisplayed();
    await frame(300);
    await typed("Call the venue about the offsite", 3);
    await browser.$(".dialog #sched").click();
    await typed("today");
    await frame(800);
    await browser.keys(Key.Enter);
    await find(".agenda .row .title", "Call the venue about the offsite");
  },
  today: async () => {
    await frame(1200);
    await row("Fix the crash when a photo upload is cancelled");
    await browser.keys("+"); // priority up
    await frame(900, 600);
    await row("Review the release checklist");
    await browser.keys("x"); // done
    await frame(900, 600);
    await row("Book a dentist appointment");
    await browser.keys(">"); // a day later
    await frame(900, 600);
    await row("Write the partner onboarding guide");
    await browser.keys("s");
    await find(".picker label span", "Schedule for");
    await frame(400);
    await browser.keys([Key.Ctrl, "a"]);
    await typed("+3d");
    await frame(600);
    await browser.keys(Key.Enter);
  },
  time: async () => {
    await browser.keys([Key.Ctrl, "4"]);
    await browser.$(".time h1").waitForDisplayed();
    await frame(1500, 600);
    await button("Pause");
    await find(".time .status small", "On a break");
    await frame(1200, 300);
    await button("Resume");
    await find(".time .status small", "Tracking since");
    await frame(1000, 300);
    await button("Switch project");
    await find(".picker label span", "What did you do on");
    await frame(900, 300);
    await browser.keys(Key.Enter); // keep the task as the note
    await find(".picker label span", "Switch to project");
    await frame(400, 300);
    await typed("orch");
    await frame(600);
    await browser.keys(Key.Enter);
    await find(".time .status h2", "Orchard");
  },
  links: async () => {
    await frame(600);
    await picker([Key.Ctrl, "p"], "Open a note");
    await typed("offline");
    await frame(500, 300);
    await browser.keys(Key.Enter);
    await opened("sync-design.org");
    await browser.keys(Key.Escape);
    await frame(1000, 400);
    await browser.keys(" "); await type("nb"); // notes linking here
    await find(".links h3", "Linked from");
    await frame(1200, 400);
    await browser.keys(" "); await type("ng"); // the local graph
    await find(".links h3", "Graph");
    await frame(1500, 800);
    await type("/ADR");
    await frame(400);
    await browser.keys(Key.Enter);
    await frame(700, 300);
    await browser.keys(Key.Enter); // follow the link under the cursor
    await opened("adr.org");
    await frame(1500, 800);
  },
  vim: async () => {
    await picker([Key.Ctrl, "p"], "Open a note");
    await type("Orchard platform");
    await pause(300);
    await browser.keys(Key.Enter);
    await opened("orchard.org");
    await browser.keys(Key.Escape);
    await browser.keys(" "); await type("nb"); // the links clip's panels off again
    await browser.keys(" "); await type("ng");
    await type("gg");
    await frame(900, 600);
    await type("/200");
    await frame(400);
    await browser.keys(Key.Enter);
    await frame(600, 300);
    await type("ciw");
    await frame(500, 300);
    await typed("150");
    await browser.keys(Key.Escape);
    await frame(700, 300);
    await type("o");
    await frame(300, 200);
    await typed("- [ ] Load test at twice the partner traffic", 4);
    await browser.keys(Key.Escape);
    await frame(700, 300);
    await type("dd");
    await frame(700, 300);
    await type("u");
    await frame(700, 300);
    await type(":w");
    await frame(500);
    await browser.keys(Key.Enter);
  },
};

async function gifs() {
  for (const name of GIFS) {
    await browser.keys([Key.Ctrl, "1"]); // every clip starts on Today
    await find(".agenda h1", "Today");
    await pause(800);
    if (want(name)) await clip(name, clips[name]);
  }
}

// Mid-afternoon in the screenshots whenever this runs: the UTC offset that makes it about 15:00
// (Node and the app both follow TZ).
const off = ((15 - new Date().getUTCHours() + 36) % 24) - 12;
process.env.TZ = `Etc/GMT${off > 0 ? "-" : "+"}${Math.abs(off)}`;
for (const d of [out, gifOut]) mkdirSync(d, { recursive: true });
let failed = false;
try {
  // Dark is the app's default look; GTK_THEME keeps WebKit from reporting a light preference.
  await session("Adwaita:dark", pngs);
  if (!only || GIFS.some(want)) await session("Adwaita", gifs); // the GIFs in light, on fresh data
} catch (e) {
  console.error(e);
  failed = true;
}
process.exit(failed ? 1 : 0);
