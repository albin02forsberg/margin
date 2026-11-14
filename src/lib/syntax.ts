// Org syntax completion (#1345): `#+` keywords, dynamic block options, src languages and header arguments, and
// property drawer keys, as you type. Only what Margin itself reads; the tests guard the skeletons against the parsers.

/** One completion: LABEL (what the typed query filters), DETAIL, the text INSERTed over the query and the CURSOR offset
 *  into it (default its end). */
export type SyntaxOption = { label: string; detail: string; insert: string; cursor?: number };
/** Where a line sits: inside a `:PROPERTIES:` drawer, inside a src/example block's body, or neither. */
export type LineCtx = "drawer" | "code" | null;

/** The context of a line from the lines ABOVE it, nearest first (looks at most 200 lines up). */
export function lineContext(above: Iterable<string>): LineCtx {
  let n = 0;
  for (const l of above) {
    if (++n > 200 || /^\*+\s/.test(l)) return null;
    const t = l.trim().toLowerCase();
    if (/^#\+end_(src|example)\b/.test(t) || t === ":end:" || t === ":logbook:") return null;
    if (/^#\+begin_(src|example)\b/.test(t)) return "code";
    if (t === ":properties:") return "drawer";
  }
  return null;
}

// [name, detail, text after `#+` with `|` at the cursor (default the end)]; keyword words follow the query's case.
const KEYWORDS: [string, string, string][] = [
  ["title", "the note's title", "title: "],
  ["filetags", "tags of the whole note", "filetags: :"],
  ["category", "agenda category", "category: "],
  ["startup", "initial folding: overview, content, showall, show2levels", "startup: "],
  ["toc", "the note's outline, shown under the line", "toc: headlines"],
  ["name", "names the block, table or result below (for :var and #+CALL:)", "name: "],
  ["call", "runs a named src block: name(x=1)", "call: "],
  ["link", "link abbreviation: NAME TEMPLATE", "link: "],
  ["transclude", "shows another note's text here: [[link]]", "transclude: "],
  ["print_bibliography", "lists the cited entries", "print_bibliography:"],
  ["wordgoal", "word goal for this note", "wordgoal: "],
  ["tblfm", "formulas of the table above", "tblfm: "],
  ["begin_src", "code block (Ctrl+Enter runs sh)", "begin_src |\n#+end_src"],
  ["begin_quote", "quote block", "begin_quote\n|\n#+end_quote"],
  ["begin_example", "verbatim block", "begin_example\n|\n#+end_example"],
  ["begin_verse", "verse, line breaks kept", "begin_verse\n|\n#+end_verse"],
  ["begin: clocktable", "hours from the time log, regenerated", "begin: clocktable :block thisweek|\n#+end:"],
  ["begin: tasks", "tasks matching a query, regenerated", 'begin: tasks :query "|"\n#+end:'],
  ["begin: columnview", "table of heading properties, regenerated", 'begin: columnview :query "|" :cols "ITEM"\n#+end:'],
];
/** Dynamic block names (`clockBlocks`): [name, detail, skeleton from the name on]. */
const DYN = KEYWORDS.filter(([k]) => k.startsWith("begin: ")).map(([k, d, r]) => [k.slice(7), d, r.slice(7)]);
/** Options of each dynamic block (`clocktable_opts`, `tasks_opts`, `columnview_opts` in lib.rs) and their fixed values. */
const OPTS: Record<string, Record<string, string>> = {
  clocktable: { block: "today, thisweek, thismonth or YYYY-MM (required)", project: "only this project (\"quoted\" for spaces)", by: "project or tag" },
  tasks: { query: "task query, \"quoted\" (required)", limit: "at most N tasks (default 20)", sort: "priority, date or title" },
  columnview: { query: "heading query, \"quoted\" (required)", cols: "columns, \"ITEM TODO EFFORT\"", sort: "column, -COLUMN descending", limit: "at most N rows (default 50, max 500)" },
};
const VALUES: Record<string, string[]> = { "clocktable:block": ["today", "thisweek", "thismonth"], "clocktable:by": ["project", "tag"], "tasks:sort": ["priority", "date", "title"] };
/** `#+begin_src` languages: the ones `babel.rs` runs, and mermaid diagrams. */
const LANGS: [string, string][] = [["sh", "runs with Ctrl+Enter (when listed in Run code blocks)"], ["shell", "runs as sh"], ["mermaid", "diagram shown under the block"]];
/** Src header arguments `babel.ts` reads. */
const HEADERS: [string, string][] = [["tangle", "file to tangle into, yes or no"], ["shebang", "first line of the tangled file, makes it executable"], ["mkdirp", "yes: create missing folders when tangling"], ["var", "x=NAME: a named result or value"]];
/** Property keys Margin reads (org.rs, links). */
const PROPS: [string, string][] = [
  ["ID", "unique id for [[id:…]] links"], ["CUSTOM_ID", "anchor for [[#…]] links"], ["CATEGORY", "agenda category"], ["EFFORT", "estimate, 1:30"],
  ["STYLE", "habit: a repeating habit"], ["ORDERED", "t: children done in order"], ["BLOCKER", "ids this task waits for"], ["TARGET", "goal value"],
  ["START", "start value"], ["BIRTHDAY", "YYYY-MM-DD"], ["ANNIVERSARY", "YYYY-MM-DD"], ["ROAM_ALIASES", "other names for links"],
];

const opt = (label: string, detail: string, insert = label): SyntaxOption => ({ label, detail, insert });
/** An option whose TEXT has a `|` at the cursor and its next lines indented by IND. */
function skel(label: string, detail: string, text: string, ind: string): SyntaxOption {
  const t = text.replace(/\n/g, "\n" + ind), c = t.indexOf("|");
  return c < 0 ? opt(label, detail, t) : { label, detail, insert: t.slice(0, c) + t.slice(c + 1), cursor: c };
}

/** What to complete at the end of BEFORE (the line up to the cursor) in a line with context CTX: the QUERY typed so far
 *  (the text the options replace) and the OPTIONS; null for none. Keywords follow the query's case (`#+T` → `TITLE:`). */
export function syntaxAt(before: string, ctx: LineCtx): { query: string; options: SyntaxOption[] } | null {
  if (ctx === "code") return null;
  let m: RegExpExecArray | null;
  if (ctx === "drawer") return (m = /^\s*:([\w-]*)$/.exec(before)) ? { query: m[1], options: PROPS.map(([k, d]) => opt(k, d, `${k}: `)) } : null;
  if ((m = /^(\s*)#\+([\w-]*)$/.exec(before))) {
    const [, ind, q] = m, up = /[A-Z]/.test(q), cased = (s: string) => (up ? s.replace(/^\w+|#\+\w+/g, (w) => w.toUpperCase()) : s);
    return { query: q, options: KEYWORDS.map(([k, d, r]) => skel(cased(k), d, cased(r), ind)) };
  }
  if ((m = /^(\s*)#\+begin:[ \t]+(\w*)$/i.exec(before))) {
    const ind = m[1], up = before.includes("#+BEGIN");
    return { query: m[2], options: DYN.map(([k, d, r]) => skel(k, d, up ? r.replace("#+end:", "#+END:") : r, ind)) };
  }
  if ((m = /^\s*#\+begin:[ \t]+(clocktable|tasks|columnview)\b.*[ \t]:(\w+)[ \t]+(\w*)$/i.exec(before))) {
    const v = VALUES[`${m[1].toLowerCase()}:${m[2].toLowerCase()}`];
    return v ? { query: m[3], options: v.map((x) => opt(x, "")) } : null;
  }
  if ((m = /^\s*#\+begin:[ \t]+(clocktable|tasks|columnview)\b(.*[ \t])?:(\w*)$/i.exec(before))) {
    const used = m[2] ?? "";
    return { query: m[3], options: Object.entries(OPTS[m[1].toLowerCase()]).filter(([k]) => !used.includes(`:${k} `)).map(([k, d]) => opt(k, d, `${k} `)) };
  }
  if ((m = /^\s*#\+begin_src[ \t]+([\w-]*)$/i.exec(before))) return { query: m[1], options: LANGS.map(([k, d]) => opt(k, d)) };
  if ((m = /^\s*#\+begin_src[ \t]+\S+.*[ \t]:(\w*)$/i.exec(before))) return { query: m[1], options: HEADERS.map(([k, d]) => opt(k, d, `${k} `)) };
  return null;
}
