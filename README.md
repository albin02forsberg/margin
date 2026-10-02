<img src="assets/icon.svg" width="96" align="right" alt="">

# Margin

Notes, tasks, a journal and time tracking in one fast desktop app (Linux,
macOS, Windows). Notes are plain org files, so they stay readable anywhere;
the editor has vim keys.

- **Notes** — org documents that render like a page: links, backlinks,
  unlinked mentions and a local note graph, tables with formulas, code blocks, inline images, folding.
- **Tasks** — Today and All tasks views, schedules, deadlines, priorities,
  tags, repeaters; capture from anywhere with a global shortcut.
- **Journal** — a daily note, one keypress away.
- **Time** — a timeclock with breaks, projects, flex balance and reports, in
  the window or the tray.
- **Stays in sync** with Emacs, git or any sync tool editing the same files,
  and updates itself from GitHub releases.

## Install

Download the latest build from
[Releases](https://github.com/albin02forsberg/margin/releases/latest):

- **Linux** — `.AppImage` (updates itself; `chmod +x` and run), or `.deb` / `.rpm`
- **macOS** — `.dmg` (universal). Not signed yet: right-click → Open the first time.
- **Windows** — `-setup.exe`. Not signed yet: SmartScreen → More info → Run anyway.

## Build from source

```bash
npm install
npm run tauri dev      # run
npm run tauri build    # release bundle for the current OS
cd src-tauri && cargo test
```

## Tests

`npm test` and `cargo test` cover the parsers and engines. `npm run test:e2e`
builds a debug binary (identifier `dev.albin.margin.e2e`, so it never touches
your running copy or settings) and drives the real app through
[tauri-driver](https://v2.tauri.app/develop/tests/webdriver/) against fixture
notes in a temp dir. It needs Linux with `tauri-driver` (`cargo install
tauri-driver --locked`) and `WebKitWebDriver` (`webkit2gtk-driver` on
Debian/Ubuntu); use `xvfb-run -a npm run test:e2e` without a display. CI runs it
in the `e2e` job.

CI runs the checks and tests on every push and PR. To release, bump the
version in `src-tauri/tauri.conf.json`, then push a tag (`git tag v0.2.0 &&
git push --tags`): installers for all three OSes land in a draft GitHub release.
Publishing it makes installed copies offer the update on their next start
(or `Space f u`). Release builds are signed for the updater with the
`TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` repo secrets.

## Staying in sync

Everything stays in sync on its own: notes save shortly after you stop typing,
and any change to the notes or time folders — from the app, Emacs or a sync
tool — refreshes open tabs, task lists, links, reports and the timer. If a file
changes underneath unsaved edits, non-overlapping changes are merged; real
conflicts are flagged (`:w` keeps yours, `Space f r` reloads).

## Getting around

- **Ctrl+K** — command palette: every action, with its shortcut.
- **Space** (vim normal mode, or in any list view) — the same actions as a menu.
- Sidebar: **Today** (Ctrl+1), **Tasks** (Ctrl+2), **Inbox** (Ctrl+3),
  **Journal** (Ctrl+J), **Time** (Ctrl+4), your saved searches, recent notes.
  Toggle with Ctrl+\\.

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

Completing a repeating task logs `- State "DONE" from "TODO" [date time]`
under it and sets `:LAST_REPEAT:`, as org does. Give it `:STYLE: habit` in its
properties and Today (and Tasks) shows a 21-day consistency bar and how many
completions in a row were on time, like org-habit. Past due days follow the
repeater: `+` steps back from the scheduled date, `++` stays on its grid, `.+`
counts from each completion. With a max (`.+2d/3d`) the days between min and
max show amber (due, not yet overdue), after it red:

```org
* TODO Run
SCHEDULED: <2026-10-02 Fri .+1d>
:PROPERTIES:
:STYLE: habit
:END:
```

### Saved searches

"Search tasks…" (`Space v f`) lists the open tasks matching a query, with the
same keys. Terms are space-separated and must all match; `-` negates one:

| | |
|---|---|
| `todo:NEXT` | Status; a done status (`todo:DONE`) searches done tasks too |
| `tag:work`, `-tag:home` | Has / hasn't the tag |
| `pri:A` | Priority |
| `file:inbox` | File name contains |
| `due:today`, `due:<=+7d`, `scheduled:<=fri` | Date compared with `<` `<=` `>` `>=` (none: on that day); any date input works |
| `due:none`, `scheduled:any` | Has no / has a date |
| other words, `"a phrase"` | Title contains |

`<` and `>` are strict: `due:<+7d` leaves out day 7, so `<=` is usually what
you want.

"Save this search as a view…" (`Space v v`, from a search tab) adds it to the
sidebar; it's kept in settings, where you can rename or edit it:

```toml
[[views]]
name = "Work this week"
query = "tag:work due:<=+7d"
```

### Capture templates

`Space c` (or "Capture with template…" in Ctrl+K) picks a template, asks its
fields and files the entry, then opens it with the cursor at `%?`. Undo with `u`
in the task views. Define them in settings:

```toml
[[templates]]
key = "m"
name = "Meeting notes"
file = "meetings.org"   # under the notes folder; strftime ok; default: inbox
heading = "Meetings"    # optional: file under this heading, created if missing
body = "* %^{Title} :meeting:\n%U\n- %?"
```

`%t`/`%T` date / date+time (`<2026-10-02 Fri 14:30>`), `%u`/`%U` the inactive
`[…]` forms, `%^{Prompt}` asks (repeat a name to reuse the answer), `%i` the
selected text (extra lines indented like its line), `%?` cursor, `%%` a literal `%`.

Or keep each template as a file in `notes/templates/` (`templates_dir` in
settings); they're listed after the settings ones and left out of the agenda,
tasks and search:

```org
#+title: Meeting notes
#+key: m
#+file: meetings.org
#+heading: Meetings
* %^{Title} :meeting:
%U
- %?
```

All `#+` lines are optional: the key defaults to the file name's first letter,
the name to the file name, the target to the inbox. The rest is the body.

Quick capture (Super+Shift+N) asks "Task" or a template when you have any; a
template captured with the window hidden is filed without opening it.

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
| Space n b | Notes linking here, plus unlinked mentions of this note's title (**Link** turns one into an `[[id:]]` link; Space n u undoes) |
| Space n g | Graph of notes within two links of this one; click a note to open it |

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

Drop files onto a note, or paste an image (saved as `pasted-YYYYMMDD-HHMMSS.png`),
and Margin copies them into `attachments/<note name>/` in your notes folder and
inserts a relative `[[file:…]]` link, so notes stay portable. Same-named files get
`-1`, `-2` suffixes; set `attachments_dir` to use another folder.
**Clean up unused attachments…** in the palette lists files in that folder no note
links to and moves the ones you pick to `attachments/.trash/` (nothing is deleted).

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
`i` start, `p` pause, `r` resume, `c` switch project, `o` stop, `e` export CSV,
`E` export a report as HTML/PDF.
Space t … has every timeclock command (same letters as the old Emacs menu).

The tray icon shows what you're tracking and for how long, and has start,
break, resume, switch and stop. Closing the window while a timer runs hides it
to the tray (`close_to_tray = false` in settings to quit instead).

Coming back after 10 minutes idle (or asleep) with a timer running asks whether
to keep that time, discard it (clocked out when you left, back in when you
returned) or discard it and stop. `idle_threshold_minutes` sets the time, 0
turns it off. On Wayland this uses the compositor's idle notifications
(KDE Plasma 6, Sway/wlroots, GNOME 46+), and the threshold applies after a restart;
elsewhere only sleep and suspend are noticed.

## Export

`Space e` exports the open note (including unsaved edits) as HTML (`e h`),
Markdown (`e m`) or PDF (`e p`), and the time report for the last 7 days, this
month or last month as HTML or PDF (`e t`, or `E` in the Time view). Files go to
`export_dir` (default `~/Desktop`). PDF writes the HTML page and opens it in your
browser with the print dialog up — choose "Save as PDF" there. Property drawers,
planning lines, comments and `#+` keywords other than the title are left out;
`id:` links become plain text. Images are embedded in the HTML, so the page
works on its own; bare URLs become links, and `term :: text` lists and verse
blocks keep their shape.

## Reminders

Timed entries (`SCHEDULED: <… 14:00>`, deadlines with a time, appointments)
pop up a desktop notification 10 minutes before; date-only deadlines remind you
once a day from the day before. Clicking one opens the task (Linux; elsewhere
notifications are display-only for now). Settings: `reminders`, `remind_before_minutes`,
`deadline_warning_days`.

## Calendar

Set `calendar_file` (e.g. `"~/Sync/margin.ics"`) and Margin keeps an iCalendar
file of open tasks' schedules, deadlines and appointments up to date, with
repeaters. Subscribe to it from your calendar app (or a synced copy of it).
Timed entries show as one hour; the rest are all-day.

## Settings

Ctrl+, opens `config.toml` (in the OS config dir): notes folder (`~/notes`),
time data folder (`~/timeclock`), inbox file, journal folder, task statuses,
profiles and expected daily hours. Old Emacs timeclock data can be imported from
the Time view.
