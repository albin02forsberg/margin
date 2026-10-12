import { test } from "node:test";
import assert from "node:assert/strict";
import { changed, journalNeighbour, mergeChanged, relativeTo } from "./paths.ts";

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

test("mergeChanged", () => {
  assert.deepEqual(mergeChanged(["/n/a.org"], ["/n/b.org", "/n/a.org"]), ["/n/a.org", "/n/b.org"]);
  assert.equal(mergeChanged(undefined, ["/n/a.org"]), undefined);
  assert.equal(mergeChanged(["/n/a.org"], undefined), undefined);
  assert.deepEqual(mergeChanged([], []), []);
});

test("changed", () => {
  assert.ok(changed("/n/a.org"));
  assert.ok(changed("/n/a.org", ["/n/b.org", "/n/a.org"]));
  assert.ok(!changed("/n/a.org", ["/n/b.org"]));
  assert.ok(changed("/n/sub/a.org", ["/n/sub"]), "under a moved folder");
  assert.ok(!changed("/n/subway.org", ["/n/sub"]));
  assert.ok(changed("C:\\n\\sub\\a.org", ["C:\\n\\sub"]));
  assert.ok(!changed("/n/a.org", []));
});
