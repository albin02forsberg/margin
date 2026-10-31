// Pure helpers for scripts/autopilot-ui.mjs (no side effects, so tests can import them).

/** Shortens `s` to `n` chars, ending in "…" when cut. */
export const cut = (s, n) => (n < 2 ? "" : s.length > n ? s.slice(0, n - 1) + "…" : s);

/** Plan size of an issue (`S`/`M`/`L`): a `size:*` label, else the `**Size X**` line of the last plan comment. */
export function sizeOf(i) {
  const l = (i.labels ?? []).map((x) => x.name).find((n) => /^size:/i.test(n));
  if (l) return l.slice(5).trim().toUpperCase();
  const plan = [...(i.comments ?? [])].reverse().find((c) => c.body.includes("## Plan"));
  return plan?.body.match(/\*\*Size\s+([SML])\b/i)?.[1].toUpperCase() ?? "";
}

/** One ready-queue row: number and size columns aligned, title cut to fit width `w`. */
export const readyRow = (i, w) => `#${String(i.number).padEnd(5)} ${(sizeOf(i) || "·").padEnd(2)} ${cut(i.title, Math.max(10, w - 14))}`;

/** argv for `gh` to file a task for the agents; title and body stay separate entries (no shell). */
export const createArgs = (repo, title, body) => ["issue", "create", "-R", repo, "--title", title, "--body", body || "Queued by the owner from the autopilot dashboard."];

/** Issue number from `gh issue create` output (it prints the new issue URL). */
export const issueNo = (out) => out.match(/\/issues\/(\d+)/)?.[1] ?? "";
