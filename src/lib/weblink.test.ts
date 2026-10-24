import { test } from "node:test";
import assert from "node:assert/strict";
import { linkFor, asUrl, asWebUrl } from "./weblink.ts";

test("linkFor builds a described or bare link and keeps ] out of the text", () => {
  assert.equal(linkFor("https://a.b", "site"), "[[https://a.b][site]]");
  assert.equal(linkFor("https://a.b"), "[[https://a.b]]");
  assert.equal(linkFor("id:1", "a [b] c"), "[[id:1][a [b) c]]");
});

test("asUrl accepts one URL (trimmed) and nothing else", () => {
  assert.equal(asUrl(" https://example.com/x?y=1\n"), "https://example.com/x?y=1");
  assert.equal(asUrl("mailto:a@b.c"), "mailto:a@b.c");
  assert.equal(asUrl("see https://example.com"), null);
  assert.equal(asUrl("https://a.b https://c.d"), null);
  assert.equal(asUrl("plain"), null);
});

test("asWebUrl takes only http(s) URLs", () => {
  assert.equal(asWebUrl(" HTTPS://example.com/a "), "HTTPS://example.com/a");
  assert.equal(asWebUrl("http://x.y"), "http://x.y");
  assert.equal(asWebUrl("mailto:a@b.c"), null);
  assert.equal(asWebUrl("file:///etc/passwd"), null);
  assert.equal(asWebUrl("two words"), null);
});
