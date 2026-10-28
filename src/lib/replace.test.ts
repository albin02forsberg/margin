import { test } from "node:test";
import assert from "node:assert/strict";
import { accepted, changeLabel, hitKey, type Hit } from "./replace.ts";

test("replace label: the changed span with a little context (#872)", () => {
  assert.equal(changeLabel("  Call Acme today", "  Call Initech today"), "Call [Acme → Initech] today");
  assert.equal(changeLabel("acme and acme", "x and x"), "[acme and acme → x and x]");
  assert.equal(changeLabel("a".repeat(30) + "Acme" + "b".repeat(30), "a".repeat(30) + "Ini" + "b".repeat(30), 5), "…aaaaa[Acme → Ini]bbbbb…");
  assert.equal(changeLabel("Acme", "Acme Inc"), "Acme[ →  Inc]");
});

test("replace accept list: every hit but the unticked ones", () => {
  const hits: Hit[] = [["/n/a.org", 0, "Acme", "Ini"], ["/n/a.org", 3, "acme", "Ini"], ["/n/b.org", 0, "Acme", "Ini"]];
  assert.deepEqual(accepted(hits, new Set([hitKey(hits[1])])), [["/n/a.org", 0, "Acme"], ["/n/b.org", 0, "Acme"]]);
  assert.deepEqual(accepted(hits, new Set(hits.map(hitKey))), []);
});
