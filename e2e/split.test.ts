// End-to-end test of split view (#874); setup and helpers are in setup.ts.
import { after, before, describe, it } from "node:test";
import { strict as assert } from "node:assert";
import { join } from "node:path";
import { writeFileSync } from "node:fs";
import { answer, browser, close, fileMatches, find, Key, launch, palette, read, type } from "./setup.ts";

let notes: string;
const shown = (css: string) => browser.execute((c: string) => { const e = document.querySelector(c); return !!e && e.getClientRects().length > 0; }, css);

describe("Split view", () => {
  before(async () => {
    ({ notes } = await launch({
      config: ["split_view = true"],
      files: ({ notes }) => {
        writeFileSync(join(notes, "alpha.org"), "#+title: Alpha\n\nAlpha text.\n");
        writeFileSync(join(notes, "beta.org"), "#+title: Beta\n\nBeta text.\n");
      },
    }));
  });
  after(close, { timeout: 30_000 });

  it("edits a second note in the right pane and autosaves it, leaving the left note alone", async () => {
    await find(".agenda h1", "Today");
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "alpha");
    await find(".status .file", "alpha.org");
    await palette("Split view: open a note");
    await answer("Open beside this note", "beta");
    await find(".editor.split .cm-content", "Beta text.");
    await find(".status .file", "beta.org"); // the status bar follows the focused (right) pane
    await type("Go");
    await type("Typed in beta");
    await browser.keys(Key.Escape);
    await fileMatches(join(notes, "beta.org"), (s) => s.includes("Typed in beta"), "the right pane's note wasn't autosaved");
    assert.equal(read(join(notes, "alpha.org")), "#+title: Alpha\n\nAlpha text.\n", "the left pane's note changed");
    await find(".pane .editor .cm-content", "Alpha text.");
  });

  it("follows a click into the left pane, and a tab open on the right focuses that pane", async () => {
    await (await browser.$(".pane .editor .cm-content")).click();
    await find(".status .file", "alpha.org");
    await find(".tabs .tab.cur", "alpha");
    await find(".tabs .tab.cur2", "beta");
    await (await find(".tabs .tab button", "beta")).click();
    await find(".status .file", "beta.org");
    await find(".pane .editor .cm-content", "Alpha text."); // not loaded into the left pane too
    await find(".editor.split .cm-content", "Typed in beta");
  });

  it("closes the split with the right pane's tab", async () => {
    await (await browser.$(".tabs .tab.cur .x")).click();
    await browser.waitUntil(async () => !(await shown(".editor.split")), { timeoutMsg: "split pane still shown" });
    await find(".status .file", "alpha.org");
    await find(".pane .editor .cm-content", "Alpha text.");
  });
});
