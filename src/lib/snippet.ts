import { inactiveStamp } from "./clock.ts";

/** A user snippet (`[[snippets]]` in config.toml): `<KEY` + Tab inserts TEXT. */
export type Snippet = { key: string; text: string };
/** An expansion: TEXT to put in place of the `<key` line and the CURSOR offset within it. */
export type Expansion = { text: string; cursor: number };

/** Built-in `<key` blocks (org-tempo's structure templates): key → what follows `#+begin_`. */
export const BLOCKS: Record<string, string> = {
  s: "src", q: "quote", e: "example", v: "verse", c: "center", C: "comment",
  l: "export latex", h: "export html", a: "export ascii", el: "src emacs-lisp",
};

/** `#+begin_SPEC` … `#+end_<first word>` at INDENT; the cursor waits for the language after a bare
 *  `src `, else sits on the empty middle line. */
export function block(spec: string, indent = ""): Expansion {
  const begin = `${indent}#+begin_${spec}${spec === "src" ? " " : ""}`;
  const text = `${begin}\n${indent}\n${indent}#+end_${spec.split(" ")[0]}`;
  return { text, cursor: spec === "src" ? begin.length : begin.length + 1 + indent.length };
}

/** Snippet TEXT expanded at NOW: %t %T active date/timestamp, %u %U inactive ones, %% a literal %,
 *  %? the cursor (else the end); lines after the first get INDENT (empty ones stay empty). */
export function expandSnippet(text: string, indent: string, now: Date): Expansion {
  const U = inactiveStamp(now), u = U.slice(0, -7) + "]";
  const act = (s: string) => `<${s.slice(1, -1)}>`;
  const map: Record<string, string> = { t: act(u), T: act(U), u, U, "%": "%", "?": "\u0001" };
  const out = text.replace(/%([tTuU%?])/g, (_, c) => map[c])
    .split("\n").map((l, i) => (i && l ? indent + l : l)).join("\n");
  const at = out.indexOf("\u0001");
  return at < 0 ? { text: indent + out, cursor: indent.length + out.length } : { text: indent + out.replaceAll("\u0001", ""), cursor: indent.length + at };
}

/** What `<key` alone on LINE expands to (USER snippets win over BLOCKS), or null so Tab does its usual thing. */
export function snippetAt(line: string, user: Snippet[], now = new Date()): Expansion | null {
  const m = /^(\s*)<([\w-]+)$/.exec(line);
  if (!m) return null;
  const s = user.find((s) => s.key === m[2]);
  if (s) return expandSnippet(s.text, m[1], now);
  return Object.hasOwn(BLOCKS, m[2]) ? block(BLOCKS[m[2]], m[1]) : null;
}
