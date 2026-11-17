/** Paths whose current text (CURRENT, by path) no longer equals what the recorded action wrote (`after`). A missing entry counts as changed. */
export function stale(records: { path: string; after: string }[], current: Map<string, string | null>): string[] {
  return records.filter((r) => current.get(r.path) !== r.after).map((r) => r.path);
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
