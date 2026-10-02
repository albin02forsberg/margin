// README screenshots: start the e2e build through tauri-driver on a demo dataset
// in a temp dir and save PNGs to docs/screenshots/. `npm run screenshots`
// (under `xvfb-run` without a display; CI: the manual "Screenshots" workflow).
// All content below is made up.
import { spawn } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { connect } from "node:net";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { remote, Key } from "webdriverio";

const app = resolve("src-tauri/target/debug/margin");
const out = resolve(process.env.SCREENSHOTS_DIR ?? "docs/screenshots");
const root = mkdtempSync(join(tmpdir(), "margin-shots-"));
const notes = join(root, "notes");
const data = join(root, "data");

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

function fixtures() {
  const cfgDir = join(root, "config", "dev.albin.margin.e2e");
  write(join(cfgDir, "config.toml"), `
notes_dir = ${JSON.stringify(notes)}
data_dir = ${JSON.stringify(data)}
export_dir = ${JSON.stringify(join(root, "export"))}
profiles = ["Work"]
capture_shortcut = ""
reminders = false
idle_threshold_minutes = 0
close_to_tray = false
templates = []

[[views]]
name = "Work this week"
query = "tag:work due:<=+7d"
`);
  write(join(cfgDir, "tutorial.org"), ""); // not a first run

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
`);

  write(join(notes, "sync-design.org"), `
:PROPERTIES:
:ID: sync-design
:END:
#+title: Offline sync

Last-writer-wins per field, with a vector clock per record. See [[id:adr][ADR 7]].
Server side lives in [[id:orchard][Orchard]].
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

async function waitForPort(port: number) {
  for (let i = 0; i < 50; i++) {
    const up = await new Promise<boolean>((r) => { const s = connect(port, "127.0.0.1").once("connect", () => { s.end(); r(true); }).once("error", () => r(false)); });
    if (up) return;
    await new Promise((r) => setTimeout(r, 200));
  }
  throw new Error("tauri-driver didn't start");
}

// Mid-afternoon in the screenshots whenever this runs: the UTC offset that makes it about 15:00
// (Node and the app both follow TZ).
const off = ((15 - new Date().getUTCHours() + 36) % 24) - 12;
process.env.TZ = `Etc/GMT${off > 0 ? "-" : "+"}${Math.abs(off)}`;
fixtures();
mkdirSync(out, { recursive: true });
// Dark is the app's default look; GTK_THEME keeps WebKit from reporting a light preference.
const env = { ...process.env, GTK_THEME: "Adwaita:dark", XDG_CONFIG_HOME: join(root, "config"), XDG_DATA_HOME: join(root, "share"), XDG_CACHE_HOME: join(root, "cache") };
const driver = spawn("tauri-driver", [], { stdio: "inherit", env });
let failed = false;
try {
  await waitForPort(4444);
  const browser = await remote({
    hostname: "127.0.0.1",
    port: 4444,
    logLevel: "warn",
    waitforTimeout: 10_000,
    capabilities: { "tauri:options": { application: app }, "wdio:enforceWebDriverClassic": true } as WebdriverIO.Capabilities,
  });
  const pause = (ms = 600) => browser.pause(ms);
  const type = async (s: string) => { for (const c of s) await browser.keys(c); };
  const shot = async (name: string) => { await pause(); await browser.saveScreenshot(join(out, `${name}.png`)); console.log(`saved ${name}.png`); };
  const open = async (q: string, file: string) => {
    await browser.keys([Key.Ctrl, "p"]);
    await browser.$(".picker input").waitForDisplayed();
    await type(q);
    await pause(300);
    await browser.keys(Key.Enter);
    // Guard: an unmatched title would create a new empty note instead.
    await browser.waitUntil(async () => (await browser.$(".status .file").getText()) === file, { timeoutMsg: `${q} didn't open ${file}` });
    await pause();
  };
  const command = async (q: string) => {
    await browser.keys([Key.Ctrl, "k"]);
    await browser.$(".picker input").waitForDisplayed();
    await type(q);
    await pause(300);
    await browser.keys(Key.Enter);
    await pause();
  };

  await browser.setWindowSize(1280, 800).catch((e) => console.warn("setWindowSize:", e.message));
  await browser.$(".agenda h1").waitForDisplayed();

  // Open a few notes first so the sidebar and tabs look lived in.
  for (const [q, file] of [["Reading list", "reading.org"], ["Offline sync", "sync-design.org"], ["Harbor app", "harbor.org"], ["Orchard platform", "orchard.org"]]) await open(q, file);
  await browser.keys(Key.Escape);
  await browser.keys("}"); // cursor off the title line, so it renders
  await shot("note");

  await command("Show note graph");
  await shot("graph");
  await command("Show note graph"); // toggle off again

  await browser.keys([Key.Ctrl, "1"]);
  await browser.$(".agenda h1").waitForDisplayed();
  await shot("today");

  await browser.keys([Key.Ctrl, "4"]);
  await browser.$(".time h1").waitForDisplayed();
  await shot("time");

  await browser.keys([Key.Ctrl, "1"]);
  await browser.keys([Key.Ctrl, "k"]);
  await browser.$(".picker input").waitForDisplayed();
  await type("task");
  await shot("palette");
  await browser.keys(Key.Escape);

  await browser.deleteSession().catch(() => {});
} catch (e) {
  console.error(e);
  failed = true;
} finally {
  driver.kill();
  rmSync(root, { recursive: true, force: true, maxRetries: 5 }); // the app may still be writing
}
process.exit(failed ? 1 : 0);
