import { test } from "node:test";
import assert from "node:assert/strict";
import { intoWeek, weekDays, weekStartOf } from "./week.ts";

test("weekStartOf: Monday by default, Sunday or Saturday from week_start, across a year end", () => {
  const d = (y: number, m: number, day: number) => new Date(y, m - 1, day);
  const thu = d(2026, 10, 15);
  assert.deepEqual(weekStartOf(thu), d(2026, 10, 12));
  assert.deepEqual(weekStartOf(thu, "bogus"), d(2026, 10, 12));
  assert.deepEqual(weekStartOf(thu, "sunday"), d(2026, 10, 11));
  assert.deepEqual(weekStartOf(thu, "saturday"), d(2026, 10, 10));
  assert.deepEqual(weekStartOf(d(2026, 10, 11), "sunday"), d(2026, 10, 11)); // the start day itself
  assert.deepEqual(weekStartOf(d(2026, 10, 18)), d(2026, 10, 12)); // Sunday in a Monday week
  assert.deepEqual(weekStartOf(d(2027, 1, 1), "sunday"), d(2026, 12, 27));
  assert.deepEqual(weekStartOf(d(2027, 1, 1), "saturday"), d(2026, 12, 26));
  assert.equal(intoWeek(0, "monday"), 6);
});

test("weekDays: column headers rotate with the week start", () => {
  assert.deepEqual(weekDays(), ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]);
  assert.deepEqual(weekDays("sunday"), ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]);
  assert.deepEqual(weekDays("saturday"), ["Sat", "Sun", "Mon", "Tue", "Wed", "Thu", "Fri"]);
});
