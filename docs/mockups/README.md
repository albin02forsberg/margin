# UI direction mockups (#251)

Static mockups of the Today and Time views, not shipped in the app. They use the app's real
`src/lib/theme.css` tokens and the demo data from `e2e/screenshots.ts`.

- **A. Quiet** (`a-quiet.css`): whitespace, one accent colour, 1px dividers, muted metadata.
- **B. Cards** (`b-cards.css`): soft cards with subtle shadows on a tinted canvas.
- **C. Dense** (`c-dense.css`): tight rows, monospace metadata, IDE-like panes and status line.

Round 3 (`b.html`, `b-variants.{js,css}`): five layouts of direction B.
B1 icon rail + one column of cards, B2 dashboard grid, B3 focus card + side panel,
B4 top bar + timelines, B5 board columns (tasks by state, time by project).

Round 4 (`r4.html`, `r4.{js,css}`, reusing round 3's helpers): B1, B2 and B5 on the editor, Tasks,
Projects, Settings, command palette and graph pages (`?s=1|2|5&p=…`), B1's rail with B5's board as
Tasks (`?s=mix&p=tasks`), and the motion they would use: view change (fade + 4 px rise, 180 ms), card
hover lift and focus ring (140 ms), palette open/close (160 ms), a task moving between board columns
(200 ms) and the clock ticking (160 ms), all off under `prefers-reduced-motion`. `?demo=view|hover|
palette|move|clock` loops one of them; `&t=ms` pauses it at that point, which is how `gif/` is made.

Round 5 (`r5.html`, `r5.{js,css}`, reusing rounds 3 and 4), the combined draft: B1's top bar (its rail laid
out sideways: logo, views, search, settings, clock, plus the floating key-hint pill), B5's layout (title row with
the agenda chips, then full-width columns) and B2's cards inside the columns. `?p=today|time|tasks|projects|
settings|editor|palette|graph`, `&bar=rail` for B1's real left rail instead, `&w=420` for a narrow window (the bar
keeps its icons, columns stack with the agenda first). `?demo=view|palette|move` re-does the round-4 demos that
look different in this shell (`gif/r5-*.gif`); hover and clock are unchanged from round 4.

Open `index.html?d=a|b|c&v=today|time`, `b.html?b=1..5&v=today|time`, `r4.html?…` or `r5.html?…` in a browser
(light/dark follows the system), or run `docs/mockups/render.sh [all|r4|gif|r5|gif5]` to regenerate `png/`
(1280x800) and `gif/` (25 fps, 720-800 px wide). Needs Chrome, and ffmpeg for the GIFs.
