import { test } from "node:test";
import assert from "node:assert/strict";
import { fmtEffort, planned } from "./effort.ts";

test("fmtEffort: minutes, whole hours, hours and minutes", () => {
  assert.deepEqual([15, 60, 90, 240, 125].map(fmtEffort), ["15m", "1h", "1h 30m", "4h", "2h 5m"]);
});

test("planned: open tasks only, each heading once, none without estimates", () => {
  const it = (line: number, effort: number | null, done = false) => ({ path: "/n/a.org", line, effort, done });
  const isDone = (i: { done: boolean }) => i.done;
  assert.equal(planned([it(1, 30), it(1, 30), it(2, 60), it(3, 120, true), it(4, null)], isDone), 90);
  assert.equal(planned([it(4, null)], isDone), 0);
});
