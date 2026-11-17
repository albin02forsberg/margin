/** Paths whose current text (CURRENT, by path) no longer equals what the recorded action wrote (`after`). A missing entry counts as changed; line endings are ignored. */
export function stale(records: { path: string; after: string }[], current: Map<string, string | null>): string[] {
  const lf = (s: string | null | undefined) => s?.replace(/\r\n?/g, "\n"); // tab text is LF, untabbed read_file text may be CRLF
  return records.filter((r) => lf(current.get(r.path)) !== lf(r.after)).map((r) => r.path);
}

/** Wrap F so calls run one at a time, in order (a rejected call doesn't block the next): key-repeat `u u` must not read the same stack top twice. */
export function serial<A extends unknown[], R>(f: (...a: A) => Promise<R>): (...a: A) => Promise<R> {
  let last: Promise<unknown> = Promise.resolve();
  return (...a) => {
    const r = last.then(() => f(...a));
    last = r.catch(() => {});
    return r;
  };
}
