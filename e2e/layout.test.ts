// End-to-end tests of the responsive layout (breakpoints in +page.svelte); setup and helpers are in setup.ts.
import { after, before, describe, it } from "node:test";
import { strict as assert } from "node:assert";
import { answer, browser, close, find, Key, launch, noPicker, palette } from "./setup.ts";

const width = (w: number) => browser.waitUntil(async () => {
  await browser.setWindowSize(w, 800);
  return Math.abs((await browser.execute(() => innerWidth)) - w) < 40;
}, { timeoutMsg: `window did not resize to ${w}` });
const drawer = (on: boolean) => browser.waitUntil(async () => (await browser.execute(() => getComputedStyle([...document.querySelectorAll<HTMLElement>(".links")].find((e) => e.offsetParent)!).position === "absolute")) === on, { timeoutMsg: `links drawer not ${on ? "on" : "off"}` });
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

  it("keeps the default region order, and a layout variable moves the top bar to a left rail that still works", async () => {
    const boxes = () => browser.execute(() => Object.fromEntries(["nav", "body", "tabs", "main", "status"].map((r) => {
      const b = document.querySelector(`[data-region="${r}"]`)?.getBoundingClientRect();
      return [r, b && { top: b.top, bottom: b.bottom, left: b.left, right: b.right }];
    })));
    let b = await boxes();
    for (const r of ["nav", "body", "tabs", "main", "status"]) assert.ok(b[r], `no [data-region=${r}]`);
    assert.ok(b.nav!.bottom <= b.tabs!.top + 0.5 && b.tabs!.bottom <= b.main!.top + 0.5 && b.main!.bottom <= b.status!.top + 0.5, `default order changed: ${JSON.stringify(b)}`);
    await browser.execute(() => document.head.insertAdjacentHTML("beforeend", `<style id="rail-test">:root { --layout-shell: row; }</style>`));
    b = await boxes();
    assert.ok(b.nav!.right <= b.body!.left + 0.5, `top bar not left of the body: ${JSON.stringify(b)}`);
    await (await find(".topnav .views button", "Tasks")).click();
    await find(".topnav .views button.on", "Tasks");
    await (await find(".topnav .views button", "Today")).click();
    await find(".agenda h1", "Today");
    await browser.execute(() => document.getElementById("rail-test")?.remove());
  });

  it("lays out the emacs look as menu line, buffer line, editor and mode-line without overflowing (#1386)", async () => {
    await browser.execute(() => Object.assign(document.documentElement.dataset, { look: "emacs", layout: "emacs" }));
    const r = await browser.execute(() => {
      const box = (r: string) => document.querySelector(`[data-region="${r}"]`)!.getBoundingClientRect();
      const [nav, tabs, main, status] = ["nav", "tabs", "main", "status"].map(box);
      const nb = document.querySelector<HTMLElement>('[data-region="nav"]')!;
      return {
        order: nav.bottom <= tabs.top + 0.5 && tabs.bottom <= main.top + 0.5 && main.bottom <= status.top + 0.5,
        oneLine: nav.height < 40, logo: getComputedStyle(document.querySelector(".topnav .logo")!).display,
        overflow: document.documentElement.scrollWidth - innerWidth, navOverflow: nb.scrollWidth - nb.clientWidth,
        font: getComputedStyle(document.querySelector(".status")!).fontFamily === getComputedStyle(nb).fontFamily,
      };
    });
    assert.ok(r.order, "regions out of order");
    assert.ok(r.oneLine, "top bar is not one line");
    assert.equal(r.logo, "none", "logo still shown");
    assert.ok(r.overflow <= 0 && r.navOverflow <= 0, `overflow: ${JSON.stringify(r)}`);
    assert.ok(r.font, "top bar not monospace like the mode-line");
    await (await find(".topnav .views button", "Tasks")).click();
    await find(".topnav .views button.on", "Tasks");
    await (await find(".topnav .views button", "Today")).click();
    await find(".agenda h1", "Today");
    await browser.execute(() => { delete document.documentElement.dataset.look; delete document.documentElement.dataset.layout; });
  });

  it("gives the emacs layout a File/Edit/View/Buffers/Org/Help menu bar (F10) and line:col in the mode-line (#1448)", async () => {
    assert.equal(await displayed(".topnav .menus"), false, "menu bar outside the emacs layout");
    await browser.execute(() => void (document.documentElement.dataset.layout = "emacs"));
    await (await find(".topnav .menus button", "File")).click();
    await find(".picker label span", "File");
    await find(".picker", "New note");
    await browser.keys(Key.Escape);
    await noPicker();
    await (await find(".topnav .menus button", "Org")).click();
    await find(".picker", "Task: archive");
    await browser.keys(Key.Escape);
    await noPicker();
    await browser.keys(Key.F10);
    await find(".picker label span", "Menu bar");
    await browser.keys("v"); // one key picks the View menu
    await find(".picker label span", "View");
    await answer("View", "Go to Inbox");
    await find(".topnav .views button.on", "Inbox");
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "emacspos");
    await find(".status .file", "emacspos.org");
    await browser.waitUntil(async () => /^L\d+:C\d+ (All|Top|Bot|\d+%)$/.test(await (await browser.$(".status .pos")).getText()), { timeoutMsg: "no L:C and scroll position in the mode-line" });
    await browser.execute(() => void delete document.documentElement.dataset.layout);
    assert.equal(await displayed(".status .pos"), false, "L:C shown outside the emacs layout");
    await (await browser.$(".tabs .tab.cur .x")).click();
    await (await find(".topnav .views button", "Today")).click(); // the Inbox may be a note: leave no editor focused for the zen test
    await find(".agenda h1", "Today");
  });

  it("puts the emacs layout's panel rule on the editor side when the panels are on the left (#1448)", async () => {
    await browser.execute(() => {
      document.head.insertAdjacentHTML("beforeend", `<style id="mirror-test">:root { --layout-main: row-reverse; }</style>`);
      document.documentElement.dataset.layout = "emacs";
    });
    try {
      await width(900); // drawer-left is re-checked when the window crosses the drawer width
      await palette("Show note graph");
      await find(".links", "Graph");
      await drawer(true);
      await width(1200);
      await drawer(false);
      const edge = () => browser.execute(() => { const s = getComputedStyle([...document.querySelectorAll<HTMLElement>(".links")].find((e) => e.offsetParent)!); return [s.borderLeftWidth, s.borderRightWidth]; });
      const edgeIs = (want: string[]) => browser.waitUntil(async () => (await edge()).join() === want.join(), { timeoutMsg: `panel rule is not ${want}` }).catch(async (e) => { throw new Error(`${e.message}; got ${await edge()}; ${await browser.execute(() => `w=${innerWidth} main=${document.querySelector("main")!.className} fd=${getComputedStyle(document.querySelector("main")!).flexDirection} layout=${document.documentElement.dataset.layout}`)}`); });
      await edgeIs(["0px", "1px"]);
      await browser.execute(() => document.getElementById("mirror-test")?.remove());
      await width(900);
      await drawer(true);
      await width(1200);
      await drawer(false);
      await edgeIs(["1px", "0px"]);
    } finally { // leave no state behind for the tests after this one
      await browser.execute(() => { document.getElementById("mirror-test")?.remove(); delete document.documentElement.dataset.layout; });
      if (await displayed(".links")) await palette("Show note graph"); // toggles it off
    }
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

  it("keeps every saved view in the rail, where the column has room (#1326)", async () => {
    const folded = () => browser.execute(() => document.querySelectorAll(".topnav .views button.saved.fold").length);
    await width(900);
    await browser.waitUntil(async () => (await folded()) > 0, { timeoutMsg: "no view folded at 900 px" });
    await browser.execute(() => void (document.documentElement.dataset.layout = "rail"));
    await browser.waitUntil(async () => (await folded()) === 0, { timeoutMsg: "saved views still folded into More in the rail" });
    assert.ok(await browser.execute(() => document.querySelector(".topnav .views button.more")!.classList.contains("fold")), "More shown with nothing in it");
    await browser.execute(() => void delete document.documentElement.dataset.layout);
    await browser.waitUntil(async () => (await folded()) > 0, { timeoutMsg: "views not folded again in the top bar" });
    await width(1200);
  });

  it("opens the narrow drawer from the left when a look puts the side panels left (#1326)", async () => {
    await browser.execute(() => document.head.insertAdjacentHTML("beforeend", `<style id="mirror-test">:root { --layout-main: row-reverse; }</style>`));
    await width(900);
    await palette("Show note graph");
    await find(".links", "Graph");
    const [links, main] = await browser.execute(() => [[...document.querySelectorAll<HTMLElement>(".links")].find((e) => e.offsetParent)!, document.querySelector("main")!].map((e) => e.getBoundingClientRect().left));
    assert.ok(Math.abs(links - main) < 1, `drawer not at the left edge: ${links} vs ${main}`);
    await browser.keys(Key.Escape);
    await browser.waitUntil(async () => !(await displayed(".links")), { timeoutMsg: "drawer still open after Esc" });
    await browser.execute(() => document.getElementById("mirror-test")?.remove());
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
