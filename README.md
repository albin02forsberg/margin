# Margin

Notes, tasks, a journal and time tracking in one fast desktop app (Linux,
macOS, Windows). Notes are plain org files, so they stay readable anywhere;
the editor has vim keys.

```bash
npm install
npm run tauri dev      # run
npm run tauri build    # release bundle for the current OS
cd src-tauri && cargo test
```

CI runs the checks and tests on every push and PR. To release, bump the
version in `src-tauri/tauri.conf.json`, then push a tag (`git tag v0.2.0 &&
git push --tags`): installers for all three OSes land in a draft GitHub release.
Publishing it makes installed copies offer the update on their next start
(or `Space f u`). Release builds are signed for the updater with the
`TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` repo secrets.

Everything stays in sync on its own: notes save shortly after you stop typing,
and any change to the notes or time folders — from the app, Emacs or a sync
tool — refreshes open tabs, task lists, links, reports and the timer. If a file
changes underneath unsaved edits, non-overlapping changes are merged; real
conflicts are flagged (`:w` keeps yours, `Space f r` reloads).

## Getting around

- **Ctrl+K** — command palette: every action, with its shortcut.
- **Space** (vim normal mode, or in any list view) — the same actions as a menu.
- Sidebar: **Today** (Ctrl+1), **Tasks** (Ctrl+2), **Inbox** (Ctrl+3),
  **Journal** (Ctrl+J), **Time** (Ctrl+4), recent notes. Toggle with Ctrl+\\.

| | |
|---|---|
| Ctrl+N | New task (title, when, due, priority, tags, file) |
| Super+Shift+N | Quick capture from anywhere, even with the window hidden (`capture_shortcut` in settings) |
| Ctrl+P | Open a note — or type a title to create one |
| Ctrl+Shift+N | New note |
| Ctrl+Shift+F | Search all notes as you type |
| Ctrl+L | Insert a link to another note |
| Ctrl+S / `:w` | Save (also saves automatically) |

If the shortcut doesn't fire (common on Wayland), bind `margin --capture` to a
key in your desktop's settings instead: it opens the same form in the running app.

## Tasks (Today / Tasks views)

`j`/`k` move, `x` done, `s` schedule, `d` due date, `p` priority, `t` status,
`#` tags, `m` move to another file/heading, `A` archive, `I` track time,
`Enter` open, `<`/`>` reschedule a day, `+`/`-` priority, `n` new task, `u` undo,
`v` day/week, `h`/`l` previous/next, `.` today, `/` filter, `?` all keys.
Click the checkbox to complete a task.

## Editing notes

Vim keys everywhere. On top of that:

| | |
|---|---|
| Tab / Shift+Tab | Fold heading / fold everything |
| Alt+Enter | New heading, or next list item |
| Alt+Shift+Enter | New task heading |
| Alt+H / Alt+L (+Shift) | Promote / demote heading (with children) |
| Alt+J / Alt+K | Move heading down / up |
| Shift+← / → | Change task status, or move a date by a day |
| Shift+↑ / ↓ | Change priority, or the date part under the cursor |
| Enter, gf, Ctrl+click | Follow a link |
| click `[ ]` | Toggle a checkbox |
| Space x … | Task commands for the heading at the cursor |

Dates accept `today`, `tomorrow`, `fri`, `+3d`, `-1w`, `12-24`,
`2026-12-24 14:00`; the input shows what it understood.

## Writing in org

Notes render like a document: markup (stars, link brackets, `*bold*` markers,
checkbox brackets, block delimiters) is hidden except on the line you're on, so
the file stays plain org text. Prose uses a proportional font; tables, code and
dates stay monospace (`Space v m` switches everything to monospace).

| | |
|---|---|
| Tab / Shift+Tab in a table | Align and move to the next / previous cell (adds rows at the end) |
| Enter in a table (insert mode) | Same column, next row |
| Alt+H/J/K/L in a table | Move column / row |
| Alt+Shift+H/L, Alt+Shift+J/K | Delete / insert column, insert / delete row |
| `\|-` then Tab | Expands into a separator line |
| `<s`, `<q`, `<e`, `<v` + Tab | Code, quote, example, verse block |
| Ctrl+Shift+O | Go to a heading in this note |
| Space i … | Insert table, code block, quote, link, date, rule |
| Space T … | Table: rows, columns, separator, sort, align |
| `[/]` or `[%]` in a heading | Progress cookie, updated when you tick checkboxes or finish child tasks |

Image links (`[[file:pic.png]]`, `[[https://…/pic.jpg]]`) show the image inline.

Code blocks (`#+begin_src python` …) are syntax highlighted for any language
CodeMirror knows (python, rust, js/ts, sh, sql, go, java, c, html, css, yaml, …).

### Table formulas

Put formulas on a `#+TBLFM:` line under the table, separated by `::` — or type
`=expr` (whole column) or `:=expr` (just this cell) into a cell and press Tab.
They recalculate as you Tab/Enter through the table, on `Space T r`, or with Tab
on the `#+TBLFM` line.

```org
| Item   | Qty | Price | Total |
|--------+-----+-------+-------|
| Coffee |   3 |   2.5 |   7.5 |
| Paper  |  10 |   0.2 |     2 |
|--------+-----+-------+-------|
| Sum    |  13 |       |  9.50 |
#+TBLFM: $4=$2*$3::@>$4=vsum(@I..@II);%.2f::@>$2=vsum(@I..@II)
```

References: `$3` column, `@2` row, `@2$3` field, relative `@-1` / `$+1`, `@<` /
`@>` first / last row, `@I` / `@II` separator lines, ranges `@2..@-1` or
`@2$1..@4$3`. Operators `+ - * / ^`; functions `vsum vmean vmin vmax vcount
vmedian abs round floor ceil sqrt exp ln`; format with `;%.2f` or `;%d`.

## Time tracking

The Time view shows what you're tracking, today's sessions (click ✎ to edit),
the week's hours against expected, flex balance and per-project totals. Keys:
`i` start, `p` pause, `r` resume, `c` switch project, `o` stop, `e` export CSV.
Space t … has every timeclock command (same letters as the old Emacs menu).

The tray icon shows what you're tracking and for how long, and has start,
break, resume, switch and stop. Closing the window while a timer runs hides it
to the tray (`close_to_tray = false` in settings to quit instead).

## Settings

Ctrl+, opens `config.toml` (in the OS config dir): notes folder (`~/notes`),
time data folder (`~/timeclock`), inbox file, journal folder, task statuses,
profiles and expected daily hours. Old Emacs timeclock data can be imported from
the Time view.
