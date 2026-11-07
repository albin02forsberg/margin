import { test } from "node:test";
import assert from "node:assert/strict";
import { isCryptKey, saveRoute } from "./crypttabs.ts";

test("isCryptKey", () => {
  assert.ok(isCryptKey("crypt:/n/a.org:3"));
  assert.ok(isCryptKey("crypt:/n/a.org:3", "/n/a.org"));
  assert.ok(!isCryptKey("crypt:/n/ab.org:3", "/n/a.org"));
  assert.ok(!isCryptKey("/n/a.org"));
});

test("saveRoute: a decrypted tab is never autosaved", () => {
  const c = { crypt: { path: "/n/a.org", line: 0, plain: "x" } };
  assert.equal(saveRoute(c, false), "none");
  assert.equal(saveRoute(c, true), "relock");
  assert.equal(saveRoute({}, false), "file");
  assert.equal(saveRoute({}, true), "file");
});
