// Shared e2e setup: drive the real app through tauri-driver (WebDriver).
// Needs `tauri-driver` and WebKitWebDriver on PATH; run with `npm run test:e2e`
// (under `xvfb-run` when there's no display). Everything lives in a temp dir:
// the app reads its config from $XDG_CONFIG_HOME/dev.albin.margin.e2e/config.toml.
import { spawn, spawnSync, type ChildProcess } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { connect } from "node:net";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { remote, Key } from "webdriverio";

export { Key };
const app = resolve("src-tauri/target/debug/margin");
export const read = (p: string) => { try { return readFileSync(p, "utf8"); } catch { return ""; } };

const now = new Date();
export const pad = (n: number) => String(n).padStart(2, "0");
export const isoDay = (d: Date) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
export const orgDay = (d: Date) => `${isoDay(d)} ${d.toLocaleDateString("en-US", { weekday: "short" })}`;
export const today = `<${orgDay(now)}>`;

export type Dirs = { root: string; notes: string; data: string; cfgDir: string; exportDir: string };

/** The current session's browser (one app session at a time: the app is single-instance). */
export let browser: WebdriverIO.Browser;
let driver: ChildProcess | undefined;
let dirs: Dirs | undefined;

/** Start the app on a fresh temp config. CONFIG: extra config.toml lines (top-level keys, then tables);
 *  FILES: notes and data files to write first; TUTORIAL false makes this a first run. */
export async function launch({ config = [], files = () => {}, tutorial = true }: { config?: string[]; files?: (d: Dirs) => void; tutorial?: boolean } = {}) {
  const root = mkdtempSync(join(tmpdir(), "margin-e2e-"));
  const d: Dirs = { root, notes: join(root, "notes"), data: join(root, "data"), cfgDir: join(root, "config", "dev.albin.margin.e2e"), exportDir: join(root, "export") };
  dirs = d;
  for (const p of [d.cfgDir, d.notes, join(d.data, "work")]) mkdirSync(p, { recursive: true });
  writeFileSync(join(d.cfgDir, "config.toml"), [
    `notes_dir = ${JSON.stringify(d.notes)}`,
    `data_dir = ${JSON.stringify(d.data)}`,
    `export_dir = ${JSON.stringify(d.exportDir)}`,
    `profiles = ["Work"]`,
    `capture_shortcut = ""`,
    `reminders = false`,
    `idle_threshold_minutes = 0`,
    `close_to_tray = false`,
    ...config,
  ].join("\n") + "\n");
  if (tutorial) writeFileSync(join(d.cfgDir, "tutorial.org"), ""); // not a first run, so the tutorial doesn't take focus
  files(d);
  // HOME too, so `~` paths (the defaults, when config.toml is ignored) stay in the temp dir.
  const env = { ...process.env, HOME: root, XDG_CONFIG_HOME: join(root, "config"), XDG_DATA_HOME: join(root, "share"), XDG_CACHE_HOME: join(root, "cache") };
  // Own process group, so close() takes WebKitWebDriver and the app down with it.
  driver = spawn("tauri-driver", [], { stdio: "inherit", env, detached: true });
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
  return d;
}

/** End the session and wait for the app to exit, so the next launch isn't handed to it as a second instance. */
export async function close() {
  await browser?.deleteSession().catch(() => {});
  if (driver?.pid && driver.exitCode == null) {
    const exited = new Promise((r) => driver!.once("exit", r));
    try { process.kill(-driver.pid, "SIGTERM"); } catch {}
    await exited;
  }
  for (let i = 0; i < 50 && spawnSync("pgrep", ["-f", app]).status === 0; i++) await new Promise((r) => setTimeout(r, 200));
  spawnSync("pkill", ["-f", app]);
  if (dirs) rmSync(dirs.root, { recursive: true, force: true });
  driver = dirs = undefined;
}

/** Wait until FILE's text satisfies OK. */
export async function fileMatches(file: string, ok: (s: string) => boolean, msg: string) {
  try {
    await browser.waitUntil(async () => ok(read(file)));
  } catch {
    throw new Error(`${msg}; ${file} is:\n${read(file)}`);
  }
}

/** textContent of the displayed (laid out, not visibility: hidden) elements matching CSS.
 *  Read in one script, so a re-render can't leave a stale element handle in between;
 *  textContent, since WebKitWebDriver's getText is "" for text-overflow: ellipsis spans. */
export async function texts(css: string): Promise<string[]> {
  return browser.execute((css: string) => [...document.querySelectorAll(css)]
    .filter((e) => e.getClientRects().length > 0 && getComputedStyle(e).visibility !== "hidden")
    .map((e) => e.textContent ?? ""), css);
}

/** The first displayed element matching CSS whose text includes TEXT, once there is one. */
export async function find(css: string, text: string) {
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

/** Click BTN inside the first CSS element containing TEXT, retrying when a re-render makes the handle stale. */
export async function clickIn(css: string, text: string, btn: string) {
  await browser.waitUntil(async () => {
    try { await (await (await find(css, text)).$(btn)).click(); return true; } catch { return false; }
  }, { timeoutMsg: `could not click ${btn} in ${css} with “${text}”` });
}

/** Press keys one at a time (keys("abc") would hold them all down together). */
export async function type(s: string) {
  for (const c of s) await browser.keys(c);
}

/** Wait for the picker with PROMPT, replace its text with TEXT and press Enter.
 *  Typed as key presses: WebDriver's clear blurs the input, and blur cancels the picker. */
export async function answer(prompt: string, text: string) {
  await find(".picker label span", prompt);
  await browser.keys([Key.Ctrl, "a"]);
  await type(text);
  await browser.keys(Key.Enter);
}

/** Run the palette command whose label starts with LABEL (Ctrl+K). */
export async function palette(label: string) {
  await browser.keys([Key.Ctrl, "k"]);
  await answer("Run a command", label);
}
