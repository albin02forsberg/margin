// End-to-end tests: drive the real app through tauri-driver (WebDriver).
// Needs `tauri-driver` and WebKitWebDriver on PATH; run with `npm run test:e2e`
// (under `xvfb-run` when there's no display). Everything lives in a temp dir:
// the app reads its config from $XDG_CONFIG_HOME/dev.albin.margin.e2e/config.toml.
import { after, before, describe, it } from "node:test";
import { spawn, type ChildProcess } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { connect } from "node:net";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { remote, Key } from "webdriverio";

const app = resolve("src-tauri/target/debug/margin");
const root = mkdtempSync(join(tmpdir(), "margin-e2e-"));
const notes = join(root, "notes");
const data = join(root, "data");
const read = (p: string) => { try { return readFileSync(p, "utf8"); } catch { return ""; } };

const now = new Date();
const pad = (n: number) => String(n).padStart(2, "0");
const today = `<${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())} ${now.toLocaleDateString("en-US", { weekday: "short" })}>`;

function fixtures() {
  const cfgDir = join(root, "config", "dev.albin.margin.e2e");
  for (const d of [cfgDir, notes, join(data, "work")]) mkdirSync(d, { recursive: true });
  writeFileSync(join(cfgDir, "config.toml"), [
    `notes_dir = ${JSON.stringify(notes)}`,
    `data_dir = ${JSON.stringify(data)}`,
    `export_dir = ${JSON.stringify(join(root, "export"))}`,
    `profiles = ["Work"]`,
    `capture_shortcut = ""`,
    `reminders = false`,
    `idle_threshold_minutes = 0`,
    `close_to_tray = false`,
    `templates = []`,
  ].join("\n"));
  writeFileSync(join(cfgDir, "tutorial.org"), ""); // not a first run, so the tutorial doesn't take focus
  writeFileSync(join(notes, "scratch.org"), "#+title: Scratch\n\nFirst line.\n");
  writeFileSync(join(notes, "tasks.org"), `* TODO Fixture task\nSCHEDULED: ${today}\n`);
  writeFileSync(join(data, "work", "projects.toml"), `[Acme]\nexport_code = "ACME"\n`);
}

let driver: ChildProcess;
let browser: WebdriverIO.Browser;

/** Wait until FILE's text satisfies OK. */
async function fileMatches(file: string, ok: (s: string) => boolean, msg: string) {
  try {
    await browser.waitUntil(async () => ok(read(file)));
  } catch {
    throw new Error(`${msg}; ${file} is:\n${read(file)}`);
  }
}

/** The first displayed element matching CSS whose text includes TEXT, once there is one.
 *  textContent, since WebKitWebDriver's getText is "" for text-overflow: ellipsis spans. */
async function find(css: string, text: string) {
  const hit = async () => {
    for (const el of await browser.$$(css)) if ((await el.isDisplayed()) && String(await el.getProperty("textContent")).includes(text)) return el;
    return false;
  };
  try {
    return (await browser.waitUntil(hit)) as unknown as WebdriverIO.Element;
  } catch {
    const seen = [];
    for (const el of await browser.$$(css)) seen.push(`${await el.isDisplayed()} ${JSON.stringify(await el.getText())} ${JSON.stringify(await el.getProperty("textContent"))}`);
    throw new Error(`no ${css} with “${text}”; saw (displayed, text, textContent):\n${seen.join("\n")}`);
  }
}

/** Press keys one at a time (keys("abc") would hold them all down together). */
async function type(s: string) {
  for (const c of s) await browser.keys(c);
}

/** Wait for the picker with PROMPT, replace its text with TEXT and press Enter.
 *  Typed as key presses: WebDriver's clear blurs the input, and blur cancels the picker. */
async function answer(prompt: string, text: string) {
  await find(".picker label span", prompt);
  await browser.keys([Key.Ctrl, "a"]);
  await type(text);
  await browser.keys(Key.Enter);
}

before(async () => {
  fixtures();
  const env = { ...process.env, XDG_CONFIG_HOME: join(root, "config"), XDG_DATA_HOME: join(root, "share"), XDG_CACHE_HOME: join(root, "cache") };
  driver = spawn("tauri-driver", [], { stdio: "inherit", env });
  // Wait for tauri-driver to listen.
  for (let i = 0; ; i++) {
    const up = await new Promise<boolean>((r) => { const s = connect(4444, "127.0.0.1").once("connect", () => { s.end(); r(true); }).once("error", () => r(false)); });
    if (up) break;
    if (i > 50) throw new Error("tauri-driver didn't start");
    await new Promise((r) => setTimeout(r, 200));
  }
  browser = await remote({
    hostname: "127.0.0.1",
    port: 4444,
    logLevel: "warn",
    waitforTimeout: 10_000,
    capabilities: { "tauri:options": { application: app }, "wdio:enforceWebDriverClassic": true } as WebdriverIO.Capabilities,
  });
});

after(async () => {
  await browser?.deleteSession().catch(() => {});
  driver?.kill();
  rmSync(root, { recursive: true, force: true });
}, { timeout: 30_000 });

describe("Margin", () => {
  it("starts on Today", async () => {
    await find(".agenda h1", "Today");
    await find(".agenda .row .title", "Fixture task");
  });

  it("opens a note with Ctrl+P, edits and saves it", async () => {
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "scratch");
    await find(".status .file", "scratch.org");
    await type("Go");
    await type("Typed by e2e");
    await browser.keys(Key.Escape);
    await browser.keys([Key.Ctrl, "s"]);
    await fileMatches(join(notes, "scratch.org"), (s) => /First line\.\n\s*Typed by e2e/.test(s), "note wasn't saved");
  });

  it("creates a task with Ctrl+N that shows in Today and the inbox", async () => {
    await browser.keys([Key.Ctrl, "1"]);
    await find(".agenda h1", "Today");
    await browser.keys([Key.Ctrl, "n"]);
    await browser.$(".dialog input.title").addValue("Water the e2e plants");
    await browser.$(".dialog #sched").addValue("today");
    await browser.keys(Key.Enter);
    await fileMatches(join(notes, "inbox.org"), (s) => /\* TODO Water the e2e plants\n\s*SCHEDULED: </.test(s), "task not in inbox.org");
    await find(".agenda .row .title", "Water the e2e plants");
  });

  it("marks a task done with x in Today", async () => {
    await (await find(".agenda .row .title", "Fixture task")).click();
    await browser.keys("x");
    await fileMatches(join(notes, "tasks.org"), (s) => s.startsWith("* DONE Fixture task"), "task not marked done");
  });

  it("starts and stops the timer from the Time view", async () => {
    await browser.keys([Key.Ctrl, "4"]);
    await (await find(".time button", "Start tracking")).click();
    await answer("Track time on project", "Acme");
    await answer("What are you working on?", "e2e run");
    await find(".time .status h2", "Acme");
    await (await find(".time .buttons button", "Stop")).click();
    await answer("What did you do?", "e2e done");
    const log = join(data, "work", "timelog.jsonl");
    await fileMatches(log, (s) => /"ev":"in".*"project":"Acme"/.test(s) && /"ev":"out".*"note":"e2e done"/.test(s), "no in/out entries in the time log");
  });
});
