import { test } from "node:test";
import assert from "node:assert/strict";
import { journalNeighbour, relativeTo } from "./paths.ts";

test("relativeTo", () => {
  assert.equal(relativeTo("/n/a.org", "/n/b.org"), "b.org");
  assert.equal(relativeTo("/n/daily/2026-10-02.org", "/n/ideas/x.org"), "../ideas/x.org");
  assert.equal(relativeTo("/n/daily/d.org", "/n/x.org"), "../x.org");
  assert.equal(relativeTo("/n/a.org", "/n/sub/deep/x.org"), "sub/deep/x.org");
  assert.equal(relativeTo("C:\\n\\daily\\d.org", "C:\\n\\x.org"), "../x.org");
});

test("journalNeighbour", () => {
  const f = ["/n/daily/2026-10-01.org", "/n/daily/2026-10-05.org", "/n/daily/notes.org", "/n/x/2026-10-03.org", "/n/daily/2026-10-09.org"];
  assert.equal(journalNeighbour(f, "daily", "2026-10-05", -1), "/n/daily/2026-10-01.org");
  assert.equal(journalNeighbour(f, "daily", "2026-10-05", 1), "/n/daily/2026-10-09.org");
  assert.equal(journalNeighbour(f, "daily", "2026-10-07", -1), "/n/daily/2026-10-05.org"); // today not in list
  assert.equal(journalNeighbour(f, "daily", "2026-10-01", -1), undefined);
  assert.equal(journalNeighbour(f, "daily", "2026-10-09", 1), undefined);
  assert.equal(journalNeighbour(["C:\\n\\a\\b\\2026-01-02.org", "C:\\n\\b\\2026-01-01.org"], "a/b", "2026-01-05", -1), "C:\\n\\a\\b\\2026-01-02.org");
  assert.equal(journalNeighbour([], "daily", "2026-10-01", 1), undefined);
});
