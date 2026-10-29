import { test } from "node:test";
import assert from "node:assert/strict";
import { addDays, hours, levels, streaks, weeks } from "./activity.ts";

test("levels: 0 for none, quartiles of the nonzero values, the largest always 4", () => {
  assert.deepEqual(levels([0, 1, 2, 3, 4, 5, 6, 7, 8]), [0, 1, 1, 2, 2, 3, 3, 4, 4]);
  assert.deepEqual(levels([0, 5, 5]), [0, 4, 4]);
  assert.deepEqual(levels([3600]), [4]);
  assert.deepEqual(levels([0, 0]), [0, 0]);
  assert.deepEqual(levels([]), []);
});

test("weeks: Monday-first columns, null outside the year, leap years", () => {
  const w = weeks(2026); // 2026-01-01 is a Thursday, 2026-12-31 a Thursday
  assert.deepEqual(w[0], [null, null, null, "2026-01-01", "2026-01-02", "2026-01-03", "2026-01-04"]);
  assert.deepEqual(w.at(-1), ["2026-12-28", "2026-12-29", "2026-12-30", "2026-12-31", null, null, null]);
  assert.equal(w.flat().filter(Boolean).length, 365);
  assert.equal(weeks(2024).flat().filter(Boolean).length, 366);
  assert.ok(weeks(2024).flat().includes("2024-02-29"));
  assert.equal(weeks(2024)[0][0], "2024-01-01"); // a Monday: no leading blanks
  assert.ok(w.every((c) => c.length === 7));
});

test("streaks: longest in the year, current across New Year, empty", () => {
  const on = (...ds: string[]) => Object.fromEntries(ds.map((d) => [d, 3600]));
  const days = { ...on("2025-12-30", "2025-12-31", "2026-01-01", "2026-01-02", "2026-03-10", "2026-03-11"), "2026-03-12": 0 };
  assert.deepEqual(streaks(days, 2026, "2026-03-12"), { longest: 4, current: 2 }); // today empty: yesterday's run counts
  assert.deepEqual(streaks(days, 2026, "2026-03-14"), { longest: 4, current: 0 });
  assert.deepEqual(streaks(on("2025-12-31", "2026-01-01"), 2026, "2026-01-01"), { longest: 2, current: 2 });
  assert.deepEqual(streaks(days, 2025, "2026-03-12").longest, 2); // 2025 stops at Dec 31
  assert.deepEqual(streaks({}, 2026, "2026-05-01"), { longest: 0, current: 0 });
});

test("addDays and hours", () => {
  assert.equal(addDays("2024-02-28", 1), "2024-02-29");
  assert.equal(addDays("2026-01-01", -1), "2025-12-31");
  assert.deepEqual([hours(0), hours(20), hours(1500), hours(12600)], ["–", "1 min", "25 min", "3.5 h"]);
});
