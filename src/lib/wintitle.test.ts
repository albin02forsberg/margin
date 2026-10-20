import { test } from "node:test";
import assert from "node:assert/strict";
import { windowTitle } from "./wintitle.ts";

test("windowTitle", () => {
  assert.equal(windowTitle(), "Margin");
  assert.equal(windowTitle({ title: "Today", dirty: false }), "Today — Margin");
  assert.equal(windowTitle({ title: "Notes", dirty: true }), "• Notes — Margin");
  assert.equal(windowTitle({ title: "x".repeat(200), dirty: false }), "x".repeat(79) + "… — Margin");
});
