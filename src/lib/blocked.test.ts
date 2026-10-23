import { test } from "node:test";
import assert from "node:assert/strict";
import { visible } from "./blocked.ts";

test("visible: blocked tasks hide unless dated today or earlier, or all are shown", () => {
  const t = "2026-10-23";
  assert.equal(visible({ blocked: false, date: "2026-11-01" }, t, false), true);
  assert.equal(visible({ date: "2026-11-01" }, t, false), true);
  assert.equal(visible({ blocked: true, date: "2026-11-01" }, t, false), false); // future
  assert.equal(visible({ blocked: true, date: "+262142-12-31" }, t, false), false); // undated (Tasks)
  assert.equal(visible({ blocked: true, date: t }, t, false), true); // due today: kept, dimmed
  assert.equal(visible({ blocked: true, date: "2026-10-01" }, t, false), true); // overdue
  assert.equal(visible({ blocked: true, date: "2026-11-01" }, t, true), true); // B shows all
});
