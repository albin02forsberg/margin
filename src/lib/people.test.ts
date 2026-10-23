import { test } from "node:test";
import assert from "node:assert/strict";
import { peopleFrom, personName, personRows } from "./people.ts";

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
