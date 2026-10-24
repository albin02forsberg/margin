/** Picks a note path from ITEMS ([path, title, mtime secs]), likelier the older it is; R in [0,1) is the random draw. "" when empty. */
export function pickStale(items: [string, string, number][], now: number, r: number): string {
  if (!items.length) return "";
  const w = items.map((i) => 1 + Math.max(0, now - i[2]));
  let x = r * w.reduce((a, b) => a + b, 0);
  for (let i = 0; i < items.length; i++) if ((x -= w[i]) < 0) return items[i][0];
  return items[items.length - 1][0];
}
