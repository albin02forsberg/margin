import { test } from "node:test";
import assert from "node:assert/strict";
import { parseSession } from "./session.ts";

const empty = { tabs: [], active: -1, split: -1 };

test("unreadable or missing sessions are empty", () => {
  for (const raw of [null, "", "{", "null", "[]", '{"tabs":3}', '"x"']) assert.deepEqual(parseSession(raw), empty, String(raw));
});

test("keeps good entries, drops bad ones and moves the active index with them", () => {
  const tabs = [
    { path: "/n/a.org", line: 3 },
    { path: "/n/b.org" }, // no line
    { kind: "todo", query: "tag:x", title: "x" },
    { kind: "report", title: "r" },
    { path: "/n/c.org", line: 0 },
  ];
  assert.deepEqual(parseSession(JSON.stringify({ tabs, active: 4 })), { tabs: [tabs[0], tabs[2], tabs[4]], active: 2, split: -1 });
  assert.equal(parseSession(JSON.stringify({ tabs, active: 1 })).active, -1); // the active one was dropped
  assert.equal(parseSession(JSON.stringify({ tabs })).active, -1);
});

test("caps the number of tabs", () => {
  const tabs = Array.from({ length: 30 }, (_, i) => ({ path: `/n/${i}.org`, line: i }));
  const s = parseSession(JSON.stringify({ tabs, active: 25 }));
  assert.equal(s.tabs.length, 20);
  assert.equal(s.active, -1);
  assert.equal(parseSession(JSON.stringify({ tabs, active: 5 })).active, 5);
});

test("split is an index like active, -1 when absent (old sessions) or dropped", () => {
  const tabs = [{ path: "/n/a.org", line: 0 }, { path: "/n/b", line: -1 }, { path: "/n/c.org", line: 0 }];
  assert.deepEqual(parseSession(JSON.stringify({ tabs, active: 0, split: 2 })), { tabs: [tabs[0], tabs[2]], active: 0, split: 1 });
  assert.equal(parseSession(JSON.stringify({ tabs, active: 0, split: 1 })).split, -1);
  assert.equal(parseSession(JSON.stringify({ tabs, active: 0 })).split, -1);
});
