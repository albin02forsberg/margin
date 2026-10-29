// End-to-end test of the outline panel (#915); setup and helpers are in setup.ts.
import { after, before, describe, it } from "node:test";
import { join } from "node:path";
import { writeFileSync } from "node:fs";
import { answer, browser, close, find, Key, launch, palette, texts, type } from "./setup.ts";

describe("Outline panel", () => {
  before(async () => {
    await launch({
      files: ({ notes }) => writeFileSync(join(notes, "plan.org"), "#+title: Plan\n\n* TODO Draft [1/2] :work:\nbody\n** DONE Outline\n* Review\nmore\n"),
    });
  });
  after(close, { timeout: 30_000 });

  it("lists the note's headings, follows the cursor and jumps on click", async () => {
    await find(".agenda h1", "Today");
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "plan");
    await find(".status .file", "plan.org");
    await palette("View: toggle outline");
    await find(".outline button em", "TODO");
    await find(".outline button small", "[1/2]");
    await find(".outline button.done", "Outline");
    await (await find(".outline button", "Review")).click();
    await browser.waitUntil(async () => (await texts(".cm-activeLine")).some((t) => t.includes("* Review")), { timeoutMsg: "cursor not on Review" });
    await find(".outline button.cur", "Review");
    await type("kk"); // onto "body", under Draft
    await find(".outline button.cur", "Draft");
  });
});
