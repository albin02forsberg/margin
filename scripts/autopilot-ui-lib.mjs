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

const ANSI = /\x1b\[[0-9;]*m/g;
/** Terminal cells of `s`: ANSI codes count 0, emoji and CJK 2, the rest 1. */
export function vlen(s) {
  let n = 0;
  for (const ch of s.replace(ANSI, "")) { const c = ch.codePointAt(0); n += c >= 0x1f300 || (c >= 0x1100 && c <= 0x115f) || (c >= 0x2e80 && c <= 0xa4cf) || (c >= 0xff00 && c <= 0xff60) ? 2 : 1; }
  return n;
}

/** Pads `s` with spaces to `w` visible cells (ANSI-aware); longer strings are left as is. */
export const padVisible = (s, w) => s + " ".repeat(Math.max(0, w - vlen(s)));

/** Puts line arrays side by side: left padded to `lw` cells, `sep` between. */
export function columns(left, right, lw, sep = " │ ") {
  return Array.from({ length: Math.max(left.length, right.length) }, (_, i) => padVisible(left[i] ?? "", lw) + sep + (right[i] ?? ""));
}

/**
 * Lines per section when `avail` rows are left for Up next (`up` lines), totals (`totals` lines) and Recent
 * (`rmin`..`rmax`). Recent shrinks to its minimum first, then totals go, then Up next is cut (never below 1).
 */
export function budget(avail, up, totals, rmin, rmax) {
  const r = Math.min(rmax, avail - up - totals);
  if (r >= rmin) return { up, totals, recent: r };
  if (avail - up >= rmin) return { up, totals: 0, recent: Math.min(rmax, avail - up) };
  return { up: Math.max(1, Math.min(up, avail - rmin)), totals: 0, recent: rmin };
}

/** First `n` of `lines`, the last slot replaced by `more(cutCount)` when some are cut. */
export const clip = (lines, n, more) => (lines.length <= n ? lines : [...lines.slice(0, Math.max(0, n - 1)), more(lines.length - Math.max(0, n - 1))]);
