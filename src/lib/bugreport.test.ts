import { test } from "node:test";
import assert from "node:assert/strict";
import { bugReportUrl, ISSUES, isError, osName } from "./bugreport.ts";

test("isError flags only ⚠ messages", () => {
  assert.ok(isError("⚠ save failed"));
  assert.ok(!isError("Renamed to a.org"));
  assert.ok(!isError(" ⚠ not at start"));
});

test("bug report URL carries version, OS, channel and the last error", () => {
  const u = new URL(bugReportUrl({ version: "0.3.1", os: "Linux", nightly: true, error: "⚠ save failed & retried" }));
  assert.equal(`${u.origin}${u.pathname}`, ISSUES);
  assert.equal(u.searchParams.get("labels"), "bug");
  const body = u.searchParams.get("body")!;
  assert.match(body, /- Margin: 0\.3\.1 \(nightly updates\)/);
  assert.match(body, /- OS: Linux/);
  assert.match(body, /- Last error: ⚠ save failed & retried/);
});

test("bug report URL without an error, version or OS, and a capped long error", () => {
  const body = (e?: string) => new URL(bugReportUrl({ version: "", os: "", nightly: false, error: e })).searchParams.get("body")!;
  assert.match(body(), /Margin: unknown \(stable updates\)\n- OS: unknown\n- Last error: none/);
  assert.match(body("x".repeat(500)), new RegExp(`Last error: x{300}…$`));
});

test("osName picks the OS from a user agent", () => {
  assert.equal(osName("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15"), "Mac OS X 10.15.7");
  assert.equal(osName("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Edg/120"), "Windows NT 10.0");
  assert.equal(osName("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15"), "Linux");
  assert.equal(osName("Weird"), "Weird");
});
