# tool

Notes, tasks, a journal and time tracking in one fast desktop app (Linux,
macOS, Windows). Notes are plain org files, so they stay readable anywhere;
the editor has vim keys.

```bash
npm install
npm run tauri dev      # run
npm run tauri build    # release bundle for the current OS
cd src-tauri && cargo test
```

## Getting around

- **Ctrl+K** — command palette: every action, with its shortcut.
- **Space** (vim normal mode, or in any list view) — the same actions as a menu.
- Sidebar: **Today** (Ctrl+1), **Tasks** (Ctrl+2), **Inbox** (Ctrl+3),
  **Journal** (Ctrl+J), **Time** (Ctrl+4), recent notes. Toggle with Ctrl+\\.

| | |
|---|---|
| Ctrl+N | New task (title, when, due, priority, tags, file) |
| Ctrl+P | Open a note — or type a title to create one |
| Ctrl+Shift+N | New note |
| Ctrl+Shift+F | Search all notes as you type |
| Ctrl+L | Insert a link to another note |
| Ctrl+S / `:w` | Save (also saves automatically) |

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

## Time tracking

The Time view shows what you're tracking, today's sessions (click ✎ to edit),
the week's hours against expected, flex balance and per-project totals. Keys:
`i` start, `p` pause, `r` resume, `c` switch project, `o` stop, `e` export CSV.
Space t … has every timeclock command (same letters as the old Emacs menu).

## Settings

Ctrl+, opens `config.toml` (in the OS config dir): notes folder (`~/notes`),
time data folder (`~/timeclock`), inbox file, journal folder, task statuses,
profiles and expected daily hours. Old Emacs timeclock data can be imported from
the Time view.
