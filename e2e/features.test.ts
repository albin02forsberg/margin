// End-to-end tests of features beyond the basics; setup and helpers are in setup.ts.
import { after, before, describe, it } from "node:test";
import { strict as assert } from "node:assert";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { answer, browser, close, fileMatches, find, isoDay, Key, launch, orgDay, palette, read, texts, type, type Dirs } from "./setup.ts";

describe("First run", () => {
  let d: Dirs;
  before(async () => { d = await launch({ tutorial: false }); });
  after(close, { timeout: 30_000 });

  it("opens the tutorial, and Space ? reopens it", async () => {
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

describe("Broken config.toml", () => {
  let d: Dirs;
  const bad = Buffer.from('notes_dir = "\xff"\n', "latin1"); // not UTF-8
  before(async () => { d = await launch({ files: ({ cfgDir }) => writeFileSync(join(cfgDir, "config.toml"), bad) }); });
  after(close, { timeout: 30_000 });

  it("says it runs on defaults and leaves the file alone", async () => {
    await find(".status .msg", "config.toml was ignored");
    assert.deepEqual(readFileSync(join(d.cfgDir, "config.toml")), bad);
    // The default ~/timeclock landed under the temp HOME, not the real one.
    assert.ok(existsSync(join(d.root, "timeclock")), "default data dir not under the temp HOME");
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
    await find(".side nav button", "Work stuff");
    await find(".tabs .tab.cur button", "Work stuff");
    await fileMatches(cfg, (s) => s.endsWith(`\n[[views]]\nname = "Work stuff"\nquery = "tag:work"\n`), "view not appended");
    const before = read(cfg);

    await palette("Save this search as a view");
    await answer("already shows this search", "Rename existing view");
    await answer("Rename “Work stuff” to", "Office");
    await find(".tabs .tab.cur button", "Office");
    await find(".side nav button", "Office");
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
});
