# UI direction mockups (#251)

Static mockups of the Today and Time views, not shipped in the app. They use the app's real
`src/lib/theme.css` tokens and the demo data from `e2e/screenshots.ts`.

- **A. Quiet** (`a-quiet.css`): whitespace, one accent colour, 1px dividers, muted metadata.
- **B. Cards** (`b-cards.css`): soft cards with subtle shadows on a tinted canvas.
- **C. Dense** (`c-dense.css`): tight rows, monospace metadata, IDE-like panes and status line.

Open `index.html?d=a|b|c&v=today|time` in a browser (light/dark follows the system), or run
`docs/mockups/render.sh` to regenerate `png/` (1280x800, needs Chrome).
