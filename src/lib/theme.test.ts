import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { effectiveTheme, nextTheme } from "./theme.ts";

test("effectiveTheme", () => {
  assert.equal(effectiveTheme("", true), "light");
  assert.equal(effectiveTheme("", false), "dark");
  assert.equal(effectiveTheme("light", false), "light");
  assert.equal(effectiveTheme("dark", true), "dark");
  assert.equal(effectiveTheme("junk", true), "light");
  assert.equal(effectiveTheme(undefined, false), "dark");
});

test("theme.css: the forced-light and media-query light blocks match", () => {
  const css = readFileSync(new URL("./theme.css", import.meta.url), "utf8");
  const body = (sel: string) => css.split(sel)[1].split("}")[0].replace(/\s+/g, " ").trim();
  assert.equal(body(':root[data-theme="light"] {'), body(':root:not([data-theme="dark"]) {'));
});

test("nextTheme cycles system -> light -> dark -> system", () => {
  assert.equal(nextTheme(""), "light");
  assert.equal(nextTheme("light"), "dark");
  assert.equal(nextTheme("dark"), "");
  assert.equal(nextTheme("junk"), "light");
  assert.equal(nextTheme(undefined), "light");
});
