// End-to-end tests of features beyond the basics; setup and helpers are in setup.ts.
import { after, before, describe, it } from "node:test";
import { strict as assert } from "node:assert";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { answer, browser, close, fileMatches, find, isoDay, Key, launch, orgDay, palette, read, savedView, texts, type, type Dirs } from "./setup.ts";

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
  const day = (n: number) => { const t = new Date(); t.setDate(t.getDate() + n); return `<${orgDay(t)}>`; };
  /** Click TITLE's row in Today (re-found by text: the agenda re-sorts after each change). */
  const select = async (title: string) => { await (await find(".agenda .row .title", title)).click(); };
  before(async () => {
    d = await launch({
      files: ({ notes }) => {
        writeFileSync(join(notes, "keys.org"), `* TODO Shift me\nSCHEDULED: ${day(0)}\n* TODO [#B] Rank me\nSCHEDULED: ${day(0)}\n* TODO Due me\nSCHEDULED: ${day(0)}\n* TODO Finish me\nSCHEDULED: ${day(0)}\n* TODO Repeat me\nSCHEDULED: ${day(0)}\n`);
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

describe("Journal, archive and move", () => {
  let d: Dirs;
  const chores = "* TODO Archive me\n* TODO Move me\n* TODO Stay put\n";
  const projects = "#+title: Projects\n\n* Garden\n* Shed\n";
  const f = (n: string) => join(d.notes, n);
  /** Leader key sequence KEYS from the editor's normal mode. */
  const leader = async (keys: string) => { await browser.keys(Key.Escape); await browser.keys(" "); await type(keys); };
  before(async () => {
    d = await launch({
      files: ({ notes }) => {
        mkdirSync(join(notes, "daily"));
        writeFileSync(join(notes, "daily", "2020-01-01.org"), "#+title: 2020-01-01\n\n* First day\n");
        writeFileSync(join(notes, "daily", "2020-01-02.org"), "#+title: 2020-01-02\n\n* Second day\n");
        writeFileSync(join(notes, "chores.org"), chores);
        writeFileSync(join(notes, "projects.org"), projects);
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
});

describe("Tables, lists and attachments", () => {
  let d: Dirs;
  const att = (...p: string[]) => join(d.notes, "attachments", ...p);
  before(async () => {
    d = await launch({
      files: ({ notes }) => {
        writeFileSync(join(notes, "tbl.org"), "#+title: Tbl\n\n| a | b |\n|-\n| long cell | 1 |\n");
        writeFileSync(join(notes, "calc.org"), "#+title: Calc\n\n| item | qty | price | total |\n|-\n| a | 2 | 3 | |\n| b | 4 | 5 | |\n");
        writeFileSync(join(notes, "emph.org"), "#+title: Emph\n\nsay hello now\n");
        writeFileSync(join(notes, "list.org"), "#+title: List\n\n1. a\n2. b\n3. c\n");
        writeFileSync(join(notes, "pics.org"), "#+title: Pics\n\n[[file:attachments/pics/used.txt][used]]\n");
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

  it("Ctrl+3 opens the inbox", async () => {
    await browser.keys([Key.Ctrl, "3"]);
    await find(".status .file", "inbox.org");
    await find(".cm-content", "Sort the mail");
  });
});
