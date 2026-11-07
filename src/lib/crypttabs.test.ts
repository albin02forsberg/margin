import { test } from "node:test";
import assert from "node:assert/strict";
import { EditorState, type ChangeSpec } from "@codemirror/state";
import { cryptKey, findCryptTab, isCryptKey, remapLine, saveRoute } from "./crypttabs.ts";
import { diffChange } from "./merge.ts";

test("isCryptKey", () => {
  assert.ok(isCryptKey("crypt:/n/a.org:3"));
  assert.ok(isCryptKey("crypt:/n/a.org:3", "/n/a.org"));
  assert.ok(!isCryptKey("crypt:/n/ab.org:3", "/n/a.org"));
  assert.ok(!isCryptKey("/n/a.org"));
});

test("saveRoute: a decrypted tab is never autosaved", () => {
  const c = { crypt: { path: "/n/a.org", line: 0, plain: "x" } };
  assert.equal(saveRoute(c, false), "none");
  assert.equal(saveRoute(c, true), "relock");
  assert.equal(saveRoute({}, false), "file");
  assert.equal(saveRoute({}, true), "file");
});

// Heading B (line 2) spans 6..17; its line break is at 17.
const N = "* A\nx\n* B :crypt:\n-----BEGIN MARGIN ENCRYPTED-----\nabc\n-----END MARGIN ENCRYPTED-----\n* C\n";
/** Heading LINE of DOC after CHANGES. */
const at = (changes: ChangeSpec, line = 2, doc = N) => {
  const s = EditorState.create({ doc }), tr = s.update({ changes });
  return remapLine(line, s.doc, tr.changes, tr.state.doc);
};

test("remapLine: lines added or removed above move the heading", () => {
  assert.equal(at([]), 2);
  assert.equal(at({ from: 6, insert: "y\nz\n" }), 4); // right at B's line start: lands above it
  assert.equal(at({ from: 4, insert: "y\n" }), 3);
  assert.equal(at({ from: 4, to: 6 }), 1); // the line under A deleted
  assert.equal(at({ from: 5, to: 6 }), 1); // B joined onto the line above (the relock then refuses it)
  assert.equal(at({ from: N.length - 4, insert: "\n\n" }), 2); // below
});

test("remapLine: edits on the heading line or its body keep it; deleting the line makes the tab stale", () => {
  assert.equal(at({ from: 9, insert: "!" }), 2);
  assert.equal(at({ from: 6, to: 17 }), 2); // emptied in place (cc)
  assert.equal(at({ from: 51, to: 54, insert: "xyz\nw" }), 2); // its body
  assert.equal(at({ from: 6, to: 18 }), -1); // dd
  assert.equal(at({ from: 5, to: 17 }), -1); // deleted with the line break above
  assert.equal(at({ from: 0, to: N.length }), -1);
  assert.equal(at([], -1), -1);
});

test("remapLine: a relock above (block length changes) shifts a heading below, through diffChange", () => {
  const old = "* L :crypt:\n-----BEGIN MARGIN ENCRYPTED-----\nab\n-----END MARGIN ENCRYPTED-----\n* M :crypt:\nq\n";
  const text = old.replace("ab\n", "abcdefg\nhij\n");
  assert.equal(at(diffChange(old, text), 4, old), 5);
  assert.equal(at(diffChange(old, text), 0, old), 0); // the relocked heading itself stays
  assert.equal(at(diffChange(text, old), 5, text), 4);
});

test("remapLine: one span covering an unchanged heading finds it by text; edited or twinned stays -1", () => {
  const old = "a\n* H :crypt:\nbody\nz\n", span = (text: string) => diffChange(old, text);
  assert.equal(at(span("A\nb\n* H :crypt:\nbody\nZ\n"), 1, old), 2);
  assert.equal(at(span("A\n* H2 :crypt:\nbody\nZ\n"), 1, old), -1);
  assert.equal(at(span("A\n* H :crypt:\n* H :crypt:\nZ\n"), 1, old), -1);
  const twin = "a\n* H :crypt:\nz\n* H :crypt:\nq\n";
  assert.equal(at({ from: 2, to: 15 }, 1, twin), -1); // deleted, a twin remains
});

test("findCryptTab and cryptKey: the current line, not the opening key", () => {
  const tabs = [{ key: "crypt:/n/a.org:2", crypt: { path: "/n/a.org", line: 3 } }, { key: "/n/a.org" }];
  assert.equal(findCryptTab(tabs, "/n/a.org", 3), tabs[0]);
  assert.equal(findCryptTab(tabs, "/n/a.org", 2), undefined);
  assert.equal(findCryptTab(tabs, "/n/b.org", 3), undefined);
  assert.equal(cryptKey(tabs, "/n/a.org", 2), "crypt:/n/a.org:2:1");
  assert.equal(cryptKey(tabs, "/n/a.org", 5), "crypt:/n/a.org:5");
});
