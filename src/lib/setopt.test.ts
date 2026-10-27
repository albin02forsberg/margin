import { test } from "node:test";
import assert from "node:assert/strict";
import { setArg } from "./setopt.ts";

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
