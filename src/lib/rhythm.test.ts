import { test } from "node:test";
import assert from "node:assert/strict";
import { mins, shade, summary, type Rhythm } from "./rhythm.ts";

const grid = () => Array.from({ length: 7 }, () => Array(24).fill(0));

test("summary: peak cell, evening share and total; empty range", () => {
  const minutes = grid();
  minutes[1][10] = 52.4;
  const r: Rhythm = { minutes, peak: [1, 10], evening: 0.184, hours: 34.55, pieces: [] };
  assert.equal(summary(r), "Peak: Tue 10–11, 52 min avg · 18% after 17:00 · 34.5 h in total");
  assert.equal(summary({ ...r, peak: null }), "No clocked time in this range.");
});

test("shade: 0 only for no time, 4 for the busiest; mins rounds up from under a minute", () => {
  assert.deepEqual([0, 0.1, 15, 30, 31, 60].map((m) => shade(m, 60)), [0, 1, 1, 2, 3, 4]);
  assert.equal(shade(5, 0), 0);
  assert.deepEqual([mins(0), mins(0.2), mins(29.6)], ["–", "1 min", "30 min"]);
});
