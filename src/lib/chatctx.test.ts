// Run: npm test
import { test } from "node:test";
import assert from "node:assert/strict";
import { chatContext } from "./chatctx.ts";

const doc = "intro\n* A\na1\na2\n** A1\nx\n* B\nb1\n";

test("selection wins, even when the note fits", () => {
  assert.deepEqual(chatContext(doc, 2, 5, 2, "N"), { text: "tro", label: "selection" });
});
test("short note is sent whole", () => {
  assert.deepEqual(chatContext(doc, 0, 0, 0, "N"), { text: doc, label: "N" });
});
test("long note: outermost section around the cursor that fits", () => {
  const head = doc.indexOf("x");
  assert.deepEqual(chatContext(doc, head, head, head, "N", 30), { text: "* A\na1\na2\n** A1\nx", label: "section A" });
  assert.deepEqual(chatContext(doc, head, head, head, "N", 12), { text: "** A1\nx", label: "section A1" });
});
test("before the first heading, or no section fits: lines around the cursor", () => {
  assert.deepEqual(chatContext(doc, 0, 0, 0, "N", 12), { text: "intro\n* A", label: "text around the cursor" });
  const head = doc.indexOf("a1");
  assert.deepEqual(chatContext(doc, head, head, head, "N", 7), { text: "* A\na1", label: "text around the cursor" });
});
test("limit counts UTF-8 bytes", () => {
  const t = "ååå\nåå\n";
  assert.equal(chatContext(t, 0, 0, 0, "N", 9).label, "text around the cursor");
  assert.equal(chatContext(t, 0, 0, 0, "N", 20).label, "N");
});
