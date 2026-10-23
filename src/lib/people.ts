/** People for `Space n @` (#704): `@name` tags, one per name in any case (first spelling kept), sorted. */
export function peopleFrom(tags: string[]): string[] {
  const seen = new Map<string, string>();
  for (const t of tags) {
    const n = personName(t);
    if (t.trim().startsWith("@") && n && !seen.has(n.toLowerCase())) seen.set(n.toLowerCase(), n);
  }
  return [...seen.values()].sort((a, b) => a.localeCompare(b, undefined, { sensitivity: "base" }));
}

/** A typed or picked person without the leading `@`s and spaces. */
export const personName = (s: string) => s.trim().replace(/^@+/, "").trim();

type Task = { path: string; line: number; keyword: string | null; title: string; date: string; priority: string | null };
type Note = { path: string; line: number; title: string; text: string };

/** Picker rows for a person view: open tasks first (with priority and date), then the notes mentioning them. REL names a file. */
export function personRows(v: { tasks: Task[]; notes: Note[] }, rel: (p: string) => string) {
  const due = (d: string) => (d.startsWith("+") ? "" : ` · ${d}`);
  return [
    ...v.tasks.map((t) => ({ label: `${t.keyword ?? ""} ${t.priority ? `[#${t.priority}] ` : ""}${t.title}`.trim(), detail: `task · ${rel(t.path)}${due(t.date)}`, value: [t.path, t.line] as unknown })),
    ...v.notes.map((n) => ({ label: n.text, detail: `${n.title || rel(n.path)} · line ${n.line + 1}`, value: [n.path, n.line] as unknown })),
  ];
}
