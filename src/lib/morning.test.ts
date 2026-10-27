import { test } from "node:test";
import assert from "node:assert/strict";
import { morningKeys, planCandidates, planSummary } from "./morning.ts";

const T = "2026-10-27";
const row = (title: string, kind: string, line: number, o: { date?: string; label?: string; keyword?: string | null; habit?: unknown; blocked?: boolean; path?: string } = {}) =>
  ({ title, kind, line, date: T, label: "", keyword: "TODO", habit: null, blocked: false, path: "a.org", ...o });

test("keys map to actions; n and space both leave the task", () => {
  assert.deepEqual([..."tmsn "].map((k) => morningKeys[k]), ["today", "tomorrow", "date", "skip", "skip"]);
  assert.equal(morningKeys["x"], undefined);
});

test("overdue first, then due soon, then undated; once per heading at its best rank", () => {
  const agenda = [
    row("Due today", "deadline", 1),
    row("Late scheduled", "scheduled", 2, { label: "Scheduled 2d ago" }),
    row("Soon", "scheduled", 3, { date: "2026-10-29" }),
    row("Late deadline", "overdue", 4, { label: "Overdue 1d" }),
    row("Late deadline", "deadline", 4, { date: "2026-10-26" }),
    row("Soon", "scheduled", 3, { date: "2026-10-30" }),
  ];
  const undated = [row("Pick me", "todo", 5), row("Soon", "todo", 3), row("Other file", "todo", 3, { path: "b.org" })];
  assert.deepEqual(planCandidates(agenda, undated, T, ["TODO"]).map((i) => [i.title, i.why]), [
    ["Late scheduled", "overdue"], ["Late deadline", "overdue"], ["Due today", "soon"], ["Soon", "soon"], ["Pick me", "undated"], ["Other file", "undated"],
  ]);
});

test("skips today's scheduled, done, keyword-less, habits, blocked, timestamps and warnings", () => {
  const agenda = [
    row("Already today", "scheduled", 1), row("Already today", "deadline", 1, { date: "2026-10-28" }),
    row("Done", "deadline", 2, { keyword: "DONE" }), row("Plain", "scheduled", 3, { keyword: null, date: "2026-10-28" }),
    row("Habit", "scheduled", 4, { label: "Scheduled 1d ago", habit: { bar: "", streak: 1 } }),
    row("Blocked", "deadline", 5, { blocked: true }), row("Meeting", "timestamp", 6), row("Far", "warning", 7, { label: "Due in 9d" }),
  ];
  assert.deepEqual(planCandidates(agenda, [row("Blocked", "todo", 8, { blocked: true })], T, ["TODO"]), []);
});

test("summary names how many were planned and today's estimate against the target", () => {
  assert.equal(planSummary(2, 210, 8), "Planned 2 for today · 3h 30m estimated of 8h 00m");
  assert.equal(planSummary(0, 0, 0), "Planned 0 for today · 0m estimated");
});
