// Run: npm test
import { test } from "node:test";
import assert from "node:assert/strict";
import { chatContext, cleanChat, toolLine } from "./chatctx.ts";

const doc = "intro\n* A\na1\na2\n** A1\nx\n* B\nb1\n";

test("selection wins, even when the note fits", () => {
  assert.deepEqual(chatContext(doc, 2, 5, 2, "N"), { text: "tro", label: "selection" });
});
test("a selection over the limit is cut to it, on a code point", () => {
  const sel = (s: string, limit: number) => chatContext(s, 0, s.length, 0, "N", limit);
  assert.deepEqual(sel("abcd", 4), { text: "abcd", label: "selection" });
  assert.equal(sel("abcde", 4).text, "abcd");
  const r = chatContext("å".repeat(3000), 0, 3000, 0, "N");
  assert.equal(r.label, "selection (first 4 KB)");
  assert.equal(r.text, "å".repeat(2000));
  assert.ok(!r.text.includes("�"));
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
test("cleanChat strips think blocks (closed or open) and turns * bullets into -", () => {
  assert.equal(cleanChat("<think>hm</think>\n\n* a\n** b\n*bold* **b** x"), "- a\n- b\n*bold* **b** x");
  assert.equal(cleanChat("hi <think>still going"), "hi");
  assert.equal(cleanChat("<think>only"), "");
});
test("limit counts UTF-8 bytes", () => {
  const t = "ååå\nåå\n";
  assert.equal(chatContext(t, 0, 0, 0, "N", 9).label, "text around the cursor");
  assert.equal(chatContext(t, 0, 0, 0, "N", 20).label, "N");
});
test("tool calls read as one line each, and unknown tools by name (#963)", () => {
  assert.equal(toolLine({ name: "search_notes", args: { query: " rollout " } }), "searched notes: rollout");
  assert.equal(toolLine({ name: "open_note", args: { title: "Launch" } }), "opened Launch");
  assert.equal(toolLine({ name: "neighbors", args: { title: "Plans" } }), "looked up links of Plans");
  assert.equal(toolLine({ name: "open_note", args: {} }), "opened");
  assert.equal(toolLine({ name: "rm", args: { path: "/" } }), "called rm");
  assert.match(toolLine({ no_tools: true }), /can't use tools/);
});
