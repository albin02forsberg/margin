/** Path from the folder of FROM (a file) to TO, with "/" separators, as org file: links expect. */
export function relativeTo(from: string, to: string): string {
  const parts = (p: string) => p.split(/[\\/]+/).filter(Boolean);
  const a = parts(from).slice(0, -1), b = parts(to);
  let i = 0;
  while (i < a.length && i < b.length - 1 && a[i] === b[i]) i++;
  return [...Array(a.length - i).fill(".."), ...b.slice(i)].join("/");
}
