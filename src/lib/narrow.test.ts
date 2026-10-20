import { test } from "node:test";
import assert from "node:assert/strict";
import { EditorState } from "@codemirror/state";
import { narrowEffect, narrowField, narrowRange, narrowedTo } from "./narrow.ts";

const doc = "#+title: N\n* One\nalpha\n** Sub\nbeta\n* Two :x:\ngamma\n";
const st = (d = doc) => EditorState.create({ doc: d, extensions: narrowField });
const slice = (s: EditorState, r: { from: number; to: number } | null) => (r ? s.sliceDoc(r.from, r.to) : null);

test("narrowRange: the subtree around the cursor", () => {
  const s = st();
  assert.equal(slice(s, narrowRange(s, doc.indexOf("alpha"))), "* One\nalpha\n** Sub\nbeta");
  assert.equal(slice(s, narrowRange(s, doc.indexOf("beta"))), "** Sub\nbeta");
  assert.equal(slice(s, narrowRange(s, doc.indexOf("* Two"))), "* Two :x:\ngamma"); // not the empty last line
  assert.equal(narrowRange(s, 0), null); // before the first heading
  const one = st("* Only\nbody");
  assert.deepEqual(narrowRange(one, 9), { from: 0, to: 11 }); // heading on line 1
});

test("narrowField: maps through edits and clamps the cursor", () => {
  let s = st();
  const r = narrowRange(s, doc.indexOf("beta"))!;
  s = s.update({ effects: narrowEffect.of(r) }).state;
  assert.equal(narrowedTo(s), "Sub");
  // Typing at either edge stays inside; edits above shift the range.
  s = s.update({ changes: [{ from: r.from, insert: "X" }, { from: r.to, insert: "Y" }, { from: 0, insert: "zz" }] }).state;
  assert.equal(slice(s, s.field(narrowField)), "X** Sub\nbetaY");
  // gg / G style moves are clamped into the range.
  s = s.update({ selection: { anchor: 0 } }).state;
  assert.equal(s.selection.main.head, s.field(narrowField)!.from);
  s = s.update({ selection: { anchor: s.doc.length } }).state;
  assert.equal(s.selection.main.head, s.field(narrowField)!.to);
  // Deleting the whole range widens.
  const n = s.field(narrowField)!;
  s = s.update({ changes: { from: n.from, to: n.to } }).state;
  assert.equal(s.field(narrowField), null);
  assert.equal(narrowedTo(s), "");
});

test("narrowField: a heading on line 1 and an effect of null widen cleanly", () => {
  let s = st("* Only\nbody\n* Next\n");
  s = s.update({ effects: narrowEffect.of(narrowRange(s, 0)) }).state;
  assert.equal(narrowedTo(s), "Only");
  s = s.update({ effects: narrowEffect.of(null) }).state;
  assert.equal(s.field(narrowField), null);
});
