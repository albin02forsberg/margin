// Run: npm test
import { test } from "node:test";
import assert from "node:assert/strict";
import * as T from "./orgtable.ts";

test("align pads cells, right-aligns numbers, expands |-", () => {
  assert.deepEqual(T.align(["|Name|Qty|", "|-", "|apple|3|", "| kiwi | 12 |"]), [
    "| Name  | Qty |",
    "|-------+-----|",
    "| apple |   3 |",
    "| kiwi  |  12 |",
  ]);
  assert.deepEqual(T.align(["  |a|"]), ["  | a |"]);
});

test("cell positions", () => {
  const l = "| apple |   3 |";
  assert.equal(T.cellIndex(l, 3), 0);
  assert.equal(T.cellIndex(l, 10), 1);
  assert.equal(l.slice(T.cellOffset(l, 0)), "apple |   3 |");
  assert.equal(l.slice(T.cellOffset(l, 1)), "3 |");
  assert.equal(T.cellOffset("|   |", 0), 2);
});

test("tab moves through cells, skips separators, adds a row at the end", () => {
  const t = { lines: ["| a | b |", "|---+---|", "| c | d |"], row: 0, col: 1 };
  assert.deepEqual(T.nextCell(t, 1), { ...t, row: 2, col: 0 });
  assert.deepEqual(T.nextCell({ ...t, row: 2, col: 0 }, -1), { ...t, row: 0, col: 1 });
  const grown = T.nextCell({ ...t, row: 2, col: 1 }, 1);
  assert.deepEqual(grown, { lines: [...t.lines, "|   |   |"], row: 3, col: 0 });
  assert.deepEqual(T.nextRow({ ...t, row: 0 }).lines, ["| a | b |", "|   |   |", "|---+---|", "| c | d |"]);
});

test("row and column edits", () => {
  const t = { lines: ["| a | b |", "| c | d |"], row: 0, col: 0 };
  assert.deepEqual(T.insertCol(t).lines, ["| a |   | b |", "| c |   | d |"]);
  assert.deepEqual(T.deleteCol(t)!.lines, ["| b |", "| d |"]);
  assert.deepEqual(T.moveRow(t, 1).lines, ["| c | d |", "| a | b |"]);
  assert.deepEqual(T.moveCol(t, 1), { lines: ["| b | a |", "| d | c |"], row: 0, col: 1 });
  assert.deepEqual(T.insertRow(t, true).lines, ["| a | b |", "|   |   |", "| c | d |"]);
  assert.deepEqual(T.deleteRow(t)!.lines, ["| c | d |"]);
  assert.deepEqual(T.insertSep(t).lines, ["| a | b |", "|---+---|", "| c | d |"]);
});

test("sort keeps the header above the separator", () => {
  const t = { lines: ["| n |", "|---|", "| 10 |", "| 9 |", "| 100 |"], row: 3, col: 0 };
  assert.deepEqual(T.sortRows(t).lines, ["|   n |", "|-----|", "|   9 |", "|  10 |", "| 100 |"]);
  assert.deepEqual(T.create(2, 1), ["| Column 1 | Column 2 |", "|----------+----------|", "|          |          |"]);
});
