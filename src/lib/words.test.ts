import { test } from "node:test";
import assert from "node:assert/strict";
import { Text } from "@codemirror/state";
import { bufferPos, lineCol, parseWordGoal, wordCount, wordsLabel } from "./words.ts";

test("lineCol: 1-based line and column of the cursor", () => {
  const doc = Text.of(["* Heading", "", "body text"]);
  assert.equal(lineCol(doc, 0), "L1:C1");
  assert.equal(lineCol(doc, 9), "L1:C10"); // end of line 1
  assert.equal(lineCol(doc, 10), "L2:C1"); // empty line
  assert.equal(lineCol(doc, 15), "L3:C5");
  assert.equal(lineCol(doc, doc.length), "L3:C10");
});

test("wordCount splits on any whitespace", () => {
  assert.equal(wordCount(""), 0);
  assert.equal(wordCount("  \n\t "), 0);
  assert.equal(wordCount("#+title: Notes\n\n* TODO Water  the plants\n"), 6);
});

test("wordCount ignores punctuation-only tokens", () => {
  assert.equal(wordCount("- [ ] item | a | b\n-----"), 3);
  assert.equal(wordCount("Åsa läser 3 böcker"), 4);
});

test("wordCount org skips markup", () => {
  const org = "#+title: My notes\n* TODO Write report :work:\nSCHEDULED: <2026-10-16 Fri>\n:PROPERTIES:\n:ID: abc\n:END:\n- [ ] item\n";
  assert.equal(wordCount(org, true), 6); // My notes, TODO Write report, item
});

test("parseWordGoal reads #+WORDGOAL: from the top lines", () => {
  assert.equal(parseWordGoal("#+TITLE: x\n#+wordgoal: 1500\nbody"), 1500);
  assert.equal(parseWordGoal("body\n#+WORDGOAL: abc"), 0);
  assert.equal(parseWordGoal("#+WORDGOAL: -5"), 0);
  assert.equal(parseWordGoal("\n".repeat(40) + "#+WORDGOAL: 9"), 0);
  assert.equal(wordCount("#+WORDGOAL: 1500\nhello", true), 2); // only the keyword name is skipped
});

test("wordsLabel shows progress to a goal for the whole note", () => {
  assert.equal(wordsLabel(812, undefined, "", 1500), "812 / 1500 words");
  assert.equal(wordsLabel(812, 38, "", 1500), "38 of 812 / 1500 words");
  assert.equal(wordsLabel(812, undefined, "", 0), "812 words");
});

test("wordsLabel shows the selection out of the whole note", () => {
  assert.equal(wordsLabel(412), "412 words");
  assert.equal(wordsLabel(1), "1 word");
  assert.equal(wordsLabel(412, 38), "38 of 412 words");
  assert.equal(wordsLabel(312, undefined, "heading"), "312 words in heading");
  assert.equal(wordsLabel(312, 38, "heading"), "38 of 312 words in heading");
});

test("bufferPos: All / Top / Bot / percent like the Emacs mode-line", () => {
  assert.equal(bufferPos(0, 500, 400), "All");
  assert.equal(bufferPos(0, 500, 500), "All");
  assert.equal(bufferPos(0, 0, 0), "All");
  assert.equal(bufferPos(0, 500, 1500), "Top");
  assert.equal(bufferPos(1000, 500, 1500), "Bot");
  assert.equal(bufferPos(999.5, 500, 1500), "Bot"); // 1px tolerance
  assert.equal(bufferPos(500, 500, 1500), "50%");
  assert.equal(bufferPos(333, 500, 1500), "33%");
});
