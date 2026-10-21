import { test } from "node:test";
import assert from "node:assert/strict";
import { orderNotes, parsePins, togglePin } from "./pins.ts";

test("orderNotes puts pins first in pin order, then recent, then the rest", () => {
  const files = ["a", "b", "c", "d", "e"];
  assert.deepEqual(orderNotes(files, ["c", "b"], ["e", "a"]), ["e", "a", "c", "b", "d"]);
  assert.deepEqual(orderNotes(files, ["a", "d"], []), ["a", "d", "b", "c", "e"]);
  // a pinned note that's also recent stays in the pin group
  assert.deepEqual(orderNotes(files, ["b", "e"], ["e"]), ["e", "b", "a", "c", "d"]);
  // pins of vanished files are ignored
  assert.deepEqual(orderNotes(["a", "b"], [], ["gone", "b"]), ["b", "a"]);
});

test("togglePin adds at the end and removes", () => {
  assert.deepEqual(togglePin(["a"], "b"), ["a", "b"]);
  assert.deepEqual(togglePin(["a", "b"], "a"), ["b"]);
});

test("parsePins falls back to [] on bad input", () => {
  assert.deepEqual(parsePins('["a","b"]'), ["a", "b"]);
  assert.deepEqual(parsePins("{oops"), []);
  assert.deepEqual(parsePins('{"a":1}'), []);
  assert.deepEqual(parsePins('["a",3]'), ["a"]);
});
