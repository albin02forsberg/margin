/** Emacs-layout menu bar (#1448): each menu lists palette commands by label (menubar.test.ts checks they exist). */
export const MENUS: [string, string[]][] = [
  ["File", ["New note", "Open note…", "Save", "Save all", "Reload from disk (discard unsaved changes)", "Close tab (also :q)", "Export: note as HTML", "Export: note as PDF", "Settings"]],
  ["Edit", ["Undo last edit made from a task view or the links panel", "Redo what Undo last edit took back", "Search in this note (occur)…", "Search in all notes", "Go to heading…"]],
  ["View", ["Go to Today", "Go to All tasks", "Go to Inbox", "Go to Time tracking", "View: toggle outline (headings of this note)", "Show note graph", "View: zen mode (hide top bar and status bar)", "View: pick a theme (look and its layout)", "Toggle top bar"]],
  ["Help", ["Help: tutorial", "Command palette", "Show what's new (notes of the last update)", "Report a bug (opens a prefilled GitHub issue)", "About Margin"]],
];
