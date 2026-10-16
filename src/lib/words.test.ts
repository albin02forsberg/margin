import { test } from "node:test";
import assert from "node:assert/strict";
import { wordCount, wordsLabel } from "./words.ts";

test("wordCount splits on any whitespace", () => {
  assert.equal(wordCount(""), 0);
  assert.equal(wordCount("  \n\t "), 0);
  assert.equal(wordCount("#+title: Notes\n\n* TODO Water  the plants\n"), 7);
});

test("wordsLabel shows the selection out of the whole note", () => {
  assert.equal(wordsLabel(412), "412 words");
  assert.equal(wordsLabel(1), "1 word");
  assert.equal(wordsLabel(412, 38), "38 of 412 words");
});
