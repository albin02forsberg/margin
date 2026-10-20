// End-to-end tests of the time-tracking flows past start/stop: started/stopped earlier, break/resume,
// switching project and the CSV export. Own launch, so the time log starts empty.
import { after, before, describe, it } from "node:test";
import assert from "node:assert/strict";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { answer, browser, close, fileMatches, find, isoDay, Key, launch, noPicker, pad, palette, read, texts, timeFocused, type Dirs } from "./setup.ts";

let d: Dirs, log: string;
type Ev = { ev: string; t: string; project?: string; note?: string };
const events = (s: string): Ev[] => s.split("\n").filter(Boolean).map((l) => JSON.parse(l));
const evs = (s: string) => events(s).map((e) => e.ev + (e.ev === "in" ? ":" + e.project : "")).join(" ");
const hm = (t: Date) => `${pad(t.getHours())}:${pad(t.getMinutes())}`;
/** Within a minute of T (minute-truncated; the clock-in keeps its seconds, so it lands <30 s off after rounding). */
const near = (iso: string, t: Date) => Math.abs(Date.parse(iso) - t.getTime()) < 60_000;

before(async () => {
  d = await launch({ files: ({ data }) => writeFileSync(join(data, "work", "projects.toml"), `[Acme]\nexport_code = "ACME"\n\n[Beta]\nexport_code = "BETA"\n`) });
  log = join(d.data, "work", "timelog.jsonl");
  await find(".agenda h1", "Today"); // loaded
});
after(close, { timeout: 30_000 });

async function start(project: string, task: string) {
  await noPicker();
  await browser.keys([Key.Ctrl, "4"]);
  // A failed earlier test may leave a timer running; stop it so one failure doesn't cascade.
  if ((await texts(".time .buttons button")).some((t) => t.includes("Stop"))) {
    await (await find(".time .buttons button", "Stop")).click();
    await answer("What did you do?", "leftover");
    await fileMatches(log, (s) => events(s).at(-1)?.ev === "out", "leftover session not stopped");
  }
  await (await find(".time button", "Start tracking")).click();
  await answer("Track time on project", project);
  await answer("What are you working on?", task);
  await find(".time .status h2", project);
}

describe("Time tracking", () => {
  it("moves the start back with I started earlier… and stops at an earlier time", async () => {
    const minute = (ms: number) => new Date(new Date(Date.now() - ms).setSeconds(0, 0)); // what the test types, exactly
    const [from, to] = [minute(60 * 60_000), minute(30 * 60_000)];
    await start("Acme", "earlier");
    await noPicker();
    await palette("Time: I started earlier");
    await answer("When did you actually start?", hm(from));
    await fileMatches(log, (s) => { const e = events(s); return e.length === 1 && near(e[0].t, from); }, "clock-in not moved back");
    await noPicker();
    await timeFocused();
    await browser.keys("O");
    await answer("When did you actually stop?", hm(to));
    await answer("What did you do?", "half an hour");
    await fileMatches(log, (s) => { const e = events(s); return evs(s) === "in:Acme out" && e[1].t === `${isoDay(to)}T${hm(to)}:00` && e[1].note === "half an hour"; }, "not clocked out at the earlier time");
  });

  it("opens the edit-session and time-off prompts from the keys S and L", async () => {
    await noPicker();
    await timeFocused();
    await browser.keys("S");
    await find(".picker label span", "Edit a session on");
    await browser.keys(Key.Escape);
    await noPicker();
    await timeFocused();
    await browser.keys("L");
    await find(".picker label span", "Kind of time off");
    await browser.keys(Key.Escape);
    await noPicker();
  });

  it("takes a break and resumes on the same project", async () => {
    await start("Acme", "break check");
    await palette("Time: take a break");
    await fileMatches(log, (s) => evs(s).endsWith("in:Acme break"), "no break in the log");
    await noPicker();
    await (await find(".time .buttons button", "Resume")).click();
    await fileMatches(log, (s) => evs(s).endsWith("in:Acme break in:Acme"), "not resumed on Acme");
    await (await find(".time .buttons button", "Stop")).click();
    await answer("What did you do?", "after the break");
    await fileMatches(log, (s) => evs(s) === "in:Acme out in:Acme break in:Acme out", "break/resume not logged in order");
  });

  it("switches project, closing the first session", async () => {
    await start("Acme", "switch check");
    await palette("Time: switch project");
    await answer("What did you do on", "acme part");
    await answer("Switch to project", "Beta");
    await find(".time .status h2", "Beta");
    await fileMatches(log, (s) => { const e = events(s); return evs(s).endsWith("in:Acme out in:Beta") && e.at(-2)?.note === "acme part"; }, "switch didn't close Acme and open Beta");
    await (await find(".time .buttons button", "Stop")).click();
    await answer("What did you do?", "beta part");
    await fileMatches(log, (s) => evs(s).endsWith("in:Beta out"), "Beta not stopped");
  });

  it("exports a CSV with the project's export code and no 0 h rows", async () => {
    const csv = join(d.exportDir, "time_work.csv"); // the export dir doesn't exist yet
    await noPicker();
    await palette("Time: export CSV");
    await answer("Export from", isoDay(new Date(Date.now() - 86_400_000)));
    await answer("Export to", "today");
    await answer("Save CSV as", csv);
    await fileMatches(csv, (s) => /^"ACME","half an hour","[\d-]+","0,(4[89]|5[012]?)"$/m.test(s), "no ACME row in the CSV"); // ~30 min: the start moved back by whole minutes
    const rows = read(csv).trim().split("\n").slice(1);
    assert.ok(!rows.some((r) => r.endsWith(',"0"') || r.startsWith('"BETA"')), `0 h rows in the CSV:\n${read(csv)}`); // Beta only has seconds
  });

  it("logs a past session on an earlier day", async () => {
    const day = isoDay(new Date(Date.now() - 3 * 86_400_000));
    await noPicker();
    await palette("Time: log a past session");
    await answer("Log a session on", day);
    await answer("Start", "09:00");
    await answer("End", "10:30");
    await answer("Log 09:00–10:30 to project", "Acme");
    await answer("What did you do?", "late entry");
    await fileMatches(log, (s) => events(s).some((e) => e.ev === "out" && e.t === `${day}T10:30:00` && e.note === "late entry"), "past session not logged");
  });

  it("edits that session's description and duration, moving its end", async () => {
    const day = isoDay(new Date(Date.now() - 3 * 86_400_000));
    await noPicker();
    await palette("Time: edit a session");
    await answer("Edit a session on", day);
    await answer("Which session?", "late entry");
    await answer("Description", "edited entry");
    await answer("Duration in hours", "2");
    await fileMatches(log, (s) => events(s).some((e) => e.ev === "out" && e.t === `${day}T11:00:00` && e.note === "edited entry") && !s.includes("late entry"), "session not edited");
  });

  it("reads a weekday name as the previous such day when logging a session", async () => {
    const name = ["sun", "mon", "tue", "wed", "thu", "fri", "sat"][new Date(Date.now() - 2 * 86_400_000).getDay()];
    const day = isoDay(new Date(Date.now() - 2 * 86_400_000));
    await noPicker();
    await palette("Time: log a past session");
    await answer("Log a session on", name);
    await answer("Start", "11:00");
    await answer("End", "11:30");
    await answer("Log 11:00–11:30 to project", "Acme");
    await answer("What did you do?", "weekday entry");
    await fileMatches(log, (s) => events(s).some((e) => e.ev === "out" && e.t === `${day}T11:30:00` && e.note === "weekday entry"), "weekday session not logged on the previous such day");
  });

  it("Space t h lists the public holidays in a Holidays tab", async () => {
    await noPicker();
    await palette("Time: public holidays");
    await find(".tabs .tab > button:not(.x)", "Holidays");
    await find(".cm-content", "Christmas Day"); // a fixed-date red day, whatever year it is
  });

  it("Space t D reports a clean log, then flags an unparseable line", async () => {
    const orig = read(log);
    try {
      writeFileSync(log, `{"ev":"in","t":"2020-01-06T09:00:00","project":"Acme"}\n{"ev":"out","t":"2020-01-06T10:00:00","note":"clean"}\n`);
      await noPicker();
      await palette("Time: check log for problems");
      await find(".tabs .tab > button:not(.x)", "Log check");
      await find(".cm-content", "No issues found");
      writeFileSync(log, read(log) + "this is not json\n");
      await noPicker();
      await palette("Time: check log for problems");
      await find(".cm-content", "Line 3: unrecognized entry");
    } finally {
      writeFileSync(log, orig);
    }
  });
});
