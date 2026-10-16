// Run: npm test
import { test } from "node:test";
import assert from "node:assert/strict";
import { toggleEmphasis } from "./emphasis.ts";

/** Toggle MARK on TEXT, where [ and ] (or a lone |) mark the selection; returns
 *  the result in the same notation. */
function run(text: string, mark: string) {
  const from = text.search(/[[|]/);
  const to = text.includes("|") ? from : text.indexOf("]") - 1;
  const plain = text.replace(/[[\]|]/g, "");
  const r = toggleEmphasis(plain, from, to, mark);
  let out = plain;
  for (const c of [...r.changes].sort((x, y) => y.from - x.from)) out = out.slice(0, c.from) + c.insert + out.slice(c.to);
  return r.from === r.to ? out.slice(0, r.from) + "|" + out.slice(r.from) : out.slice(0, r.from) + "[" + out.slice(r.from, r.to) + "]" + out.slice(r.to);
}

test("wraps the selection, trimming spaces, and keeps it selected", () => {
  assert.equal(run("say [hello there ]now", "*"), "say *[hello there]* now");
  assert.equal(run("[a]", "~"), "~[a]~");
});

test("wraps the word at the cursor", () => {
  assert.equal(run("say he|llo now", "/"), "say /he|llo/ now");
  assert.equal(run("say |hello", "+"), "say +|hello+");
  assert.equal(run("well-kno|wn.", "*"), "*well-kno|wn*.");
});

test("inserts an empty pair with nothing at the cursor", () => {
  const r = toggleEmphasis("a  b", 2, 2, "*");
  assert.deepEqual(r, { changes: [{ from: 2, to: 2, insert: "**" }], from: 3, to: 3, empty: true });
});

test("unwraps a span of the same mark at the cursor or selection", () => {
  assert.equal(run("say *hel|lo there* now", "*"), "say hel|lo there now");
  assert.equal(run("say [*hello*] now", "*"), "say [hello] now");
  assert.equal(run("say *[hello]* now", "*"), "say [hello] now");
  assert.equal(run("x |*b*", "*"), "x |b");
});

test("a different mark, or the cursor just past a span, wraps", () => {
  assert.equal(run("*bo|ld*", "/"), "*/bo|ld/*");
  assert.equal(run("*bold*|", "*"), "*bold**|*"); // just past a span: a new pair
});
