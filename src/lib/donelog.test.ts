import { test } from "node:test";
import assert from "node:assert/strict";
import { doneLog, withLog } from "./donelog.ts";

const it = (title: string, date: string, line: number, closed: string | null = null) =>
  ({ title, date, time: null as string | null, label: "DONE", kind: "todo", path: "/n/a.org", line, closed });

test("doneLog: rows on their close day by close time; unstamped tasks left out", () => {
  const closed = [it("late", "9999-12-31", 1, "2026-10-14T17:05:00"), it("early", "2026-10-01", 2, "2026-10-14T09:30:00"), it("unstamped", "2026-10-14", 4)];
  assert.deepEqual(doneLog(closed).map((r) => `${r.date} ${r.time} ${r.title} ${r.kind} ${r.label}`),
    ["2026-10-14 09:30 early closed ", "2026-10-14 17:05 late closed "]);
});

test("withLog: each day's log rows after that day's items; a logged task leaves that day's items", () => {
  const items = [it("a", "2026-10-14", 1), it("b", "2026-10-14", 2), it("c", "2026-10-15", 3), it("y", "2026-10-14", 5)];
  const log = [it("x", "2026-10-14", 4), it("y", "2026-10-15", 5), it("b", "2026-10-14", 2), it("z", "2026-10-16", 6)];
  assert.deepEqual(withLog(items, log).map((r) => r.title).join(""), "ayxbcyz");
});
