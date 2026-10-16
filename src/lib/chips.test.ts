import { test } from "node:test";
import assert from "node:assert/strict";
import { chips } from "./chips.ts";

const it = (title: string, date: string, time: string | null, done = false) => ({ title, date, time, done });
const items = [it("late", "2026-10-02", "16:30"), it("untimed", "2026-10-02", null), it("early", "2026-10-02", "9:00-10:00"),
  it("finished", "2026-10-02", "14:00", true), it("tomorrow", "2026-10-03", "08:00")];
const show = (cs: ReturnType<typeof chips>) => cs.map((c) => `${c.time} ${c.title} ${c.i} ${c.state}`);

test("chips: the day's timed items by start time, past or done ones dimmed", () => {
  assert.deepEqual(show(chips(items, "2026-10-02", "2026-10-02", "13:00", (x) => x.done)),
    ["09:00–10:00 early 2 done", "14:00 finished 3 done", "16:30 late 0 "]);
  assert.deepEqual(show(chips(items, "2026-10-03", "2026-10-02", "13:00", (x) => x.done)), ["08:00 tomorrow 4 "]);
  assert.deepEqual(show(chips(items, "2026-10-02", "2026-10-03", "07:00", (x) => x.done)).map((s) => s.endsWith("done")), [true, true, true]);
});

test("chips: the running clock is a live chip on today only", () => {
  const clock = { project: "Harbor", since: "2026-10-02T13:38:00" };
  assert.deepEqual(show(chips(items, "2026-10-02", "2026-10-02", "13:40", (x) => x.done, clock)),
    ["09:00–10:00 early 2 done", "13:38 Harbor · tracking -1 live", "14:00 finished 3 done", "16:30 late 0 "]);
  assert.equal(chips(items, "2026-10-03", "2026-10-02", "13:40", (x) => x.done, clock).length, 1);
});
