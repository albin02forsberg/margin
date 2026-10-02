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

test("formulas: column, field, ranges, functions, format", () => {
  const t = ["| Item | Qty | Price | Total |", "|-", "| a | 2 | 1.5 | |", "| b | 3 | 2 | |", "| | | | |", "|-", "| Sum | | | |"];
  const out = T.align(T.applyFormulas(T.align(t), "$4=$2*$3::@>$4=vsum(@I..@II);%.2f::@>$2=vsum(@I$2..@II$2)"));
  assert.deepEqual(out, [
    "| Item | Qty | Price | Total |",
    "|------+-----+-------+-------|",
    "| a    |   2 |   1.5 |     3 |",
    "| b    |   3 |     2 |     6 |",
    "|      |     |       |       |",
    "|------+-----+-------+-------|",
    "| Sum  |   5 |       |  9.00 |",
  ]);
  const g = (rows: string[]) => T.applyFormulas(rows, "$3=$1+$-1*2^2::@2$1=vmean(@1$2..@>$2)+abs(-1)");
  assert.deepEqual(g(["| 1 | 2 | |", "| 3 | 4 | |"]), ["| 1 | 2 | 9 |", "| 4 | 4 | 20 |"]);
  assert.deepEqual(T.applyFormulas(["| 1 | |"], "$2=$1+"), ["| 1 | #ERROR |"]);
  assert.deepEqual(T.applyFormulas(["| 1 | |"], "$2=nope($1)"), ["| 1 | #ERROR |"]);
  assert.deepEqual(T.applyFormulas(["| 1 | 2 | |"], "$3=vsum($1..$2)/3;%.3f"), ["| 1 | 2 | 1.000 |"]);
});

test("cell formulas move into TBLFM", () => {
  const r = T.extractCellFormulas(["| a | =$1*2 |", "| b | :=vsum(@1..@2) |"]);
  assert.deepEqual(r, { lines: ["| a |  |", "| b |  |"], formulas: ["$2=$1*2", "@2$2=vsum(@1..@2)"] });
  assert.equal(T.mergeTblfm("$2=$1::@1$1=3", ["$2=$1*2", "$3=1"]), "$2=$1*2::@1$1=3::$3=1");
});
