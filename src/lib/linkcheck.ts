/** Web link check (`Space n W`, #1325): what the Rust side returns, and the pure parts of the flow. */

export type WebLink = { url: string; path: string; line: number; count: number };
export type LinkStatus = { kind: "ok" } | { kind: "dead"; reason: string } | { kind: "moved"; to: string };

/** Runs F over XS, N at a time, starting no new call once STOP() is true; resolves when the started ones settle. */
export async function pool<T>(xs: T[], n: number, f: (x: T) => Promise<void>, stop: () => boolean): Promise<void> {
  let i = 0;
  const worker = async () => { while (i < xs.length && !stop()) await f(xs[i++]); };
  await Promise.all(Array.from({ length: Math.min(n, xs.length) }, worker));
}

/** The links STATUS has as dead (first) or moved, each group in note order; ok and unchecked ones are left out. */
export function failing(links: WebLink[], status: Map<string, LinkStatus>): { link: WebLink; status: Exclude<LinkStatus, { kind: "ok" }> }[] {
  const out = links.flatMap((link) => { const s = status.get(link.url); return s && s.kind !== "ok" ? [{ link, status: s }] : []; });
  return [...out.filter((x) => x.status.kind === "dead"), ...out.filter((x) => x.status.kind === "moved")];
}

/** A picker label for a failing link: the URL with why it failed or where it went, and how often it's linked. */
export function linkLabel(link: WebLink, s: Exclude<LinkStatus, { kind: "ok" }>): string {
  const times = link.count > 1 ? ` · ${link.count} links` : "";
  return s.kind === "dead" ? `✗ ${link.url} · ${s.reason}${times}` : `→ ${link.url} moved to ${s.to}${times}`;
}

/** The text replace (find, replace) that rewrites a moved link: `[[old]` matches that link's target and no longer URL. */
export const rewrite = (from: string, to: string): [string, string] => [`[[${from}]`, `[[${to}]`];
