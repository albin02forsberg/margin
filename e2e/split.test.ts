// End-to-end test of split view (#874); setup and helpers are in setup.ts.
import { after, before, describe, it } from "node:test";
import { strict as assert } from "node:assert";
import { join } from "node:path";
import { writeFileSync } from "node:fs";
import { answer, browser, close, fileMatches, find, Key, launch, palette, read, type } from "./setup.ts";

let notes: string;
const width = (w: number) => browser.waitUntil(async () => {
  await browser.setWindowSize(w, 800);
  return Math.abs((await browser.execute(() => innerWidth)) - w) < 40;
}, { timeoutMsg: `window did not resize to ${w}` });
const shown = (css: string) => browser.execute((c: string) => { const e = document.querySelector(c); return !!e && e.getClientRects().length > 0; }, css);

describe("Split view", () => {
  before(async () => {
    ({ notes } = await launch({
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

  it("Space w w moves focus, Space w c saves and closes, and a narrow window shows one pane", async () => {
    await palette("Split view: open a note");
    await answer("Open beside this note", "beta");
    await find(".editor.split .cm-content", "Typed in beta");
    await browser.keys(Key.Escape);
    await browser.keys(" "); await type("ww");
    await find(".status .file", "alpha.org");
    await browser.keys(" "); await type("ww");
    await find(".status .file", "beta.org");
    await type("GoSecond line");
    await browser.keys(Key.Escape);
    await browser.keys(" "); await type("ww");
    await find(".status .file", "alpha.org");
    await browser.keys(" "); await type("ww");
    await browser.keys(" "); await type("wc");
    await browser.waitUntil(async () => !(await shown(".editor.split")), { timeoutMsg: "split pane still shown" });
    await fileMatches(join(notes, "beta.org"), (s) => s.includes("Second line"), "the right pane's note wasn't saved on close");
    await find(".tabs .tab", "beta"); // still open as a normal tab, and the focused note stays on the left
    await find(".status .file", "beta.org");
    // reopen beside it, then narrow the window: the right pane hides and its tab opens as a normal one
    await palette("Split view: open a note");
    await answer("Open beside this note", "alpha");
    await find(".editor.split .cm-content", "Alpha text.");
    await width(800);
    await browser.waitUntil(async () => !(await shown(".editor.split")), { timeoutMsg: "split pane shown when narrow" });
    await find(".status .file", "beta.org"); // the narrow effect moved focus to the left pane
    await (await find(".tabs .tab button", "alpha")).click();
    await find(".status .file", "alpha.org");
    await find(".pane .editor .cm-content", "Alpha text.");
    // Space w v when narrow opens the note as a tab instead (#893)
    await palette("Split view: open a note");
    await answer("Open beside this note", "beta");
    await find(".status .file", "beta.org");
    await find(".pane .editor .cm-content", "Second line");
    assert.ok(!(await shown(".editor.split")), "split opened in a narrow window");
    await width(1280);
    // the narrow open was refused, so there is no right pane to come back
    assert.ok(!(await shown(".editor.split")), "split came back after widening");
    await find(".status .file", "beta.org");
  });

  it("keeps each pane's vim mode when switching panes (#882)", async () => {
    await palette("Split view: open a note");
    await answer("Open beside this note", "alpha"); // the previous test left beta in the left pane, and the picker hides it
    await find(".status .mode", "NORMAL");
    await type("i");
    await find(".status .mode", "INSERT");
    await (await browser.$(".pane .editor .cm-content")).click(); // the left pane is in normal mode
    await find(".status .mode", "NORMAL");
    await (await browser.$(".editor.split .cm-content")).click();
    await find(".status .mode", "INSERT");
    await browser.keys(Key.Escape);
    await find(".status .mode", "NORMAL");
  });
});
