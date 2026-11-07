import { test } from "node:test";
import assert from "node:assert/strict";
import { isCryptKey } from "./crypttabs.ts";

test("isCryptKey", () => {
  assert.ok(isCryptKey("crypt:/n/a.org:3"));
  assert.ok(isCryptKey("crypt:/n/a.org:3", "/n/a.org"));
  assert.ok(!isCryptKey("crypt:/n/ab.org:3", "/n/a.org"));
  assert.ok(!isCryptKey("/n/a.org"));
});
