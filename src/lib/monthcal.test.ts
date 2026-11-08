import { test } from "node:test";
import assert from "node:assert/strict";
import { addDays, bucket, monthGrid } from "./monthcal.ts";

test("monthGrid: Monday-first full weeks, 4 to 6 rows", () => {
  const nov = monthGrid("2026-11"); // starts on a Sunday: six leading days of October, 6 rows
  assert.equal(nov.length, 42);
  assert.deepEqual([nov[0], nov[6], nov[41]], ["2026-10-26", "2026-11-01", "2026-12-06"]);
  const jun = monthGrid("2026-06"); // starts on a Monday, 30 days: 5 rows
  assert.deepEqual([jun[0], jun.length, jun.at(-1)], ["2026-06-01", 35, "2026-07-05"]);
  const feb = monthGrid("2027-02"); // Monday, 28 days: 4 rows
  assert.deepEqual([feb[0], feb.length, feb.at(-1)], ["2027-02-01", 28, "2027-02-28"]);
  assert.deepEqual(monthGrid("2028-02").slice(-2), ["2028-03-04", "2028-03-05"]); // leap year
});

test("monthGrid: a Sunday or Saturday week start rotates the grid", () => {
  const nov = monthGrid("2026-11", "sunday"); // starts on a Sunday: no leading days, 5 rows
  assert.deepEqual([nov[0], nov.length, nov.at(-1)], ["2026-11-01", 35, "2026-12-05"]);
  const sat = monthGrid("2026-11", "saturday"); // one leading Saturday
  assert.deepEqual([sat[0], sat[1], sat.length, sat.at(-1)], ["2026-10-31", "2026-11-01", 35, "2026-12-04"]);
  assert.deepEqual(monthGrid("2026-11", "monday"), monthGrid("2026-11"));
  assert.deepEqual(monthGrid("2027-01", "sunday").slice(0, 6), ["2026-12-27", "2026-12-28", "2026-12-29", "2026-12-30", "2026-12-31", "2027-01-01"]); // year boundary
});

test("addDays: across DST and month / year ends", () => {
  assert.equal(addDays("2026-03-28", 2), "2026-03-30");
  assert.equal(addDays("2026-10-24", 7), "2026-10-31");
  assert.equal(addDays("2026-12-31", 1), "2027-01-01");
  assert.equal(addDays("2026-03-01", -1), "2026-02-28");
});

test("bucket: scheduled, deadlines, effort once per heading; done and today-only rows left out", () => {
  const it = (date: string, kind: string, title: string, line: number, effort?: number, keyword = "TODO") => ({ date, kind, title, path: "a.org", line, effort, keyword });
  const items = [
    it("2026-11-02", "scheduled", "Water", 1, 10), it("2026-11-09", "scheduled", "Water", 1, 10), // a weekly repeater, one row per day
    it("2026-11-02", "scheduled", "Report", 5, 60), it("2026-11-02", "deadline", "Report", 5, 60),
    it("2026-11-02", "timestamp", "Meeting", 9),
    it("2026-11-02", "scheduled", "Shipped", 12, 30, "DONE"),
    it("2026-11-03", "overdue", "Report", 5, 60), it("2026-11-03", "warning", "Tax", 14),
  ];
  const b = bucket(items, (x) => x.keyword === "DONE");
  assert.deepEqual(b["2026-11-02"], { scheduled: 3, deadline: 1, effort: 70, titles: ["Water", "Report", "Meeting"], birthdays: [] });
  assert.deepEqual(b["2026-11-09"], { scheduled: 1, deadline: 0, effort: 10, titles: ["Water"], birthdays: [] });
  assert.equal(b["2026-11-03"], undefined);
});

test("bucket: birthdays and anniversaries on their day, not as tasks; heads-ups left out", () => {
  const bd = (date: string, title: string, label: string) => ({ date, kind: "birthday", title, label, path: "p.org", line: 0, keyword: null });
  const items = [
    bd("2026-11-02", "Maja", "Turns 34 in 2d"), bd("2026-11-04", "Maja", "Turns 34"), bd("2026-11-04", "Anna", "Anniversary: 5 years"),
    { date: "2026-11-04", kind: "scheduled", title: "Cake", path: "a.org", line: 3, keyword: "TODO" },
  ];
  const b = bucket(items, () => false);
  assert.equal(b["2026-11-02"], undefined);
  assert.deepEqual(b["2026-11-04"], { scheduled: 1, deadline: 0, effort: 0, titles: ["Cake"], birthdays: ["🎂 Maja", "💍 Anna"] });
});
