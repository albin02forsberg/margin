import { test } from "node:test";
import assert from "node:assert/strict";
import { setArg, foldCase, scopedArg } from "./setopt.ts";

const opts: Record<string, unknown> = { number: false, nu: false, wrap: true, textwidth: 80, tw: 80 };
const get = (n: string) => (n in opts ? opts[n] : new Error(`Unknown option: ${n}`));
test("setArg: booleans", () => {
  assert.deepEqual(setArg("nu", get), { set: ["nu", true] });
  assert.deepEqual(setArg("nonumber", get), { set: ["number", false] });
  assert.deepEqual(setArg("wrap!", get), { set: ["wrap", false] });
  assert.deepEqual(setArg("invwrap", get), { set: ["wrap", false] });
  assert.deepEqual(setArg("wrap?", get), { msg: "  wrap" });
  assert.deepEqual(setArg("nu?", get), { msg: "  nonu" });
  assert.deepEqual(setArg("nu=1", get), { msg: "Invalid argument: nu=1" });
  assert.deepEqual(setArg("nonu!", get), { msg: "Invalid argument: nonu!" });
});
test("setArg: values and unknown options", () => {
  assert.deepEqual(setArg("tw=72", get), { set: ["tw", "72"] });
  assert.deepEqual(setArg("tw", get), { msg: "  tw=80" });
  assert.deepEqual(setArg("tw?", get), { msg: "  tw=80" });
  assert.deepEqual(setArg("notw", get), { msg: "Invalid argument: notw" });
  assert.deepEqual(setArg("ignorecase", get), { msg: "Unknown option: ignorecase" });
  assert.deepEqual(setArg("nobogus", get), { msg: "Unknown option: nobogus" });
  assert.deepEqual(setArg("a b", get), { msg: "Invalid argument: a b" });
});

test("scopedArg: :setlocal / :set / :setglobal on buffer-local options (#1112)", () => {
  const local: Record<string, boolean> = {};
  assert.deepEqual(scopedArg("nu", "local", local, get), {}); // buffer only, no global set
  assert.deepEqual(local, { number: true });
  assert.deepEqual(scopedArg("nu?", "local", local, get), { msg: "  nu" }); // effective value
  assert.deepEqual(scopedArg("invnu", "local", local, get), {});
  assert.deepEqual(local, { number: false });
  assert.deepEqual(scopedArg("nu?", "global", local, get), { msg: "  nonu" }); // :setg reads the global value
  assert.deepEqual(scopedArg("wrap!", "global", local, get), { set: ["wrap", false] });
  assert.deepEqual(local, { number: false }); // :setg leaves the buffer alone
  assert.deepEqual(scopedArg("nowrap", "set", local, get), { set: ["wrap", false] }); // :set: global and this buffer
  assert.deepEqual(local, { number: false, wrap: false });
  assert.deepEqual(scopedArg("tw=72", "local", local, get), { set: ["tw", "72"] }); // not buffer-local: global
  assert.deepEqual(scopedArg("bogus", "local", local, get), { msg: "Unknown option: bogus" });
});

test("foldCase: ignorecase and smartcase", () => {
  assert.equal(foldCase("foo", true, true), true);
  assert.equal(foldCase("Foo", true, true), false);
  assert.equal(foldCase("Foo", true, false), true);
  assert.equal(foldCase("foo", false, true), false);
  assert.equal(foldCase("foo\\S", true, true), true);
  assert.equal(foldCase("foo\\\\S", true, true), false);
  assert.equal(foldCase("foo", false, false), false);
});
test("the vim library still exposes the search methods editor.ts wraps", async () => {
  const { CodeMirror } = await import("@replit/codemirror-vim");
  assert.equal(typeof CodeMirror.prototype.getSearchCursor, "function");
  assert.equal(typeof CodeMirror.prototype.addOverlay, "function");
});
