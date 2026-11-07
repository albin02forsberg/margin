import type { ChangeSet, Text } from "@codemirror/state";

/** Tab keys of decrypted-heading reports (`crypt:<path>:<line>`, see cryptView): all of them, or those of note PATH. */
export const isCryptKey = (key: string, path?: string) => key.startsWith(path == null ? "crypt:" : `crypt:${path}:`);

/** How saving tab T goes (#1154): a decrypted tab is only re-locked on an explicit save (FORCE: :w, Ctrl+S),
 *  never by autosave, a tab switch or :wa; everything else takes the normal file path. */
export const saveRoute = (t: { crypt?: unknown }, force: boolean) => (!t.crypt ? "file" : force ? "relock" : "none");

type CryptTab = { key: string; crypt?: { path: string; line: number } };

/** Heading LINE (0-based) of OLD after CHANGES turned it into DOC (#1172); -1 when a change removed the whole line
 *  with its line break (or LINE was already -1). The line start maps forward, so text inserted right at it lands above. */
export function remapLine(line: number, old: Text, changes: ChangeSet, doc: Text) {
  if (line < 0 || line >= old.lines) return -1;
  const { from, to } = old.line(line + 1);
  let gone = false;
  changes.iterChangedRanges((a, b) => { if (a <= from && b >= to && b - a > to - from) gone = true; });
  return gone ? findUnique(old.line(line + 1).text, old, doc) : doc.lineAt(changes.mapPos(from, 1)).number - 1;
}

/** A single change span (diffChange) can cover a heading it left intact: its 0-based line in DOC when TEXT is
 *  the only such line in both OLD and DOC, else -1 (a real delete, an edited heading or an ambiguous twin). */
function findUnique(text: string, old: Text, doc: Text) {
  const count = (d: Text) => { let n = 0, at = -1; for (let i = 1; i <= d.lines; i++) if (d.line(i).text === text) { n++; at = i - 1; } return n === 1 ? at : -1; };
  return count(old) < 0 ? -1 : count(doc);
}

/** The decrypted tab of the heading on LINE of note PATH (its key may still name the line it opened on). */
export const findCryptTab = <T extends CryptTab>(tabs: T[], path: string, line: number) =>
  tabs.find((t) => t.crypt?.path === path && t.crypt.line === line);

/** A key for a new decrypted tab of PATH/LINE that no open tab holds (a shifted tab keeps its opening key). */
export function cryptKey(tabs: CryptTab[], path: string, line: number) {
  let k = `crypt:${path}:${line}`;
  for (let n = 1; tabs.some((t) => t.key === k); n++) k = `crypt:${path}:${line}:${n}`;
  return k;
}
