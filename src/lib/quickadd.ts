// Quick-add: peel trailing `#tag`, `!A|B|C` and date words off a task title.
export type QuickAdd = { title: string; tags: string[]; priority: string; dates: { title: string; date: string }[] };

const DATE_WORD = /^(today|tomorrow|(mon|tue|wed|thu|fri|sat|sun)[a-z]*|\+\d+[dwmy]|(\d{4}-)?\d{1,2}-\d{1,2}|\d{1,2}:\d{2})$/i;

/** `dates` are candidate (title, raw date text) splits, longest date first; the caller validates them
 *  (the grammar lives in Rust: `date_preview`). Only a trailing run is taken and the title never ends up empty. */
export function parseQuickAdd(text: string): QuickAdd {
  const w = text.trim().split(/\s+/).filter(Boolean);
  const tags: string[] = [];
  let priority = "";
  while (w.length > 1) {
    const t = w[w.length - 1];
    if (/^#[^\s#]+$/.test(t)) { if (!tags.includes(t.slice(1))) tags.unshift(t.slice(1)); }
    else if (/^![abc]$/i.test(t)) priority ||= t[1].toUpperCase();
    else break;
    w.pop();
  }
  let n = w.length;
  while (n > 1 && DATE_WORD.test(w[n - 1])) n--;
  const dates = [];
  for (let i = n; i < w.length; i++) dates.push({ title: w.slice(0, i).join(" "), date: w.slice(i).join(" ") });
  return { title: w.join(" "), tags, priority, dates };
}
