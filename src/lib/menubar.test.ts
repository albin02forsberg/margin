import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { MENUS } from "./menubar.ts";

test("every menu bar entry names a built-in palette command", () => {
  const page = readFileSync(new URL("../routes/+page.svelte", import.meta.url), "utf8");
  for (const [menu, labels] of MENUS) for (const l of labels) assert.ok(page.includes(`{ label: ${JSON.stringify(l)},`), `${menu}: no command “${l}”`);
});

test("menus have distinct first letters (the F10 picker's one-key shortcuts)", () => {
  const first = MENUS.map(([n]) => n[0].toLowerCase());
  assert.equal(new Set(first).size, first.length);
});

test("menus follow Emacs order", () => {
  assert.deepEqual(MENUS.map(([n]) => n), ["File", "Edit", "View", "Buffers", "Org", "Help"]);
});
