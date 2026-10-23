import { test } from "node:test";
import assert from "node:assert/strict";
import { buildTimeline, timelineMarkdown } from "./timeline.ts";

const s = (start: string, end: string, hours: number, project = "Acme", desc = "work") => ({ project, desc, hours, start, end, line: 1 });
const secs = (d: string, h: number, m: number) => new Date(+d.slice(0, 4), +d.slice(5, 7) - 1, +d.slice(8, 10), h, m).getTime() / 1000;
const day = "2026-10-20";

test("buildTimeline: sessions, closed tasks and edited notes in time order; other days' notes left out", () => {
  const tl = buildTimeline(day, [s("13:00:00", "14:30:00", 1.5), s("08:15:00", "09:00:00", 0.75, "", "")], null,
    [{ title: "Ship it", category: "work", time: "10:05", path: "/n/a.org", line: 3 }],
    [["/n/b.org", "Plans", secs(day, 9, 30)], ["/n/c.org", "", secs("2026-10-19", 9, 30)]]);
  assert.deepEqual(tl.map((e) => e.text), [
    "08:15–09:00 Other: (no description) (45m)", "09:30 Edited: Plans", "10:05 Done: Ship it (work)", "13:00–14:30 Acme: work (1h 30m)",
  ]);
  assert.deepEqual(tl.map((e) => e.kind), ["session", "note", "done", "session"]);
});

test("buildTimeline: a session past midnight stays on its start day; the running clock is open-ended", () => {
  const tl = buildTimeline(day, [s("22:00:00", "01:00:00", 3)], { since: `${day}T07:45:00`, project: "Acme", task: null }, [], []);
  assert.deepEqual(tl.map((e) => e.text), ["07:45– (running) Acme: (no description)", "22:00–01:00 Acme: work (3h 00m)"]);
  const earlier = buildTimeline(day, [], { since: "2026-10-19T23:00:00", project: "Acme", task: "x" }, [], []);
  assert.deepEqual(earlier.map((e) => [e.time, e.text]), [["00:00", "2026-10-19 23:00– (running) Acme: x"]]);
  assert.deepEqual(buildTimeline(day, [], { since: "2026-10-21T08:00:00", project: "", task: "" }, [], []), []);
  assert.deepEqual(buildTimeline(day, [], { since: null, project: null, task: null }, [], []), []);
});

test("timelineMarkdown: a list under the day, or a placeholder for an empty day", () => {
  assert.equal(timelineMarkdown(day, []), "## 2026-10-20\n\n- nothing logged");
  assert.equal(timelineMarkdown(day, buildTimeline(day, [], null, [{ title: "T", category: "", time: "09:00", path: "p", line: 0 }], [])), "## 2026-10-20\n\n- 09:00 Done: T");
});
