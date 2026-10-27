// End-to-end tests of features beyond the basics; setup and helpers are in setup.ts.
import { after, before, describe, it } from "node:test";
import { strict as assert } from "node:assert";
import { existsSync, mkdirSync, readdirSync, readFileSync, utimesSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import { answer, browser, close, fileMatches, find, isoDay, Key, launch, noPicker, orgDay, palette, read, savedView, texts, type, type Dirs } from "./setup.ts";

describe("First run", () => {
  let d: Dirs;
  before(async () => { d = await launch({ tutorial: false }); });
  after(close, { timeout: 30_000 });

  it("asks for setup (Esc keeps the defaults), opens the tutorial, and Space ? reopens it", async () => {
    const cfg = read(join(d.cfgDir, "config.toml"));
    await find(".picker label span", "Setup: notes folder");
    await browser.keys(Key.Escape);
    assert.equal(read(join(d.cfgDir, "config.toml")), cfg);
    await find(".tabs .tab.cur button", "tutorial");
    await find(".status .file", "tutorial.org");
    assert.match(read(join(d.cfgDir, "tutorial.org")), /\S/, "tutorial.org not written");
    await (await find(".tabs .tab.cur .x", "×")).click();
    await find(".agenda h1", "Today");
    await browser.waitUntil(async () => !(await texts(".tabs .tab button")).some((t) => t.includes("tutorial")));
    await browser.keys(" ");
    await find(".menu", "Help: tutorial");
    await browser.keys("?");
    await find(".tabs .tab.cur button", "tutorial");
  });
});

describe("First-run setup", () => {
  let d: Dirs;
  before(async () => { d = await launch({ tutorial: false }); });
  after(close, { timeout: 30_000 });

  it("saves the answers to config.toml and creates the folders", async () => {
    const notes = join(d.root, "my notes");
    await answer("Setup: notes folder", notes);
    await answer("Setup: time-tracking folder", join(d.root, "time"));
    await answer("Setup: profiles", "Job, Home");
    await find(".tabs .tab.cur button", "tutorial");
    const cfg = read(join(d.cfgDir, "config.toml"));
    assert.ok(cfg.includes(`notes_dir = ${JSON.stringify(notes)}\n`) && cfg.includes(`profiles = ["Job", "Home"]\n`), cfg);
    assert.ok(cfg.includes(`capture_shortcut = ""\n`), "other settings lost");
    assert.ok(existsSync(notes) && existsSync(join(d.root, "time")), "folders not created");
  });
});

describe("Broken config.toml", () => {
  let d: Dirs;
  const bad = Buffer.from('notes_dir = "\xff"\n', "latin1"); // not UTF-8
  before(async () => { d = await launch({ files: ({ cfgDir }) => writeFileSync(join(cfgDir, "config.toml"), bad) }); });
  after(close, { timeout: 30_000 });

  it("says it can't load the file and leaves it alone", async () => {
    await find(".status .msg", "config.toml couldn't be loaded");
    assert.deepEqual(readFileSync(join(d.cfgDir, "config.toml")), bad);
    // The default ~/timeclock landed under the temp HOME, not the real one.
    assert.ok(existsSync(join(d.root, "timeclock")), "default data dir not under the temp HOME");
  });
});

describe("Last session's tabs", () => {
  let d: Dirs;
  before(async () => {
    d = await launch({
      files: ({ notes }) => {
        writeFileSync(join(notes, "one.org"), "#+title: One\n\nFirst.\n");
        writeFileSync(join(notes, "two.org"), "#+title: Two\n\nline a\nline b\nline c\n");
      },
    });
  });
  after(close, { timeout: 30_000 });

  const tabNames = () => texts(".tabs .tab > button:not(.x)");

  it("reopens after a restart with the active tab and its line, skipping a note that is gone", async () => {
    await find(".agenda h1", "Today");
    for (const n of ["Two", "One", "Two"]) {
      await browser.keys([Key.Ctrl, "p"]);
      await answer("Open a note", n);
      await find(".status .file", `${n.toLowerCase()}.org`);
    }
    await find(".status .file", "two.org");
    await browser.keys(Key.Escape);
    await type("4G"); // "line b"
    await browser.refresh();
    await browser.waitUntil(async () => (await tabNames()).length === 3, { timeoutMsg: "tabs not restored" });
    assert.match((await tabNames())[0], /Today/);
    await find(".tabs .tab.cur button", "two");
    await find(".status .file", "two.org");
    await type("iX");
    await browser.keys(Key.Escape);
    await browser.keys([Key.Ctrl, "s"]);
    await fileMatches(join(d.notes, "two.org"), (s) => s.includes("line a\nXline b\n"), "restored tab lost its line");

    // A note deleted while the app was closed is skipped quietly.
    await browser.execute((gone) => {
      const s = JSON.parse(localStorage.getItem("session")!);
      s.tabs.unshift({ path: gone, line: 0 });
      s.active++;
      localStorage.setItem("session", JSON.stringify(s));
      Storage.prototype.setItem = () => {}; // until the reload, so the unload save keeps this session
    }, join(d.notes, "gone.org"));
    await browser.refresh();
    await browser.waitUntil(async () => (await tabNames()).length === 3, { timeoutMsg: "tabs not restored" });
    await find(".tabs .tab.cur button", "two");
    const msgs = (await texts(".msg")).filter((m) => m.includes("⚠"));
    assert.deepEqual(msgs, [], `flashed an error: ${msgs.join(" | ")}`);
  });
});

describe("Features", () => {
  let d: Dirs;
  before(async () => {
    d = await launch({
      config: [
        `[[templates]]`,
        `key = "s"`,
        `name = "Standup"`,
        `file = "standups.org"`,
        `body = "* Standup with %^{Who}\\n- %?topics\\n"`,
      ],
      files: ({ notes }) => {
        writeFileSync(join(notes, "habits.org"), `* TODO Floss\nSCHEDULED: <${orgDay(new Date())} .+1d>\n:PROPERTIES:\n:STYLE: habit\n:END:\n`);
        writeFileSync(join(notes, "tasks.org"), "* TODO Write report :work:\n* TODO Buy milk :home:\n* NEXT Call the bank :work:money:\n");
        writeFileSync(join(notes, "alpha.org"), "#+title: Alpha Project\n\nAbout it.\n");
        writeFileSync(join(notes, "beta.org"), "#+title: Beta\n\nWe discussed the Alpha Project today.\n");
        writeFileSync(join(notes, "hub.org"), "#+title: Hub\n\nSee [[id:leaf-id][the leaf]].\n");
        writeFileSync(join(notes, "leaf.org"), ":PROPERTIES:\n:ID: leaf-id\n:END:\n#+title: Leaf\n\nA leaf.\n");
        writeFileSync(join(notes, "exportme.org"), "#+title: Export me\n\nSome *bold* text.\n\n* A heading\n- one item\n");
        writeFileSync(join(notes, "draft.org"), "#+title: Draft\n\nDraft text.\n");
        writeFileSync(join(notes, "oldie.org"), "#+title: Oldie\n\nOld text.\n");
        writeFileSync(join(notes, "wanderer.org"), "#+title: Wanderer\n\nSee [[file:elsewhere.org][Elsewhere]] and [[file:attachments/wanderer/x.png]].\n");
        writeFileSync(join(notes, "signpost.org"), "#+title: Signpost\n\nGo to [[file:wanderer.org::*Top][the wanderer]].\n");
        writeFileSync(join(notes, "refs.org"), "#+title: Refs\n\nSee [[file:draft.org::*Top][the draft]] and [[file:drafts.org]].\n");
        writeFileSync(join(notes, "doomed.org"), ":PROPERTIES:\n:ID: doomed-id\n:END:\n#+title: Doomed\n\nDoomed text.\n");
        writeFileSync(join(notes, "fan.org"), "#+title: Fan\n\nSee [[id:doomed-id][the doomed note]].\n");
        writeFileSync(join(notes, "embed.org"), "#+title: Embed\n\n#+TRANSCLUDE: [[file:source.org::*Agenda]]\nAfter.\n");
        writeFileSync(join(notes, "source.org"), "#+title: Source\n\n* Agenda\n- reuse me\n");
      },
    });
  });
  after(close, { timeout: 30_000 });

  it("shows a habit's bar in Today, and x logs it done", async () => {
    await find(".agenda h1", "Today");
    const row = await find(".agenda .row", "Floss");
    assert.ok(await row.$(".habit").isDisplayed(), "no habit bar");
    await (await row.$(".title")).click();
    await browser.keys("x");
    const tomorrow = new Date(); tomorrow.setDate(tomorrow.getDate() + 1);
    await fileMatches(join(d.notes, "habits.org"), (s) =>
      s.startsWith(`* TODO Floss\nSCHEDULED: <${orgDay(tomorrow)} .+1d>\n`) &&
      new RegExp(`:LAST_REPEAT: \\[${isoDay(new Date())} \\w+ \\d\\d:\\d\\d\\]`).test(s) &&
      new RegExp(`- State "DONE"\\s+from "TODO"\\s+\\[${isoDay(new Date())} \\w+ \\d\\d:\\d\\d\\]`).test(s), "habit not logged");
  });

  it("Space x H lists habits with streaks and on-time rate; r switches to 90 days (#802)", async () => {
    await palette("Habits: overview");
    await find(".habits .habit .pct", "100%"); // Floss, done today above
    await (await find(".habits h1", "Habits")).click();
    await browser.keys("r");
    await find(".habits .days", "Last 90 days");
    await (await find(".habits .habit button", "Floss")).click();
    await find(".status .file", "habits.org");
  });

  it("searches tasks, saves the search as a view and renames it", async () => {
    await palette("Search tasks");
    await answer("Search tasks", "tag:work");
    await find(".agenda h1", "tag:work");
    await browser.waitUntil(async () => (await texts(".agenda .row .title")).sort().join("|") === "Call the bank|Write report", {
      timeoutMsg: "search tab doesn't list exactly the tag:work tasks",
    });

    const cfg = join(d.cfgDir, "config.toml");
    await palette("Save this search as a view");
    await answer("Save this search as a view named", "Work stuff");
    await savedView("Work stuff");
    await find(".tabs .tab.cur button", "Work stuff");
    await fileMatches(cfg, (s) => s.endsWith(`\n[[views]]\nname = "Work stuff"\nquery = "tag:work"\n`), "view not appended");
    const before = read(cfg);

    await palette("Save this search as a view");
    await answer("already shows this search", "Rename existing view");
    await answer("Rename “Work stuff” to", "Office");
    await find(".tabs .tab.cur button", "Office");
    await savedView("Office");
    await fileMatches(cfg, (s) => s === before.replace(`name = "Work stuff"`, `name = "Office"`), "view not renamed in place");
  });

  it("captures with a template and opens the editor at %?", async () => {
    await browser.keys([Key.Ctrl, "1"]);
    await find(".agenda h1", "Today");
    await browser.keys(" ");
    await browser.keys("c");
    await answer("Capture with template", "Standup");
    await answer("Who", "Alice");
    const file = join(d.notes, "standups.org");
    await fileMatches(file, (s) => s.includes("* Standup with Alice\n- topics\n"), "entry not filed");
    await find(".status .file", "standups.org");
    await type("ibudget ");
    await browser.keys(Key.Escape);
    await browser.keys([Key.Ctrl, "s"]);
    await fileMatches(file, (s) => s.includes("* Standup with Alice\n- budget topics\n"), "cursor wasn't at %?");
  });

  it("Insert date keeps the repeater", async () => {
    await palette("Insert date");
    await answer("Insert date", "today +1w");
    await browser.keys([Key.Ctrl, "s"]);
    await fileMatches(join(d.notes, "standups.org"), (s) => s.includes(`<${orgDay(new Date())} +1w>`), "stamp lost its repeater");
  });

  it("lists an unlinked mention and links it", async () => {
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "Alpha Project"); // by #+title, though alpha.org has no :ID:
    await find(".status .file", "alpha.org");
    await browser.keys(" ");
    await type("nb");
    const mention = await find(".links .mention", "We discussed the Alpha Project today.");
    await (await mention.$(".link-it")).click();
    let id = "";
    await fileMatches(join(d.notes, "alpha.org"), (s) => { id = s.match(/^:PROPERTIES:\n:ID:\s+(\S+)\n:END:\n#\+title: Alpha Project\n/)?.[1] ?? ""; return !!id; }, "alpha.org got no :ID:");
    await fileMatches(join(d.notes, "beta.org"), (s) => s.includes(`We discussed the [[id:${id}][Alpha Project]] today.`), "mention not linked");
    await find(".links > button", "Beta"); // now under "Linked from"
    await browser.keys(" ");
    await type("ng");
    await find(".links h3", "Graph 2");
  });

  it("renames a note and updates the file: links to it", async () => {
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "Draft");
    await find(".status .file", "draft.org");
    await palette("Rename note");
    await answer("Rename note to", "final");
    await find(".status .file", "final.org");
    await find(".status .msg", "updated links in 1 note");
    assert.ok(existsSync(join(d.notes, "final.org")) && !existsSync(join(d.notes, "draft.org")), "not renamed");
    assert.equal(read(join(d.notes, "refs.org")), "#+title: Refs\n\nSee [[file:final.org::*Top][the draft]] and [[file:drafts.org]].\n");
    assert.equal(read(join(d.notes, "final.org")), "#+title: Draft\n\nDraft text.\n");
  });

  it("moves a note to a new folder and keeps links to and from it working", async () => {
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "Wanderer");
    await find(".status .file", "wanderer.org");
    await palette("Move note to folder");
    await answer("to folder", "archive/2026");
    await find(".status .msg", "Moved to archive/2026/ — updated links in 1 note");
    const to = join(d.notes, "archive", "2026", "wanderer.org");
    assert.ok(existsSync(to) && !existsSync(join(d.notes, "wanderer.org")), "not moved");
    assert.equal(read(to), "#+title: Wanderer\n\nSee [[file:../../elsewhere.org][Elsewhere]] and [[file:../../attachments/wanderer/x.png]].\n");
    assert.equal(read(join(d.notes, "signpost.org")), "#+title: Signpost\n\nGo to [[file:archive/2026/wanderer.org::*Top][the wanderer]].\n");
  });

  it("reopens a closed tab under the note's new name after a rename", async () => {
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "Oldie");
    await find(".status .file", "oldie.org");
    await browser.keys(Key.Escape);
    await browser.keys(" ");
    await type("bd");
    await browser.waitUntil(async () => !(await texts(".tabs .tab button")).some((t) => t.includes("Oldie")), { timeoutMsg: "tab not closed" });
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "Oldie");
    await find(".status .file", "oldie.org");
    await palette("Rename note");
    await answer("Rename note to", "newbie");
    await find(".status .file", "newbie.org");
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "Alpha");
    await find(".status .file", "alpha.org");
    await browser.keys(Key.Escape);
    await browser.keys(" ");
    await type("bu");
    try {
      await find(".status .file", "newbie.org");
    } catch (e) {
      throw new Error(`${e}\nstatus msg: ${JSON.stringify(await texts(".status .msg"))}`); // "No closed tabs" = the entry was pruned
    }
    assert.ok(!existsSync(join(d.notes, "oldie.org")), "old name recreated");
  });

  it("moves a note to .trash after a confirm naming its backlinks", async () => {
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "Doomed");
    await find(".status .file", "doomed.org");
    await palette("Delete note");
    await answer("1 note links to it", "Move");
    await find(".status .msg", "Moved to .trash");
    assert.ok(!existsSync(join(d.notes, "doomed.org")), "note not moved");
    assert.equal(read(join(d.notes, ".trash", "doomed.org")), ":PROPERTIES:\n:ID: doomed-id\n:END:\n#+title: Doomed\n\nDoomed text.\n");
    assert.ok(!(await texts(".tabs .tab button")).some((t) => t.includes("doomed")), "tab still open");
    // ...and back again.
    await palette("Restore note from .trash");
    await answer("Restore note from .trash", "doomed");
    await find(".status .msg", "Restored doomed.org");
    assert.ok(!existsSync(join(d.notes, ".trash", "doomed.org")), "still in .trash");
    assert.ok(existsSync(join(d.notes, "doomed.org")), "note not restored");
    await find(".status .file", "doomed.org");
  });

  it("copies the open file's path to the clipboard from the palette", async () => {
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "Export me");
    await find(".status .file", "exportme.org");
    // Stub the clipboard: WebKitWebDriver may refuse readText() without a user gesture.
    await browser.execute(() => {
      const w = window as unknown as { __copied: string[] };
      w.__copied = [];
      Object.defineProperty(navigator.clipboard, "writeText", { configurable: true, value: async (s: string) => { w.__copied.push(s); } });
    });
    await palette("Note: copy file path");
    await find(".status .msg", "Copied");
    const copied = await browser.execute(() => (window as unknown as { __copied: string[] }).__copied);
    assert.deepEqual(copied, [join(d.notes, "exportme.org")]);
  });

  it("exports a note as Markdown", async () => {
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "Export me");
    await find(".status .file", "exportme.org");
    await browser.keys(" ");
    await type("em");
    const md = join(d.exportDir, "exportme.md");
    await find(".picker label span", "Exported to");
    await browser.keys(Key.Escape);
    assert.ok(existsSync(md), "no exportme.md");
    assert.match(read(md), /^# Export me\n\nSome \*\*bold\*\* text\.\n[\s\S]*A heading[\s\S]*- one item\n/);

    // Again: asks before overwriting; "Keep both" writes a numbered copy.
    await browser.keys(" ");
    await type("em");
    await answer("exportme.md already exists", "Keep both");
    await find(".picker label span", "Exported to");
    await browser.keys(Key.Escape);
    assert.equal(read(join(d.exportDir, "exportme (2).md")), read(md));

    // Copy as Markdown (the webview blocks reading the clipboard back).
    await browser.keys(" ");
    await type("ec");
    await find(".status .msg", "Copied as Markdown");

    // Copy as rich text.
    await browser.keys(" ");
    await type("eC");
    await find(".status .msg", "Copied as rich text");
  });

  it("shows a #+TRANSCLUDE: target under the line, refreshes it on change and jumps to it on click", async () => {
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "embed");
    await find(".status .file", "embed.org");
    await find(".cm-transclude", "reuse me");
    writeFileSync(join(d.notes, "source.org"), "#+title: Source\n\n* Agenda\n- changed on disk\n");
    await find(".cm-transclude", "changed on disk");
    assert.equal(read(join(d.notes, "embed.org")), "#+title: Embed\n\n#+TRANSCLUDE: [[file:source.org::*Agenda]]\nAfter.\n");
    await (await find(".cm-transclude", "changed on disk")).click();
    await find(".status .file", "source.org");
  });

  it("exports a note and the notes it links to as linked HTML pages", async () => {
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "hub");
    await find(".status .file", "hub.org");
    await browser.keys(" ");
    await type("ea");
    await answer("Include notes", "Linked from this note");
    await answer("Export as", "HTML");
    await find(".picker label span", "Exported to");
    await browser.keys(Key.Escape);
    const dir = join(d.exportDir, "hub");
    assert.match(read(join(dir, "hub.html")), /<a href="leaf\.html"[^>]*>the leaf<\/a>/);
    assert.match(read(join(dir, "leaf.html")), /A leaf\./);

    // Overwriting moves pages no longer in the export into old/, and leaves other files alone.
    writeFileSync(join(dir, "gone.html"), "old");
    writeFileSync(join(dir, "pic.png"), "png");
    await browser.keys(" ");
    await type("ea");
    await answer("Include notes", "Linked from this note");
    await answer("Export as", "Markdown");
    await answer("hub already exists", "Overwrite");
    await find(".picker label span", "Exported to");
    await browser.keys(Key.Escape);
    assert.match(read(join(dir, "hub.md")), /\[the leaf\]\(leaf\.md\)/);
    assert.ok(existsSync(join(dir, "gone.html")), "Markdown export removed an HTML page");
    await browser.keys(" ");
    await type("ea");
    await answer("Include notes", "Linked from this note");
    await answer("Export as", "HTML");
    await answer("hub already exists", "Overwrite");
    await find(".picker label span", "Exported to");
    await browser.keys(Key.Escape);
    assert.ok(!existsSync(join(dir, "gone.html")), "stale page kept");
    assert.equal(read(join(dir, "old", "gone.html")), "old", "stale page not moved to old/");
    assert.ok(existsSync(join(dir, "leaf.html")) && existsSync(join(dir, "pic.png")) && existsSync(join(dir, "hub.md")));
  });

  it("reopens a closed tab at the line it was on", async () => {
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "Beta");
    await find(".status .file", "beta.org");
    await browser.keys(Key.Escape);
    await browser.keys("G");
    await browser.keys(" ");
    await type("bd");
    await browser.waitUntil(async () => !(await texts(".tabs .tab button")).some((t) => t.includes("Beta")), { timeoutMsg: "tab not closed" });
    await browser.keys(" ");
    await type("bu");
    await find(".status .file", "beta.org");
    await type("iX");
    await browser.keys(Key.Escape);
    await browser.keys([Key.Ctrl, "s"]);
    await fileMatches(join(d.notes, "beta.org"), (s) => s.endsWith("today.\nX"), "reopened tab lost its line");
  });

  it("closes other tabs and a middle-clicked tab; the tab tooltip is the path", async () => {
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "Beta");
    await find(".status .file", "beta.org");
    const tip = await browser.execute(() => [...document.querySelectorAll(".tabs .tab > button:not(.x)")].map((b) => b.getAttribute("title") ?? "").join("|"));
    assert.ok(tip.includes("beta.org"), "tab has no path tooltip");
    await browser.keys(Key.Escape);
    await browser.keys(" ");
    await type("bo");
    await browser.waitUntil(async () => (await texts(".tabs .tab > button:not(.x)")).length === 1, { timeoutMsg: "other tabs not closed" });
    await browser.keys([Key.Ctrl, Key.Shift, "t"]);
    await browser.waitUntil(async () => (await texts(".tabs .tab > button:not(.x)")).length === 2, { timeoutMsg: "closed tab not reopened" });
    // middle-click: auxclick on the first tab
    await browser.execute(() => document.querySelector(".tabs .tab")?.dispatchEvent(new MouseEvent("auxclick", { button: 1, bubbles: true })));
    await browser.waitUntil(async () => (await texts(".tabs .tab > button:not(.x)")).length === 1, { timeoutMsg: "middle-click didn't close the tab" });
  });

  it("reorders tabs with Space b < / > and by dragging, keeping the active tab", async () => {
    const names = () => texts(".tabs .tab > button:not(.x)");
    for (const n of ["Beta", "Alpha"]) {
      await browser.keys([Key.Ctrl, "p"]);
      await answer("Open a note", n);
      await find(".status .file", `${n.toLowerCase()}.org`);
    }
    await browser.keys(Key.Escape);
    for (let i = 0; i < 2; i++) { await browser.keys(" "); await type("b>"); } // Alpha to the end (clamped there)
    const o = await names();
    assert.ok(o.length >= 2);
    const swapped = [...o.slice(0, -2), o[o.length - 1], o[o.length - 2]];
    await browser.keys(" ");
    await type("b<");
    await browser.waitUntil(async () => (await names()).join() === swapped.join(), { timeoutMsg: "tab not moved left" });
    await find(".tabs .tab.cur button", "alpha");
    await browser.keys(" ");
    await type("b>");
    await browser.waitUntil(async () => (await names()).join() === o.join(), { timeoutMsg: "tab not moved right" });
    // drag the first tab to the end with pointer events
    await browser.execute(() => {
      const t = [...document.querySelectorAll<HTMLElement>(".tabs .tab")];
      const r = t[0].getBoundingClientRect(), last = t[t.length - 1].getBoundingClientRect();
      const ev = (type: string, x: number) => new PointerEvent(type, { clientX: x, clientY: r.top + 2, button: 0, bubbles: true });
      t[0].dispatchEvent(ev("pointerdown", r.left + 5));
      window.dispatchEvent(ev("pointermove", last.right - 3));
      window.dispatchEvent(ev("pointerup", last.right - 3));
    });
    const dragged = [...o.slice(1), o[0]];
    await browser.waitUntil(async () => (await names()).join() === dragged.join(), { timeoutMsg: "drag didn't reorder tabs" });
    await find(".tabs .tab.cur button", "alpha");
  });

  it("closes the tabs around a middle tab without touching their files or the kept tab's fresh edits", async () => {
    const files = ["hub.org", "leaf.org"].map((f) => join(d.notes, f));
    const before = files.map(read);
    for (const n of ["Hub", "Alpha", "Leaf", "Alpha"]) {
      await browser.keys([Key.Ctrl, "p"]);
      await answer("Open a note", n);
    }
    await find(".status .file", "alpha.org");
    await browser.keys(Key.Escape);
    await browser.keys("G");
    await type("oZZ");
    await browser.keys(Key.Escape);
    await browser.keys(" "); // straight away, before the autosave
    await type("bo");
    await browser.waitUntil(async () => (await texts(".tabs .tab > button:not(.x)")).length === 1, { timeoutMsg: "other tabs not closed" });
    await find(".tabs .tab.cur button", "alpha");
    await find(".status .file", "alpha.org");
    await fileMatches(join(d.notes, "alpha.org"), (s) => s.includes("\nZZ"), "the kept tab lost its edit");
    assert.ok((await texts(".cm-content")).some((t) => t.includes("ZZ")), "the editor lost the kept tab's edit");
    assert.deepEqual(files.map(read), before, "a closed note's file changed");
    await browser.keys([Key.Ctrl, Key.Shift, "t"]);
    await browser.waitUntil(async () => (await texts(".tabs .tab > button:not(.x)")).length === 2, { timeoutMsg: "closed tab not reopened" });
  });
});

describe("Task keys", () => {
  let d: Dirs;
  const file = () => join(d.notes, "keys.org");
  const addDays = (n: number) => { const t = new Date(); t.setDate(t.getDate() + n); return t; };
  const day = (n: number) => `<${orgDay(addDays(n))}>`;
  /** Click TITLE's row in Today (re-found by text: the agenda re-sorts after each change). */
  const select = async (title: string) => { await (await find(".agenda .row .title", title)).click(); };
  before(async () => {
    d = await launch({
      files: ({ notes }) => {
        writeFileSync(join(notes, "keys.org"), `* TODO Shift me\nSCHEDULED: ${day(0)}\n* TODO [#B] Rank me\nSCHEDULED: ${day(0)}\n* TODO Due me\nSCHEDULED: ${day(0)}\n* TODO Finish me\nSCHEDULED: ${day(0)}\n* TODO Repeat me\nSCHEDULED: ${day(0)}\n`);
        writeFileSync(join(notes, "overdue.org"), `* TODO Late one\nSCHEDULED: <${orgDay(addDays(-5))} 09:00 +1w>\n* TODO Late two\nDEADLINE: ${day(4)} SCHEDULED: ${day(-2)}\n* DONE Late done\nSCHEDULED: ${day(-3)}\n`);
        writeFileSync(join(notes, "minutes.org"), "#+title: Minutes\n\nIntro line.\nThe quarterly budget review happened on Tuesday.\nOutro line.\n");
      },
    });
  });
  after(close, { timeout: 30_000 });

  it("> pushes the schedule a day later and u restores it", async () => {
    await find(".agenda h1", "Today");
    const orig = read(file());
    await select("Shift me");
    await browser.keys(">");
    await fileMatches(file(), (s) => s.includes(`* TODO Shift me\nSCHEDULED: ${day(1)}\n`), "task not rescheduled to tomorrow");
    await browser.keys("u"); // straight away: the task has left Today
    await fileMatches(file(), (s) => s === orig, "undo didn't restore the file");
  });

  it("* marks tasks, > pushes all marked and one u restores them (#633)", async () => {
    const orig = read(file());
    await select("Shift me");
    await browser.keys("*");
    await select("Due me");
    await browser.keys("*");
    await find(".agenda .titles span", "2 marked");
    await browser.keys(">");
    await fileMatches(file(), (s) => s.includes(`* TODO Shift me\nSCHEDULED: ${day(1)}\n`) && s.includes(`* TODO Due me\nSCHEDULED: ${day(1)}\n`)
      && s.includes(`* TODO [#B] Rank me\nSCHEDULED: ${day(0)}\n`), "marked tasks not both rescheduled (or an unmarked one moved)");
    await browser.keys("u");
    await fileMatches(file(), (s) => s === orig, "one undo didn't restore both tasks");
  });

  it("s on marked tasks asks once and schedules all; Esc on d writes nothing and keeps the marks (#638)", async () => {
    const orig = read(file());
    await select("Shift me");
    await browser.keys("*");
    await select("Due me");
    await browser.keys("*");
    await find(".agenda .titles span", "2 marked");
    await browser.keys("d");
    await find(".picker label span", "Due date for 2 tasks");
    await browser.keys(Key.Escape);
    await noPicker();
    await find(".agenda .titles span", "2 marked");
    assert.equal(read(file()), orig, "a cancelled prompt wrote the file");
    await browser.keys("s");
    await answer("Schedule 2 tasks for", "+2d");
    await fileMatches(file(), (s) => s.includes(`* TODO Shift me\nSCHEDULED: ${day(2)}\n`) && s.includes(`* TODO Due me\nSCHEDULED: ${day(2)}\n`)
      && s.includes(`* TODO [#B] Rank me\nSCHEDULED: ${day(0)}\n`), "marked tasks not both scheduled (or an unmarked one moved)");
    await browser.keys("u");
    await fileMatches(file(), (s) => s === orig, "one undo didn't restore both tasks");
  });

  it("T moves every overdue task to today, keeping time and repeater, and u restores them (#599)", async () => {
    const f = join(d.notes, "overdue.org"), orig = read(f);
    await select("Shift me");
    await browser.keys("T");
    await answer("Move 2 overdue tasks to", "today");
    await fileMatches(f, (s) => s === `* TODO Late one\nSCHEDULED: <${orgDay(new Date())} 09:00 +1w>\n* TODO Late two\nDEADLINE: ${day(4)} SCHEDULED: ${day(0)}\n* DONE Late done\nSCHEDULED: ${day(-3)}\n`, "overdue tasks not moved to today");
    await noPicker();
    await select("Late one");
    await browser.keys("u");
    await fileMatches(f, (s) => s === orig, "undo didn't restore both tasks");
  });

  it("+ raises the priority and - lowers it again", async () => {
    await select("Rank me");
    await browser.keys("+");
    await fileMatches(file(), (s) => s.includes("* TODO [#A] Rank me\n"), "priority not raised");
    await browser.keys("-");
    await fileMatches(file(), (s) => s.includes("* TODO [#B] Rank me\n"), "priority not lowered");
  });

  it("e renames a task and keeps its status and priority", async () => {
    await select("Rank me");
    await browser.keys("e");
    await answer("Task title", "Rank me higher");
    await fileMatches(file(), (s) => s.includes("* TODO [#B] Rank me higher\n"), "task not renamed");
  });

  it("z adds a timestamped note under the task and u removes it", async () => {
    const orig = read(file());
    await select("Finish me");
    await browser.keys("z");
    await answer("Note", "Called Bob");
    await fileMatches(file(), (s) => new RegExp(`\\* TODO Finish me\\nSCHEDULED: <[^>]+>\\n- Note taken on \\[[^\\]]+\\] \\\\\\\\\\n  Called Bob\\n`).test(s), "note not added under the task");
    await browser.keys("u");
    await fileMatches(file(), (s) => s === orig, "undo didn't remove the note");
  });

  it("y copies a link to the task, which gives it an :ID:", async () => {
    await select("Finish me");
    await browser.keys("y");
    await fileMatches(file(), (s) => /\* TODO Finish me\n[^*]*:PROPERTIES:\n:ID: +\S+\n:END:/.test(s), "task got no :ID: drawer");
  });

  it("d sets the due date from +3d", async () => {
    await select("Due me");
    await browser.keys("d");
    await answer("Due date", "+3d");
    await fileMatches(file(), (s) => new RegExp(`\\* TODO Due me\\n[^*]*DEADLINE: ${day(3)}`).test(s), "deadline not set three days out");
  });

  it("s offers Every week, which adds a weekly repeater", async () => {
    await select("Repeat me");
    await browser.keys("s");
    await answer("Schedule for", "every week");
    await fileMatches(file(), (s) => s.includes(`* TODO Repeat me\nSCHEDULED: <${orgDay(new Date())} +1w>\n`), "weekly repeater not set");
  });

  it("D jumps the agenda to a date", async () => {
    await select("Due me"); // focus the agenda: the previous picker may have left focus on body (#394)
    await browser.keys("D");
    await answer("Go to date", "+14d");
    const t = new Date(); t.setDate(t.getDate() + 14);
    await find(".agenda h1", t.toLocaleDateString("en-US", { weekday: "long", day: "numeric", month: "long" }));
    await browser.keys(".");
    await find(".agenda h1", "Today");
  });

  it("x then c lists the task under today's Done group, with its close time", async () => {
    await select("Finish me");
    await browser.keys("x");
    await fileMatches(file(), (s) => /\* DONE Finish me\n[^*]*CLOSED: \[/.test(s), "task not closed");
    await browser.keys("c");
    await find(".agenda .logh", "Done");
    await browser.waitUntil(async () => (await texts(".agenda .logh ~ .row")).some((t) => /Finish me[\s\S]*\d\d:\d\d/.test(t)), { timeoutMsg: "done task not under Done with its time" });
    assert.equal((await texts(".agenda .row .title")).filter((t) => t === "Finish me").length, 1, "done task listed twice");
    await browser.keys("c");
    await browser.waitUntil(async () => !(await texts(".agenda .logh")).length, { timeoutMsg: "c didn't hide the Done group" });
  });

  it("Ctrl+Shift+F finds a line by words in any order and opens the note there", async () => {
    await browser.keys([Key.Ctrl, Key.Shift, "f"]);
    await find(".picker label span", "Search all notes");
    await type("review budget");
    // Results are async and come per keystroke: wait for the final list (one hit) before Enter.
    await browser.waitUntil(async () => { const t = await texts(".picker li"); return t.length === 1 && t[0].includes("quarterly budget review"); }, { timeoutMsg: "search didn't list exactly the matching line" });
    await browser.keys(Key.Enter);
    await find(".status .file", "minutes.org");
    await browser.waitUntil(async () => (await texts(".cm-activeLine")).some((t) => t.includes("quarterly budget")), { timeoutMsg: "cursor not on the matched line" });
  });
});

describe("Tags", () => {
  let d: Dirs;
  const day = `<${orgDay(new Date())}>`;
  before(async () => {
    d = await launch({
      files: ({ notes }) => {
        writeFileSync(join(notes, "tags.org"), `* TODO Tag me :alpha:beta:\nSCHEDULED: ${day}\n* TODO Other :gamma:\n`);
        writeFileSync(join(notes, "strip.org"), "* TODO Strip me :xray:yank:\n");
        writeFileSync(join(notes, "bulktags.org"), `* TODO Bulk one :x:\nSCHEDULED: ${day}\n* TODO Bulk two :y:\nSCHEDULED: ${day}\n`);
        writeFileSync(join(notes, "effort.org"), `* TODO Estimate me\nSCHEDULED: ${day}\n:PROPERTIES:\n:ID: e1\n:END:\n`);
        writeFileSync(join(notes, "inh.org"), "#+FILETAGS: :work:\n* Proj :proj:\n** TODO Child task\n* TODO Loose task\n");
        writeFileSync(join(notes, "park.org"), `* TODO Park me\nSCHEDULED: ${day}\n`);
      },
    });
  });
  after(close, { timeout: 30_000 });

  it("# toggles an existing tag, adds a new one with Ctrl+Enter and ✓ Done saves", async () => {
    await find(".agenda h1", "Today");
    await (await find(".agenda .row .title", "Tag me")).click();
    await browser.keys("#");
    await find(".picker label span", "Tags: alpha, beta");
    await type("beta");
    await browser.keys(Key.Enter); // toggles beta off
    await find(".picker label span", "Tags: alpha");
    await type("gamma");
    await browser.keys(Key.Enter); // an existing tag from another heading
    await find(".picker label span", "Tags: alpha, gamma");
    await type("fresh");
    await browser.keys([Key.Ctrl, Key.Enter]);
    await find(".picker label span", "Tags: alpha, gamma, fresh");
    await type("done");
    await browser.keys(Key.Enter);
    await fileMatches(join(d.notes, "tags.org"), (s) => s.includes("* TODO Tag me :alpha:gamma:fresh:\nSCHEDULED:"), "tags not rewritten in order");
  });

  it("# on marked tasks: + tag adds to all, - tag removes from all, nothing toggles (#638)", async () => {
    const f = join(d.notes, "bulktags.org");
    const markBoth = async () => {
      for (const t of ["Bulk one", "Bulk two"]) { await (await find(".agenda .row .title", t)).click(); await browser.keys("*"); }
      await find(".agenda .titles span", "2 marked");
    };
    await markBoth();
    await browser.keys("#");
    await answer("Tags of 2 tasks", "+ x"); // Bulk one already has x: it must keep it
    await fileMatches(f, (s) => s.includes("* TODO Bulk one :x:\n") && s.includes("* TODO Bulk two :y:x:\n"), "+x not added to every marked task");
    await markBoth();
    await browser.keys("#");
    await answer("Tags of 2 tasks", "- y"); // only Bulk two has y
    await fileMatches(f, (s) => s.includes("* TODO Bulk one :x:\n") && s.includes("* TODO Bulk two :x:\n"), "-y not removed from every marked task");
  });

  it("E sets an effort estimate: badge on the row, total in the day header, :EFFORT: in the drawer", async () => {
    await (await find(".agenda .row .title", "Estimate me")).click();
    await browser.keys("E");
    await find(".picker label span", "Effort estimate");
    await type("1:30");
    await browser.keys([Key.Ctrl, Key.Enter]);
    await find(".agenda .row .effort", "1h 30m");
    await find(".agenda header .titles span", "~1h 30m"); // "/ 8h planned" only on a workday
    await fileMatches(join(d.notes, "effort.org"), (s) => s.endsWith(":PROPERTIES:\n:ID: e1\n:EFFORT: 1:30\n:END:\n"), "effort not added to the drawer");
  });

  it("Space x g in the editor removes tags without leaving spaces", async () => {
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "strip");
    await find(".status .file", "strip.org");
    for (const t of ["xray", "yank"]) {
      await browser.keys(Key.Escape);
      await browser.keys(" ");
      await type("xg");
      await find(".picker label span", "Tags:");
      await type(t);
      await browser.keys(Key.Enter);
      await find(".picker label span", "Tags:");
      await type("done");
      await browser.keys(Key.Enter);
      await noPicker();
    }
    await fileMatches(join(d.notes, "strip.org"), (s) => s === "* TODO Strip me\n", "tags not removed cleanly");
  });

  it("tag: search matches tags inherited from #+FILETAGS and parent headings", async () => {
    await palette("Search tasks");
    await answer("Search tasks", "tag:work tag:proj");
    await find(".agenda h1", "tag:work tag:proj");
    await browser.waitUntil(async () => (await texts(".agenda .row .title")).join("|") === "Child task", { timeoutMsg: "search doesn't list exactly the inheriting child" });
  });

  it("someday/maybe: parking hides a task from Today, the review lists it, activating brings it back (#833)", async () => {
    const f = join(d.notes, "park.org");
    const today = async (shown: boolean) => {
      await browser.keys([Key.Ctrl, "1"]);
      await find(".agenda h1", "Today");
      await browser.waitUntil(async () => (await texts(".agenda .row .title")).includes("Park me") === shown, { timeoutMsg: `Park me ${shown ? "missing from" : "still in"} Today` });
    };
    await today(true);
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "park");
    await find(".status .file", "park.org");
    await palette("Task: park as someday/maybe");
    await fileMatches(f, (s) => s.startsWith("* TODO Park me :someday:\n"), "someday tag not added");
    await today(false);
    await palette("Someday/maybe review");
    await find(".agenda h1", "Someday/maybe");
    await find(".agenda .row .title", "Park me");
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "park");
    await find(".status .file", "park.org");
    await palette("Task: park as someday/maybe");
    await fileMatches(f, (s) => s.startsWith("* TODO Park me\nSCHEDULED:"), "someday tag not removed");
    await today(true);
  });
});

describe("Journal, archive and move", () => {
  let d: Dirs;
  const chores = "* TODO Archive me\n* TODO Move me\n* TODO Stay put\n";
  const projects = "#+title: Projects\n\n* Garden\n* Shed\n";
  const bulk = "* TODO Bulk parent\n** TODO Bulk child\n* TODO Bulk solo\n";
  const extract = "* TODO Big idea :x:\nwhy\n** Part\n";
  const retitle = "* TODO Old name\nsee [[*Old name][here]] and [[*Other]]\n", retitleRef = "[[file:retitle.org::*old name]] [[*Old name]]\n";
  const drill = "* Capital of France? :drill:\nParis\n* Later :drill:\n:PROPERTIES:\n:DRILL_DUE: 2999-01-01\n:END:\nnot due\n";
  const blockers = "* TODO Write spec\n* TODO Ship it\n* Phases\n** TODO One\n** TODO Two\n";
  const f = (n: string) => join(d.notes, n);
  /** Leader key sequence KEYS from the editor's normal mode. */
  const leader = async (keys: string) => { await browser.keys(Key.Escape); await browser.keys(" "); await type(keys); };
  before(async () => {
    d = await launch({
      files: ({ notes }) => {
        mkdirSync(join(notes, "daily"));
        writeFileSync(join(notes, "daily", "2020-01-01.org"), "#+title: 2020-01-01\n\n* First day\n");
        writeFileSync(join(notes, "daily", "2020-01-02.org"), "#+title: 2020-01-02\n\n* Second day\n- [X] fed cat\n- [ ] call bank\n** TODO Carry me\nbody\n");
        writeFileSync(join(notes, "chores.org"), chores);
        writeFileSync(join(notes, "bulk.org"), bulk);
        writeFileSync(join(notes, "projects.org"), projects);
        writeFileSync(join(notes, "extract.org"), extract);
        writeFileSync(join(notes, "retitle.org"), retitle);
        writeFileSync(join(notes, "linker.org"), retitleRef);
        writeFileSync(join(notes, "narrow.org"), "* One\nalpha\n* Two\nbeta\n");
        writeFileSync(join(notes, "marks.org"), "* Intro\nx\n* Target\nfirst\nsecond\n");
        writeFileSync(join(notes, "deadlinks.org"), "#+title: Dead links\nintro\nsee [[file:nowhere-737.org][gone]]\n");
        writeFileSync(join(notes, "stuck.org"), "* TODO Moving\n** NEXT Go\n* TODO Launch site\n** TODO Pick a host\n** TODO Write copy\n");
        writeFileSync(join(notes, "goals.org"), "* Intro\n* Ship v2 :GOAL:\n:PROPERTIES:\n:TARGET: 2099-01-01\n:END:\n** DONE Spec it\n** TODO Build it\n");
        writeFileSync(join(notes, "drill.org"), drill);
        writeFileSync(join(notes, "ordered.org"), "* Move flat\n:PROPERTIES:\n:ORDERED: t\n:END:\n** TODO Book van\n** TODO Pack boxes\n");
        writeFileSync(join(notes, "blockers.org"), blockers);
        writeFileSync(join(notes, "preamble.org"), "#+title: Pre\nintro\n* TODO Task\n");
        writeFileSync(join(notes, "gated.org"), "* Plan\n:PROPERTIES:\n:ORDERED: t\n:END:\n** TODO First\n** TODO Second\n");
        writeFileSync(join(notes, "clone.org"), "* DONE Check-in\nCLOSED: [2026-10-01 Thu 10:00] SCHEDULED: <2026-10-01 Thu>\n* After\n");
        writeFileSync(join(notes, "shutdown.org"), `* TODO Wrap report\nSCHEDULED: <${orgDay(new Date())}>\n* TODO Call vendor\nSCHEDULED: <${orgDay(new Date())} +1w>\n`);
      },
    });
  });
  after(close, { timeout: 30_000 });

  it("Ctrl+J creates today's entry, Space n e adds a line, Space n [ / ] step between entries", async () => {
    await find(".agenda h1", "Today");
    const day = isoDay(new Date()), file = f(join("daily", `${day}.org`));
    await browser.keys([Key.Ctrl, "j"]);
    await find(".status .file", `${day}.org`);
    await fileMatches(file, (s) => s.includes(`#+title: ${day}\n`), "today's journal not created with a title");
    await leader("ne");
    await answer("Add a line to today's journal", "standup done");
    await fileMatches(file, (s) => /\n\* \d\d:\d\d standup done\n/.test(s), "journal line not added");
    for (const [keys, want] of [["n[", "2020-01-02.org"], ["n[", "2020-01-01.org"], ["n]", "2020-01-02.org"], ["n]", `${day}.org`]]) {
      await leader(keys);
      await find(".status .file", want);
    }
    await leader("n]");
    await find(".status .msg", "No later journal entry");
  });

  it("Space f l adds a ledger entry under its month heading (#788)", async () => {
    await leader("fl");
    await answer("Ledger entry", "2026-10-25 12,50 lunch @anna #food");
    await find(".status .msg", "- 2026-10-25 -12.50 EUR lunch @anna #food");
    await fileMatches(f("ledger.org"), (s) => s.endsWith("\n* 2026-10\n- 2026-10-25 -12.50 EUR lunch @anna #food\n"), "ledger entry not filed");
  });

  it("Space v l shows this month's ledger totals; an entry opens at its line (#789)", async () => {
    const day = isoDay(new Date()), file = f("ledger.org");
    writeFileSync(file, `${read(file)}* ${day.slice(0, 7)}\n- ${day} +1000 EUR pay @acme #work\n- ${day} -2.50 EUR bus\n- ${day} -40 SEK fika @anna #food\n`);
    await leader("vl");
    await find(".ledger .total h3", "SEK");
    await find(".ledger .sums", "1000.00 EUR"); // income (the #788 entry may fall in this month too)
    await find(".ledger .cats td", "uncategorised");
    await find(".ledger .payees td", "@anna");
    await (await find(".ledger .entry button", "bus")).click();
    await find(".status .file", "ledger.org");
  });

  it("Space n k moves the previous entry's open items into today's, Space n u puts them back", async () => {
    const prev = f(join("daily", "2020-01-02.org")), file = f(join("daily", `${isoDay(new Date())}.org`)), before = read(prev);
    await leader("nk");
    await find(".status .msg", "Carried over 2 items from 2020-01-02");
    await fileMatches(file, (s) => s.includes("\n* Carried over from 2020-01-02\n- [ ] call bank\n** TODO Carry me\nbody\n"), "open items not in today's entry");
    await fileMatches(prev, (s) => s === "#+title: 2020-01-02\n\n* Second day\n- [X] fed cat\n", "open items not removed from the old entry");
    await leader("nk");
    await find(".status .msg", "Nothing open in the 2020-01-02 entry");
    await leader("nu");
    await fileMatches(prev, (s) => s === before, "undo didn't restore the old entry");
    await fileMatches(file, (s) => !s.includes("Carried over"), "undo didn't restore today's entry");
  });

  it("Space t R writes a weekly review into today's entry; a re-run replaces only the marked block", async () => {
    const file = f(join("daily", `${isoDay(new Date())}.org`));
    await leader("tR");
    await find(".status .msg", "Weekly review written");
    await fileMatches(file, (s) => /\n\* Weekly review \d{4}-W\d\d\n# review-begin\nInbox: \d+ entr/.test(s) && /\n\*\* Goals \(\d+\)\n- Ship v2: 1\/2 done \(50%\)/.test(s) && s.includes("# review-end\n** Went well\n** To change"), "review not written");
    await leader("tR");
    await find(".status .msg", "Weekly review written");
    await fileMatches(file, (s) => s.split("# review-begin").length === 2 && s.split("* Weekly review").length === 2, "re-run added a second block");
  });

  it("Space t q walks today's open tasks one key each (x done, t tomorrow) and adds the summary to the journal (#757)", async () => {
    const tomorrow = new Date(); tomorrow.setDate(tomorrow.getDate() + 1);
    await leader("tq");
    await find(".picker label span", "Shutdown 1/2: Wrap report");
    await browser.keys("x");
    await find(".picker label span", "Shutdown 2/2: Call vendor");
    await browser.keys("t");
    await find(".picker label span", "Done 1, moved 1");
    await browser.keys(Key.Enter);
    await fileMatches(f("shutdown.org"), (s) => /^\* DONE Wrap report\n/.test(s) && s.includes(`* TODO Call vendor\nSCHEDULED: <${orgDay(tomorrow)} +1w>\n`), "tasks not done / moved to tomorrow");
    await fileMatches(f(join("daily", `${isoDay(new Date())}.org`)), (s) => /\n\* \d\d:\d\d Shutdown: Done 1, moved 1 · /.test(s), "summary not in today's journal");
  });

  it("Space n J opens the journal for a chosen past date, creating it with a title", async () => {
    const file = f(join("daily", "2021-03-04.org"));
    await leader("nJ");
    await answer("Open journal for", "2021-03-04");
    await find(".status .file", "2021-03-04.org");
    await fileMatches(file, (s) => s.includes("#+title: 2021-03-04\n"), "journal for the chosen date not created");
  });

  it("Space n j logs a metric, Space n M shows its trend and Enter opens that journal day (#721)", async () => {
    const day = isoDay(new Date()), file = f(join("daily", `${day}.org`));
    await leader("nj");
    await answer("Log metric", "mood");
    await answer("mood today", "4");
    await fileMatches(file, (s) => /^:PROPERTIES:\n:ID: +\S+\n:MOOD: 4\n:END:\n/.test(s), "metric not logged");
    await leader("nJ");
    await answer("Open journal for", "2020-01-01");
    await find(".status .file", "2020-01-01.org");
    await leader("nM");
    await find(".metrics .stat .latest", "4");
    await find(".metrics tr.sel td", day);
    await browser.keys(Key.Enter);
    await find(".status .file", `${day}.org`);
    assert.equal((await texts(".metrics")).length, 0, "view still open");
  });

  it("A archives a task to <file>_archive and u restores both files", async () => {
    const archive = f("chores.org_archive");
    await browser.keys([Key.Ctrl, "2"]);
    await (await find(".agenda .row .title", "Archive me")).click();
    await browser.keys("A");
    await fileMatches(archive, (s) => s.includes("* TODO Archive me\n:PROPERTIES:\n:ARCHIVE_TIME: ") && s.includes(`\n:ARCHIVE_FILE: ${f("chores.org")}\n:END:\n`), "task not in the archive file");
    await fileMatches(f("chores.org"), (s) => s === "* TODO Move me\n* TODO Stay put\n", "task not removed from its file");
    await find(".status .msg", "Archived to chores.org_archive");
    await browser.keys("u");
    await fileMatches(f("chores.org"), (s) => s === chores, "undo didn't restore the source");
    await fileMatches(archive, (s) => s === "", "undo didn't restore the archive file"); // it didn't exist before: undo writes back ""
  });

  it("m moves a task under another file's heading and u restores both files", async () => {
    await (await find(".agenda .row .title", "Move me")).click();
    await browser.keys("m");
    await answer("Move to…", "projects.org / Shed");
    await fileMatches(f("projects.org"), (s) => s === `${projects}** TODO Move me\n`, "task not filed under Shed");
    await fileMatches(f("chores.org"), (s) => s === "* TODO Archive me\n* TODO Stay put\n", "task not removed from its file");
    await find(".status .msg", "Moved to projects.org / Shed");
    await browser.keys("u");
    await fileMatches(f("chores.org"), (s) => s === chores, "undo didn't restore the source");
    await fileMatches(f("projects.org"), (s) => s === projects, "undo didn't restore the target");
  });

  it("M copies a task under another file's heading, leaves the source and u removes the copy", async () => {
    await (await find(".agenda .row .title", "Stay put")).click();
    await browser.keys("M");
    await answer("Copy to…", "projects.org / Garden");
    await fileMatches(f("projects.org"), (s) => s === "#+title: Projects\n\n* Garden\n** TODO Stay put\n* Shed\n", "copy not filed under Garden");
    await find(".status .msg", "Copied to projects.org / Garden");
    await fileMatches(f("chores.org"), (s) => s === chores, "source changed by the copy");
    await browser.keys("u");
    await fileMatches(f("projects.org"), (s) => s === projects, "undo didn't remove the copy");
  });

  it("m on marked tasks asks once, moves within their own file and keeps a marked child under its parent (#638, #662)", async () => {
    await browser.keys([Key.Ctrl, "2"]);
    const mark = async (t: string) => { await (await find(".agenda .row .title", t)).click(); await browser.keys("*"); };
    // a mark above the target in its own file plus one in another file: the target line shifts before the other file is filed
    await mark("Bulk parent");
    await mark("Move me");
    await find(".agenda .titles span", "2 marked");
    await browser.keys("m");
    await answer("Move 2 tasks to…", "bulk.org / Bulk solo");
    await fileMatches(f("bulk.org"), (s) => s === "* TODO Bulk solo\n** TODO Bulk parent\n*** TODO Bulk child\n** TODO Move me\n", "same-file move not filed under Bulk solo");
    await fileMatches(f("chores.org"), (s) => s === "* TODO Archive me\n* TODO Stay put\n", "other file's mark not moved out");
    await browser.keys("u");
    await fileMatches(f("bulk.org"), (s) => s === bulk, "undo didn't restore the target file");
    await fileMatches(f("chores.org"), (s) => s === chores, "undo didn't restore the other file");
    await mark("Bulk parent");
    await mark("Bulk solo");
    await mark("Bulk child");
    await find(".agenda .titles span", "3 marked");
    await browser.keys("m");
    await answer("Move 3 tasks to…", "projects.org / Shed");
    await fileMatches(f("bulk.org"), (s) => !s.trim(), "marked tasks not all moved out");
    await fileMatches(f("projects.org"), (s) => s.includes("** TODO Bulk parent\n*** TODO Bulk child\n** TODO Bulk solo\n"), "marked (in order) tasks not filed under Shed (or the child flattened)");
    await browser.keys("u");
    await fileMatches(f("bulk.org"), (s) => s === bulk, "undo didn't restore the source");
    await fileMatches(f("projects.org"), (s) => s === projects, "undo didn't restore the target");
  });

  it("x on marked tasks in one file (one nested) flips all of them and one u restores it (#677)", async () => {
    await browser.keys([Key.Ctrl, "2"]);
    for (const t of ["Bulk parent", "Bulk child", "Bulk solo"]) { await (await find(".agenda .row .title", t)).click(); await browser.keys("*"); }
    await find(".agenda .titles span", "3 marked");
    await browser.keys("x");
    await fileMatches(f("bulk.org"), (s) => !s.includes("TODO") && (s.match(/^\*+ DONE Bulk /gm) ?? []).length === 3, "not every marked task done");
    await browser.keys("u");
    await fileMatches(f("bulk.org"), (s) => s === bulk, "one undo didn't restore the file");
  });

  it("Space n x extracts the heading into a new note with a link back, and Space n u restores it", async () => {
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "extract");
    await find(".status .file", "extract.org");
    await leader("nx");
    await find(".status .file", "big_idea.org");
    const name = readdirSync(d.notes).find((n) => n.endsWith("-big_idea.org"))!;
    const note = read(f(name)), id = /:ID:\s+(\S+)/.exec(note)![1];
    assert.match(note, /#\+title: Big idea\n\nwhy\n\* Part\n$/);
    await fileMatches(f("extract.org"), (s) => s === `* TODO Big idea :x:\n[[id:${id}][Big idea]]\n`, "heading not replaced by a link");
    await leader("nu");
    await fileMatches(f("extract.org"), (s) => s === extract, "undo didn't restore the heading");
  });

  it("renaming a heading (Space x e, e) updates the links to it, and one undo restores every note", async () => {
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "retitle.org");
    await find(".status .file", "retitle.org");
    await leader("xe");
    await answer("Task title", "New name");
    await fileMatches(f("retitle.org"), (s) => s === "* TODO New name\nsee [[*New name][here]] and [[*Other]]\n", "title or same-note link not updated");
    await fileMatches(f("linker.org"), (s) => s === "[[file:retitle.org::*New name]] [[*Old name]]\n", "file link not updated (or its own [[*Old name]] touched)");
    await find(".status .msg", "updated 2 links");
    await leader("nu");
    await fileMatches(f("retitle.org"), (s) => s === retitle, "undo didn't restore the renamed note");
    await fileMatches(f("linker.org"), (s) => s === retitleRef, "undo didn't restore the linking note");
    await browser.keys([Key.Ctrl, "2"]);
    await (await find(".agenda .row .title", "Old name")).click();
    await browser.keys("e");
    await answer("Task title", "Via e");
    await fileMatches(f("linker.org"), (s) => s === "[[file:retitle.org::*Via e]] [[*Old name]]\n", "file link not updated from the task view");
    await browser.keys("u");
    await fileMatches(f("retitle.org"), (s) => s === retitle, "u didn't restore the renamed note");
    await fileMatches(f("linker.org"), (s) => s === retitleRef, "u didn't restore the linking note");
  });

  it("Space m m bookmarks the cursor and Space ' jumps back after lines are added above (#672)", async () => {
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "marks");
    await find(".status .file", "marks.org");
    await browser.keys(Key.Escape);
    await browser.keys([Key.Ctrl, Key.Shift, "o"]);
    await answer("Go to heading", "Target");
    await browser.waitUntil(async () => (await texts(".cm-activeLine")).some((t) => t.includes("* Target")), { timeoutMsg: "cursor not on the heading" });
    await type("jj");
    await leader("mm");
    await answer("Bookmark name", "spot");
    await find(".status .msg", "Added bookmark spot");
    await noPicker();
    await type("ggOadded above");
    await browser.keys(Key.Escape);
    await type("gg");
    await leader("'");
    await answer("Jump to bookmark", "spot");
    await browser.waitUntil(async () => (await texts(".cm-activeLine")).some((t) => t.includes("second")), { timeoutMsg: "bookmark didn't land on its line" });
  });

  it("Space x S lists stuck projects and Enter jumps to the heading (#671)", async () => {
    await leader("xS");
    await find(".picker label span", "Stuck projects");
    await find(".picker", "2 open");
    await answer("Stuck projects", "Launch");
    await find(".status .file", "stuck.org");
    await browser.waitUntil(async () => (await texts(".cm-activeLine")).some((t) => t.includes("TODO Launch site")), { timeoutMsg: "didn't land on the project heading" });
  });

  it("Space x G lists goals with their progress and Enter jumps to the heading (#739)", async () => {
    await leader("xG");
    await find(".picker label span", "Goals");
    await find(".picker", "1/2 done (50%)");
    await answer("Goals", "Ship v2");
    await find(".status .file", "goals.org");
    await browser.waitUntil(async () => (await texts(".cm-activeLine")).some((t) => t.includes("Ship v2")), { timeoutMsg: "didn't land on the goal heading" });
  });

  it("Space n L lists broken links and Enter opens the note at the link (#737)", async () => {
    await leader("nL");
    await find(".picker label span", "Broken links");
    await find(".picker", "file not found");
    await answer("Broken links", "nowhere-737");
    await find(".status .file", "deadlinks.org");
    await browser.waitUntil(async () => (await texts(".cm-activeLine")).some((t) => t.includes("nowhere-737")), { timeoutMsg: "didn't land on the broken link" });
  });

  it("an :ORDERED: project shows only its next task in Tasks; B shows the blocked one, x on the first reveals it (#718)", async () => {
    await browser.keys(Key.Escape);
    await browser.keys([Key.Ctrl, "2"]);
    await (await find(".agenda .row .title", "Book van")).click();
    assert.ok(!(await texts(".agenda .row .title")).includes("Pack boxes"), "blocked task shown");
    await browser.keys("B");
    await find(".agenda .row.blocked .blk-chip", "blocked");
    await find(".agenda .row.blocked .title", "Pack boxes");
    // x on the blocked row asks first (#729): Cancel changes nothing
    await (await find(".agenda .row.blocked .title", "Pack boxes")).click();
    await browser.keys("x");
    await answer("waits on another task", "Cancel");
    await noPicker();
    await fileMatches(f("ordered.org"), (s) => s.includes("** TODO Pack boxes"), "cancel still marked it done");
    await browser.keys("B");
    await browser.waitUntil(async () => !(await texts(".agenda .row .title")).includes("Pack boxes"), { timeoutMsg: "B didn't hide it again" });
    await (await find(".agenda .row .title", "Book van")).click();
    await browser.keys("x");
    await fileMatches(f("ordered.org"), (s) => s.includes("** DONE Book van"), "first task not done");
    await browser.waitUntil(async () => (await texts(".agenda .row:not(.blocked) .title")).includes("Pack boxes"), { timeoutMsg: "next task didn't appear" });
  });

  it("Space x B adds a :BLOCKER: link (target gets an :ID:), Space x O toggles :ORDERED:, Space n u undoes each (#727); twice says so, Space x b removes it (#736)", async () => {
    await browser.keys(Key.Escape);
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "blockers");
    await find(".status .file", "blockers.org");
    await browser.keys(Key.Escape);
    await browser.keys([Key.Ctrl, Key.Shift, "o"]);
    await answer("Go to heading", "Ship it");
    await browser.waitUntil(async () => (await texts(".cm-activeLine")).some((t) => t.includes("TODO Ship it")), { timeoutMsg: "cursor not on the heading" });
    await leader("xB");
    await answer("Blocked by", "Write spec");
    await fileMatches(f("blockers.org"), (s) => /^\* TODO Write spec\n:PROPERTIES:\n:ID: (\S+)\n:END:\n\* TODO Ship it\n:PROPERTIES:\n:BLOCKER: \[\[id:\1\]\[Write spec\]\]\n:END:\n\* Phases\n/.test(s), "blocker or target id not written");
    await find(".status .msg", "Blocked by Write spec");
    // Again: nothing changes and it says so (#736)
    const once = readFileSync(f("blockers.org"), "utf8");
    await browser.keys([Key.Ctrl, Key.Shift, "o"]);
    await answer("Go to heading", "Ship it");
    await browser.waitUntil(async () => (await texts(".cm-activeLine")).some((t) => t.includes("TODO Ship it")), { timeoutMsg: "cursor not back on the heading" });
    await leader("xB");
    await answer("Blocked by", "Write spec");
    await find(".status .msg", "Already blocked by Write spec");
    await fileMatches(f("blockers.org"), (s) => s === once, "adding it twice changed the file");
    // Space x b drops it again; the target keeps its :ID:, Space n u restores the link
    await leader("xb");
    await answer("Unblock from", "Write spec");
    await fileMatches(f("blockers.org"), (s) => !s.includes(":BLOCKER:") && s.includes("* TODO Ship it\n* Phases\n") && /:ID: \S+/.test(s), "blocker not removed");
    await find(".status .msg", "No longer blocked by Write spec");
    await leader("nu");
    await fileMatches(f("blockers.org"), (s) => s === once, "undo didn't restore the blocker");
    await browser.keys([Key.Ctrl, Key.Shift, "o"]);
    await answer("Go to heading", "Phases");
    await browser.waitUntil(async () => (await texts(".cm-activeLine")).some((t) => t.includes("* Phases")), { timeoutMsg: "cursor not on Phases" });
    await leader("xO");
    await fileMatches(f("blockers.org"), (s) => s.includes("* Phases\n:PROPERTIES:\n:ORDERED: t\n:END:\n** TODO One\n"), ":ORDERED: not set");
    await leader("nu");
    await fileMatches(f("blockers.org"), (s) => s.includes("* Phases\n** TODO One\n") && s.includes(":BLOCKER:"), "undo didn't drop only :ORDERED:");
    await leader("nu");
    await fileMatches(f("blockers.org"), (s) => s === blockers, "one undo didn't remove both the blocker and the id");
  });

  it("Space x B / x O with the cursor above the first heading say so and write nothing (#755)", async () => {
    await browser.keys(Key.Escape);
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "preamble");
    await find(".status .file", "preamble.org");
    await browser.keys(Key.Escape);
    await browser.keys(["g", "g"]);
    await leader("xB");
    await find(".status .msg", "cursor on a task heading");
    await noPicker();
    await leader("xO");
    await find(".status .msg", "cursor on a task heading");
    await fileMatches(f("preamble.org"), (s) => s === "#+title: Pre\nintro\n* TODO Task\n", "the note changed");
  });

  it("Space x x on a blocked task in the editor asks first; Cancel leaves it, \"Mark done anyway\" finishes it (#754)", async () => {
    await browser.keys(Key.Escape);
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "gated");
    await find(".status .file", "gated.org");
    await browser.keys(Key.Escape);
    await browser.keys([Key.Ctrl, Key.Shift, "o"]);
    await answer("Go to heading", "Second");
    await browser.waitUntil(async () => (await texts(".cm-activeLine")).some((t) => t.includes("TODO Second")), { timeoutMsg: "cursor not on Second" });
    await leader("xx");
    await answer("Waits on", "Cancel");
    await noPicker();
    await fileMatches(f("gated.org"), (s) => s.includes("** TODO Second"), "cancel still marked it done");
    await leader("xx");
    await answer("Waits on", "Mark done anyway");
    await fileMatches(f("gated.org"), (s) => s.includes("** DONE Second"), "anyway didn't mark it done");
  });

  it("Space v n narrows to the heading at the cursor and widens again", async () => {
    const shown = () => browser.execute(() => document.querySelector(".cm-content")?.textContent ?? "");
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "narrow");
    await find(".status .file", "narrow.org");
    await browser.waitUntil(async () => (await shown()).includes("beta"), { timeoutMsg: "note not shown" });
    await leader("vn");
    await find(".status .narrowed", "Narrowed to One");
    await browser.waitUntil(async () => !(await shown()).includes("beta") && (await shown()).includes("alpha"), { timeoutMsg: "sibling heading still shown" });
    await leader("vn");
    await browser.waitUntil(async () => (await shown()).includes("beta"), { timeoutMsg: "widen didn't show the sibling again" });
    assert.equal((await texts(".status .narrowed")).length, 0);
    await fileMatches(f("narrow.org"), (s) => s === "* One\nalpha\n* Two\nbeta\n", "narrowing changed the file");
  });

  it("Space x C clones a task twice with weekly-shifted dates after it, as TODO (#618)", async () => {
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "clone");
    await find(".status .file", "clone.org");
    await leader("xC");
    await answer("Number of copies", "2");
    await answer("Shift each copy by", "+1w");
    const want = "* DONE Check-in\nCLOSED: [2026-10-01 Thu 10:00] SCHEDULED: <2026-10-01 Thu>\n* TODO Check-in\nSCHEDULED: <2026-10-08 Thu>\n* TODO Check-in\nSCHEDULED: <2026-10-15 Thu>\n* After\n";
    await fileMatches(f("clone.org"), (s) => s === want, "copies not inserted with shifted dates");
    await find(".status .msg", "Cloned 2×");
  });

  it("Space n d reviews due :drill: cards, grading writes the schedule and Space n u undoes it (#703)", async () => {
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "drill");
    await find(".status .file", "drill.org");
    await leader("nd");
    await find(".dialog .q", "Capital of France?");
    assert.equal((await texts(".dialog .a")).length, 0, "answer shown before Space");
    await browser.keys(" ");
    await find(".dialog .a", "Paris");
    await browser.keys("3");
    const tomorrow = new Date();
    tomorrow.setDate(tomorrow.getDate() + 1);
    const graded = drill.replace("France? :drill:\n", `France? :drill:\n:PROPERTIES:\n:DRILL_DUE: ${isoDay(tomorrow)}\n:DRILL_INTERVAL: 1\n:DRILL_EASE: 2.5\n:END:\n`);
    await fileMatches(f("drill.org"), (s) => s === graded, "grade not written to the drawer");
    await find(".status .msg", "Reviewed 1 card");
    assert.equal((await texts(".dialog")).length, 0, "not-due card was shown");
    await leader("nu");
    await fileMatches(f("drill.org"), (s) => s === drill, "undo didn't restore the card");
  });
});

// Own launch: the recently-changed test needs the fixture mtimes untouched, and pins start empty.
describe("Note pickers, pins and archive", () => {
  let d: Dirs;
  const arch = "#+title: Arch\n\n* DONE One\n* TODO Keep\n* DONE Two\n";
  const twinSrc = ":PROPERTIES:\n:ID: src-file-id\n:END:\n#+title: Source\n\nbody text\n* Part\n:PROPERTIES:\n:ID: src-head-id\n:END:\n";
  const now = new Date(), feb29 = now.getMonth() === 1 && now.getDate() === 29;
  const lastYear = isoDay(new Date(now.getFullYear() - 1, now.getMonth(), now.getDate()));
  const f = (n: string) => join(d.notes, n);
  /** Leader KEYS from normal mode, once the menu shows (it listens from an effect, so keys sent sooner go to vim). */
  const leader = async (keys: string) => {
    await browser.keys(Key.Escape);
    await browser.waitUntil(async () => {
      if ((await texts(".menu")).length) return true;
      await browser.keys(" ");
      await browser.pause(200);
      return (await texts(".menu")).length > 0;
    }, { timeoutMsg: "leader menu never opened" });
    await type(keys);
  };
  /** Open the note titled TITLE with Ctrl+P (re-sent if lost), then wait for FILE in the status bar and focus in the editor. */
  const open = async (title: string, file: string) => {
    await noPicker();
    const shown = async () => (await texts(".picker label span")).some((t) => t.includes("Open a note"));
    await browser.waitUntil(async () => {
      if (await shown()) return true;
      await browser.keys([Key.Ctrl, "p"]);
      await browser.pause(300);
      return shown();
    }, { timeoutMsg: "Ctrl+P never opened the note picker" });
    await answer("Open a note", title);
    await find(".status .file", file);
    await browser.waitUntil(() => browser.execute(() => !!document.activeElement?.closest(".cm-content")), { timeoutMsg: "editor not focused" });
  };
  before(async () => {
    d = await launch({
      files: ({ notes }) => {
        mkdirSync(join(notes, "daily"));
        writeFileSync(join(notes, "daily", `${lastYear}.org`), "#+title: Back then\n\n* A year ago\n");
        writeFileSync(join(notes, "stale.org"), "#+title: Stale\n\nold\n");
        writeFileSync(join(notes, "fresh.org"), "#+title: Fresh\n\nnew\n");
        const t = Date.now() / 1000; // explicit mtimes an hour apart: files written in a row can share a second
        utimesSync(join(notes, "stale.org"), t - 2 * 365 * 86400, t - 2 * 365 * 86400);
        utimesSync(join(notes, "fresh.org"), t - 3600, t - 3600);
        writeFileSync(join(notes, "tagged.org"), "#+title: Tagged\n#+FILETAGS: :work:\n\nwork stuff\n");
        writeFileSync(join(notes, "people.org"), "#+title: People\n\n* TODO Call Ola :@ola:\n* Sync\nAsk @Ola about the budget\n");
        writeFileSync(join(notes, "source.org"), twinSrc);
        writeFileSync(join(notes, "pinme.org"), "#+title: Pinme\n\npinned\n");
        writeFileSync(join(notes, "arch.org"), arch);
      },
    });
    await find(".agenda h1", "Today"); // loaded
  });
  after(close, { timeout: 30_000 });

  it("Space n C lists notes newest change first and opens the picked one", async () => {
    await open("Pinme", "pinme.org");
    await leader("nC");
    await find(".picker label span", "Recently changed notes");
    await find(".picker li span", "Stale");
    const items = await texts(".picker li span");
    assert.ok(items.indexOf("Fresh") >= 0 && items.indexOf("Fresh") < items.indexOf("Stale"), `Fresh not above Stale: ${items.join(" | ")}`);
    await answer("Recently changed notes", "Stale");
    await find(".status .file", "stale.org");
  });

  it("Space n Z lists notes untouched for a year and opens the picked one", async () => {
    await open("Pinme", "pinme.org");
    await leader("nZ");
    await find(".picker label span", "Stale notes");
    assert.deepEqual((await texts(".picker li span")).filter((x) => x == "Fresh" || x == "Stale"), ["Stale"]);
    await answer("Stale notes", "Stale");
    await find(".status .file", "stale.org");
  });

  it("Space n T browses notes by a #+FILETAGS tag", async () => {
    await open("Pinme", "pinme.org");
    await leader("nT");
    await answer("Browse notes by tag", "work");
    await answer("Tagged :work:", "Tagged");
    await find(".status .file", "tagged.org");
  });

  it("Space n @ lists a person's open tasks and mentions", async () => {
    await open("Pinme", "pinme.org");
    await leader("n@");
    await answer("Person", "@ola");
    await find(".picker li span", "TODO Call Ola");
    await answer("@ola", "budget");
    await find(".status .file", "people.org");
  });

  it("Space n c duplicates the note under a new title with fresh IDs", async () => {
    await open("Source", "source.org");
    await leader("nc");
    await answer("Duplicate note as", "Twin");
    await find(".status .file", "twin.org");
    const name = readdirSync(d.notes).find((n) => n.endsWith("-twin.org"))!;
    const twin = read(f(name));
    assert.match(twin, /#\+title: Twin\n/);
    assert.ok(twin.includes("body text\n* Part\n"), `body not copied:\n${twin}`);
    assert.ok(/:ID:\s+\S+/.test(twin) && !twin.includes("src-file-id") && !twin.includes("src-head-id"), `IDs not fresh:\n${twin}`);
    assert.equal(read(f("source.org")), twinSrc);
  });

  it("Space n p pins the note first in Ctrl+P, and Space n P opens it", async () => {
    await open("Pinme", "pinme.org");
    await leader("np");
    await find(".status .msg", "Pinned "); // names the tab, which may be the file name rather than the #+title
    await open("Fresh", "fresh.org");
    await browser.keys([Key.Ctrl, "p"]);
    await find(".picker li span", "★ Pinme");
    assert.equal((await texts(".picker li span"))[0], "★ Pinme");
    await browser.keys(Key.Escape);
    await noPicker();
    await leader("nP");
    await answer("Pinned notes", "Pinme");
    await find(".status .file", "pinme.org");
  });

  it("Space x a archives every done task to <file>_archive and Space n u restores both files", async () => {
    const archive = f("arch.org_archive");
    await open("Arch", "arch.org");
    await leader("xa");
    await find(".status .msg", "Archived 2 done tasks to arch.org_archive");
    await fileMatches(archive, (s) => s.includes("* DONE One\n") && s.includes("* DONE Two\n") && s.includes(`:ARCHIVE_FILE: ${f("arch.org")}\n`), "done tasks not in the archive file");
    await fileMatches(f("arch.org"), (s) => s.includes("* TODO Keep\n") && !s.includes("DONE"), "done tasks not removed from the note");
    await leader("nu");
    await fileMatches(f("arch.org"), (s) => s === arch, "undo didn't restore the note");
    await fileMatches(archive, (s) => s === "", "undo didn't restore the archive file"); // it didn't exist before: undo writes back ""
  });

  it("Space n t lists this day's journal entry from last year and opens it", { skip: feb29 && "no Feb 29 last year" }, async () => {
    await open("Pinme", "pinme.org");
    await leader("nt");
    await find(".picker li span", `${lastYear} · 1 year ago`);
    await answer("Journal: on this day", lastYear);
    await find(".status .file", `${lastYear}.org`);
  });
});

// Own launch: in the Tables suite the first keys typed into this note, and into the next one opened, got garbled.
describe("Day plan", () => {
  before(async () => {
    const at = (t: string) => `<${orgDay(new Date())} ${t}>`;
    await launch({ files: ({ notes }) => writeFileSync(join(notes, "plan.org"), `* TODO Plan e2e call\nSCHEDULED: ${at("14:00")}\n* TODO Plan e2e review\nSCHEDULED: ${at("14:00")}\n:PROPERTIES:\n:EFFORT: 1:00\n:END:\n* TODO Plan e2e untimed\nSCHEDULED: <${orgDay(new Date())}>\n`) });
  });
  after(close, { timeout: 30_000 });

  it("w shows Today's timed tasks on an hour grid, two at 14:00 overlapping, untimed ones only in the list", async () => {
    await (await find(".agenda .row .title", "Plan e2e untimed")).click();
    await browser.keys("w");
    await find(".dayplan .blk.overlap", "Plan e2e call");
    await find(".dayplan .blk.overlap", "Plan e2e review");
    assert.ok(!(await texts(".dayplan .blk")).some((t) => t.includes("untimed")), "untimed task on the grid");
    await browser.keys("w");
    await browser.waitUntil(async () => !(await browser.$(".dayplan").isExisting()), { timeoutMsg: "grid still shown" });
  });
});

describe("Insert commands", () => {
  let d: Dirs;
  before(async () => { d = await launch({ files: ({ notes }) => writeFileSync(join(notes, "ins.org"), "#+title: Ins\n") }); });
  after(close, { timeout: 30_000 });

  it("Insert code block… and Insert table… add a src block and a sized table", async () => {
    // Both leave insert mode on; Escape only once the editor shows it, or it's lost and later keys type text.
    const toNormal = async () => {
      await noPicker();
      await find(".status .mode", "INSERT");
      await browser.keys(Key.Escape);
      await find(".status .mode", "NORMAL");
    };
    await find(".agenda h1", "Today");
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "Ins");
    await find(".status .file", "ins.org");
    await palette("Insert code block"); // the cursor is on the title line, so the block goes below it
    await answer("Code block language", "rust");
    await toNormal();
    await type("G"); // the last, empty line, so the table goes below the block
    await palette("Insert table");
    await find(".dialog h2", "Insert table");
    await (await browser.$('.dialog button[aria-label="3 × 2"]')).click(); // the size grid's cell for 3 columns, 2 rows
    await toNormal();
    await browser.keys([Key.Ctrl, "s"]);
    await fileMatches(join(d.notes, "ins.org"), (s) => /^#\+title: Ins\n#\+begin_src rust\n\n#\+end_src\n\| Column 1 \| Column 2 \| Column 3 \|\n\|[-+]+\|\n(\|( +\|){3}\n?){2}$/.test(s), "code block or 3x2 table not inserted");
  });
});

// Own launch: vim's jump list is shared by all tabs, so a jump from a longer note breaks G here.
describe("Footnotes", () => {
  let d: Dirs;
  before(async () => { d = await launch({ files: ({ notes }) => writeFileSync(join(notes, "fn.org"), "#+title: Fn\nSee this.\n") }); });
  after(close, { timeout: 30_000 });

  it("Space i f adds a footnote; Enter jumps to its definition and Ctrl+O back", async () => {
    await find(".agenda h1", "Today"); // the page listens for keys only once loaded
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "Fn");
    await find(".status .file", "fn.org");
    await type("2G$"); // on the "." of "See this."
    await browser.keys(" ");
    await type("if");
    await find(".status .mode", "INSERT");
    await type("Def");
    await browser.keys(Key.Escape);
    await find(".status .mode", "NORMAL");
    await type("2Gf["); // back on the reference
    await browser.keys(Key.Enter); // to the definition's text
    await type("iX");
    await browser.keys(Key.Escape);
    await browser.keys([Key.Ctrl, "o"]); // and back to the reference
    await type("iY");
    await browser.keys(Key.Escape);
    await browser.keys([Key.Ctrl, "s"]);
    await fileMatches(join(d.notes, "fn.org"), (s) => s.startsWith("#+title: Fn\nSee thisY[fn:1].\n\n* Footnotes\n[fn:1] XDef"), "footnote not inserted or followed");
  });
});

describe("Tables, lists and attachments", () => {
  let d: Dirs;
  const att = (...p: string[]) => join(d.notes, "attachments", ...p);
  before(async () => {
    d = await launch({
      files: ({ notes }) => {
        writeFileSync(join(notes, "tbl.org"), "#+title: Tbl\n\n| a | b |\n|-\n| long cell | 1 |\n");
        writeFileSync(join(notes, "calc.org"), "#+title: Calc\n\n| item | qty | price | total |\n|-\n| a | 2 | 3 | |\n| b | 4 | 5 | |\n");
        writeFileSync(join(notes, "tools.org"), "#+title: Tools\n\n| a | b | c |\n|-\n| 1 | 2 | 3 |\n");
        writeFileSync(join(notes, "sums.org"), "#+title: Sums\n\n| item | qty |\n|-\n| a | 2 |\n| b | 3 |\n| total | |\n");
        writeFileSync(join(notes, "emph.org"), "#+title: Emph\n\nsay hello now\n");
        writeFileSync(join(notes, "list.org"), "#+title: List\n\n1. a\n2. b\n3. c\n");
        writeFileSync(join(notes, "pics.org"), "#+title: Pics\n\n[[file:attachments/pics/used.txt][used]]\n[[file:images/used.png]]\n");
        mkdirSync(join(notes, "images"));
        writeFileSync(join(notes, "images", "used.png"), "used");
        writeFileSync(join(notes, "images", "stray.png"), "stray");
        mkdirSync(join(notes, "attachments", "pics"), { recursive: true });
        writeFileSync(join(notes, "attachments", "pics", "used.txt"), "used");
        writeFileSync(join(notes, "attachments", "pics", "orphan.txt"), "orphan");
      },
    });
  });
  after(close, { timeout: 30_000 });

  const open = async (title: string, file: string) => {
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", title);
    await find(".status .file", file);
  };

  it("Tab in a table aligns it and moves the cursor to the next cell", async () => {
    await find(".agenda h1", "Today"); // the page listens for keys only once loaded
    await open("Tbl", "tbl.org");
    await type("3G");
    await browser.keys(Key.Tab);
    await type("iX");
    await browser.keys(Key.Escape);
    await browser.keys([Key.Ctrl, "s"]);
    await fileMatches(join(d.notes, "tbl.org"), (s) => /\| a\s+\| Xb\s*\|/.test(s) && /\| long cell \| 1\s*\|/.test(s), "table not aligned, or the cursor not in the second cell");
  });

  it("a typed =expr cell becomes a #+TBLFM column formula and fills the column", async () => {
    await open("Calc", "calc.org");
    await type("6G");
    for (let i = 0; i < 3; i++) await browser.keys(Key.Tab);
    await type("i=$2*$3");
    await browser.keys(Key.Tab);
    await browser.keys(Key.Escape);
    await browser.keys([Key.Ctrl, "s"]);
    await fileMatches(join(d.notes, "calc.org"), (s) => /\n#\+TBLFM: \$4=\$2\*\$3\n?$/.test(s) && /\|\s*6\s*\|/.test(s) && /\|\s*20\s*\|/.test(s), "formula or computed values missing");
  });

  it("the table toolbar shows in a table and its buttons edit it", async () => {
    await open("Tools", "tools.org");
    assert.equal((await texts(".cm-table-tools")).length, 0, "toolbar shown outside a table");
    await type("3G"); // on the "a" cell
    await (await find(".cm-table-tools button", "− Col")).click();
    await (await find(".cm-table-tools button", "+ Row ↓")).click();
    await type("iX"); // the cursor is in the new row, and vim still in normal mode
    await browser.keys(Key.Escape);
    await browser.keys([Key.Ctrl, "s"]);
    await fileMatches(join(d.notes, "tools.org"), (s) => /\| b \| c \|\n\| X *\| *\|\n\|---\+---\|\n\| 2 \| 3 \|/.test(s), "toolbar did not delete the column or add the row");
  });

  it("the formula dialog adds a total with a live preview and shows errors inline", async () => {
    await open("Sums", "sums.org");
    await type("3G");
    await (await find(".cm-table-tools button", "Formulas…")).click();
    await find(".dialog h2", "Table formulas");
    await browser.$('.dialog select[name="target"]').selectByAttribute("value", "last");
    await browser.$('.dialog input[name="column"]').setValue("2");
    await browser.$('.dialog input[name="expr"]').setValue("nope($1)");
    await find(".dialog .status.err", "unknown function nope");
    await browser.$('.dialog select[name="fn"]').selectByAttribute("value", "vsum");
    await browser.$('.dialog input[name="expr"]').setValue("@I..@-1");
    await find(".dialog .status", "@>$2=vsum(@I..@-1) — ok");
    await find(".dialog .preview", "| total |   5 |");
    await (await find(".dialog button", "Add")).click();
    await find(".dialog .list code", "@>$2=vsum(@I..@-1)");
    await (await find(".dialog button", "Save & recalculate")).click();
    await browser.keys([Key.Ctrl, "s"]);
    await fileMatches(join(d.notes, "sums.org"), (s) => /\| total \|\s+5 \|\n#\+TBLFM: @>\$2=vsum\(@I\.\.@-1\)\n?$/.test(s), "total formula not added or not computed");
  });

  it("Enter after an ordered-list item renumbers the items below", async () => {
    await open("List", "list.org");
    await type("4GA");
    await browser.keys(Key.Enter);
    await type("new");
    await browser.keys(Key.Escape);
    await browser.keys([Key.Ctrl, "s"]);
    await fileMatches(join(d.notes, "list.org"), (s) => s.includes("1. a\n2. b\n3. new\n4. c\n"), "list not renumbered");
  });

  it("Space i b bolds the word at the cursor, Ctrl+B in insert mode the word just typed", async () => {
    await open("Emph", "emph.org");
    await type("3Gw");
    await browser.keys(" ");
    await type("ib");
    await type("A x");
    await browser.keys([Key.Ctrl, "b"]);
    await browser.keys(Key.Escape);
    await browser.keys([Key.Ctrl, "s"]);
    await fileMatches(join(d.notes, "emph.org"), (s) => s.includes("say *hello* now *x*\n"), "word not bolded");
  });

  it("a pasted image goes to the shared images/ folder, linked and shown inline", async () => {
    await open("Pics", "pics.org");
    await type("G");
    await browser.execute(() => {
      const png = Uint8Array.from(atob("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg=="), (c) => c.charCodeAt(0));
      const dt = new DataTransfer();
      dt.items.add(new File([png], "x.png", { type: "image/png" }));
      document.querySelector(".cm-content")!.dispatchEvent(new ClipboardEvent("paste", { clipboardData: dt, bubbles: true, cancelable: true }));
    });
    await find(".status .msg", "Attached 1 file");
    await browser.keys([Key.Ctrl, "s"]);
    let link = "";
    await fileMatches(join(d.notes, "pics.org"), (s) => !!(link = /\[\[file:(images\/pasted-\d{8}-\d{6}\.png)\]\]/.exec(s)?.[1] ?? ""), "no images/ link inserted");
    assert.ok(existsSync(join(d.notes, link)), `${link} not saved`);
    await browser.waitUntil(async () => (await browser.execute(() => [...document.querySelectorAll(".cm-org-img img")].map((i) => decodeURIComponent((i as HTMLImageElement).src)))).some((s) => s.includes(link)), { timeoutMsg: "pasted image not shown inline" });
  });

  it("cleans up an unlinked image into images/.trash, keeping linked ones", async () => {
    await palette("Clean up unused attachments");
    await answer("Unused attachments", "stray");
    await find(".status .msg", "Moved 1 file");
    assert.equal(read(join(d.notes, "images", ".trash", "stray.png")), "stray");
    assert.ok(!existsSync(join(d.notes, "images", "stray.png")), "stray image not moved");
    assert.equal(read(join(d.notes, "images", "used.png")), "used", "linked image touched");
  });

  it("cleans up only the unlinked attachment, then reports there is nothing left", async () => {
    await palette("Clean up unused attachments");
    await answer("Unused attachments", "orphan");
    await find(".status .msg", "Moved 1 file");
    assert.ok(!existsSync(att("pics", "orphan.txt")), "orphan not moved");
    assert.equal(read(att(".trash", "pics", "orphan.txt")), "orphan");
    assert.equal(read(att("pics", "used.txt")), "used", "linked file touched");
    await palette("Clean up unused attachments");
    await find(".status .msg", "Every attachment is linked from a note.");
  });
});

describe("Note navigation", () => {
  let d: Dirs;
  before(async () => {
    d = await launch({
      files: ({ notes }) => {
        mkdirSync(join(notes, "sub"));
        mkdirSync(join(notes, "other"));
        writeFileSync(join(notes, "sub", "deep.org"), "#+title: Deep\n\nDeep text.\n");
        writeFileSync(join(notes, "sub", "writer.org"), "#+title: Writer\n\n");
        writeFileSync(join(notes, "other", "target.org"), "#+title: Target Note\n\nTarget text.\n");
        writeFileSync(join(notes, "headed.org"), "#+title: Headed\n\n* One\nfirst\n\n* Two\nsecond\n\n* Three\nthird\n");
        writeFileSync(join(notes, "inbox.org"), "* TODO Sort the mail\n");
      },
    });
    await browser.$(".status").waitForDisplayed(); // a Ctrl+P before the page listens is lost
    await browser.pause(500);
  });
  after(close, { timeout: 30_000 });

  it("restores a trashed note from a subfolder to its original path with Space n R", async () => {
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "Deep");
    await find(".status .file", "deep.org");
    await palette("Delete note");
    await answer("to the notes .trash folder", "Move");
    await find(".status .msg", "Moved to .trash");
    assert.ok(!existsSync(join(d.notes, "sub", "deep.org")), "note not trashed");
    await browser.keys(" ");
    await type("nR");
    await answer("Restore note from .trash", "deep");
    await find(".status .msg", "Restored sub");
    await find(".status .file", "deep.org");
    assert.equal(read(join(d.notes, "sub", "deep.org")), "#+title: Deep\n\nDeep text.\n");
    assert.ok(!existsSync(join(d.notes, ".trash", "sub", "deep.org")) && !existsSync(join(d.notes, ".trash", "deep.org")), "still in .trash");
  });

  it("Ctrl+L inserts a link relative to the note", async () => {
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "Writer");
    await find(".status .file", "writer.org");
    await browser.keys(Key.Escape);
    await browser.keys("G");
    await browser.keys([Key.Ctrl, "l"]);
    await answer("Link to note", "Target Note");
    await browser.keys(Key.Escape);
    await browser.keys([Key.Ctrl, "s"]);
    await fileMatches(join(d.notes, "sub", "writer.org"), (s) => s.includes("[[file:../other/target.org][Target Note]]"), "link not relative to the note");
    await type("gf"); // the cursor is on the link just inserted
    await find(".status .file", "target.org");
    await find(".cm-content", "Target text.");
    // a heading without an :ID: links as file:…::*Title, and following it lands on the heading
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "Writer");
    await find(".status .file", "writer.org");
    await browser.keys(Key.Escape);
    await type("Go");
    await browser.keys(Key.Escape);
    await browser.keys([Key.Ctrl, "l"]);
    await answer("Link to note", "headed.org / Two");
    await browser.keys(Key.Escape);
    await browser.keys([Key.Ctrl, "s"]);
    await fileMatches(join(d.notes, "sub", "writer.org"), (s) => s.includes("[[file:../headed.org::*Two][Two]]"), "heading link not inserted");
    await type("gf");
    await find(".status .file", "headed.org");
    await browser.waitUntil(async () => (await texts(".cm-activeLine")).some((t) => t.includes("* Two")), { timeoutMsg: "cursor not on the linked heading" });
  });

  it("Ctrl+Shift+O jumps to the picked heading", async () => {
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "Headed");
    await find(".status .file", "headed.org");
    await browser.keys(Key.Escape);
    await browser.keys([Key.Ctrl, Key.Shift, "o"]);
    await answer("Go to heading", "Three");
    await browser.waitUntil(async () => (await texts(".cm-activeLine")).some((t) => t.includes("* Three")), { timeoutMsg: "cursor not on the third heading" });
  });

  it("Space n H jumps to a heading in another note", async () => {
    await browser.keys([Key.Ctrl, "3"]);
    await find(".status .file", "inbox.org");
    await browser.keys(Key.Escape);
    await type(" nH");
    await answer("Go to heading in any note", "headed.org / Two");
    await find(".status .file", "headed.org");
    await browser.waitUntil(async () => (await texts(".cm-activeLine")).some((t) => t.includes("* Two")), { timeoutMsg: "cursor not on the second heading" });
  });

  it("Space n O lists notes with no links (not the inbox) and opens the picked one", async () => {
    await browser.keys([Key.Ctrl, "3"]);
    await find(".status .file", "inbox.org");
    await browser.keys(Key.Escape);
    await type(" nO");
    await find(".picker label span", "Notes with no links");
    assert.ok(!(await texts(".picker")).some((t) => t.includes("inbox.org")), "inbox listed");
    await answer("Notes with no links", "Deep");
    await find(".status .file", "deep.org");
  });

  it("Ctrl+3 opens the inbox", async () => {
    await browser.keys([Key.Ctrl, "3"]);
    await find(".status .file", "inbox.org");
    await find(".cm-content", "Sort the mail");
  });

  it("Settings > Editor > spell check toggles the editor's spellcheck attribute (#507)", async () => {
    const spell = () => browser.execute(() => document.querySelector(".cm-content")?.getAttribute("spellcheck"));
    assert.equal(await spell(), "false");
    const toggle = async () => {
      await browser.keys([Key.Ctrl, ","]);
      await (await browser.$(`//div[contains(@class, "settings")]//label[span[contains(., "Spell check")]]/input`)).click();
      await (await find(".settings button", "Save editor")).click();
      await find(".status .msg", "Settings saved");
      await browser.keys(Key.Escape);
    };
    await toggle();
    await browser.waitUntil(async () => (await spell()) === "true", { timeoutMsg: "spellcheck not on" });
    await toggle();
    await browser.waitUntil(async () => (await spell()) === "false", { timeoutMsg: "spellcheck not off" });
  });
});

describe("Org structure keys", () => {
  let d: Dirs;
  // No tables: Alt+H/J/K/L act on tables when the cursor is in one.
  const org = "* TODO A [0/2]\n- [ ] one\n- [ ] two\n** Child\n* B\n"; // lines 3-7 after the title
  const notes = ["moves", "items", "levels", "status", "newitem", "fold", "boxes", "short"]; // one per test, so a failure can't cascade
  const prio = "* TODO [#B] x\n* y\n* TODO z\n"; // lines 3-5
  // Link lines 3-6 start with the link, so the cursor at column 0 is on it; the targets follow.
  const links = "[[*Target heading]]\n[[#custom-id]]\n[[file:other.org::*Other heading]]\n[[*Missing]]\n* Target heading\n* Another\n:PROPERTIES:\n:CUSTOM_ID: custom-id\n:END:\n* Trailing\n";
  const sortOrg = "* P\n** b\n*** deep\n** a\n** c\n* Q\n"; // lines 3-8
  const f = (n: string) => join(d.notes, `${n}.org`);
  const title = (n: string) => `#+title: ${n}\n\n`;
  /** Open note N in normal mode with the cursor at the start of LINE. */
  const open = async (n: string, line: number, keys = `${line}G0`) => {
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", n);
    await find(".status .file", `${n}.org`);
    await noPicker();
    await browser.keys(Key.Escape);
    await type(keys);
    const want = read(f(n)).split("\n")[line - 1].replace(/^\*+ /, "");
    try {
      await browser.waitUntil(async () => (await texts(".cm-activeLine")).some((t) => t.includes(want)));
    } catch {
      throw new Error(`cursor not on line ${line} of ${n}; active: ${JSON.stringify(await texts(".cm-activeLine"))}`);
    }
  };
  /** Save and wait until N's text after the title is WANT. */
  const saved = async (n: string, want: string, msg: string) => {
    await browser.keys(Key.Escape);
    await browser.keys([Key.Ctrl, "s"]);
    await fileMatches(f(n), (s) => s === title(n) + want, msg);
  };
  before(async () => {
    d = await launch({ files: ({ notes: dir }) => { for (const n of notes) writeFileSync(join(dir, `${n}.org`), title(n) + org);
      writeFileSync(join(dir, "long.org"), title("long") + "x\n".repeat(40));
      writeFileSync(join(dir, "prio.org"), title("prio") + prio);
      writeFileSync(join(dir, "links.org"), title("links") + links);
      writeFileSync(join(dir, "sort.org"), title("sort") + sortOrg);
      writeFileSync(join(dir, "other.org"), title("other") + "* Lead\n* Other heading\n* Tail\n");
    } });
    await find(".agenda h1", "Today"); // the page listens for keys only once loaded
  });
  after(close, { timeout: 30_000 });

  it("Alt+J moves a heading below its sibling with its children, Alt+K moves it back", async () => {
    await open("moves", 3);
    await browser.keys([Key.Alt, "j"]);
    await saved("moves", "* B\n* TODO A [0/2]\n- [ ] one\n- [ ] two\n** Child\n", "subtree not moved down");
    await browser.keys([Key.Alt, "k"]);
    await saved("moves", org, "subtree not moved back up");
  });

  it("Alt+J / Alt+K on a list item move only the item, not its heading (#472)", async () => {
    await open("items", 4);
    await browser.keys([Key.Alt, "j"]);
    await saved("items", "* TODO A [0/2]\n- [ ] two\n- [ ] one\n** Child\n* B\n", "item not moved down");
    await browser.keys([Key.Alt, "k"]);
    await saved("items", org, "item not moved back up");
  });

  it("Alt+Enter continues a checkbox list, Alt+Shift+Enter adds a task heading after the subtree", async () => {
    await open("newitem", 5);
    await browser.keys([Key.Alt, Key.Enter]);
    await type("three");
    await saved("newitem", "* TODO A [0/2]\n- [ ] one\n- [ ] two\n- [ ] three\n** Child\n* B\n", "list item not added");
    await type("8G0"); // * B (one line down after "three")
    await browser.keys([Key.Alt, Key.Shift, Key.Enter]);
    await type("C");
    await saved("newitem", "* TODO A [0/2]\n- [ ] one\n- [ ] two\n- [ ] three\n** Child\n* B\n* TODO C\n", "task heading not added");
  });

  it("jump marks from a longer note don't break 3G0 in a shorter one (#517)", async () => {
    await open("long", 40);
    await type("gg"); // records a jump mark far past the end of the next note
    await open("short", 5); // open() asserts the cursor landed on line 5; a stale mark types a literal G instead
    await saved("short", org, "stray text typed (a literal G?)");
  });

  it("Alt+L / Alt+H demote and promote a heading, with Shift its children too", async () => {
    await open("levels", 7);
    await browser.keys([Key.Alt, "l"]);
    await saved("levels", "* TODO A [0/2]\n- [ ] one\n- [ ] two\n** Child\n** B\n", "heading not demoted");
    await browser.keys([Key.Alt, "h"]);
    await saved("levels", org, "heading not promoted");
    await type("3G0");
    await browser.keys([Key.Alt, Key.Shift, "l"]);
    await saved("levels", "** TODO A [0/2]\n- [ ] one\n- [ ] two\n*** Child\n* B\n", "subtree not demoted");
    await browser.keys([Key.Alt, Key.Shift, "h"]);
    await saved("levels", org, "subtree not promoted");
  });

  it("Space x o sorts a heading's children with their subtrees, u undoes it (#553)", async () => {
    await open("sort", 3);
    await browser.keys(" ");
    await type("xo");
    await answer("Sort headings by", "title");
    await answer("Sort by title", "desc");
    await noPicker();
    await fileMatches(f("sort"), (s) => s === title("sort") + "* P\n** c\n** b\n*** deep\n** a\n* Q\n", "children not sorted");
    await type("u");
    await saved("sort", sortOrg, "sort not undone");
  });

  it("Shift+Right / Shift+Left cycle the heading's status", async () => {
    await open("status", 7);
    await browser.keys([Key.Shift, Key.ArrowRight]);
    await fileMatches(f("status"), (s) => s.includes("\n* TODO B\n"), "status not set to TODO");
    await browser.keys([Key.Shift, Key.ArrowLeft]);
    await fileMatches(f("status"), (s) => s === title("status") + org, "status not cleared");
  });

  it("Shift+Up / Shift+Down cycle a heading's priority", async () => {
    await open("prio", 3, "ggjj0"); // no count: the digits of "3G0" get dropped here
    await browser.keys([Key.Shift, Key.ArrowUp]);
    await fileMatches(f("prio"), (s) => s.includes("\n* TODO [#A] x\n"), "priority not raised to A");
    await browser.keys([Key.Shift, Key.ArrowDown]);
    await fileMatches(f("prio"), (s) => s.includes("\n* TODO [#B] x\n"), "priority not lowered to B");
    await browser.keys([Key.Shift, Key.ArrowDown]);
    await fileMatches(f("prio"), (s) => s.includes("\n* TODO [#C] x\n"), "priority not lowered to C");
    await type("j0"); // * y: no priority yet, Shift+Up gives C
    await browser.keys([Key.Shift, Key.ArrowUp]);
    await fileMatches(f("prio"), (s) => s.includes("\n* [#C] y\n"), "first press didn't set [#C]");
  });

  it("Enter on a link jumps to its heading, :CUSTOM_ID: or other note, and flashes when nothing matches", async () => {
    await open("links", 3);
    await browser.keys(Key.Enter);
    await browser.waitUntil(async () => (await texts(".cm-activeLine")).some((t) => t.includes("* Target heading")), { timeoutMsg: "[[*heading]] link not followed" });
    await type("4G0");
    await browser.keys(Key.Enter);
    await browser.waitUntil(async () => (await texts(".cm-activeLine")).some((t) => t.includes("* Another")), { timeoutMsg: "[[#custom-id]] link not followed" });
    await type("6G0");
    await browser.keys(Key.Enter);
    await find(".status .msg", 'No match for "*Missing"');
    await type("5G0");
    await browser.keys(Key.Enter);
    await find(".status .file", "other.org");
    await browser.waitUntil(async () => (await texts(".cm-activeLine")).some((t) => t.includes("* Other heading")), { timeoutMsg: "[[file:…::*heading]] link not followed" });
  });

  it("Tab on a heading folds its subtree and Tab again unfolds it", async () => {
    await open("fold", 3);
    const shown = async () => (await texts(".cm-line")).some((t) => t.includes("Child"));
    await browser.waitUntil(shown, { timeoutMsg: "child heading not shown before folding" });
    await browser.keys(Key.Tab);
    try {
      await browser.waitUntil(async () => !(await shown()));
    } catch {
      const at = await browser.execute(() => `focus: ${document.activeElement?.className}; cursor line: ${document.querySelector(".cm-activeLine")?.textContent}`);
      throw new Error(`Tab didn't fold the subtree; ${at}; lines: ${JSON.stringify(await texts(".cm-line"))}`);
    }
    await browser.keys(Key.Tab);
    await browser.waitUntil(shown, { timeoutMsg: "Tab didn't unfold the subtree" });
  });

  it("clicking a checkbox ticks it and updates the heading's [n/m] cookie", async () => {
    await open("boxes", 7); // off the item lines, so they show the checkbox widget
    await (await (await find(".cm-line", "one")).$(".cm-org-check")).click();
    await saved("boxes", "* TODO A [1/2]\n- [X] one\n- [ ] two\n** Child\n* B\n", "checkbox or cookie not updated");
  });
});

describe("Web links and note graph", () => {
  let d: Dirs;
  const note = (n: string) => join(d.notes, `${n}.org`);
  const open = async (n: string) => {
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", n);
    await find(".status .file", `${n}.org`);
    await noPicker();
    await browser.keys(Key.Escape);
  };
  /** Dispatch a paste of TEXT on the editor; a plain object stands in for the clipboard when DataTransfer can't be built. */
  const paste = (text: string) => browser.execute((text: string) => {
    const ev = new ClipboardEvent("paste", { bubbles: true, cancelable: true });
    if (!ev.clipboardData) Object.defineProperty(ev, "clipboardData", { value: { getData: (t: string) => (t === "text/plain" ? text : ""), files: [] } });
    document.querySelector(".cm-content")!.dispatchEvent(ev);
  }, text);
  before(async () => {
    d = await launch({ files: ({ notes }) => {
      writeFileSync(join(notes, "hub.org"), "#+title: Hub\n\nsee [[file:spoke.org][Spoke]] here\n");
      writeFileSync(join(notes, "spoke.org"), "#+title: Spoke\n\nback to [[file:hub.org][Hub]]\n");
      writeFileSync(join(notes, "paste.org"), "#+title: Paste\n\nsay hello now\nsay world now\n");
      writeFileSync(join(notes, "sheet.org"), "#+title: Sheet\n\n");
    } });
    await find(".agenda h1", "Today");
  });
  after(close, { timeout: 30_000 });

  it("clicking a node in the note graph opens that note", async () => {
    await open("hub");
    await palette("Show note graph");
    await find(".links", "Graph");
    await browser.waitUntil(async () => (await browser.$$(".links svg a")).length > 1, { timeoutMsg: "graph has no linked node" });
    // SVG anchors aren't "interactable" to WebDriver; click through the DOM.
    await browser.execute(() => document.querySelector<SVGAElement>(".links svg g.h1 a")!.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true })));
    await find(".status .file", "spoke.org");
    await find(".tabs .tab.cur button", "spoke");
  });

  it("a URL pasted over a selected word links it", async () => {
    await open("paste");
    await type("3G0wviw");
    await paste("https://example.com");
    await browser.keys(Key.Escape);
    await browser.keys([Key.Ctrl, "s"]);
    await fileMatches(note("paste"), (s) => s.includes("say [[https://example.com][hello]] now\n"), "URL not pasted as a link over the selection");
  });

  it("spreadsheet cells (TSV) paste as an aligned table, and u undoes it", async () => {
    const table = "| Name  | Qty |\n|-------+-----|\n| apple | 3   |";
    await open("sheet");
    await type("G");
    await paste("Name\tQty\napple\t3\n");
    await browser.keys([Key.Ctrl, "s"]);
    await fileMatches(note("sheet"), (s) => s.includes(table), "TSV not pasted as a table");
    await type("u");
    await browser.keys([Key.Ctrl, "s"]);
    await fileMatches(note("sheet"), (s) => !s.includes("| Name"), "u did not undo the pasted table");
  });

  it("Insert web link… links the selected word with the URL asked for", async () => {
    await open("paste");
    await type("4G0wviw");
    await palette("Insert web link");
    await answer("Link address (URL)", "https://margin.test");
    await answer("Link text (optional)", "world");
    await browser.keys([Key.Ctrl, "s"]);
    await fileMatches(note("paste"), (s) => s.includes("say [[https://margin.test][world]] now\n"), "web link not inserted over the selection");
  });
});

describe("Note history (git)", () => {
  let d: Dirs;
  const f = (n: string) => join(d.notes, n);
  const v1 = "#+title: Hist\n\nfirst draft\n", v2 = "#+title: Hist\n\nrewritten\n";
  before(async () => {
    d = await launch({ files: ({ notes }) => {
      const git = (...a: string[]) => assert.equal(spawnSync("git", a, { cwd: notes }).status, 0, `git ${a.join(" ")}`);
      git("init", "-q");
      git("config", "user.email", "t@example.com"); git("config", "user.name", "t"); git("config", "commit.gpgsign", "false");
      writeFileSync(join(notes, "hist.org"), v1); git("add", "-A"); git("commit", "-qm", "first version");
      writeFileSync(join(notes, "hist.org"), v2); git("commit", "-qam", "second version");
      writeFileSync(join(notes, "fresh.org"), "#+title: Fresh\n\nnot committed\n");
    } });
    await find(".agenda h1", "Today"); // the app is ready for keys
  });
  after(close, { timeout: 30_000 });
  const leader = async (keys: string) => {
    await browser.keys(Key.Escape);
    await browser.waitUntil(async () => {
      if ((await texts(".menu")).length) return true;
      await browser.keys(" ");
      await browser.pause(200);
      return (await texts(".menu")).length > 0;
    }, { timeoutMsg: "leader menu never opened" });
    await type(keys);
  };
  const open = async (title: string, file: string) => {
    await noPicker();
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", title);
    await find(".status .file", file);
    await noPicker();
    await browser.waitUntil(() => browser.execute(() => !!document.activeElement?.closest(".cm-content")), { timeoutMsg: "editor not focused" });
  };

  it("Space n V lists the note's commits, shows the diff and restores a version as an undoable edit (#768)", async () => {
    await open("Hist", "hist.org");
    await leader("nV");
    await find(".picker li span", "second version");
    assert.deepEqual((await texts(".picker li span")).filter((t) => t.endsWith("version")), ["second version", "first version"]);
    await answer("Note history", "first version");
    await find(".picker label span", "Version");
    await find(".picker .body", "- rewritten");
    await find(".picker .body", "+ first draft");
    await browser.keys("r");
    await find(".status .msg", "Restored version");
    await browser.keys([Key.Ctrl, "s"]);
    await fileMatches(f("hist.org"), (s) => s === v1, "old version not restored");
    await type("u");
    await browser.keys([Key.Ctrl, "s"]);
    await fileMatches(f("hist.org"), (s) => s === v2, "u did not undo the restore");
  });

  it("Space n V on a note with no commits says so", async () => {
    await open("Fresh", "fresh.org");
    await leader("nV");
    await find(".status .msg", "No commits touch this note yet.");
  });
});

describe("Run code blocks (babel)", () => {
  let d: Dirs;
  const file = () => join(d.notes, "run.org");
  const doc = "#+title: Run\n\n#+begin_src sh\necho hi; pwd\n#+end_src\n\n#+begin_src python\nprint(1)\n#+end_src\n";
  before(async () => {
    d = await launch({ config: [`babel_languages = ["sh"]`], files: ({ notes }) => writeFileSync(join(notes, "run.org"), doc) });
    await find(".agenda h1", "Today");
  });
  after(close, { timeout: 30_000 });

  it("Ctrl+Enter asks first, writes #+RESULTS: under the block, re-runs without asking, and u undoes it", async () => {
    await browser.keys([Key.Ctrl, "p"]);
    await answer("Open a note", "Run");
    await find(".status .file", "run.org");
    await noPicker();
    await browser.keys(Key.Escape);
    await type("4G");
    await browser.keys([Key.Ctrl, Key.Enter]);
    await find(".picker", "echo hi; pwd"); // the exact code is shown
    await answer("Run this sh block", "Run");
    const ran = `#+begin_src sh\necho hi; pwd\n#+end_src\n\n#+RESULTS:\n: hi\n: ${d.notes}\n\n#+begin_src python`;
    await find(".cm-content", ": hi");
    await browser.keys([Key.Ctrl, "s"]);
    await fileMatches(file(), (s) => s.includes(ran), "results not written under the block");
    // Same code again: no question, results replaced, not added.
    await browser.keys([Key.Ctrl, Key.Enter]);
    await browser.pause(1000);
    assert.equal((await texts(".picker")).length, 0, "asked again for unchanged code");
    await browser.keys([Key.Ctrl, "s"]);
    await fileMatches(file(), (s) => s.split("#+RESULTS:").length === 2 && s.includes(ran), "results duplicated");
    // A language not in babel_languages never runs.
    await type("12G");
    await browser.keys([Key.Ctrl, Key.Enter]);
    await find(".status .msg", "python blocks don't run");
    await type("uu");
    await browser.keys([Key.Ctrl, "s"]);
    await fileMatches(file(), (s) => s === doc, "undo didn't remove the results");
  });
});

describe("Tag rename and open task-search tabs", () => {
  let d: Dirs;
  before(async () => {
    d = await launch({ files: ({ notes }) => writeFileSync(join(notes, "tasks.org"), "* TODO Buy milk :home:\n* NEXT Call the bank :work:money:\n") });
    await find(".agenda h1", "Today");
  });
  after(close, { timeout: 30_000 });

  it("renaming a tag rewrites open task-search tabs, and closes one that becomes a duplicate (#824)", async () => {
    const rename = async (from: string, to: string) => {
      await palette("rename or merge a tag");
      await answer("Rename or merge a tag", from);
      await answer(`Rename :${from}: to`, to);
      await answer(`Rename :${from}: to :${to}:?`, "Rename");
    };
    const tabs = async () => (await texts(".tabs .tab")).filter((t) => t.includes("tag:"));
    await palette("Search tasks");
    await answer("Search tasks", "tag:money");
    await find(".agenda h1", "tag:money");
    await rename("money", "cash");
    await find(".agenda h1", "tag:cash");
    await browser.waitUntil(async () => (await texts(".agenda .row .title")).join("|") === "Call the bank", { timeoutMsg: "renamed tab doesn't list its task" });
    await palette("Search tasks");
    await answer("Search tasks", "tag:home");
    await rename("cash", "home"); // tag:cash becomes tag:home, already open
    await browser.waitUntil(async () => (await tabs()).length === 1, { timeoutMsg: "duplicate tab kept" });
    await browser.waitUntil(async () => (await texts(".agenda .row .title")).sort().join("|") === "Buy milk|Call the bank", { timeoutMsg: "merged tab doesn't list both" });
  });
});
