// End-to-end tests of the basics; setup and helpers are in setup.ts.
import { after, before, describe, it } from "node:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { answer, browser, close, fileMatches, find, Key, launch, today, type, type Dirs } from "./setup.ts";

let notes: string, data: string;

before(async () => {
  const d: Dirs = await launch({
    config: [`templates = []`],
    files: ({ notes, data }) => {
      writeFileSync(join(notes, "scratch.org"), "#+title: Scratch\n\nFirst line.\n");
      writeFileSync(join(notes, "tasks.org"), `* TODO Fixture task\nSCHEDULED: ${today}\n`);
      writeFileSync(join(data, "work", "projects.toml"), `[Acme]\nexport_code = "ACME"\n`);
    },
  });
  ({ notes, data } = d);
});

after(close, { timeout: 30_000 });

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
