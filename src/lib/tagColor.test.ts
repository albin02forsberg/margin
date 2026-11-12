import { test } from "node:test";
import assert from "node:assert/strict";
import { tagStyle } from "./tagColor.ts";

test("tagStyle: valid hex, exact match, ignores bad values", () => {
  const c = { work: "#FF8800", "@home": "#00aa00", bad: "red", short: "#fff" };
  assert.match(tagStyle("work", c), /--tag-color:#FF8800;--tag-bg:color-mix\(in srgb, #FF8800 20%/);
  assert.match(tagStyle("@home", c), /#00aa00/);
  for (const t of ["Work", "bad", "short", "none"]) assert.equal(tagStyle(t, c), "");
  assert.equal(tagStyle("work"), "");
});
