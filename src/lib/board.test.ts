import { test } from "node:test";
import assert from "node:assert/strict";
import { columns, moveTo } from "./board.ts";

const todo = ["TODO", "NEXT", "WAIT"], done = ["DONE", "CANCELLED"];
const it = (title: string, keyword: string | null) => ({ title, keyword });

test("board columns: one per open keyword in order, then Done", () => {
  const cols = columns([it("a", "NEXT"), it("b", "TODO"), it("c", "CANCELLED"), it("d", "DONE"), it("e", "ODD"), it("f", null)], todo, done);
  assert.deepEqual(cols.map((c) => [c.title, c.done, c.items.map((i) => i.title).join("")]),
    [["TODO", false, "bef"], ["NEXT", false, "a"], ["WAIT", false, ""], ["DONE", true, "cd"]]);
  assert.deepEqual(columns([], ["TODO", "DOING"], ["DONE"]).map((c) => c.title), ["TODO", "DOING", "DONE"]);
});

test("board moves: left/right along the columns, null at the edges", () => {
  assert.equal(moveTo("TODO", 1, todo, done), "NEXT");
  assert.equal(moveTo("WAIT", 1, todo, done), "DONE");
  assert.equal(moveTo("CANCELLED", -1, todo, done), "WAIT");
  assert.equal(moveTo("DONE", 1, todo, done), null);
  assert.equal(moveTo("TODO", -1, todo, done), null);
});
