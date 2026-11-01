import { test } from "node:test";
import assert from "node:assert/strict";
import { EditorState } from "@codemirror/state";
import { changeList, changeStep } from "./changelist.ts";

const doc = "one\ntwo\nthree\nfour\n";
const st = () => EditorState.create({ doc, extensions: changeList });
const ins = (s: EditorState, at: string, text: string) => s.update({ changes: { from: s.doc.toString().indexOf(at), insert: text } }).state;
/** Walk D and return the line the cursor would land on, or the error. */
const walk = (s: EditorState, d: number): [EditorState, string] => {
  const r = changeStep(s, d);
  return "msg" in r ? [s, r.msg] : [s.update({ effects: r.effect }).state, s.doc.lineAt(r.pos).text];
};

test("changeStep: g; walks back through edits, g, forward", () => {
  let s = st();
  assert.deepEqual(changeStep(s, -1), { msg: "E664: Changelist is empty" });
  s = ins(s, "two", "A");
  s = ins(s, "Atwo", "B"); // same line: replaces the entry
  s = ins(s, "four", "C");
  s = ins(s, "one", "XX\n"); // above the others: they map down
  assert.deepEqual(s.field(changeList).list.map((p) => s.doc.lineAt(p).number), [3, 5, 1]);
  let line: string;
  [s, line] = walk(s, -1); assert.equal(line, "XX");
  [s, line] = walk(s, -1); assert.equal(line, "Cfour");
  [s, line] = walk(s, -1); assert.equal(line, "BAtwo");
  [s, line] = walk(s, -1); assert.equal(line, "E662: At start of changelist");
  [s, line] = walk(s, 2); assert.equal(line, "XX");
  [s, line] = walk(s, 1); assert.equal(line, "E663: At end of changelist");
  s = ins(s, "three", "D"); // a new edit resets the walk past the newest
  [, line] = walk(s, -1); assert.equal(line, "Dthree");
});

test("changeStep: a count past the start clamps; never past the line's end", () => {
  let s = st();
  s = s.update({ changes: { from: 2, to: 3 } }).state; // "one" → "on": the edit sits at the line's end
  const r = changeStep(s, -5);
  assert.ok(!("msg" in r) && r.pos === 1); // on "n", not after it
});
