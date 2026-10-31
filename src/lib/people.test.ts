import { test } from "node:test";
import assert from "node:assert/strict";
import { byLastContact, contactAge, lastLine, peopleFrom, personName, personRows } from "./people.ts";

test("contactAge, byLastContact and lastLine (#969)", () => {
  assert.equal(contactAge("2026-09-19", "2026-10-30"), "41 d ago");
  assert.equal(contactAge("2026-10-30", "2026-10-30"), "today");
  assert.equal(contactAge(undefined, "2026-10-30"), "never dated");
  const items = byLastContact(["Anna", "bo", "cy", "Dan"], { anna: ["2026-09-19", "journal"], bo: ["2026-01-02", "task"], dan: ["2026-10-30", "note"] }, "2026-10-30");
  assert.deepEqual(items.map((i) => [i.label, i.detail, i.value]), [
    ["@cy", "never dated", "cy"],
    ["@bo", "301 d ago", "bo"],
    ["@Anna", "41 d ago", "Anna"],
    ["@Dan", "today", "Dan"],
  ]);
  assert.equal(lastLine({ date: "2026-09-19", source: "journal" }, "2026-10-30"), "Last mentioned 41 days ago (journal 2026-09-19)");
  assert.equal(lastLine({ date: "2026-10-29", source: "ledger" }, "2026-10-30"), "Last mentioned 1 day ago (ledger 2026-10-29)");
  assert.equal(lastLine(null, "2026-10-30"), "Never mentioned with a date");
});

test("peopleFrom keeps @ tags once per name in any case, sorted", () => {
  assert.deepEqual(peopleFrom(["work", "@ola", "@Anna", "@anna", "email@x", "@", "@bo"]), ["Anna", "bo", "ola"]);
  assert.deepEqual(peopleFrom(["work"]), []);
});

test("personName strips @ and spaces", () => {
  assert.equal(personName("  @@Anna "), "Anna");
  assert.equal(personName("ola"), "ola");
  assert.equal(personName(" @ "), "");
});

test("personRows lists tasks with priority and date, then notes", () => {
  const rows = personRows({
    tasks: [
      { path: "/n/w.org", line: 3, keyword: "TODO", title: "Call @anna", date: "2026-10-24", priority: "A" },
      { path: "/n/w.org", line: 5, keyword: "NEXT", title: "Send deck", date: "+262142-12-31", priority: null },
    ],
    notes: [{ path: "/n/m.org", line: 2, title: "Meeting", text: "Ask @anna" }],
  }, (p) => p.slice(3));
  assert.deepEqual(rows.map((r) => [r.label, r.detail, r.value]), [
    ["TODO [#A] Call @anna", "task · w.org · 2026-10-24", ["/n/w.org", 3]],
    ["NEXT Send deck", "task · w.org", ["/n/w.org", 5]],
    ["Ask @anna", "Meeting · line 3", ["/n/m.org", 2]],
  ]);
});

test("personRows lists ledger payments last", () => {
  const rows = personRows({ tasks: [], notes: [], payments: [{ path: "/n/ledger.org", line: 4, date: "2026-10-25", cents: -1250, currency: "EUR", text: "lunch @anna" }] }, (p) => p);
  assert.deepEqual(rows.map((r) => [r.label, r.detail, r.value]), [["−12.50 EUR lunch @anna", "ledger · 2026-10-25", ["/n/ledger.org", 4]]]);
});
