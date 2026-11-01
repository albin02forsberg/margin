// End-to-end tests of the basics; setup and helpers are in setup.ts.
import { after, before, describe, it } from "node:test";
import assert from "node:assert/strict";
import { existsSync, mkdirSync, readFileSync, unlinkSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { answer, browser, close, fileMatches, find, isoDay, Key, launch, nextWeekday, noPicker, palette, savedView, texts, today, type, type Dirs } from "./setup.ts";

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

  it("titles the window after the active tab", async () => {
    assert.equal(await browser.execute(() => document.title), "Today — Margin");
  });

  it("shows the app version in Settings > About", async () => {
    const { version } = JSON.parse(readFileSync(join(import.meta.dirname, "../src-tauri/tauri.conf.json"), "utf8"));
    await (await browser.$(".topnav .set")).click();
    await find(".settings .about", "v" + version);
    await (await browser.$(".tabs .tab.cur .x")).click();
  });

  it("\"Show what's new\" opens the kept release notes (#513)", async () => {
    const { version } = JSON.parse(readFileSync(join(import.meta.dirname, "../src-tauri/tauri.conf.json"), "utf8"));
    await browser.execute((v) => localStorage.setItem("last-notes", JSON.stringify({ from: "0.0.1", to: v, body: "### Features\n- zoom" })), version);
    await palette("Show what's new");
    await find(".tabs .tab.cur", `What's new in ${version}`);
    await (await browser.$(".tabs .tab.cur .x")).click();
  });

  it("opens a note with Ctrl+P, edits and saves it", async () => {
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "scratch");
    await find(".status .file", "scratch.org");
    await find(".status .words", "3 words");
    await type("Go");
    await type("Typed by e2e");
    await browser.keys(Key.Escape);
    await find(".status .words", "6 words");
    await browser.keys([Key.Ctrl, "s"]);
    await fileMatches(join(notes, "scratch.org"), (s) => /First line\.\n\s*Typed by e2e/.test(s), "note wasn't saved");
  });

  it(":set number shows the line-number gutter and :set nonumber hides it", async () => {
    await type(":set number");
    await browser.keys(Key.Enter);
    await find(".cm-lineNumbers", "");
    await type(":set nonumber");
    await browser.keys(Key.Enter);
    await browser.waitUntil(async () => (await browser.$$(".cm-lineNumbers")).length === 0);
  });

  it(":set nowrap / wrap toggle line wrapping; an unknown option says so (#480)", async () => {
    const ex = async (s: string) => { await type(s); await browser.keys(Key.Enter); };
    await ex(":set nowrap");
    await browser.waitUntil(async () => (await browser.$$(".cm-content.cm-lineWrapping")).length === 0, { timeoutMsg: "still wrapping" });
    await ex(":set wrap");
    await browser.$(".cm-content.cm-lineWrapping").waitForExist();
    await ex(":set bogus");
    await find(".cm-vim-message", "Unknown option: bogus");
  });

  it(":e opens a note by name, :x saves and closes it (#480)", async () => {
    await type(":e tasks");
    await browser.keys(Key.Enter);
    await find(".status .file", "tasks.org");
    await type("Go");
    await type("Edited by e2e");
    await browser.keys(Key.Escape);
    await type(":x");
    await browser.keys(Key.Enter);
    await find(".status .file", "scratch.org");
    await fileMatches(join(notes, "tasks.org"), (s) => s.includes("\nEdited by e2e"), ":x didn't save");
  });

  it(":b switches tabs by name, g; returns to the last edit (#480)", async () => {
    const ex = async (s: string) => { await type(s); await browser.keys(Key.Enter); };
    await ex(":e tasks");
    await find(".status .file", "tasks.org");
    await ex(":b scratch");
    await find(".status .file", "scratch.org");
    await ex(":b tasks");
    await find(".status .file", "tasks.org");
    await type("ggAX");
    await browser.keys(Key.Escape);
    await type("G");
    await type("g;");
    await find(".cm-activeLine", "Fixture taskX");
    await type("u");
    await ex(":bd");
    await find(".status .file", "scratch.org");
  });

  it("keeps a note deleted outside the app on screen, flagged", async () => {
    const file = join(notes, "scratch.org");
    unlinkSync(file);
    await find(".status .file", "deleted on disk");
    await find(".cm-content", "Typed by e2e");
    await browser.pause(500);
    assert.ok(!existsSync(file), "recreated without :w");
    await browser.keys([Key.Ctrl, "s"]);
    await fileMatches(file, (s) => /First line\.\n\s*Typed by e2e/.test(s), ":w didn't recreate it");
    await browser.waitUntil(async () => !(await browser.$(".status .file").getText()).includes("deleted on disk"), { timeoutMsg: "still flagged" });
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

  it("takes a date, tag and priority typed at the end of the title", async () => {
    await browser.keys([Key.Ctrl, "n"]);
    await browser.$(".dialog input.title").addValue("Call e2e Anna today #work !A");
    await find(".dialog .qa .tok", "today");
    await browser.keys(Key.Enter);
    await fileMatches(join(notes, "inbox.org"), (s) => /\* TODO \[#A\] Call e2e Anna\s+:work:\n\s*SCHEDULED: </.test(s), "quick-add tokens not applied");
    await find(".agenda .row .title", "Call e2e Anna");
  });

  it("marks a task done with x in Today", async () => {
    await (await find(".agenda .row .title", "Fixture task")).click();
    await browser.keys("x");
    await fileMatches(join(notes, "tasks.org"), (s) => /^\* DONE Fixture task\nCLOSED: \[\d{4}-\d\d-\d\d \S+ \d\d:\d\d\] SCHEDULED: </.test(s), "task not marked done");
  });

  it("moves a task across the Tasks board", async () => {
    await browser.keys([Key.Ctrl, "2"]);
    await (await find(".agenda .seg button", "Board")).click();
    await find(".board .kcol h2", "NEXT"); // a column per open keyword
    await find(".board .kcol.donecol .kcard .title", "Fixture task"); // closed this week, from Today
    await (await find(".board .kcol:first-child .kcard .title", "Water the e2e plants")).click();
    await browser.keys("L");
    await fileMatches(join(notes, "inbox.org"), (s) => s.includes("* NEXT Water the e2e plants"), "card not moved to NEXT");
    await find(".board .kcol:nth-child(2) .kcard .title", "Water the e2e plants");
    await browser.keys("x");
    await fileMatches(join(notes, "inbox.org"), (s) => s.includes("* DONE Water the e2e plants"), "card not done");
    await find(".board .kcol.donecol .kcard .title", "Water the e2e plants");
    await (await find(".agenda .seg button", "List")).click();
    await browser.waitUntil(async () => !(await browser.$(".board").isExisting()), { timeoutMsg: "still a board" });
  });

  it("starts and stops the timer from the Time view", async () => {
    await browser.keys([Key.Ctrl, "4"]);
    await (await find(".time button", "Start tracking")).click();
    await answer("Track time on project", "Acme");
    await answer("What are you working on?", "e2e run");
    await find(".time .status h2", "Acme");
    // Times are whole seconds and a zero-length session is dropped (no project card), so let a second pass.
    await browser.pause(1100);
    await (await find(".time .buttons button", "Stop")).click();
    await answer("What did you do?", "e2e done");
    const log = join(data, "work", "timelog.jsonl");
    await fileMatches(log, (s) => /"ev":"in".*"project":"Acme"/.test(s) && /"ev":"out".*"note":"e2e done"/.test(s), "no in/out entries in the time log");
    await find(".time .worked h3", "Worked today");
    await find(".time .pcard h3", "Acme"); // a card per project worked this week, its Track preselects it
    await (await find(".time .pcard button", "Track")).click();
    await find(".picker label span", "Track time on project");
    await browser.waitUntil(async () => (await texts(".picker li"))[0]?.includes("Acme"), { timeoutMsg: "Track didn't preselect the project" });
    await browser.keys(Key.Escape);
    await noPicker();
  });

  it("Space t w says when to leave, and the Time view shows it", async () => {
    const said = /Leave at \d\d:\d\d \(.+ left\)|No expected hours today|Done for today/; // weekends and red days have no target
    await noPicker();
    await (await find(".time button", "Start tracking")).click();
    await answer("Track time on project", "Acme");
    await answer("What are you working on?", "leave check");
    await find(".time .status h2", "Acme");
    await browser.waitUntil(async () => (await texts(".time .total .leave")).some((t) => said.test(t)), { timeoutMsg: "no leave time in the Time view" });
    await browser.keys([Key.Ctrl, "1"]);
    await find(".agenda h1", "Today");
    await find(".agenda .clock .run b", "Acme"); // the Today clock column shows the running clock
    await find(".agenda .clock .worked h3", "Worked today");
    await find(".agenda .chips .ag.live", "Acme · tracking"); // and the chips strip has it as a live chip
    await browser.keys(" ");
    await type("tw");
    await browser.waitUntil(async () => (await texts(".status .msg")).some((t) => said.test(t)), { timeoutMsg: "Space t w flashed no leave time" });
    await browser.keys([Key.Ctrl, "4"]);
    await (await find(".time .buttons button", "Stop")).click();
    await answer("What did you do?", "leave checked");
  });

  it("Space t l starts the time-off flow", async () => {
    await browser.keys([Key.Ctrl, "1"]);
    await find(".agenda h1", "Today");
    await browser.keys(" ");
    await type("tl");
    await find(".picker label span", "Kind of time off");
    await find(".picker li", "vacation");
    await browser.keys(Key.Escape);
  });

  it("logs and removes time off in the Time view", async () => {
    const file = join(data, "work", "leave.jsonl");
    const day = nextWeekday(); // weekends carry no expected hours, so time off needs a weekday
    const flow = async (hours: string) => {
      await (await find(".timeoff button", "Log time off")).click();
      await answer("Kind of time off", "vacation");
      await answer("First day off", isoDay(day));
      await answer("Last day off", "No date"); // one day
      await answer("Hours off per day", "Hours a day");
      await find(".picker label span", "Hours off a day");
      await browser.keys([Key.Ctrl, "a"]);
      await browser.keys(Key.Backspace); // clear the suggested 4
      await type(hours);
      await browser.keys(Key.Enter);
    };
    await browser.keys([Key.Ctrl, "4"]);
    await flow(""); // empty hours must not become a full day
    await find(".status .msg", "Enter the hours off per day");
    assert.ok(!existsSync(file) || readFileSync(file, "utf8") === "", "empty hours added a time-off entry");
    await flow("4");
    await answer("Note (optional)", "e2e trip");
    await fileMatches(file, (s) => /"kind":"vacation".*"hours":4\.0.*"note":"e2e trip"/.test(s), "time off not saved");
    await find(".timeoff td", "4h 00m/day");
    await (await browser.$(`.timeoff button[title="Remove time off"]`)).click();
    await fileMatches(file, (s) => s === "", "time off not removed");
    assert.ok(!(await browser.$(".time .status h2").getText()).includes("Acme"), "typing the note started the timer");
  });

  it("edits a project's settings on the Projects page", async () => {
    const acmeOrg = join(notes, "acme.org");
    writeFileSync(acmeOrg, "* TODO Acme fixture task\n");
    await (await find(".time button", "Project settings")).click();
    await find(".projects td", "Acme");
    await find(".projects .pcard h3", "Acme"); // a card per project, with its open tasks (category = file name)
    await find(".projects .pcard li", "Acme fixture task");
    await browser.$(`select[aria-label="Rounding for Acme"]`).selectByVisibleText("Quarter hour");
    await browser.$(`input[aria-label="Budget hours for Acme"]`).click();
    await type("20");
    await browser.$(`input[aria-label="Export code for Acme"]`).click();
    await browser.keys([Key.Ctrl, "a"]);
    await type("ACME-2");
    await browser.keys(Key.Enter); // saves the row
    const toml = join(data, "work", "projects.toml");
    await fileMatches(toml, (s) => /export_code = "ACME-2"/.test(s) && /rounding = 0\.25/.test(s) && /budget_hours = 20/.test(s), "project settings not saved");
    await find(".projects .pcard .budget", "/ 20 h this month"); // the card's budget line
    await (await browser.$(".tabs .tab.cur .x")).click();
    await palette("Time: project settings");
    await browser.waitUntil(async () => (await browser.$(`input[aria-label="Export code for Acme"]`).getValue()) === "ACME-2", { timeoutMsg: "reopened page doesn't show the saved code" });
    assert.equal(await browser.$(`select[aria-label="Rounding for Acme"]`).getValue(), "0.25");
    await browser.$(`input[aria-label="Export code for time off vacation"]`).click();
    await type("VAC-1");
    await browser.keys(Key.Enter);
    await fileMatches(join(data, "work", "leave.toml"), (s) => /\[vacation\]\nexport_code = "VAC-1"/.test(s), "time off code not saved");
    assert.ok(!readFileSync(join(data, "work", "projects.toml"), "utf8").includes("vacation"), "time off leaked into projects.toml");
    unlinkSync(acmeOrg); // last, so its file-change refresh can't re-render a row mid-edit (#405)
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
    await browser.keys(Key.End); // the click lands mid-text in a narrow card
    await type("-edited");
    writeFileSync(cfg, readFileSync(cfg, "utf8").replace(/^expected_daily_hours = .*$/m, "expected_daily_hours = 6.25 # by hand").replace(/^export_dir = .*$/m, `export_dir = "/elsewhere"`));
    await browser.waitUntil(async () => (await (await field("Expected hours per day")).getValue()) === "6.25", { timeoutMsg: "settings page didn't reload" });
    await find(".status .msg", "config.toml changed on disk");
    await find(".settings .error", "Export folder"); // the unsaved edit is kept, with a notice
    assert.match(await (await field("Export folder")).getValue(), /-edited$/);
  });

  it("resets a setting to its default", async () => {
    const cfg = join(cfgDir, "config.toml");
    const label = `//div[contains(@class, "settings")]//label[span[contains(., "Expected hours per day")]]`;
    await (await browser.$(`${label}//button[contains(., "Reset to default")]`)).click();
    await fileMatches(cfg, (s) => !/expected_daily_hours/.test(s) && /^templates = \[\]$/m.test(s), "setting not reset");
    await browser.waitUntil(async () => (await browser.$(`${label}/input`).getValue()) === "8", { timeoutMsg: "field doesn't show the default" });
    assert.ok(!(await browser.$(`${label}//button`).isExisting()), "reset shown for a default value");
  });

  it("narrows the settings page by category chip and filter", async () => {
    const label = (text: string) => browser.$(`//div[contains(@class, "settings")]//label[span[contains(., "${text}")]]`);
    const views = () => browser.$(`//div[contains(@class, "settings")]//form[.//legend[.="Saved views"]]`);
    await (await find(".settings .chips button", "Time")).click();
    await (await label("Expected hours per day")).waitForDisplayed();
    assert.ok(!(await (await label("Export folder")).isExisting()), "Folders field shown under Time");
    assert.ok(!(await (await views()).isDisplayed()), "Saved views shown under Time");
    await (await find(".settings .chips button", "All")).click();
    await (await browser.$(".settings input.filter")).click();
    await type("idle");
    await (await label("Ask about idle time")).waitForDisplayed();
    assert.ok(!(await (await label("Expected hours per day")).isExisting()), "filter kept a non-matching field");
    await browser.keys(Key.Escape);
    await (await label("Export folder")).waitForDisplayed({ timeoutMsg: "Escape didn't clear the filter" });
    assert.ok(await (await views()).isDisplayed(), "Saved views hidden under All");
  });

  it("hides an area's palette commands when it is unticked in Settings, and brings them back", async () => {
    const cfg = join(cfgDir, "config.toml");
    const box = async () => {
      const el = await browser.$(`//div[contains(@class, "settings")]//label[span[.="Ledger"]]/input`);
      await el.waitForClickable({ timeoutMsg: "Ledger checkbox not clickable" });
      return el;
    };
    const ledgerCmds = async () => {
      await browser.keys([Key.Ctrl, "k"]);
      await find(".picker label span", "Run a command");
      await type("Ledger: add");
      await browser.pause(300);
      const t = (await texts(".picker li")).filter((s) => s.includes("Ledger: add an entry"));
      await browser.keys(Key.Escape);
      await noPicker();
      return t.length;
    };
    const ledgerTab = async () => (await texts(".tabs .tab button")).some((t) => t.includes("Ledger"));
    writeFileSync(join(notes, "ledger.org"), "* 2026-10\n- 2026-10-25 -12.50 EUR lunch\n"); // "Go to Ledger" only opens once the file exists
    await palette("Go to Ledger");
    await browser.waitUntil(ledgerTab, { timeoutMsg: "no Ledger tab" });
    await browser.keys([Key.Ctrl, ","]);
    await (await box()).click();
    const save = await find(".settings button", "Save areas");
    await save.waitForClickable();
    await save.click();
    await fileMatches(cfg, (s) => /^hidden_areas = \["ledger"\]$/m.test(s), "hidden area not saved");
    await browser.waitUntil(async () => !(await ledgerTab()), { timeoutMsg: "Ledger tab still open after hiding the area" });
    assert.equal(await ledgerCmds(), 0, "Ledger command still in the palette");
    await (await box()).click();
    const save2 = await find(".settings button", "Save areas");
    await save2.waitForClickable();
    await save2.click();
    await fileMatches(cfg, (s) => /^hidden_areas = \[\]$/m.test(s), "area not shown again");
    assert.equal(await ledgerCmds(), 1, "Ledger command not back in the palette");
  });

  it("adds and reorders saved views on the settings page", async () => {
    const cfg = join(cfgDir, "config.toml");
    const last = async (css: string) => Array.from(await browser.$$(css)).at(-1)!;
    const add = async (name: string, query: string) => {
      await (await find(".settings button", "Add view")).click();
      await (await last(".settings input[aria-label='View name']")).click();
      await type(name);
      await (await last(".settings input[aria-label='View query']")).click();
      await type(query);
    };
    await add("Work", "tag:work");
    await add("Next up", "todo:NEXT");
    await (await browser.$$(".settings .row button[title='Move up']"))[1].click();
    await (await find(".settings button", "Save views")).click();
    await fileMatches(cfg, (s) => /\[\[views\]\]\nname = "Next up"\nquery = "todo:NEXT"\n\n\[\[views\]\]\nname = "Work"/.test(s) && /^templates = \[\]$/m.test(s), "views not saved");
    await savedView("Next up");
    await (await browser.$$(".settings .row button[title='Remove']"))[0].click();
    await (await find(".settings button", "Save views")).click();
    await fileMatches(cfg, (s) => !s.includes("Next up") && s.includes(`name = "Work"`), "view not removed");
  });

  it("adds a capture template on the settings page", async () => {
    const cfg = join(cfgDir, "config.toml");
    await (await find(".settings button", "Add template")).click();
    for (const [label, text] of [["key", "j"], ["name", "Journal"], ["heading", "Log"], ["body", "* %?"]]) {
      await (await browser.$(`.settings [aria-label='Template ${label}']`)).click();
      await type(text);
    }
    await (await find(".settings button", "Save templates")).click();
    await fileMatches(cfg, (s) => /\[\[templates\]\]/.test(s) && /^key = "j"$/m.test(s) && /^heading = "Log"$/m.test(s) && !/^templates = \[\]$/m.test(s) && !/^file =/m.test(s), "template not saved");
    await fileMatches(cfg, (s) => s.includes(`name = "Work"`), "views lost");
    await browser.waitUntil(async () => !(await (await find(".settings button", "Save templates")).isEnabled()), { timeoutMsg: "template still unsaved after reload" });
  });

  // config.toml doesn't autosave, so its edits stay unsaved until asked. Each test opens it itself, so one failure doesn't cascade.
  const dirty = async () => (await texts(".tabs .tab.cur > button:not(.x)"))[0]?.endsWith(" •");
  const openCfg = async () => {
    await noPicker();
    await palette("Settings: edit config.toml");
    await find(".status .file", "config.toml");
    await noPicker(); // the closing palette hands focus back to the editor
  };

  it("warns when config.toml saved in the editor has a shortcut that won't register", async () => {
    const cfg = join(cfgDir, "config.toml");
    await openCfg();
    await type("/capture_shortcut");
    await browser.keys(Key.Enter);
    await type(`Ccapture_shortcut = "Nope+Bogus"`);
    await browser.keys(Key.Escape);
    await find(".status .mode", "NORMAL");
    await browser.keys([Key.Ctrl, "s"]);
    await fileMatches(cfg, (s) => /^capture_shortcut = "Nope\+Bogus"$/m.test(s), "config.toml not saved");
    await find(".status .msg", "quick capture shortcut Nope+Bogus");
  });

  it("recreates a deleted config.toml from the current settings", async () => {
    const cfg = join(cfgDir, "config.toml");
    await browser.keys([Key.Ctrl, ","]);
    await find(".settings h1", "Settings");
    unlinkSync(cfg);
    await find(".settings .error", "couldn't be loaded");
    assert.ok(!(await browser.$("//div[contains(@class, 'settings')]//button[contains(., 'Edit config.toml')]").isExisting()), "Edit config.toml offered for a missing file");
    await (await find(".settings button", "Recreate config.toml")).click();
    await fileMatches(cfg, (s) => s.includes(`name = "Work"`) && /^key = "j"$/m.test(s) && /^capture_shortcut = "Nope\+Bogus"$/m.test(s), "settings not recreated");
    await find(".status .msg", "config.toml recreated");
    await browser.waitUntil(async () => !(await browser.$(".settings").getText()).includes("couldn't be loaded"), { timeoutMsg: "error banner still shown" });
  });

  it("Space f C recreates a deleted config.toml before opening it", async () => {
    const cfg = join(cfgDir, "config.toml");
    await browser.keys([Key.Ctrl, "1"]);
    await find(".agenda h1", "Today");
    unlinkSync(cfg);
    await type(" fC");
    await fileMatches(cfg, (s) => s.includes(`name = "Work"`) && /^capture_shortcut = "Nope\+Bogus"$/m.test(s), "settings not recreated");
    await find(".status .file", "config.toml");
    await find(".cm-content", `capture_shortcut = "Nope+Bogus"`);
  });

  it("Reload from disk drops the unsaved edits", async () => {
    const cfg = join(cfgDir, "config.toml");
    await openCfg();
    await type("ggO# dropped by e2e");
    await browser.keys(Key.Escape);
    await find(".status .mode", "NORMAL"); // Ctrl+K in insert mode is a vim digraph, not the palette
    await browser.waitUntil(dirty, { timeoutMsg: "tab not marked unsaved" });
    await palette("Reload from disk");
    await find(".status .msg", "Reloaded from disk");
    await browser.waitUntil(async () => !(await dirty()) && !(await texts(".cm-content"))[0]?.includes("dropped by e2e"), { timeoutMsg: "buffer not reloaded" });
    assert.ok(!readFileSync(cfg, "utf8").includes("dropped by e2e"), "the edit reached the file");
  });

  it("Save all writes the unsaved tab", async () => {
    await openCfg();
    await type("ggO# kept by e2e");
    await browser.keys(Key.Escape);
    await find(".status .mode", "NORMAL");
    await browser.waitUntil(dirty, { timeoutMsg: "tab not marked unsaved" });
    await palette("Save all");
    await fileMatches(join(cfgDir, "config.toml"), (s) => s.startsWith("# kept by e2e\n") && /^profiles = \["Work", "Home"\]$/m.test(s), "Save all didn't save config.toml");
    await browser.waitUntil(async () => !(await dirty()), { timeoutMsg: "tab still marked unsaved" });
  });

  it("switching profile logs time to that profile's log, not Work's", async () => {
    const [work, home] = ["work", "home"].map((p) => join(data, p, "timelog.jsonl"));
    mkdirSync(join(data, "home"), { recursive: true });
    writeFileSync(join(data, "home", "projects.toml"), "[Garden]\n");
    const before = readFileSync(work, "utf8");
    try {
      await palette("Time: switch profile");
      await answer("Switch profile", "Home");
      await browser.keys([Key.Ctrl, "4"]);
      await (await find(".time button", "Start tracking")).click();
      await answer("Track time on project", "Garden");
      await answer("What are you working on?", "weeding");
      await find(".time .status h2", "Garden");
      await browser.pause(1100); // a zero-length session is dropped
      await (await find(".time .buttons button", "Stop")).click();
      await answer("What did you do?", "weeded");
      await fileMatches(home, (s) => /"ev":"in".*"project":"Garden"/.test(s) && /"ev":"out".*"note":"weeded"/.test(s), "session not in Home's log");
      assert.equal(readFileSync(work, "utf8"), before, "Work's log changed");
    } finally {
      await noPicker().catch(() => browser.keys(Key.Escape));
      await palette("Time: switch profile");
      await answer("Switch profile", "Work");
      await fileMatches(join(data, "active-profile.txt"), (s) => s.trim() === "Work", "not switched back to Work");
    }
  });
});
