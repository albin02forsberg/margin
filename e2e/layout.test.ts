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

  it("shows the top bar with icons only under 760 px and restores the labels", async () => {
    await find(".agenda h1", "Today");
    await find(".topnav .views button.on", "Today");
    await width(700);
    assert.equal(await displayed(".topnav .lbl"), false, "labels still shown at 700 px");
    assert.ok(await displayed(".topnav .views button .ico"), "icons hidden at 700 px");
    await width(1200);
    assert.ok(await displayed(".topnav .lbl"), "labels not back at 1200 px");
  });

  it("toggles the top bar with Ctrl+\\", async () => {
    await browser.keys([Key.Ctrl, "\\"]);
    await browser.waitUntil(async () => !(await browser.$(".topnav").isExisting()), { timeoutMsg: "top bar still shown" });
    await browser.keys([Key.Ctrl, "\\"]);
    await find(".topnav .views button", "Today");
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
