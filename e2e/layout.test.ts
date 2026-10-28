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

  it("hides the top bar and status bar in zen mode; the palette still opens and Esc leaves", async () => {
    await palette("View: zen mode");
    await browser.waitUntil(async () => !(await browser.$(".topnav").isExisting()), { timeoutMsg: "top bar still shown in zen" });
    assert.equal(await displayed(".status"), false, "status bar still shown in zen");
    await palette("View: zen mode"); // Ctrl+K works in zen; toggling again restores
    await find(".topnav .views button", "Today");
    assert.ok(await displayed(".status"), "status bar not back");
    await palette("View: zen mode");
    await browser.waitUntil(async () => !(await browser.$(".topnav").isExisting()), { timeoutMsg: "top bar still shown in zen" });
    await browser.keys(Key.Escape); // outside the editor Esc leaves zen
    await find(".topnav .views button", "Today");
    assert.equal(await browser.execute(() => localStorage.getItem("sidebar")), "1", "zen wrote the top bar setting");
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

  it("keeps the Vim status line below the editor's scroller, inside the card", async () => {
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "vimbar");
    await find(".status .file", "vimbar.org");
    await find(".cm-vim-panel", "--"); // a new note opens in insert mode
    const r = await browser.execute(() => {
      const box = (css: string) => document.querySelector(css)!.getBoundingClientRect();
      const [ed, sc, p] = [box(".cm-editor"), box(".cm-scroller"), box(".cm-panels-bottom")];
      return { below: p.top >= sc.bottom - 0.5, inside: p.bottom <= ed.bottom + 0.5, border: getComputedStyle(document.querySelector(".cm-panels-bottom")!).borderTopColor };
    });
    assert.ok(r.below, "Vim status line overlaps the scroller");
    assert.ok(r.inside, "Vim status line sticks out of the card");
    assert.notEqual(r.border, "rgb(221, 221, 221)", "CodeMirror's light panel edge on the card");
    await (await browser.$(".tabs .tab.cur .x")).click();
  });

  it("zooms with Ctrl+= / - / 0, also with the palette open, and remembers the level", async () => {
    const zoom = (z: string) => browser.waitUntil(async () => (await browser.execute(() => localStorage.getItem("zoom"))) === z, { timeoutMsg: `zoom not stored as ${z}` });
    await browser.keys([Key.Ctrl, "="]);
    await zoom("1.1");
    await browser.keys([Key.Ctrl, "="]);
    await browser.keys([Key.Ctrl, "-"]);
    await browser.keys([Key.Ctrl, "="]);
    await zoom("1.2");
    await browser.keys([Key.Ctrl, "p"]);
    await find(".picker label span", "Open a note");
    await browser.keys([Key.Ctrl, "0"]);
    await zoom("1");
    await browser.keys(Key.Escape);
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
