import { test } from "node:test";
import assert from "node:assert/strict";
import { fmtEffort, fmtSpent, overbooked, planned, plannedLabel } from "./effort.ts";

test("fmtSpent: against the estimate, over, unestimated, nothing clocked", () => {
  assert.deepEqual(fmtSpent(70, 120), { text: "1:10 / 2:00", over: false });
  assert.deepEqual(fmtSpent(125, 120), { text: "2:05 / 2:00", over: true });
  assert.deepEqual(fmtSpent(120, 120), { text: "2:00 / 2:00", over: false });
  assert.deepEqual(fmtSpent(5, null), { text: "0:05", over: false });
  assert.equal(fmtSpent(0, 30), null);
  assert.equal(fmtSpent(undefined, 30), null);
});

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
