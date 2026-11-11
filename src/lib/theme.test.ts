import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { applyLook, effectiveTheme, nextTheme, setUserLook } from "./theme.ts";

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

test("theme.css: retro overrides every palette token, and its two light blocks match", () => {
  const css = readFileSync(new URL("./theme.css", import.meta.url), "utf8");
  const body = (sel: string) => css.split(sel)[1].split("}")[0].replace(/\s+/g, " ").trim();
  const names = (b: string) => [...b.matchAll(/(--[\w-]+)\s*:/g)].map((m) => m[1]).sort();
  const classicLight = names(body(':root[data-theme="light"] {'));
  const retroLight = body(':root[data-look="retro"][data-theme="light"] {');
  assert.equal(retroLight, body(':root[data-look="retro"]:not([data-theme="dark"]) {'));
  assert.deepEqual(names(retroLight), classicLight);
  const retroDark = names(body(':root[data-look="retro"] {'));
  for (const n of [...classicLight, "--r", "--radius", "--radius-sm", "--sans", "--editor-font"]) assert.ok(retroDark.includes(n), n);
  // after the classic light blocks, so its dark palette isn't overridden by an equally specific classic one
  assert.ok(css.indexOf(':root[data-look="retro"] {') > css.indexOf(':root:not([data-theme="dark"]) {'));
});

test("nextTheme cycles system -> light -> dark -> system", () => {
  assert.equal(nextTheme(""), "light");
  assert.equal(nextTheme("light"), "dark");
  assert.equal(nextTheme("dark"), "");
  assert.equal(nextTheme("junk"), "light");
  assert.equal(nextTheme(undefined), "light");
});

test("applyLook sets, replaces and clears data-look", () => {
  const el = { dataset: {} as Record<string, string | undefined> };
  (globalThis as any).document = { documentElement: el };
  applyLook("retro"); assert.equal(el.dataset.look, "retro");
  applyLook("mine"); assert.equal(el.dataset.look, "mine");
  applyLook(""); assert.equal(el.dataset.look, undefined);
  applyLook("retro"); applyLook(undefined); assert.equal(el.dataset.look, undefined);
});

test("setUserLook adds, replaces and removes one style element", () => {
  const head: any[] = [];
  const mk = () => { const e: any = { id: "", textContent: "", remove: () => head.splice(head.indexOf(e), 1) }; return e; };
  (globalThis as any).document = { head: { append: (e: any) => head.push(e) }, createElement: mk, getElementById: (id: string) => head.find((e) => e.id === id) ?? null };
  setUserLook(null); assert.equal(head.length, 0);
  setUserLook(":root{--bg:red}"); assert.equal(head.length, 1); assert.equal(head[0].id, "user-look"); assert.equal(head[0].textContent, ":root{--bg:red}");
  setUserLook(":root{--bg:blue}"); assert.equal(head.length, 1); assert.equal(head[0].textContent, ":root{--bg:blue}");
  setUserLook(null); assert.equal(head.length, 0);
});
