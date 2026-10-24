import { test } from "node:test";
import assert from "node:assert/strict";
import { shutdownKeys, shutdownTasks, summarise } from "./shutdown.ts";

const row = (title: string, kind: string, keyword: string | null = "TODO", line = 0, habit: unknown = null) => ({ title, kind, keyword, habit, path: "a.org", line });

test("keys map to actions; n and space both leave the task", () => {
  assert.deepEqual([..."xtwsn "].map((k) => shutdownKeys[k]), ["done", "tomorrow", "week", "date", "skip", "skip"]);
  assert.equal(shutdownKeys["q"], undefined);
});

test("only open scheduled / due / overdue tasks, once each, SCHEDULED preferred", () => {
  const r = shutdownTasks([
    row("Overdue due", "overdue", "TODO", 1), row("Overdue due", "scheduled", "TODO", 1),
    row("Due today", "deadline", "NEXT", 2), row("Done", "scheduled", "DONE", 3), row("Plain", "scheduled", null, 4),
    row("Soon", "warning", "TODO", 5), row("Meeting", "timestamp", "TODO", 6), row("Habit", "scheduled", "TODO", 7, { bar: "", streak: 1 }),
  ], ["TODO", "NEXT"]);
  assert.deepEqual(r.map((i) => [i.title, i.plan]), [["Overdue due", "SCHEDULED"], ["Due today", "DEADLINE"]]);
});

test("summary names done, moved and hours against the target", () => {
  assert.equal(summarise(2, 1, { hours: 6.5, expected: 8 }), "Done 2, moved 1 · 6h 30m clocked of 8h 00m");
  assert.equal(summarise(0, 0, { hours: 0, expected: 0 }), "Done 0, moved 0 · 0m clocked");
});
