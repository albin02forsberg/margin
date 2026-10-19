/** Org link targets: split `file:path::search` and resolve the search part to a line. */

/** `file:a.org::*Head` -> {file: "a.org", search: "*Head"}; `*Head` / `#id` alone -> {file: "", search} (current note). */
export function splitTarget(t: string): { file: string; search: string } {
  if (!t.startsWith("file:") && /^[*#]/.test(t)) return { file: "", search: t };
  const s = t.replace(/^file:/, "");
  const i = s.indexOf("::");
  return i < 0 ? { file: s, search: "" } : { file: s.slice(0, i), search: s.slice(i + 2) };
}

const HEAD = /^\*+\s+(?:(?:TODO|DONE|NEXT|WAIT\w*|CANCEL\w*)\s+)?(?:\[#[A-Z0-9]+\]\s+)?(.*?)(?:\s+:[\w@#%:]+:)?\s*$/;

/** 0-based line a search string points at (`*Heading`, `#custom-id`, line number, or plain text), or null. */
export function findTarget(text: string, search: string): number | null {
  const lines = text.split(/\r?\n/);
  if (!search) return null;
  if (/^\d+$/.test(search)) return Math.min(Math.max(+search, 1), lines.length) - 1;
  if (search[0] === "*") {
    const want = search.slice(1).trim().toLowerCase();
    const i = lines.findIndex((l) => HEAD.exec(l)?.[1].toLowerCase() === want);
    return i < 0 ? null : i;
  }
  if (search[0] === "#") {
    const id = search.slice(1).trim();
    const re = /^\s*:CUSTOM_ID:\s*(\S+)\s*$/i;
    for (let i = 0; i < lines.length; i++) {
      if (re.exec(lines[i])?.[1] !== id) continue;
      for (let j = i - 1; j >= 0; j--) if (/^\*+\s/.test(lines[j])) return j;
      return i;
    }
    return null;
  }
  const i = lines.findIndex((l) => l.includes(search));
  return i < 0 ? null : i;
}
