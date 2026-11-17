/** Paths whose current text (CURRENT, by path) no longer equals what the recorded action wrote (`after`). A missing entry counts as changed. */
export function stale(records: { path: string; after: string }[], current: Map<string, string | null>): string[] {
  return records.filter((r) => current.get(r.path) !== r.after).map((r) => r.path);
}
