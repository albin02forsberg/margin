// Org links built from a target and optional description, and URL detection for paste.

/** `[[target][desc]]` (or `[[target]]` without a description); `]` in DESC becomes `)` so it can't end the link. */
export const linkFor = (target: string, desc = "") => (desc ? `[[${target}][${desc.replace(/\]/g, ")")}]]` : `[[${target}]]`);

/** The trimmed TEXT when it is a single http(s) / mailto URL, else null. */
export const asUrl = (text: string) => {
  const t = text.trim();
  return /^(https?:\/\/|mailto:)[^\s\[\]]+$/i.test(t) ? t : null;
};
