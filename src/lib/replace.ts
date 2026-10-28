/** Text replace across notes (#872): a preview hit is (note, line, old text, new text). */
export type Hit = [string, number, string, string];

export const hitKey = (h: Hit) => `${h[1]}:${h[0]}`;

/** The hits not unticked (keys in OFF), as the apply's accept list: (note, line, previewed text). */
export const accepted = (hits: Hit[], off: Set<string>) =>
  hits.filter((h) => !off.has(hitKey(h))).map(([p, l, o]) => [p, l, o] as [string, number, string]);

/** A hit's preview label: the changed span as `[old → new]`, with up to CTX characters of the line around it. */
export function changeLabel(old: string, nu: string, ctx = 24) {
  let a = 0, b = 0;
  while (a < old.length && a < nu.length && old[a] === nu[a]) a++;
  while (b < old.length - a && b < nu.length - a && old[old.length - 1 - b] === nu[nu.length - 1 - b]) b++;
  const pre = old.slice(0, a).trimStart(), post = old.slice(old.length - b).trimEnd();
  const head = pre.length > ctx ? "…" + pre.slice(-ctx) : pre, tail = post.length > ctx ? post.slice(0, ctx) + "…" : post;
  return `${head}[${old.slice(a, old.length - b)} → ${nu.slice(a, nu.length - b)}]${tail}`;
}
