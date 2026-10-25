import { test } from "node:test";
import assert from "node:assert/strict";
import { directive, transcludeProps, transcludeTarget } from "./transclude.ts";

test("transcludeTarget reads the #+TRANSCLUDE: link, any case, with or without a description", () => {
  assert.equal(transcludeTarget("#+TRANSCLUDE: [[file:x.org::*Agenda]]"), "file:x.org::*Agenda");
  assert.equal(transcludeTarget("  #+transclude: [[id:abc-1][Agenda]] :level 2"), "id:abc-1");
  assert.equal(transcludeTarget("#+Transclude:[[*Here]]"), "*Here");
  assert.equal(transcludeTarget("text #+TRANSCLUDE: [[id:a]]"), null);
  assert.equal(transcludeTarget("#+TRANSCLUDE: file:x.org"), null);
  assert.equal(transcludeTarget("#+TRANSCLUDE: [[]]"), null);
  assert.equal(transcludeTarget(directive("file:../a b.org::*Héad")), "file:../a b.org::*Héad");
});

test("transcludeProps reads what follows the link (sent to the backend)", () => {
  assert.equal(transcludeProps("#+TRANSCLUDE: [[file:x.org::#cid]] :lines 2-5 :only-contents"), ":lines 2-5 :only-contents");
  assert.equal(transcludeTarget("#+TRANSCLUDE: [[file:x.org::12]] :lines -3"), "file:x.org::12");
  assert.equal(transcludeProps("#+TRANSCLUDE: [[id:a][A]]"), "");
  assert.equal(transcludeProps("no directive"), "");
});
