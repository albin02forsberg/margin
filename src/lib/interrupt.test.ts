import { test } from "node:test";
import assert from "node:assert/strict";
import { resumeOffer, type Clock } from "./interrupt.ts";

const A = { project: "Acme", task: "Review" }, B = { project: "Beta", task: "" };
const on = (project: string, profile = "Work") => ({ profile, project });
const off = (profile = "Work") => ({ profile, project: null });

test("interrupt stack: offered on clock-out in order, not on switches, dropped on a profile change", () => {
  const s: Clock[] = [A, B];
  assert.equal(resumeOffer(s, on("Call"), on("Other")), null); // switching keeps the stack
  assert.equal(resumeOffer(s, off(), off()), null); // nothing stopped
  assert.deepEqual(resumeOffer(s, on("Call"), off()), B); // innermost first
  assert.deepEqual(resumeOffer(s, on("Beta"), off()), A);
  assert.equal(resumeOffer(s, on("Acme"), off()), null); // empty
  assert.deepEqual(resumeOffer([A], on(""), off()), A); // a project-less clock counts as running
  const t: Clock[] = [A];
  assert.equal(resumeOffer(t, on("Call"), off("Home")), null); // profile switch's auto clock-out
  assert.deepEqual(t, []);
  assert.equal(resumeOffer([A], {}, off()), null); // first status after start
});
