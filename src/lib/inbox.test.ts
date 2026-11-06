import { test } from "node:test";
import assert from "node:assert/strict";
import { entryKeys, inboxKeys, summariseInbox } from "./inbox.ts";

test("keys map to actions; n and space both leave the entry", () => {
  assert.deepEqual([..."tmsrgpdn "].map((k) => inboxKeys[k]), ["today", "tomorrow", "date", "refile", "tags", "park", "drop", "skip", "skip"]);
});

test("entry keys tell same-titled entries apart and ignore line and keyword", () => {
  const a = entryKeys([{ title: "Call", line: 0 }, { title: "Mail", line: 2 }, { title: "Call", line: 4 }]).map((e) => e.key);
  assert.deepEqual(a, ["Call\n0", "Mail\n0", "Call\n1"]);
  assert.deepEqual(entryKeys([{ title: "Mail", line: 9, keyword: "TODO" }]).map((e) => e.key), [a[1]]);
});

test("summary counts every step and lists only non-zero kinds", () => {
  assert.equal(summariseInbox({ scheduled: 3, refiled: 2, tagged: 0, parked: 1, dropped: 1 }), "Processed 7: 3 scheduled, 2 refiled, 1 parked, 1 dropped");
  assert.equal(summariseInbox({ scheduled: 0, refiled: 0, tagged: 0, parked: 0, dropped: 0 }), "Processed 0");
});
