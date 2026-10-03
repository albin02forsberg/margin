// End-to-end tests of the basics; setup and helpers are in setup.ts.
import { after, before, describe, it } from "node:test";
import assert from "node:assert/strict";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { answer, browser, close, fileMatches, find, Key, launch, palette, today, type, type Dirs } from "./setup.ts";

let notes: string, data: string, cfgDir: string;

before(async () => {
  const d: Dirs = await launch({
    config: [`templates = []`],
    files: ({ notes, data }) => {
      writeFileSync(join(notes, "scratch.org"), "#+title: Scratch\n\nFirst line.\n");
      writeFileSync(join(notes, "tasks.org"), `* TODO Fixture task\nSCHEDULED: ${today}\n`);
      writeFileSync(join(data, "work", "projects.toml"), `[Acme]\nexport_code = "ACME"\n`);
    },
  });
  ({ notes, data, cfgDir } = d);
});

after(close, { timeout: 30_000 });

describe("Margin", () => {
  it("starts on Today", async () => {
    await find(".agenda h1", "Today");
    await find(".agenda .row .title", "Fixture task");
  });

  it("shows the app version in the sidebar", async () => {
    const { version } = JSON.parse(readFileSync(join(import.meta.dirname, "../src-tauri/tauri.conf.json"), "utf8"));
    await find(".side .version", "v" + version);
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

  it("edits a project's settings on the Projects page", async () => {
    await (await find(".time button", "Project settings")).click();
    await find(".projects td", "Acme");
    await browser.$(`select[aria-label="Rounding for Acme"]`).selectByVisibleText("Quarter hour");
    await browser.$(`input[aria-label="Export code for Acme"]`).click();
    await browser.keys([Key.Ctrl, "a"]);
    await type("ACME-2");
    await browser.keys(Key.Enter); // saves the row
    const toml = join(data, "work", "projects.toml");
    await fileMatches(toml, (s) => /export_code = "ACME-2"/.test(s) && /rounding = 0\.25/.test(s), "project settings not saved");
    await (await browser.$(".tabs .tab.cur .x")).click();
    await palette("Time: project settings");
    await browser.waitUntil(async () => (await browser.$(`input[aria-label="Export code for Acme"]`).getValue()) === "ACME-2", { timeoutMsg: "reopened page doesn't show the saved code" });
    assert.equal(await browser.$(`select[aria-label="Rounding for Acme"]`).getValue(), "0.25");
  });

  it("chooses the AI draft model in settings, keeping the rest of config.toml", async () => {
    const cfg = join(cfgDir, "config.toml");
    await palette("AI drafts: choose model");
    await answer("AI drafts", "Ollama");
    await answer("Ollama model", "qwen2.5:3b");
    await fileMatches(cfg, (s) => /^ai_backend = "ollama"$/m.test(s) && /^ai_model = "qwen2.5:3b"$/m.test(s) && /^templates = \[\]$/m.test(s), "Ollama choice not saved");
    // A pinned model that isn't here asks first; declining downloads and saves nothing.
    const models = join(cfgDir, "..", "..", "share", "dev.albin.margin.e2e", "models");
    await palette("AI drafts: choose model");
    await answer("AI drafts", "1.5B");
    await find(".picker .body", "GB of free RAM");
    await answer("Download Qwen2.5 1.5B", "Not now");
    assert.ok(!existsSync(models), "nothing should be downloaded");
    assert.match(readFileSync(cfg, "utf8"), /^ai_backend = "ollama"$/m);
    // One that is here (in the app's local data dir) is just chosen, and can be deleted.
    const file = join(models, "qwen2.5-1.5b-instruct-q4_k_m.gguf");
    mkdirSync(models, { recursive: true });
    writeFileSync(file, "fake model");
    await palette("AI drafts: choose model");
    await find(".picker li small", "downloaded · needs ~2 GB");
    await answer("AI drafts", "1.5B downloaded"); // not "Delete Qwen2.5 1.5B…"
    await fileMatches(cfg, (s) => /^ai_backend = "embedded"$/m.test(s) && /^ai_model = "qwen2.5-1.5b-instruct-q4"$/m.test(s), "built-in model choice not saved");
    await find(".status .msg", "can't run it itself"); // e2e builds without the embedded-ai feature
    await palette("AI drafts: choose model");
    await answer("AI drafts", "Delete Qwen2.5 1.5B");
    await browser.waitUntil(async () => !existsSync(file), { timeoutMsg: "model not deleted" });
    await palette("AI drafts: choose model");
    await answer("AI drafts", "Off");
    await fileMatches(cfg, (s) => /^ai_model = ""$/m.test(s), "drafts not turned off");
  });

  it("saves a group of the settings page, keeping the rest of config.toml", async () => {
    const cfg = join(cfgDir, "config.toml");
    const field = (label: string) => browser.$(`//div[contains(@class, "settings")]//label[span[contains(., "${label}")]]/*[self::input or self::textarea]`);
    await browser.keys([Key.Ctrl, ","]);
    await find(".settings h1", "Settings");
    await (await field("Expected hours per day")).click();
    await browser.keys([Key.Ctrl, "a"]);
    await type("7.5");
    await (await field("Profiles")).click();
    await browser.keys([Key.Ctrl, "End"]);
    await browser.keys(Key.Enter);
    await type("Home");
    await (await find(".settings button", "Save time")).click();
    await fileMatches(cfg, (s) => /^expected_daily_hours = 7\.5$/m.test(s) && /^profiles = \["Work", "Home"\]$/m.test(s) && /^templates = \[\]$/m.test(s), "settings not saved");
    await find(".status .msg", "Settings saved");
    assert.match(readFileSync(cfg, "utf8"), /^reminders = false$/m, "other settings lost");
  });

  it("registers a new quick-capture shortcut on save", async () => {
    const cfg = join(cfgDir, "config.toml");
    await (await browser.$(`//div[contains(@class, "settings")]//label[span[contains(., "Quick-capture shortcut")]]/input`)).click();
    await type("Ctrl+Alt+F12");
    await (await find(".settings button", "Save reminders")).click();
    await fileMatches(cfg, (s) => /^capture_shortcut = "Ctrl\+Alt\+F12"$/m.test(s), "shortcut not saved");
    await browser.pause(500); // the save message comes after the reload (the last test's may still show)
    const msg = await find(".status .msg", "Settings saved");
    assert.doesNotMatch(await msg.getText(), /shortcut/, "shortcut didn't register");
  });

  it("reloads the settings page when config.toml changes on disk", async () => {
    const cfg = join(cfgDir, "config.toml");
    const field = (label: string) => browser.$(`//div[contains(@class, "settings")]//label[span[contains(., "${label}")]]/input`);
    await browser.pause(500);
    assert.doesNotMatch(await browser.$(".status").getText(), /changed on disk/, "our own save echoed back");
    await (await field("Export folder")).click();
    await type("-edited");
    writeFileSync(cfg, readFileSync(cfg, "utf8").replace(/^expected_daily_hours = .*$/m, "expected_daily_hours = 6.25 # by hand").replace(/^export_dir = .*$/m, `export_dir = "/elsewhere"`));
    await browser.waitUntil(async () => (await (await field("Expected hours per day")).getValue()) === "6.25", { timeoutMsg: "settings page didn't reload" });
    await find(".status .msg", "config.toml changed on disk");
    await find(".settings .error", "Export folder"); // the unsaved edit is kept, with a notice
    assert.match(await (await field("Export folder")).getValue(), /-edited$/);
  });
});
