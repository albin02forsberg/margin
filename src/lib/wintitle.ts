/** Window title for the active tab: "• name — Margin" (bullet = unsaved), long names cut at 80 chars; no tab is just "Margin". */
export function windowTitle(tab?: { title: string; dirty: boolean }) {
  if (!tab?.title) return "Margin";
  const n = tab.title.length > 80 ? tab.title.slice(0, 79) + "…" : tab.title;
  return `${tab.dirty ? "• " : ""}${n} — Margin`;
}
