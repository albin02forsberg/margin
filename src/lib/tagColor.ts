/** Inline style for a tag chip from config `tag_colors` (#1301): exact tag match, only `#rrggbb` is honoured; "" otherwise. */
export function tagStyle(tag: string, colors?: Record<string, string>): string {
  const c = colors?.[tag];
  return c && /^#[0-9a-f]{6}$/i.test(c) ? `--tag-color:${c};--tag-bg:color-mix(in srgb, ${c} 20%, transparent)` : "";
}
