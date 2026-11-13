import { test } from "node:test";
import assert from "node:assert/strict";
import { failing, linkLabel, pool, rewrite, type LinkStatus, type WebLink } from "./linkcheck.ts";

const l = (url: string, line = 0, count = 1): WebLink => ({ url, path: "/n/a.org", line, count });

test("pool runs N at a time and stops starting calls once stopped", async () => {
  let live = 0, most = 0;
  const seen: number[] = [];
  await pool([1, 2, 3, 4, 5, 6, 7], 3, async (x) => {
    most = Math.max(most, ++live);
    await new Promise((r) => setTimeout(r, 5));
    seen.push(x);
    live--;
  }, () => seen.length >= 4);
  assert.equal(most, 3);
  assert.ok(seen.length >= 4 && seen.length < 7, `stopped early: ${seen}`);
  await pool([], 6, async () => assert.fail(), () => false);
});

test("failing lists dead then moved, in note order, without ok or unchecked links", () => {
  const links = [l("https://m1", 0), l("https://d1", 1), l("https://ok", 2), l("https://new", 3), l("https://d2", 4)];
  const st = new Map<string, LinkStatus>([
    ["https://m1", { kind: "moved", to: "https://m2" }], ["https://d1", { kind: "dead", reason: "HTTP 404" }],
    ["https://ok", { kind: "ok" }], ["https://d2", { kind: "dead", reason: "timed out" }],
  ]);
  assert.deepEqual(failing(links, st).map((x) => x.link.url), ["https://d1", "https://d2", "https://m1"]);
});

test("labels and the rewrite pair", () => {
  assert.equal(linkLabel(l("https://a", 0, 3), { kind: "dead", reason: "HTTP 404" }), "✗ https://a · HTTP 404 · 3 links");
  assert.equal(linkLabel(l("http://a"), { kind: "moved", to: "https://a/" }), "→ http://a moved to https://a/");
  assert.deepEqual(rewrite("http://a", "https://a/"), ["[[http://a]", "[[https://a/]"]);
});
