import { test } from "node:test";
import assert from "node:assert/strict";
import { blocks, gapLabel, gaps, shiftTime } from "./dayplan.ts";

const D = "2026-10-23";
const it = (line: number, time: string | null, effort: number | null = null, done = false, date = D) => ({ date, time, title: `t${line}`, path: "/n/a.org", line, effort, done });
const isDone = (x: { done: boolean }) => x.done;
const span = (bs: ReturnType<typeof blocks>) => bs.map((b) => [b.i, b.start, b.end, b.overlap, b.lane, b.lanes]);

test("blocks: effort or 30 min, a range uses its own end, untimed / other days excluded, one per heading", () => {
  const bs = blocks([it(1, "9:00", 90), it(2, "10:30-11:15", 120), it(3, null, 60), it(4, "12:00"), it(5, "13:00", 30, false, "2026-10-24"), it(4, "12:00")], D, isDone);
  assert.deepEqual(span(bs), [[0, 540, 630, false, 0, 1], [1, 630, 675, false, 0, 1], [3, 720, 750, false, 0, 1]]);
});

test("blocks: two tasks at 14:00 overlap side by side; a done one doesn't count", () => {
  const bs = blocks([it(1, "14:00", 60), it(2, "14:00", 30), it(3, "14:30", 30), it(4, "14:00", 60, true), it(5, "16:00")], D, isDone);
  assert.deepEqual(span(bs), [[1, 840, 870, true, 0, 2], [0, 840, 900, true, 1, 2], [3, 840, 900, false, 0, 1], [2, 870, 900, true, 0, 2], [4, 960, 990, false, 0, 1]]);
});

test("blocks: clamped at midnight", () => {
  assert.equal(blocks([it(1, "23:30", 120)], D, isDone)[0].end, 1440);
});

test("gaps: within working hours, past now, around open blocks only", () => {
  const bs = blocks([it(1, "7:00", 90), it(2, "10:00", 60), it(3, "10:30", 60), it(4, "13:00", 60, true), it(5, "17:50", 30), it(6, "19:00")], D, isDone);
  assert.deepEqual(gaps(bs).map(gapLabel), ["1h 30m free 08:30–10:00", "6h 20m free 11:30–17:50"]);
  assert.deepEqual(gaps(bs, 480, 1080, 12 * 60 + 5).map(gapLabel), ["5h 45m free 12:05–17:50"]);
  assert.deepEqual(gaps([]).map(gapLabel), ["10h free 08:00–18:00"]);
  assert.deepEqual(gaps(blocks([it(1, "9:00", 1)], D, isDone), 480, 1080).map(gapLabel), ["1h free 08:00–09:00", "8h 59m free 09:01–18:00"]);
});

test("shiftTime: move keeps length (bare stays bare), H/L write a range, 15 min floor, pinned at 00:00 / 23:59", () => {
  assert.equal(shiftTime("9:00", 15, 0), "09:15");
  assert.equal(shiftTime("09:00-10:30", -15, 0), "08:45-10:15");
  assert.equal(shiftTime("09:00", 0, 15), "09:00-09:45");
  assert.equal(shiftTime("09:00", 0, 15, 60), "09:00-10:15");
  assert.equal(shiftTime("09:00-10:00", 0, -15), "09:00-09:45");
  assert.equal(shiftTime("09:00-09:15", 0, -15), null);
  assert.equal(shiftTime("09:00-09:20", 0, -15), "09:00-09:15");
  assert.equal(shiftTime("00:00", -15, 0), null);
  assert.equal(shiftTime("00:10-01:00", -15, 0), "00:00-00:50");
  assert.equal(shiftTime("23:00-23:59", 15, 0), null);
  assert.equal(shiftTime("23:00-23:50", 15, 0), "23:09-23:59");
  assert.equal(shiftTime("23:00-23:50", 0, 15), "23:00-23:59");
  assert.equal(shiftTime("23:50", 0, -15), null);
});
