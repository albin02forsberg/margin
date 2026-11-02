import { test } from "node:test";
import assert from "node:assert/strict";
import { ago, changed, findNote, altTab, findTab, folderName, journalNeighbour, mergeChanged, noteFolders, onThisDay, onlyEchoes, relativeTo, resolvePath, weekJournals } from "./paths.ts";

test("findNote", () => {
  const files = ["/n/inbox.org", "/n/work/Plan.org", "/n/20260101120000-big_idea.org", "/n/x.org"];
  const titles = new Map([["/n/x.org", "Reading list"]]);
  const f = (q: string) => findNote(q, files, titles, "/n");
  assert.equal(f("inbox"), "/n/inbox.org");
  assert.equal(f("work/plan.org"), "/n/work/Plan.org");
  assert.equal(f("/n/work/Plan.org"), "/n/work/Plan.org");
  assert.equal(f("plan"), "/n/work/Plan.org");
  assert.equal(f("big idea"), "/n/20260101120000-big_idea.org");
  assert.equal(f("Reading List"), "/n/x.org");
  assert.equal(f("nope"), undefined);
});

test("findTab", () => {
  const tabs = [{ title: "Agenda" }, { title: "Plan", path: "/n/work/Plan.org" }, { title: "Reading list", path: "/n/x.org" }, { title: "Planet", path: "/n/planet.org" }];
  const f = (q: string) => findTab(q, tabs, "/n");
  assert.equal(f("2"), 1);
  assert.equal(f("9"), -1);
  assert.equal(f("0"), -1);
  assert.equal(f("plan"), 1); // file name beats the substring match on "Planet"
  assert.equal(f("work/plan"), 1);
  assert.equal(f("reading list"), 2);
  assert.equal(f("agen"), 0); // views match by title
  assert.equal(f("net"), 3);
  assert.equal(f("nope"), -1);
  assert.equal(findTab("today", [{ title: "Today's tasks" }, { title: "Today" }], "/n"), 1); // an exact title first
  const x = (q: string) => findTab(q, tabs, "/n", true); // `:bd NAME`: no substring fallback
  assert.equal(x("net"), -1);
  assert.equal(x("agen"), -1);
  assert.equal(x("agenda"), 0);
  assert.equal(x("plan"), 1);
  assert.equal(x("work/plan"), 1);
  assert.equal(x("3"), 2);
});

test("resolvePath", () => {
  assert.equal(resolvePath("/n/sub", "../other/target.org"), "/n/other/target.org");
  assert.equal(resolvePath("/n/sub/", "./a/../b.org"), "/n/sub/b.org");
  assert.equal(resolvePath("/n", "x.org"), "/n/x.org");
  assert.equal(resolvePath("/n", "../../../x.org"), "/x.org");
  assert.equal(resolvePath("C:\\n\\daily", "../x.org", "\\"), "C:\\n\\x.org");
  assert.equal(resolvePath("C:\\n", "..\\..\\x.org", "\\"), "C:\\x.org");
});

test("relativeTo", () => {
  assert.equal(relativeTo("/n/a.org", "/n/b.org"), "b.org");
  assert.equal(relativeTo("/n/daily/2026-10-02.org", "/n/ideas/x.org"), "../ideas/x.org");
  assert.equal(relativeTo("/n/daily/d.org", "/n/x.org"), "../x.org");
  assert.equal(relativeTo("/n/a.org", "/n/sub/deep/x.org"), "sub/deep/x.org");
  assert.equal(relativeTo("C:\\n\\daily\\d.org", "C:\\n\\x.org"), "../x.org");
});

test("journalNeighbour", () => {
  const f = ["/n/daily/2026-10-01.org", "/n/daily/2026-10-05.org", "/n/daily/notes.org", "/n/x/2026-10-03.org", "/n/daily/2026-10-09.org"];
  assert.equal(journalNeighbour(f, "daily", "2026-10-05", -1), "/n/daily/2026-10-01.org");
  assert.equal(journalNeighbour(f, "daily", "2026-10-05", 1), "/n/daily/2026-10-09.org");
  assert.equal(journalNeighbour(f, "daily", "2026-10-07", -1), "/n/daily/2026-10-05.org"); // today not in list
  assert.equal(journalNeighbour(f, "daily", "2026-10-01", -1), undefined);
  assert.equal(journalNeighbour(f, "daily", "2026-10-09", 1), undefined);
  assert.equal(journalNeighbour(["C:\\n\\a\\b\\2026-01-02.org", "C:\\n\\b\\2026-01-01.org"], "a/b", "2026-01-05", -1), "C:\\n\\a\\b\\2026-01-02.org");
  assert.equal(journalNeighbour([], "daily", "2026-10-01", 1), undefined);
});

test("onThisDay", () => {
  const f = ["/n/daily/2025-10-20.org", "/n/daily/2024-10-20.org", "/n/daily/2026-07-20.org", "/n/daily/2026-10-20.org", "/n/daily/2025-09-20.org",
    "/n/daily/2026-09-19.org", "/n/x/2023-10-20.org", "/n/daily/notes.org", "/n/daily/2026-12-20.org"];
  assert.deepEqual(onThisDay(f, "daily", "2026-10-20").map((x) => [x.date, x.ago]),
    [["2026-07-20", "3 months ago"], ["2025-10-20", "1 year ago"], ["2024-10-20", "2 years ago"]]);
  assert.deepEqual(onThisDay(f, "daily", "2026-10-21"), []);
  assert.deepEqual(onThisDay(["/n/daily/2024-02-29.org", "/n/daily/2025-12-31.org"], "daily", "2026-02-28"), []);
  assert.deepEqual(onThisDay(["/n/daily/2024-02-29.org"], "daily", "2028-02-29").map((x) => x.ago), ["4 years ago"]); // leap day
  assert.deepEqual(onThisDay(["/n/daily/2026-01-31.org"], "daily", "2026-03-31").map((x) => x.ago), ["2 months ago"]);
  assert.deepEqual(onThisDay([], "daily", "2026-10-20"), []);
});

test("mergeChanged", () => {
  assert.deepEqual(mergeChanged(["/n/a.org"], ["/n/b.org", "/n/a.org"]), ["/n/a.org", "/n/b.org"]);
  assert.equal(mergeChanged(undefined, ["/n/a.org"]), undefined);
  assert.equal(mergeChanged(["/n/a.org"], undefined), undefined);
  assert.deepEqual(mergeChanged([], []), []);
});

test("changed", () => {
  assert.ok(changed("/n/a.org"));
  assert.ok(changed("/n/a.org", ["/n/b.org", "/n/a.org"]));
  assert.ok(!changed("/n/a.org", ["/n/b.org"]));
  assert.ok(changed("/n/sub/a.org", ["/n/sub"]), "under a moved folder");
  assert.ok(!changed("/n/subway.org", ["/n/sub"]));
  assert.ok(changed("C:\\n\\sub\\a.org", ["C:\\n\\sub"]));
  assert.ok(!changed("/n/a.org", []));
});

test("noteFolders", () => {
  const f = ["a.org", "daily/d.org", "projects/x/y.org", "projects\\z.org", ".trash/t.org", "a/.hid/h.org", "attachments/a/n.org", "files/att/k.org", "files/ok.org"];
  assert.deepEqual(noteFolders(f, "attachments"), ["", "a", "daily", "files", "files/att", "projects", "projects/x"]);
  assert.deepEqual(noteFolders(f, "./files/att/"), ["", "a", "attachments", "attachments/a", "daily", "files", "projects", "projects/x"]);
  assert.deepEqual(noteFolders([], ""), [""]);
  const g = ["a.org", "images/i.org", "images/sub/j.org", "projects/x/images/k.org", "projects/x/n.org", "assets/img/m.org", "assets/o.org", "myimages/p.org"];
  assert.deepEqual(noteFolders(g, "", "images"), ["", "assets", "assets/img", "myimages", "projects", "projects/x"]);
  assert.deepEqual(noteFolders(g, "", "assets/img"), ["", "assets", "images", "images/sub", "myimages", "projects", "projects/x", "projects/x/images"]);
  assert.ok(noteFolders(g, "").includes("images"));
});

test("folderName", () => {
  assert.equal(folderName(" projects/2026 "), "projects/2026");
  assert.equal(folderName("a\\b//c/"), "a/b/c");
  assert.equal(folderName("./x/./y"), "x/y");
  assert.equal(folderName("."), "");
  for (const bad of ["/etc", "\\x", "C:\\x", "..", "a/../../b", "a/.."]) assert.equal(folderName(bad), null, bad);
});


test("onlyEchoes", () => {
  const echo = new Set(["/a"]);
  assert.equal(onlyEchoes(["/a"], ["/a"], echo), true); // our own save
  assert.equal(onlyEchoes(["/b"], ["/a"], echo), true); // not an open file
  assert.equal(onlyEchoes(["/a", "/b"], ["/b"], echo), false); // /b changed under us
  assert.equal(onlyEchoes([], ["/a"], echo), true);
  assert.equal(onlyEchoes(["/x"], [], echo), true);
  const big = Array.from({ length: 10000 }, (_, i) => `/n/${i}.org`);
  assert.equal(onlyEchoes(big, big, new Set(big)), true);
  assert.equal(onlyEchoes(big, big, new Set()), false);
});

test("ago", () => {
  const n = 1_000_000_000;
  assert.equal(ago(n + 5, n), "just now");
  assert.equal(ago(n - 59, n), "just now");
  assert.equal(ago(n - 60, n), "1 min ago");
  assert.equal(ago(n - 3599, n), "59 min ago");
  assert.equal(ago(n - 3600, n), "1 h ago");
  assert.equal(ago(n - 86399, n), "23 h ago");
  assert.equal(ago(n - 86400, n), "yesterday");
  assert.equal(ago(n - 2 * 86400, n), "2 days ago");
  assert.equal(ago(n - 7 * 86400, n), "2001-09-02");
});

test("weekJournals", () => {
  const f = ["/n/daily/2026-09-29.org", "/n/daily/2026-10-01.org", "/n/daily/2026-09-28.org", "/n/other/2026-09-30.org", "/n/daily/x.org", "/n/daily/2026-10-02.org"];
  assert.deepEqual(weekJournals(f, "daily", "2026-09-28", "2026-10-01"), ["/n/daily/2026-09-28.org", "/n/daily/2026-09-29.org", "/n/daily/2026-10-01.org"]);
});

test("altTab", () => {
  const tabs = [{ key: "a" }, { key: "b" }, { key: "c" }];
  assert.equal(altTab("b", tabs, 0), 1);
  assert.equal(altTab("b", tabs, 1), -1); // already there
  assert.equal(altTab("x", tabs, 0), -1); // closed
  assert.equal(altTab("", tabs, 0), -1);
});
