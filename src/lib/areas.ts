/** Areas the user can hide in Settings (`hidden_areas`, #858): each one's command labels start with one of these prefixes. */
export const AREAS: Record<string, string[]> = {
  ledger: ["Ledger:", "Go to Ledger"],
  habits: ["Habits:"],
  flashcards: ["Flashcards:"],
  goals: ["Tasks: goals"],
};

/** CMDS without those of the HIDDEN areas (names as in AREAS, any case; unknown names are ignored). */
export function visibleCmds<T extends { label: string }>(cmds: T[], hidden: string[]): T[] {
  const pre = hidden.flatMap((a) => AREAS[a.trim().toLowerCase()] ?? []);
  return pre.length ? cmds.filter((c) => !pre.some((p) => c.label.startsWith(p))) : cmds;
}
