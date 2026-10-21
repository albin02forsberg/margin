import test from "node:test";
import assert from "node:assert/strict";
import { bareTitle, findTarget, linkHeadings, norm, retitlePlan, splitTarget } from "./links.ts";

const doc = "#+title: T\n* TODO [#A] Install :tag:\n:PROPERTIES:\n:CUSTOM_ID: setup\n:END:\nbody text\n** Usage\r\nmore\n";

test("splitTarget", () => {
  assert.deepEqual(splitTarget("file:a.org::*Foo"), { file: "a.org", search: "*Foo" });
  assert.deepEqual(splitTarget("file:a.org"), { file: "a.org", search: "" });
  assert.deepEqual(splitTarget("*Install"), { file: "", search: "*Install" });
  assert.deepEqual(splitTarget("#setup"), { file: "", search: "#setup" });
  assert.deepEqual(splitTarget("C:\\n\\a.org::5"), { file: "C:\\n\\a.org", search: "5" });
  assert.deepEqual(splitTarget("a.org"), { file: "a.org", search: "" });
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
