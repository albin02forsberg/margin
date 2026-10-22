import { test } from "node:test";
import assert from "node:assert/strict";
import { carryEntry, extractOpen } from "./carryover.ts";

const ex = (t: string) => extractOpen(t, ["TODO", "WAIT"], ["DONE"]);

test("open checkboxes move with their indented lines; checked ones stay", () => {
  const r = ex("#+TITLE: Mon\n- [X] done\n- [ ] call bank\n  about the card\n+ [ ] plus\n- note\n");
  assert.deepEqual(r.items, ["- [ ] call bank\n  about the card", "+ [ ] plus"]);
  assert.equal(r.kept, "#+TITLE: Mon\n- [X] done\n- note\n");
});

test("mixed nesting: only unchecked items move, with their children", () => {
  const r = ex("- [X] parent\n  - [ ] child\n    detail\n  - [X] other\n- [ ] open\n  - [X] sub\n");
  assert.deepEqual(r.items, ["- [ ] child\n  detail", "- [ ] open\n  - [X] sub"]);
  assert.equal(r.kept, "- [X] parent\n  - [X] other\n");
});

test("TODO subtrees move whole, re-levelled to 2, after the checkboxes; checkboxes inside travel with them", () => {
  const r = ex("* 09:00 standup\n- [ ] ask Bob\n** TODO write report :work:\nbody\n- [ ] inner\n*** sub\n** WAIT reply\n\n* 10:00 lunch\n");
  assert.deepEqual(r.items, ["- [ ] ask Bob", "** TODO write report :work:\nbody\n- [ ] inner\n*** sub", "** WAIT reply"]);
  assert.equal(r.kept, "* 09:00 standup\n\n* 10:00 lunch\n");
});

test("DONE and archived subtrees stay, open checkboxes inside them too", () => {
  const t = "* DONE shipped\n- [ ] leftover\n** TODO nested\n* TODO old :x:ARCHIVE:\n- [ ] y\n";
  assert.deepEqual(ex(t), { kept: t, items: [] });
});

test("CRLF is kept; running twice finds nothing", () => {
  const r = ex("* Day\r\n- [ ] a\r\n- [X] b\r\n");
  assert.equal(r.kept, "* Day\r\n- [X] b\r\n");
  assert.deepEqual(r.items, ["- [ ] a"]);
  assert.deepEqual(ex(r.kept).items, []);
});

test("carryEntry puts the items under one heading", () => {
  assert.equal(carryEntry("2026-10-21", ["- [ ] a", "** TODO b"]), "* Carried over from 2026-10-21\n- [ ] a\n** TODO b");
});
