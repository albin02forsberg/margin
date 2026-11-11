import test from "node:test";
import assert from "node:assert/strict";
import { bareTitle, bracketRetitle, completionAt, findTarget, fuzzyTarget, linkHeadings, linkTarget, norm, peekBuffer, peekTarget, retitlePlan, splitTarget, linkAbbrevs, expandLink, hereLink } from "./links.ts";

test("#+LINK abbreviations", () => {
  const a = linkAbbrevs("#+LINK: gh https://g/i/%s\n#+link: q https://x/?q=%h\n#+LINK: p https://p/\n#+LINK: gh dup\n#+LINK: id https://evil/%s\n#+LINK: file x%s\n");
  assert.equal(expandLink("gh:12", a), "https://g/i/12");
  assert.equal(expandLink("q:a b&c", a), "https://x/?q=a%20b%26c");
  assert.equal(expandLink("p:x", a), "https://p/x");
  assert.equal(expandLink("zz:1", a), "zz:1");
  assert.equal(expandLink("id:1", a), "id:1");
  assert.equal(expandLink("file:a.org", a), "file:a.org");
  assert.equal(expandLink("gh:%s", a), "https://g/i/%s"); // no recursion
});

const doc = "#+title: T\n* TODO [#A] Install :tag:\n:PROPERTIES:\n:CUSTOM_ID: setup\n:END:\nbody text\n** Usage\r\nmore\n";

test("splitTarget", () => {
  assert.deepEqual(splitTarget("file:a.org::*Foo"), { file: "a.org", search: "*Foo" });
  assert.deepEqual(splitTarget("file:a.org"), { file: "a.org", search: "" });
  assert.deepEqual(splitTarget("*Install"), { file: "", search: "*Install" });
  assert.deepEqual(splitTarget("#setup"), { file: "", search: "#setup" });
  assert.deepEqual(splitTarget("C:\\n\\a.org::5"), { file: "C:\\n\\a.org", search: "5" });
  assert.deepEqual(splitTarget("a.org"), { file: "a.org", search: "" });
  // A radio mention's target (#1265): the current note, found by plain search.
  assert.deepEqual(splitTarget("::<<<Margin App>>>"), { file: "", search: "<<<Margin App>>>" });
  assert.equal(findTarget("* Glossary\nThe <<<Margin App>>> is it.\n", "<<<Margin App>>>"), 1);
});

test("fuzzyTarget", () => {
  const t = "* TODO [#A] Goals [1/2] :work:\nsome <<Anchor Here>> text\n#+NAME: tbl one\n| a |\n<<<radio>>>\n#+BEGIN_SRC\n<<hidden>>\n#+END_SRC\n* v1.2 notes";
  assert.equal(fuzzyTarget(t, "goals"), 0);
  assert.equal(fuzzyTarget(t, "anchor  here"), 1);
  assert.equal(fuzzyTarget(t, "Tbl One"), 2);
  assert.equal(fuzzyTarget(t, "v1.2 notes"), 8); // dotted bare name still fuzzy
  assert.equal(fuzzyTarget(t, "radio"), null);
  assert.equal(fuzzyTarget(t, "hidden"), null);
  assert.equal(fuzzyTarget(t, "nothing"), null);
  for (const x of ["goals.org", "sub/goals", "file:goals", "https://x", "*Goals", "#id"]) assert.equal(fuzzyTarget(t, x), null, x);
  assert.equal(fuzzyTarget("<<x>>\n* x", "x"), 0);
});

test("findTarget", () => {
  assert.equal(findTarget(doc, "*install"), 1);
  assert.equal(findTarget(doc, "*Usage"), 6);
  assert.equal(findTarget(doc, "*Nope"), null);
  assert.equal(findTarget(doc, "#setup"), 1);
  assert.equal(findTarget(doc, "#nope"), null);
  assert.equal(findTarget(doc, "3"), 2);
  assert.equal(findTarget(doc, "999"), 8);
  assert.equal(findTarget(doc, "body"), 5);
  assert.equal(findTarget(doc, ""), null);
});

test("findTarget: custom keywords and statistics cookies", () => {
  const d = "* DOING Write report\n* Release [2/5]\n* TODO Ship   it [40%] :work:\n* C++ [/]\n";
  assert.equal(findTarget(d, "*Write report"), null);
  assert.equal(findTarget(d, "*DOING Write report"), 0);
  assert.equal(findTarget(d, "*Write report", ["TODO", "DOING", "DONE"]), 0);
  assert.equal(findTarget(d, "*Ship it", ["DOING"]), null);
  assert.equal(findTarget(d, "*Ship it"), 2);
  assert.equal(findTarget(d, "*Release"), 1);
  assert.equal(findTarget(d, "*Release [2/5]"), 1);
  assert.equal(findTarget(d, "*Release [3/5]"), 1);
  assert.equal(findTarget(d, "*C++", ["C++", "DONE"]), null);
  assert.equal(findTarget(d, "*c++"), 3);
  assert.equal(findTarget("* A+ Go\n", "*Go", ["A+"]), 0);
});

test("linkHeadings", () => {
  const t = (path: string, line: number | null, title: string) => ({ path, line, title });
  const got = linkHeadings(
    [t("a", null, "Note A"), t("a", 1, "Plan [2/5]"), t("a", 4, "plan"), t("a", 6, "Has id"), t("a", 8, "[[x]] link"), t("a", 9, " "), t("b", 2, "Plan")],
    new Set(["a:6"]),
  );
  assert.deepEqual(got.map((h) => [h.path, h.line, h.bare]), [["a", 1, "Plan"], ["b", 2, "Plan"]]);
  assert.equal(bareTitle("  Ship  it [40%] [/]"), "Ship it");
});

// Same cases as `heading_norm_matches_links_ts` in src-tauri/src/notes.rs.
test("norm", () => {
  for (const [a, b] of [["Ship  it [40%] [/]", "ship it"], ["  Release [2/5]x ", "release x"], ["Plan :café:", "plan :café:"], ["ÅÄÖ Tab\there", "åäö tab here"], ["[a] [1/2/3]", "[a] [1/2/3]"], ["C++ [/]", "c++"]])
    assert.equal(norm(a), b);
});

test("retitlePlan", () => {
  const kw = ["TODO", "DONE"];
  const plan = (before: string, after: string, line = 0) => retitlePlan(before, after, line, kw);
  assert.deepEqual(plan("* TODO [#A] Old [1/2] :x:\n", "* TODO [#A] New name [1/2] :x:\n"), { old: "old", new: "New name" });
  assert.deepEqual(plan("* a\n* Plan :café:\n", "* a\n* Done :café:\n", 1), { old: "plan :café:", new: "Done :café:" });
  assert.equal(plan("* Old\n", "* OLD [0/1]\n"), null); // case/cookie only
  assert.equal(plan("text\n", "text2\n"), null);
  assert.match(plan("* Old\n* old\n", "* New\n* old\n") as string, /unique/);
  assert.match(plan("* Old\n", "* New [x]\n") as string, /link/);
  assert.match(plan("* Old\n", "* [1/2]\n") as string, /link/);
  assert.match(plan("* Old\n** new\n", "* New\n** new\n") as string, /another heading/);
  assert.deepEqual(retitlePlan("* DOING Old\n", "* DOING New\n", 0, ["DOING"]), { old: "old", new: "New" });
});

test("bracketRetitle", () => {
  assert.deepEqual(bracketRetitle("* Old\n", "* New [x]\n", 0), { old: "old", new: "New [x]" });
  assert.equal(bracketRetitle("* Old\n* Old\n", "* New [x]\n* Old\n", 0), null);
  assert.equal(bracketRetitle("* Old\n", "* New\n", 0), null);
  assert.equal(findTarget("* New [x]\n", "*New [x]"), 0);
});

test("peekTarget passes org targets, prefixes bare paths and refuses URLs", () => {
  for (const t of ["id:abc", "file:a.org::*Foo", "*Install", "#setup"]) assert.equal(peekTarget(t), t);
  assert.equal(peekTarget("./a b.org::*X"), "file:./a b.org::*X");
  assert.equal(peekTarget("C:\\n\\a.org"), "file:C:\\n\\a.org");
  assert.equal(peekTarget("https://x.org"), null);
  assert.equal(peekTarget("mailto:a@b.c"), null);
});

test("peekBuffer finds a radio target", () => {
  const buf = "* A\nsee <<<Term>>> here\nmore\n";
  assert.deepEqual(peekBuffer(buf, "<<<Term>>>")?.line, 1);
  assert.equal(peekBuffer(buf, "<<<Nope>>>"), null);
});

test("peekBuffer reads the unsaved buffer", () => {
  const buf = "#+title: T\n* A\nbody a\n** Sub\nsub\n* New name\nnew body\n";
  assert.deepEqual(peekBuffer(buf, "*New name"), { line: 5, text: "new body" });
  assert.deepEqual(peekBuffer(buf, "*A"), { line: 1, text: "body a\n** Sub\nsub" });
  assert.equal(peekBuffer(buf, "*Old name"), null);
  assert.deepEqual(peekBuffer(buf, ""), { line: 0, text: buf });
});

test("linkTarget: id, file and heading links relative to the note (#899)", () => {
  assert.equal(linkTarget("/n/a.org", { id: "x1" }), "id:x1");
  assert.equal(linkTarget("/n/sub/a.org", { path: "/n/b.org" }), "file:../b.org");
  assert.equal(linkTarget("/n/a.org", { path: "/n/sub/b.org", head: "Usage" }), "file:sub/b.org::*Usage");
  assert.equal(linkTarget("/n/a.org", { path: "/n/a.org", head: "Usage" }), "*Usage");
});

test("completionAt: [[ links, @ people after whitespace, :tags only on heading lines (#899)", () => {
  assert.deepEqual(completionAt("see [[Proj", false), { kind: "link", query: "Proj" });
  assert.deepEqual(completionAt("[[", false), { kind: "link", query: "" });
  assert.equal(completionAt("see [[a][b]] done", false), null);
  assert.deepEqual(completionAt("ask @al", false), { kind: "person", query: "al" });
  assert.deepEqual(completionAt("@", false), { kind: "person", query: "" });
  assert.equal(completionAt("mail a@b", false), null);
  assert.deepEqual(completionAt("* TODO Plan :wo", true), { kind: "tag", query: "wo" });
  assert.deepEqual(completionAt("* Plan :work:", true), { kind: "tag", query: "" });
  assert.equal(completionAt("note :wo", false), null);
  assert.equal(completionAt("* Time: 10", true), null);
  assert.equal(completionAt("plain text", false), null);
});

test("hereLink: %a links back to the heading at the cursor without writing to the note", () => {
  const note = "#+title: Proj [x]\n* Intro\ntext\n** TODO [#A] Fix [1/3] the bug :work:\nSCHEDULED: <2026-10-02 Fri>\n:PROPERTIES:\n:EFFORT: 1h\n:END:\nbody\n* With id\n:PROPERTIES:\n:ID: abc-1\n:END:\nmore\n* Odd [[x]]\n";
  const from = "/n/inbox.org", at = "/n/sub/proj.org";
  assert.equal(hereLink(from, at, note, 8), "[[file:sub/proj.org::*Fix the bug][Fix the bug]]");
  assert.equal(hereLink(from, at, note, 3), "[[file:sub/proj.org::*Fix the bug][Fix the bug]]");
  assert.equal(hereLink(from, at, note, 13), "[[id:abc-1][With id]]");
  assert.equal(hereLink(at, at, note, 2), "[[*Intro][Intro]]");
  // Above the first heading, or a title `::*` can't carry: the note itself.
  assert.equal(hereLink(from, at, note, 0), "[[file:sub/proj.org][Proj x]]");
  assert.equal(hereLink(from, at, note, 14), "[[file:sub/proj.org][Proj x]]");
  assert.equal(hereLink(from, at, ":PROPERTIES:\n:ID: n-1\n:END:\nhi\n", 3), "[[id:n-1][proj.org]]");
  // A child's drawer isn't its parent's: the ID scan stops at the next heading.
  assert.equal(hereLink(from, at, "* P\n** C\n:PROPERTIES:\n:ID: c\n:END:\n", 0), "[[file:sub/proj.org::*P][P]]");
});

test("linkAbbrevs skips src/example blocks", () => {
  assert.equal(linkAbbrevs("#+begin_src org\n#+LINK: gh https://x/%s\n#+end_src\n").size, 0);
  assert.equal(linkAbbrevs("#+BEGIN_EXAMPLE\n#+LINK: gh https://x/%s\n#+END_EXAMPLE\n#+LINK: gh https://real/%s\n").get("gh"), "https://real/%s");
  assert.equal(linkAbbrevs("#+begin_src\n#+LINK: gh https://x/%s\n").size, 0);
});
