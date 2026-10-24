import { test } from "node:test";
import assert from "node:assert/strict";
import { num, sparkline, type Row } from "./metrics.ts";

const row = (date: string, ...values: (number | null)[]): Row => ({ date, values, hours: 0 });

test("sparkline: x by day in the range, y from min (bottom) to max (top), gaps skipped", () => {
  const rows = [row("2026-10-21", 2, 7), row("2026-10-22", null, 7), row("2026-10-23", 4, 7)];
  assert.deepEqual(sparkline(rows, 0, "2026-10-23", 3, 100, 20), [[0, 20], [100, 0]]);
  assert.deepEqual(sparkline(rows, 1, "2026-10-23", 3, 100, 20), [[0, 10], [50, 10], [100, 10]]); // flat: mid-height
  assert.deepEqual(sparkline(rows, 0, "2026-11-01", 30, 290, 20).map((p) => p[0]), [180, 200]); // across a month end
  assert.deepEqual(sparkline([], 0, "2026-10-23", 30, 100, 20), []);
});

test("num: one decimal at most", () => {
  assert.deepEqual([num(10 / 3), num(4), num(null)], ["3.3", "4", "–"]);
});
