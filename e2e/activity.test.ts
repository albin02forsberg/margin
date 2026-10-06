// End-to-end test of ActivityWatch suggestions against a stand-in AW server on loopback.
import { after, before, describe, it } from "node:test";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";
import assert from "node:assert/strict";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { answer, browser, clickIn, close, fileMatches, find, isoDay, Key, launch, texts, type Dirs } from "./setup.ts";

// Yesterday, so the sessions are over whenever the test runs.
const y = new Date();
y.setDate(y.getDate() - 1);
const day = isoDay(y);
const at = (h: number, m: number) => new Date(y.getFullYear(), y.getMonth(), y.getDate(), h, m).toISOString();
const ev = (from: [number, number], mins: number, data: object) => ({ id: 1, timestamp: at(...from), duration: mins * 60, data });
const PR = "PR #61 · albin02forsberg/margin";
const events: Record<string, object[]> = {
  "aw-watcher-window_pc": [
    ev([9, 0], 60, { app: "firefox", title: `${PR} — Mozilla Firefox` }),
    ev([11, 0], 30, { app: "Code", title: "notes - Visual Studio Code" }),
  ],
  "aw-watcher-web-firefox": [ev([9, 0], 60, { url: "https://github.com/albin02forsberg/margin/pull/61", title: PR, incognito: false })],
};
const buckets = Object.fromEntries(Object.keys(events).map((id) => [id, { id, type: id.includes("web") ? "web.tab.current" : "currentwindow", last_updated: at(12, 0) }]));

/** A minimal ActivityWatch REST API: the bucket list and each bucket's events. */
function fakeAw() {
  return createServer((req, res) => {
    const id = req.url!.match(/^\/api\/0\/buckets\/([^/?]+)\/events/)?.[1];
    const body = req.url === "/api/0/buckets/" ? buckets : id ? events[id] : undefined;
    res.writeHead(body ? 200 : 404, { "Content-Type": "application/json" }).end(JSON.stringify(body ?? {}));
  });
}

describe("ActivityWatch suggestions", () => {
  let d: Dirs, aw: Server;
  before(async () => {
    aw = fakeAw();
    await new Promise<void>((r) => aw.listen(0, "127.0.0.1", r));
    d = await launch({
      config: [`activitywatch_url = "http://127.0.0.1:${(aw.address() as AddressInfo).port}"`],
      files: ({ data, notes }) => {
        writeFileSync(join(data, "work", "projects.toml"), `[Margin]\nexport_code = "M"\n`);
        writeFileSync(join(notes, "tasks.org"), "* TODO Ship the suggestions #61\n");
      },
    });
  });
  after(async () => { await close(); aw.close(); }, { timeout: 30_000 });

  it("shows URLs and tasks, dismisses for good, and accepts with edited times", async () => {
    await find(".agenda h1", "Today");
    await browser.keys([Key.Ctrl, "4"]);
    await find(".time h1", "Time");
    await find(".suggested p", "Nothing to suggest");
    await browser.keys("h");
    const row = (text: string) => find(".suggested tr", text);
    await row("github.com/albin02forsberg/margin");
    await row("Margin / Ship the suggestions #61"); // task guessed from the PR number

    await clickIn(".suggested tr", "Visual Studio Code", "button[title=Dismiss]");
    await fileMatches(join(d.data, "work", "activity_dismissed.json"), (s) => s.includes(`"${day}":[["${day}T11:00:00","${day}T11:30:00"]]`), "dismissal not kept");
    await browser.waitUntil(async () => !(await texts(".suggested tr")).some((t) => t.includes("Visual Studio Code")));

    await clickIn(".suggested tr", "github.com", "button[title^=Change]");
    await answer("Start (HH:MM)", "09:10");
    await answer("End (HH:MM)", "9:50");
    await answer("Log 09:10–09:50 to project", "Margin");
    await answer("What did you do?", "reviewed");
    await fileMatches(join(d.data, "work", "timelog.jsonl"), (s) =>
      s.includes(`{"ev":"in","t":"${day}T09:10:00","project":"Margin"}\n{"ev":"out","t":"${day}T09:50:00","note":"reviewed"}`), "edited session not logged");
    await find(".suggested p", "Nothing to suggest"); // what's left is 10 minutes either side
  });
});

describe("Built-in activity watcher", () => {
  let d: Dirs;
  before(async () => {
    d = await launch({
      config: [`activity_watcher = true`],
      files: ({ data }) => {
        writeFileSync(join(data, "work", "projects.toml"), `[Margin]\nexport_code = "M"\n\n[Acme]\nexport_code = "A"\n`);
        mkdirSync(join(data, "activity"));
        const rec = (s: string, e: string, app: string, title: string) => JSON.stringify({ start: `${day}T${s}`, end: `${day}T${e}`, app, title });
        writeFileSync(join(data, "activity", `${day}.jsonl`), [rec("09:00:00", "09:40:00", "Code", "lib.rs - margin"), rec("09:41:00", "10:00:00", "KeePassXC", "bank.kdbx")].join("\n") + "\n");
      },
    });
  });
  after(close, { timeout: 30_000 });

  it("suggests sessions from its own log when ActivityWatch is off", async () => {
    await find(".agenda h1", "Today");
    await browser.keys([Key.Ctrl, "4"]);
    await find(".suggested h3", "this computer's activity");
    await browser.keys("h");
    await find(".suggested tr", "Margin");
    const rows = await texts(".suggested tr");
    assert.equal(rows.length, 1, rows.join("\n"));
    assert.match(rows[0], /09:00–09:40/);
    assert.doesNotMatch(rows[0], /KeePass/, "excluded app shown");
  });

  it("learns the project picked over the guess, and deletes the rule in Settings", async () => {
    const rules = join(d.data, "work", "activity_rules.toml");
    await clickIn(".suggested tr", "Margin", "button");
    await answer("Log 09:00–09:40 to project", "Acme");
    await answer("What did you do?", "");
    await fileMatches(rules, (s) => s.includes(`project = "Acme"`), "rule not learned");
    await browser.keys([Key.Ctrl, ","]);
    await find(".settings .rules .row", "→ Acme");
    await clickIn(".settings .rules .row", "Acme", "button[title='Delete rule']");
    await fileMatches(rules, (s) => !s.includes("[[rule]]"), "rule not deleted");
    await browser.waitUntil(async () => (await texts(".settings .rules .row")).length === 0, { timeoutMsg: "deleted rule still listed" });
  });
});
