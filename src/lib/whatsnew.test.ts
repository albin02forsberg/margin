import { test } from "node:test";
import assert from "node:assert/strict";
import { notesText, notesToShow } from "./whatsnew.ts";

const raw = (o: object) => JSON.stringify(o);

test("notesToShow only returns notes for the running version", () => {
  const n = { from: "0.1.0", to: "0.2.0", body: "### Features\n- zoom\n" };
  assert.deepEqual(notesToShow(raw(n), "0.2.0"), n);
  assert.equal(notesToShow(raw(n), "0.1.0"), null); // update failed
  assert.equal(notesToShow(raw({ ...n, body: "  \n" }), "0.2.0"), null);
  assert.equal(notesToShow("{oops", "0.2.0"), null);
  assert.equal(notesToShow(null, "0.2.0"), null);
  assert.equal(notesToShow(raw({ to: "0.2.0", body: "x" }), "0.2.0")?.from, "");
});

test("notesText titles the notes", () => {
  assert.equal(notesText({ from: "0.1.0", to: "0.2.0", body: "- a\n" }), "What's new in 0.2.0 (from 0.1.0)\n\n- a\n");
});
