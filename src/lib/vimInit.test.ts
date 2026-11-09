import { test } from "node:test";
import assert from "node:assert/strict";
import { parseVimInit, mapTarget, runVimInit } from "./vimInit.ts";

test("parseVimInit: skips blanks and comments, strips a leading colon", () => {
  assert.deepEqual(parseVimInit('" my vimrc\n\n  :nnoremap ; :\n\tset rnu  \n"set nu'), [{ line: 3, cmd: "nnoremap ; :" }, { line: 4, cmd: "set rnu" }]);
});

test("mapTarget: map commands and their abbreviations", () => {
  assert.deepEqual(mapTarget("inoremap jk <Esc>"), { lhs: "jk", ctx: "insert" });
  assert.deepEqual(mapTarget("nn ; :"), { lhs: ";", ctx: "normal" });
  assert.deepEqual(mapTarget("ino jk <Esc>"), { lhs: "jk", ctx: "insert" });
  assert.deepEqual(mapTarget("noremap Y y$"), { lhs: "Y", ctx: undefined });
  assert.deepEqual(mapTarget("vmap < <gv"), { lhs: "<", ctx: "visual" });
  assert.equal(mapTarget("nnoremap ;"), null);
  assert.equal(mapTarget("set nu"), null);
  assert.equal(mapTarget("nunmap gt"), null);
  assert.equal(mapTarget("m a b"), null);
});

test("runVimInit: a failing line is reported with its number and the rest still run", () => {
  const ran: string[] = [];
  const r = runVimInit("inoremap jk <Esc>\nbogus\n\nset nu\nnmap Q bad", (c) => { ran.push(c); return c === "bogus" ? 'Not an editor command ":bogus"' : c === "nmap Q bad" ? "nope" : undefined; });
  assert.deepEqual(ran, ["inoremap jk <Esc>", "bogus", "set nu", "nmap Q bad"]);
  assert.deepEqual(r.errors, ['line 2: Not an editor command ":bogus"', "line 5: nope"]);
  assert.deepEqual(r.mapped, [{ lhs: "jk", ctx: "insert" }]);
});

test("runVimInit: <Space> mappings in normal/visual mode are skipped", () => {
  const ran: string[] = [];
  const r = runVimInit("nnoremap <Space>w :w\nmap <space>x x\ninoremap <Space><Space> <Esc>", (c) => void ran.push(c));
  assert.deepEqual(ran, ["inoremap <Space><Space> <Esc>"]);
  assert.deepEqual(r.errors, ["line 1: <Space> is the leader key", "line 2: <Space> is the leader key"]);
});
