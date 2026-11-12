import { test } from "node:test";
import assert from "node:assert/strict";
import { dropWorkspace, parseWorkspaces, putWorkspace, renameInWorkspaces, workspaceName } from "./workspaces.ts";

const a = { tabs: [{ path: "/n/a.org", line: 2 }, { kind: "todo" as const, query: "tag:x", title: "x" }], active: 1, split: -1 };
const b = { tabs: [{ path: "/n/b.org", line: 0 }, { path: "/n/c.org", line: 5 }], active: 0, split: 1 };

test("unreadable stores are empty", () => {
  for (const raw of [null, "", "{", "null", "[]", "3", '"x"']) assert.deepEqual(parseWorkspaces(raw), {}, String(raw));
});

test("round trip, with bad entries, blank names and empty sessions dropped", () => {
  const ws = putWorkspace(putWorkspace({}, "Client", a), "Home", b);
  assert.deepEqual(parseWorkspaces(JSON.stringify(ws)), { Client: a, Home: b });
  const raw = JSON.stringify({ ...ws, " ": a, Empty: { tabs: [] }, Bad: 3, Mixed: { tabs: [{ path: "/n/x.org" }, a.tabs[0]], active: 1 } });
  assert.deepEqual(parseWorkspaces(raw), { Client: a, Home: b, Mixed: { tabs: [a.tabs[0]], active: 0, split: -1 } });
});

test("renameInWorkspaces re-points tabs in every workspace, keeping indexes and views", () => {
  const ws = { Client: a, Home: b, Other: { ...b, tabs: [{ path: "/n/b.org", line: 9 }] } };
  const snap = JSON.stringify(ws);
  const r = renameInWorkspaces(ws, "/n/b.org", "/m/b.org");
  assert.equal(JSON.stringify(ws), snap);
  assert.deepEqual(r.Home, { tabs: [{ path: "/m/b.org", line: 0 }, b.tabs[1]], active: 0, split: 1 });
  assert.deepEqual(r.Other.tabs, [{ path: "/m/b.org", line: 9 }]);
  assert.deepEqual(r.Client, a);
});

test("renameInWorkspaces returns the input when nothing matches", () => {
  const ws = { Client: a };
  assert.equal(renameInWorkspaces(ws, "/n/zzz.org", "/n/y.org"), ws);
});

test("put replaces, an empty session or drop removes", () => {
  const ws = putWorkspace({ Client: a }, "Client", b);
  assert.deepEqual(ws, { Client: b });
  assert.deepEqual(putWorkspace(ws, "Client", { tabs: [], active: -1, split: -1 }), {});
  assert.deepEqual(dropWorkspace({ Client: a, Home: b }, "Client"), { Home: b });
});

test("names are trimmed and blank is none", () => {
  assert.equal(workspaceName("  Client "), "Client");
  for (const n of ["", "  ", null, undefined]) assert.equal(workspaceName(n), null);
});
