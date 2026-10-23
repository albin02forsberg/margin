import { test } from "node:test";
import assert from "node:assert/strict";
import { block, expandSnippet, snippetAt } from "./snippet.ts";

const now = new Date(2026, 9, 12, 9, 5); // Mon

test("built-in blocks: cursor after `src ` or on the middle line, indent kept", () => {
  assert.deepEqual(snippetAt("<s", []), { text: "#+begin_src \n\n#+end_src", cursor: 12 });
  assert.deepEqual(snippetAt("  <q", []), { text: "  #+begin_quote\n  \n  #+end_quote", cursor: 18 });
  assert.equal(snippetAt("<l", [])!.text, "#+begin_export latex\n\n#+end_export");
  assert.equal(snippetAt("<el", [])!.text, "#+begin_src emacs-lisp\n\n#+end_src");
  assert.deepEqual(block("src"), snippetAt("<s", []));
});

test("no match falls through (null): unknown keys, headings, text around the key", () => {
  for (const l of ["<zz", "* Heading", "** TODO x", "a <s", "<s ", "<", "<constructor", ""]) assert.equal(snippetAt(l, []), null, l);
});

test("user snippets win over built-ins and expand placeholders, indent and cursor", () => {
  const user = [{ key: "s", text: "mine" }, { key: "mtg", text: "* Meeting %u\n- %?\n\nat %T, 100%%" }];
  assert.deepEqual(snippetAt("<s", user, now), { text: "mine", cursor: 4 });
  const e = snippetAt("  <mtg", user, now)!;
  assert.equal(e.text, "  * Meeting [2026-10-12 Mon]\n  - \n\n  at <2026-10-12 Mon 09:05>, 100%");
  assert.equal(e.text.slice(0, e.cursor), "  * Meeting [2026-10-12 Mon]\n  - ");
});

test("expandSnippet: cursor at the end without %?, first %? wins", () => {
  assert.deepEqual(expandSnippet("%t %U", "", now), { text: "<2026-10-12 Mon> [2026-10-12 Mon 09:05]", cursor: 39 });
  assert.deepEqual(expandSnippet("a%?b%?c", "", now), { text: "abc", cursor: 1 });
});
