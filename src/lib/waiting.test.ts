import { test } from "node:test";
import assert from "node:assert/strict";
import { waitAge, waitingRows } from "./waiting.ts";
import { personRows } from "./people.ts";

test("waitAge counts days, unknown without a date", () => {
  assert.equal(waitAge("2026-10-18", "2026-10-27"), "waiting 9 days");
  assert.equal(waitAge("2026-10-26", "2026-10-27"), "waiting 1 day");
  assert.equal(waitAge("2026-10-27", "2026-10-27"), "waiting since today");
  assert.equal(waitAge("2026-03-01", "2026-03-30"), "waiting 29 days"); // across a DST change
  assert.equal(waitAge(null, "2026-10-27"), "age unknown");
});

const w = (title: string, who: string | null, since: string | null, date = "+262142-12-31") => ({ path: "/n/w.org", line: 1, title, who, since, date });

test("waitingRows: who · title, age, follow-up warns once due", () => {
  const rows = waitingRows([w("Contract", "@anna", "2026-10-20", "2026-10-27"), w("Quote", "Bob", null, "2026-11-02"), w("Reply", null, "2026-10-27")], "2026-10-27", (p) => p.slice(3));
  assert.deepEqual(rows.map((r) => [r.label, r.detail, r.warn]), [
    ["@anna · Contract", "waiting 7 days · follow up 2026-10-27 · w.org", true],
    ["Bob · Quote", "age unknown · follow up 2026-11-02 · w.org", false],
    ["(unspecified) · Reply", "waiting since today · w.org", false],
  ]);
});

test("personRows puts tasks waiting on them first, without repeating them as tasks", () => {
  const rows = personRows({
    waiting: [w("Contract", "@anna", "2026-10-20")],
    tasks: [{ path: "/n/w.org", line: 1, keyword: "WAIT", title: "Contract", date: "+262142-12-31", priority: null }, { path: "/n/w.org", line: 4, keyword: "TODO", title: "Call", date: "+262142-12-31", priority: null }],
    notes: [],
  }, (p) => p.slice(3), "2026-10-27");
  assert.deepEqual(rows.map((r) => [r.label, r.detail]), [["Waiting on them: Contract", "waiting 7 days · w.org"], ["TODO Call", "task · w.org"]]);
});
