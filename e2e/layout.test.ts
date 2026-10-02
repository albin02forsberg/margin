// End-to-end tests of the responsive layout (breakpoints in +page.svelte); setup and helpers are in setup.ts.
import { after, before, describe, it } from "node:test";
import { strict as assert } from "node:assert";
import { browser, close, find, Key, launch, palette } from "./setup.ts";

const width = (w: number) => browser.waitUntil(async () => {
  await browser.setWindowSize(w, 800);
  return Math.abs((await browser.execute(() => innerWidth)) - w) < 40;
}, { timeoutMsg: `window did not resize to ${w}` });
const displayed = async (css: string) => (await browser.$(css)).isDisplayed();

describe("Responsive layout", () => {
  before(async () => { await launch(); });
  after(close, { timeout: 30_000 });

  it("collapses the sidebar to an icon rail under 760 px and restores it", async () => {
    await find(".agenda h1", "Today");
    await width(700);
    assert.equal(await displayed(".side .lbl"), false, "labels still shown at 700 px");
    assert.ok((await (await browser.$(".side")).getSize("width")) < 60, "sidebar is not a rail");
    await width(1200);
    assert.ok(await displayed(".side .lbl"), "labels not back at 1200 px");
  });

  it("shows the links panel as a drawer under 1000 px, and Esc closes it", async () => {
    await width(900);
    await palette("Show note graph");
    const links = await find(".links", "Graph");
    assert.equal(await links.getCSSProperty("position").then((p) => p.value), "absolute");
    await browser.keys(Key.Escape);
    await browser.waitUntil(async () => !(await displayed(".links")), { timeoutMsg: "drawer still open after Esc" });
    await width(1200);
  });
});
