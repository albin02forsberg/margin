import { test } from "node:test";
import assert from "node:assert/strict";
import { addDays, due, grade, propValue, type Card } from "./drill.ts";

const card = (title: string, d: string | null, interval: number | null = null, ease: number | null = null): Card => ({ path: "/n/a.org", line: 0, title, body: "x", due: d, interval, ease });

test("addDays: month and year boundaries", () => {
  assert.equal(addDays("2026-10-30", 3), "2026-11-02");
  assert.equal(addDays("2026-12-31", 1), "2027-01-01");
  assert.equal(addDays("2028-02-28", 1), "2028-02-29");
});

test("due: past and today's cards and new ones, overdue first; future ones wait", () => {
  const cs = [card("new", null), card("later", "2026-10-24"), card("today", "2026-10-23"), card("old", "2026-10-01")];
  assert.deepEqual(due(cs, "2026-10-23").map((c) => c.title), ["old", "today", "new"]);
});

test("grade: a new card", () => {
  const today = "2026-10-23";
  assert.deepEqual(grade(card("q", null), 1, today), { due: "2026-10-24", interval: 1, ease: 2.3 });
  assert.deepEqual(grade(card("q", null), 2, today), { due: "2026-10-24", interval: 1, ease: 2.35 });
  assert.deepEqual(grade(card("q", null), 3, today), { due: "2026-10-24", interval: 1, ease: 2.5 });
  assert.deepEqual(grade(card("q", null), 4, today), { due: "2026-10-27", interval: 4, ease: 2.65 });
});

test("grade: a reviewed card grows by its ease; again resets; ease floors at 1.3", () => {
  const c = card("q", "2026-10-23", 10, 2.5);
  assert.deepEqual(grade(c, 3, "2026-10-23"), { due: "2026-11-17", interval: 25, ease: 2.5 });
  assert.deepEqual(grade(c, 4, "2026-10-23").interval, 33);
  assert.deepEqual(grade(c, 2, "2026-10-23").interval, 12);
  assert.deepEqual(grade(c, 1, "2026-10-23").interval, 1);
  assert.equal(grade(card("q", null, 1, 1.35), 1, "2026-10-23").ease, 1.3);
  assert.equal(grade(card("q", null, 1, 1.3), 3, "2026-10-23").interval, 2); // always grows
});

test("propValue: one KEY VALUE per line", () => {
  assert.equal(propValue({ due: "2026-10-24", interval: 1, ease: 2.3 }), "DRILL_DUE 2026-10-24\nDRILL_INTERVAL 1\nDRILL_EASE 2.3");
});
