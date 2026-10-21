import { test } from "node:test";
import assert from "node:assert/strict";
import { markKey, orderForEdit } from "./bulk.ts";

test("bulk order: grouped by file, bottom line first; input untouched", () => {
  const xs = [{ path: "/b.org", line: 2 }, { path: "/a.org", line: 1 }, { path: "/b.org", line: 9 }, { path: "/a.org", line: 7 }];
  assert.deepEqual(orderForEdit(xs).map(markKey), ["/a.org:7", "/a.org:1", "/b.org:9", "/b.org:2"]);
  assert.equal(markKey(xs[0]), "/b.org:2");
  assert.deepEqual(orderForEdit([]), []);
});
