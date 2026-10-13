// End-to-end tests of the responsive layout (breakpoints in +page.svelte); setup and helpers are in setup.ts.
import { after, before, describe, it } from "node:test";
import { strict as assert } from "node:assert";
import { answer, browser, close, find, Key, launch, palette } from "./setup.ts";

const width = (w: number) => browser.waitUntil(async () => {
  await browser.setWindowSize(w, 800);
  return Math.abs((await browser.execute(() => innerWidth)) - w) < 40;
}, { timeoutMsg: `window did not resize to ${w}` });
const displayed = async (css: string) => (await browser.$(css)).isDisplayed();

describe("Responsive layout", () => {
  // Five saved views: more than the bar has room for at 900 px.
  const views = ["Alpha view", "Beta view", "Gamma view", "Delta view", "Zeta view"];
  before(async () => { await launch({ config: views.flatMap((v, i) => ["", "[[views]]", `name = "${v}"`, `query = "tag:t${i}"`]) }); });
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

  it("folds saved views that don't fit into More, which shows the open one", async () => {
    await width(900);
    await (await find(".topnav .views button.more", "More")).click();
    await answer("Saved views", "Zeta view");
    await find(".tabs .tab.cur button", "Zeta view");
    await find(".topnav .views button.more.on", "More");
    await width(1200);
  });

  it("opens Settings at the About card from the palette", async () => {
    await palette("About Margin");
    await find(".settings .about", "About");
    await browser.waitUntil(() => browser.execute(() => {
      const r = document.querySelector(".settings .about")!.getBoundingClientRect();
      return r.top >= 0 && r.bottom <= innerHeight;
    }), { timeoutMsg: "About card not scrolled into view" });
  });
});
