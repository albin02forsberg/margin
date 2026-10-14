import { test } from "node:test";
import assert from "node:assert/strict";
import { worked, parseWhen, naive } from "./clock.ts";

const day = (date: string, hours: number, expected: number) => ({ date, hours, expected, leave: "", leave_hours: 0 });

test("worked today: progress against the day's target, capped, with time left", () => {
  const week = [day("2026-10-12", 8, 8), day("2026-10-13", 5.6, 8), day("2026-10-14", 9, 8)];
  assert.deepEqual(worked(week, "2026-10-13"), { hours: 5.6, expected: 8, progress: 0.7, left: 8 - 5.6 });
  assert.deepEqual(worked(week, "2026-10-14"), { hours: 9, expected: 8, progress: 1, left: 0 });
});

test("worked today: no target (weekend, time off) or no data", () => {
  assert.deepEqual(worked([day("2026-10-18", 0, 0)], "2026-10-18"), { hours: 0, expected: 0, progress: 0, left: 0 });
  assert.equal(worked([day("2026-10-18", 2, 0)], "2026-10-18").progress, 1);
  assert.deepEqual(worked([], "2026-10-18"), { hours: 0, expected: 0, progress: 0, left: 0 });
});

test("parseWhen: minutes ago, HH:MM today, yesterday rollover, garbage", () => {
  const now = new Date(2026, 9, 14, 10, 0, 0);
  assert.equal(naive(parseWhen("25", now)!), "2026-10-14T09:35:00");
  assert.equal(naive(parseWhen(" 9:05 ", now)!), "2026-10-14T09:05:00");
  assert.equal(naive(parseWhen("23:30", now)!), "2026-10-13T23:30:00");
  for (const bad of ["", "abc", "25:00", "10:75", "-5", "1.5"]) assert.equal(parseWhen(bad, now), null, bad);
});
