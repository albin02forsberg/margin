import { test } from "node:test";
import assert from "node:assert/strict";
import { wordCount, wordsLabel } from "./words.ts";

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

test("wordsLabel shows the selection out of the whole note", () => {
  assert.equal(wordsLabel(412), "412 words");
  assert.equal(wordsLabel(1), "1 word");
  assert.equal(wordsLabel(412, 38), "38 of 412 words");
});
