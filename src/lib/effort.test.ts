import { test } from "node:test";
import assert from "node:assert/strict";
import { fmtEffort, overbooked, planned, plannedLabel } from "./effort.ts";

test("fmtEffort: minutes, whole hours, hours and minutes", () => {
  assert.deepEqual([15, 60, 90, 240, 125].map(fmtEffort), ["15m", "1h", "1h 30m", "4h", "2h 5m"]);
});

test("planned: open tasks only, each heading once, none without estimates", () => {
  const it = (line: number, effort: number | null, done = false) => ({ path: "/n/a.org", line, effort, done });
  const isDone = (i: { done: boolean }) => i.done;
  assert.equal(planned([it(1, 30), it(1, 30), it(2, 60), it(3, 120, true), it(4, null)], isDone), 90);
  assert.equal(planned([it(4, null)], isDone), 0);
});

test("overbooked / plannedLabel: fits, over, day off, unknown", () => {
  assert.deepEqual(([[330, 7.5], [450, 7.5], [451, 7.5], [30, 0], [0, 0], [600, undefined]] as [number, number?][]).map(([p, e]) => overbooked(p, e)), [false, false, true, true, false, false]);
  assert.equal(plannedLabel(330, 7.5), "~5h 30m / 7h 30m planned");
  assert.equal(plannedLabel(30, 0), "~30m planned");
  assert.equal(plannedLabel(30), "~30m planned");
});
