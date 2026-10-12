// End-to-end tests of syncing external edits into open notes (doSync / mergeFromDisk in +page.svelte); setup and helpers are in setup.ts.
// External writes happen right after Esc, inside the 700 ms autosave delay, so the sync sees an unsaved tab.
import { after, before, describe, it } from "node:test";
import assert from "node:assert/strict";
import { appendFileSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { answer, browser, close, fileMatches, find, Key, launch, type } from "./setup.ts";

let file: string;
// textContent: getText is "" for the status bar's ellipsis span (see texts() in setup.ts).
const status = async () => String(await browser.$(".status .file").getProperty("textContent"));

describe("Sync with the disk", () => {
  before(async () => {
    const d = await launch({ files: ({ notes }) => writeFileSync(join(notes, "sync.org"), "#+title: Sync\n\nFirst line.\nSecond line.\n") });
    file = join(d.notes, "sync.org");
  });
  after(close, { timeout: 30_000 });

  it("shows a change made on disk in an open, unedited note", async () => {
    await find(".agenda h1", "Today");
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "sync");
    await find(".status .file", "sync.org");
    appendFileSync(file, "Appended on disk.\n");
    await find(".cm-content", "Appended on disk.");
    assert.doesNotMatch(await status(), /unsaved|changed on disk/);
  });

  it("merges a disk change into unsaved edits elsewhere in the note", async () => {
    await type("ggo");
    await type("Top edit by e2e");
    await browser.keys(Key.Escape);
    appendFileSync(file, "Second append.\n");
    await find(".cm-content", "Second append.");
    await find(".cm-content", "Top edit by e2e");
    await browser.keys([Key.Ctrl, "s"]);
    await fileMatches(file, (s) => /^#\+title: Sync\n\s*Top edit by e2e\n/.test(s) && s.endsWith("Appended on disk.\nSecond append.\n"), "merged edits not saved");
  });

  it("flags an edit to the same line as a conflict, and Ctrl+S keeps yours", async () => {
    await fileMatches(file, (s) => s.includes("Top edit by e2e"), "previous save missing");
    await type("/First line");
    await browser.keys(Key.Enter);
    await type("CMine wins.");
    await browser.keys(Key.Escape);
    writeFileSync(file, readFileSync(file, "utf8").replace("First line.", "Theirs."));
    await find(".status .file", "changed on disk");
    await browser.keys([Key.Ctrl, "s"]);
    await fileMatches(file, (s) => s.includes("Mine wins.\nSecond line.") && !s.includes("Theirs."), "your version not saved");
    await browser.waitUntil(async () => !/changed on disk/.test(await status()), { timeoutMsg: "conflict flag not cleared" });
  });
});
