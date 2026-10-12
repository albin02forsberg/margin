import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

// .github/scripts/changelog.sh (#254) against a throwaway repo.
const script = resolve(import.meta.dirname, "../../.github/scripts/changelog.sh");

test("changelog.sh groups, skips and handles empty ranges", () => {
  const dir = mkdtempSync(join(tmpdir(), "changelog-"));
  const git = (...a: string[]) => execFileSync("git", a, { cwd: dir, encoding: "utf8" }).trim();
  const log = (...a: string[]) => execFileSync("bash", [script, ...a], { cwd: dir, encoding: "utf8" });
  const commit = (m: string) => git("-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "--allow-empty", "-m", m);
  try {
    git("init", "-q");
    commit("chore: init");
    git("tag", "v0.1.0");
    for (const m of ["feat(ui): zoom (#237)", "fix: crash on *empty* file (#12)", "chore: refresh graphify graph [skip ci]",
      "docs: README", "test: more", "chore: release v0.2.0", "plain subject"]) commit(m);
    assert.equal(log("v0.1.0", "HEAD"),
      "### Features\n- zoom (#237)\n\n### Fixes\n- crash on *empty* file (#12)\n\n### Other\n- plain subject\n- more\n\n");
    git("tag", "v0.2.0");
    assert.equal(log("v0.2.0", "HEAD"), "No changes.\n");
    commit("chore: deps");
    assert.equal(log("v0.2.0"), "Maintenance only.\n\n### Other\n- deps\n\n");
    assert.match(log(""), /### Other\n- deps\n- plain subject\n- more\n- init\n/); // empty <from> = whole history
    assert.doesNotMatch(log(""), /[0-9a-f]{40}/);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
