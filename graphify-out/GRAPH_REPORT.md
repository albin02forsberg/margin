# Graph Report - margin  (2026-10-22)

## Corpus Check
- 150 files · ~1,033,478 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 16 file(s) not represented in the graph (top: .css 8, (none) 4, .plist 1)

## Summary
- 2172 nodes · 5030 edges · 139 communities (105 shown, 34 thin omitted)
- Extraction: 97% EXTRACTED · 2% INFERRED · 0% AMBIGUOUS · INFERRED: 118 edges (avg confidence: 0.87)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `a8af51a1`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- screenshots.ts
- timeclock.rs
- AppHandle
- editor.ts
- watcher.rs
- orgtable.ts
- tauri.conf.json
- package.json
- compilerOptions
- lib/FormulaDialog.svelte
- Calendar
- join
- GitHub Releases
- scripts
- cfg
- Notes
- AppImage
- lib/TaskDialog.svelte
- lib/Menu.svelte
- default.json
- media.rs
- activity.rs
- export.rs
- State
- Sync
- +layout.ts
- build.rs
- +page.svelte
- margin
- ai.rs
- level
- ics.rs
- generate
- config.rs
- CLAUDE.md
- State
- quit.rs
- day_prompt
- release/SKILL.md
- paths.test.ts
- verify-all/SKILL.md
- chatctx.ts
- App Icon
- .new
- Configuration
- DMG
- Tauri Global Shortcut Plugin
- Windows Installer
- Journal
- Value
- Node.js
- Tauri Notification Plugin
- Tauri Single Instance Plugin
- Tauri Action
- ollama
- Time Tracking
- TypeScript
- Event
- setup.ts
- backup.rs
- r4.js
- Tc
- triager.md
- extension.js
- main
- session.ts
- lib/Agenda.svelte
- b-variants.js
- notes.rs
- download
- load
- R
- chrono
- NaiveDate
- merge.test.ts
- lib.rs
- Changelog
- svelte
- changelog.sh
- ref_node_assert
- agendatext.ts
- .parse
- Calendar
- dependencies
- itemSwap
- ship/SKILL.md
- retrieve.rs
- mock.js
- devDependencies
- r5.js
- autopilot-ui.mjs
- chips.test.ts
- frag
- render.sh
- pills4
- inp
- boardCols
- field
- gfilters
- graphCard
- keycap
- rail4
- README.md
- set_keyword
- decorate
- wlr.rs
- build.sh
- emphasis.test.ts
- projects.ts
- org.rs
- settings.ts
- createState
- App
- links.ts
- bugreport.test.ts
- implementer.md
- remind.rs
- footnote.ts
- clock.ts
- NOTE_WRITES
- Kw
- media-size.ts
- releaser.md
- activity.test.ts
- tray.rs
- board.test.ts
- narrow.ts
- bulk.ts
- super
- autopilot-forever.sh
- whatsnew.test.ts
- autopilot/SKILL.md
- OrgFile
- gh

## God Nodes (most connected - your core abstractions)
1. `App` - 116 edges
2. `cfg()` - 50 edges
3. `Tc` - 42 edges
4. `OrgFile` - 38 edges
5. `Kw` - 34 edges
6. `kw()` - 28 edges
7. `to_lines()` - 22 edges
8. `join()` - 22 edges
9. `w()` - 21 edges
10. `reload_config()` - 20 edges

## Surprising Connections (you probably didn't know these)
- `House rules` --references--> `main()`  [INFERRED]
  .claude/agents/implementer.md → scripts/coverage-ratchet.ts
- `1. Gate (all must hold, else report which failed and stop)` --references--> `main()`  [INFERRED]
  .claude/agents/releaser.md → scripts/coverage-ratchet.ts
- `Run` --references--> `main()`  [INFERRED]
  .claude/agents/scout.md → scripts/coverage-ratchet.ts
- `Repo Conventions` --references--> `main()`  [INFERRED]
  CLAUDE.md → scripts/coverage-ratchet.ts
- `3. Review each report` --references--> `main()`  [INFERRED]
  .claude/skills/ship/SKILL.md → scripts/coverage-ratchet.ts

## Import Cycles
- 1-file cycle: `src-tauri/src/ai.rs -> src-tauri/src/ai.rs`

## Communities (139 total, 34 thin omitted)

### Community 0 - "screenshots.ts"
Cohesion: 0.12
Nodes (28): ADR-0007, ADR-0008, button(), clip(), clips, config, day(), fixtures() (+20 more)

### Community 1 - "timeclock.rs"
Cohesion: 0.09
Nodes (30): adjust_start_refuses_to_overlap_the_previous_session(), clock_flow_on_disk(), csv_quotes_and_comma(), csv_says_why_time_off_wrote_no_row(), csv_skips_zero_rows(), csv_time_off_rows(), d(), dedupe_lines() (+22 more)

### Community 2 - "AppHandle"
Cohesion: 0.14
Nodes (24): ai_chat(), ai_download(), ai_draft(), ai_model_delete(), ai_models(), ai_set(), apply_config(), apply_spellcheck() (+16 more)

### Community 3 - "editor.ts"
Cohesion: 0.05
Nodes (40): Block, BULLETS, clicks, convertToTable(), findTable(), hide, hooks, inTable() (+32 more)

### Community 5 - "watcher.rs"
Cohesion: 0.13
Nodes (27): active(), active_window(), AFK_MINS, append(), clean(), flush(), flush_one(), flush_writes_the_open_record_once() (+19 more)

### Community 6 - "orgtable.ts"
Cohesion: 0.09
Nodes (42): align(), applyFormulas(), cells(), colRef(), create(), deleteCol(), deleteRow(), done() (+34 more)

### Community 7 - "tauri.conf.json"
Cohesion: 0.06
Nodes (34): app, security, windows, enable, scope, build, beforeBuildCommand, beforeDevCommand (+26 more)

### Community 8 - "package.json"
Cohesion: 0.07
Nodes (27): description, engines, node, license, name, type, version, @codemirror/commands (+19 more)

### Community 9 - "compilerOptions"
Cohesion: 0.14
Nodes (13): ./.svelte-kit/tsconfig.json, compilerOptions, allowImportingTsExtensions, allowJs, checkJs, esModuleInterop, forceConsistentCasingInFileNames, moduleResolution (+5 more)

### Community 10 - "lib/FormulaDialog.svelte"
Cohesion: 0.10
Nodes (21): active, col, commit(), draft, error, expr, fm, fmt (+13 more)

### Community 11 - "Calendar"
Cohesion: 0.67
Nodes (3): Calendar, iCalendar, Tasks

### Community 12 - "join"
Cohesion: 0.14
Nodes (42): add_note(), append_archived(), archive(), archive_done(), archive_many(), capture_insert(), check_target(), clone_shift() (+34 more)

### Community 14 - "scripts"
Cohesion: 0.13
Nodes (15): scripts, build, check, check:watch, coverage, coverage:ratchet, dev, e2e:build (+7 more)

### Community 15 - "cfg"
Cohesion: 0.11
Nodes (20): ai_excluded(), ai_note_prompt(), attach_bytes(), attach_file(), attach_in(), attach_in_numbers_clashes_retries_races_and_rolls_back(), capture_templates(), cfg() (+12 more)

### Community 16 - "Notes"
Cohesion: 0.29
Nodes (7): Links and Backlinks, Notes, Org Format, Search, Syntax Highlighting, Table Formulas, Vim Keys

### Community 18 - "lib/TaskDialog.svelte"
Cohesion: 0.16
Nodes (12): svelte, close(), preview, active, close(), closed(), duePreview, error (+4 more)

### Community 19 - "lib/Menu.svelte"
Cohesion: 0.29
Nodes (4): active, node, show(), filter()

### Community 20 - "default.json"
Cohesion: 0.33
Nodes (5): description, identifier, permissions, $schema, windows

### Community 21 - "media.rs"
Cohesion: 0.07
Nodes (32): clean(), images_share_the_note_folders_images_dir(), is_image(), LINK, names_and_links(), normalize(), target(), unused() (+24 more)

### Community 22 - "activity.rs"
Cohesion: 0.07
Nodes (48): AwEvent, browser_urls(), Bucket, Buckets, CEST, COMMON, cut(), dismiss() (+40 more)

### Community 23 - "export.rs"
Cohesion: 0.09
Nodes (48): BLOCK, Heading, List, Para, Pre, Quote, Rule, Table (+40 more)

### Community 25 - "Sync"
Cohesion: 0.67
Nodes (3): Emacs, Git, Sync

### Community 29 - "+page.svelte"
Cohesion: 0.05
Nodes (7): cmd(), edit(), done(), effort(), note(), priority(), state()

### Community 31 - "ai.rs"
Cohesion: 0.09
Nodes (11): CHAT_WAIT, CTX, IDLE, LOADED, MAX_CHAT, MAX_PROMPT, READ_TIMEOUT, RUNS (+3 more)

### Community 32 - "level"
Cohesion: 0.17
Nodes (28): curLine(), drawerRange(), emphasis(), expandTemplate(), headings(), insertBlock(), insertMode(), insertTable() (+20 more)

### Community 33 - "ics.rs"
Cohesion: 0.16
Nodes (18): builds_feed(), CALENDAR, calendar_from_a_url(), CALENDAR_TTL, date(), event(), events_on(), feed() (+10 more)

### Community 34 - "generate"
Cohesion: 0.15
Nodes (15): Backend, Embedded, Ollama, body(), chat(), chat_messages(), chats(), clean() (+7 more)

### Community 35 - "config.rs"
Cohesion: 0.08
Nodes (28): add_view(), add_views(), atomic_writes(), Config, defaults_round_trip(), expand(), extra_days_off(), public_holidays() (+20 more)

### Community 36 - "CLAUDE.md"
Cohesion: 0.20
Nodes (8): Build & Test, Code Style, Project, Release & CI/CD, Repo Conventions, Tauri-Specific, Testing, Tools

### Community 37 - "State"
Cohesion: 0.11
Nodes (39): activity_suggestions(), agenda(), ai_retrieve(), capture_insert(), capture_template(), expected_hours_for(), export_html_fragment(), export_linked() (+31 more)

### Community 38 - "quit.rs"
Cohesion: 0.13
Nodes (9): app_menu(), ask(), ASKED, BUSY, GO, IDLE, quit_reply(), SEQ (+1 more)

### Community 39 - "day_prompt"
Cohesion: 0.19
Nodes (9): capped(), day_prompt(), dt(), embedded_drafts(), kept(), note_prompt(), prompts(), t() (+1 more)

### Community 40 - "release/SKILL.md"
Cohesion: 0.40
Nodes (4): Notes, Secrets required (in GitHub repo settings), Usage, What it does

### Community 41 - "paths.test.ts"
Cohesion: 0.35
Nodes (10): ago(), changed(), folderName(), journalNeighbour(), mergeChanged(), noteFolders(), onlyEchoes(), onThisDay() (+2 more)

### Community 42 - "verify-all/SKILL.md"
Cohesion: 0.50
Nodes (3): Usage, What it does, When to use

### Community 43 - "chatctx.ts"
Cohesion: 0.31
Nodes (5): chatContext(), cleanChat(), enc, level(), title()

### Community 45 - ".new"
Cohesion: 0.16
Nodes (23): add_past_sessions(), apply_carry(), breaks_accumulate_and_flex(), csv(), csv_field(), daily_report(), export_code(), import_timelog() (+15 more)

### Community 52 - "Value"
Cohesion: 0.17
Nodes (10): activity_dismiss(), activity_rules_list(), context_of(), dismissed(), keys(), org_context_falls_back_to_the_file(), read_or_empty(), status_json() (+2 more)

### Community 57 - "ollama"
Cohesion: 0.18
Nodes (11): available_gb(), cancel_draft(), cancel_hits_only_the_running_draft(), free_gb(), ollama(), ollama_chat(), parse(), read_chat() (+3 more)

### Community 60 - "Event"
Cohesion: 0.16
Nodes (12): append_diary(), clock_out_earlier_refuses_before_the_start(), edit_session_bounds(), Event, Break, In, Out, hours() (+4 more)

### Community 61 - "setup.ts"
Cohesion: 0.13
Nodes (27): session(), answer(), app, browser, close(), Dirs, fileMatches(), isoDay() (+19 more)

### Community 62 - "backup.rs"
Cohesion: 0.12
Nodes (36): active_profile_is_untracked_and_survives_the_pull(), activity_files_union_merge(), attributes_are_appended_once(), BACKUP, backup_syncs_the_data_repo(), clean_env(), commit(), commit_all() (+28 more)

### Community 63 - "r4.js"
Cohesion: 0.05
Nodes (26): agendaDays, backlinks, board, byTag, DEMO, DEMOS, done3, doneWeek (+18 more)

### Community 64 - "Tc"
Cohesion: 0.18
Nodes (6): leave_on_disk(), malformed_projects_are_reported(), read_or_empty(), rewrite_with(), Tc, unreadable_files_are_not_overwritten()

### Community 65 - "triager.md"
Cohesion: 0.40
Nodes (4): Pass, Report (ends your run), Rules for what you post, Watch mode (only when asked)

### Community 67 - "main"
Cohesion: 0.31
Nodes (9): Baseline, Count, main(), parseLcov(), pct(), regressions(), Report, Suite (+1 more)

### Community 68 - "session.ts"
Cohesion: 0.25
Nodes (7): Review, Rules, Entry, KINDS, ok(), parseSession(), Session

### Community 69 - "lib/Agenda.svelte"
Cohesion: 0.18
Nodes (16): copyList(), dayInput(), goTo(), key(), op(), pointer(), refresh(), setBoard() (+8 more)

### Community 70 - "b-variants.js"
Cohesion: 0.14
Nodes (29): b1(), b2(), b3(), b4(), b5(), bars(), blocks, card() (+21 more)

### Community 71 - "notes.rs"
Cohesion: 0.06
Nodes (50): adjacency(), ANY_LINK, backlinks(), ensure_id(), ensures_ids(), files(), gen_id(), Graph (+42 more)

### Community 72 - "download"
Cohesion: 0.25
Nodes (7): chat_http(), download(), download_resumes_and_checks(), download_stall_times_out(), part(), serve(), sha256_file()

### Community 73 - "load"
Cohesion: 0.25
Nodes (6): CANCELLED, load(), run(), RUNNING, TICKETS, whole()

### Community 74 - "R"
Cohesion: 0.07
Nodes (46): ai_day_prompt(), append_journal_template(), bad_meeting_pattern_and_calendar_only_warn(), capture_file(), capture_file_fills_the_pattern_under_notes(), capture_path(), check_in(), csv_dest() (+38 more)

### Community 76 - "NaiveDate"
Cohesion: 0.19
Nodes (15): add_interval(), Habit, move_planning(), norm_time(), occurrences(), parse_ts(), planning(), planning_ts() (+7 more)

### Community 77 - "merge.test.ts"
Cohesion: 0.42
Nodes (7): applyText(), Change, diffChange(), mergeEdits(), perKey(), apply(), merged()

### Community 78 - "lib.rs"
Cohesion: 0.05
Nodes (34): attachment_files(), attachment_files_skips_trash_and_unused_ignores_linked(), CANCEL, dash_fixture(), dashboard(), dashboard_and_status_show_the_clock(), dashboard_builds_the_week(), dashboard_projects_month() (+26 more)

### Community 79 - "Changelog"
Cohesion: 0.14
Nodes (13): Changelog, Features, Features, Features, Fixes, Fixes, Fixes, Other (+5 more)

### Community 80 - "svelte"
Cohesion: 0.10
Nodes (6): active, close(), [hc, hr], key(), SvelteKit, Vite

### Community 82 - "ref_node_assert"
Cohesion: 0.11
Nodes (15): layout(), Pt, orderNotes(), parsePins(), togglePin(), empty, asUrl(), linkFor() (+7 more)

### Community 83 - "agendatext.ts"
Cohesion: 0.24
Nodes (10): agendaText(), DAYS, heading(), MONTHS, Row, r(), fmtEffort(), overbooked() (+2 more)

### Community 84 - ".parse"
Cohesion: 0.23
Nodes (15): agenda(), agenda_week(), d(), dates(), deadline_warning_cookie(), effort_parses_org_durations(), habits(), overdue_and_move() (+7 more)

### Community 85 - "Calendar"
Cohesion: 0.27
Nodes (14): Calendar, day_done(), day_off_rules(), easter(), expected_hours(), flex(), holidays(), holidays_report() (+6 more)

### Community 86 - "dependencies"
Cohesion: 0.15
Nodes (13): dependencies, @codemirror/commands, @codemirror/language, @codemirror/language-data, @codemirror/search, @codemirror/state, @codemirror/view, @lezer/highlight (+5 more)

### Community 87 - "itemSwap"
Cohesion: 0.50
Nodes (6): indent(), isItem(), itemSwap(), renumber(), fix(), swap()

### Community 89 - "ship/SKILL.md"
Cohesion: 0.25
Nodes (7): 1. Survey, 2. Dispatch, 3. Review each report, 4. Clean up, 5. Report to the owner, Rules, Running it repeatedly

### Community 90 - "retrieve.rs"
Cohesion: 0.11
Nodes (27): APP, app_source(), app_sources(), app_sources_and_mixed_packing(), BUDGET, cost(), cut(), EXCERPT (+19 more)

### Community 91 - "mock.js"
Cohesion: 0.22
Nodes (8): habit, late, nav, q, row(), tags(), today, week

### Community 92 - "devDependencies"
Cohesion: 0.18
Nodes (11): devDependencies, svelte, svelte-check, @sveltejs/adapter-static, @sveltejs/kit, @sveltejs/vite-plugin-svelte, @tauri-apps/cli, @types/node (+3 more)

### Community 93 - "r5.js"
Cohesion: 0.32
Nodes (6): bar(), DEMOS5, layered(), P5, page(), worked

### Community 94 - "autopilot-ui.mjs"
Cohesion: 0.15
Nodes (21): agents, bar(), [bold, dim, red, green, yellow, cyan], checkRotate(), clock(), count(), cut(), describe() (+13 more)

### Community 95 - "chips.test.ts"
Cohesion: 0.33
Nodes (3): Chip, chips(), items

### Community 96 - "frag"
Cohesion: 0.50
Nodes (5): frag(), today1(), today2(), todayHints(), view()

### Community 97 - "render.sh"
Cohesion: 0.80
Nodes (4): gif(), grab(), render.sh script, shot()

### Community 98 - "pills4"
Cohesion: 0.50
Nodes (4): pills4(), shell2(), shell5(), today5()

### Community 99 - "inp"
Cohesion: 0.67
Nodes (3): allProjects, inp(), timeOff

### Community 100 - "boardCols"
Cohesion: 0.67
Nodes (3): boardCols(), kc(), kcol()

### Community 107 - "set_keyword"
Cohesion: 0.31
Nodes (9): at(), closed_stamp(), cycle_and_repeat(), cycle_todo(), fmt_time(), heading_end(), notes(), repeat_with_unclosed_properties_stays_put() (+1 more)

### Community 108 - "decorate"
Cohesion: 0.13
Nodes (9): codeTokens(), decorate(), Glyph, imageSrc(), Img, inline(), langFor(), mark() (+1 more)

### Community 109 - "wlr.rs"
Cohesion: 0.14
Nodes (12): active(), apply(), Ev, App, Closed, State, Title, focused() (+4 more)

### Community 111 - "emphasis.test.ts"
Cohesion: 0.60
Nodes (3): EMPH, run(), toggleEmphasis()

### Community 112 - "projects.ts"
Cohesion: 0.22
Nodes (13): budgetUsage(), Card, cards(), merge(), period(), Project, ROUNDINGS, Row (+5 more)

### Community 113 - "org.rs"
Cohesion: 0.13
Nodes (25): ALIAS, archive_done_moves_top_most_done_subtrees_in_order(), capture_template(), CLOSED, COOKIE, CURSOR, DOC, EFFORT (+17 more)

### Community 114 - "settings.ts"
Cohesion: 0.18
Nodes (17): CATEGORIES, changes(), checkTemplates(), checkViews(), Field, Form, fromForm(), GROUPS (+9 more)

### Community 115 - "createState"
Cohesion: 0.20
Nodes (7): createState(), Num, numExt(), orgShiftTab(), setSpellcheck(), spellExt(), vimInsert()

### Community 116 - "App"
Cohesion: 0.10
Nodes (34): activity_learn(), activity_rules_delete(), App, backup_now(), day_done_now(), export_report(), file_exists(), in_notes() (+26 more)

### Community 117 - "links.ts"
Cohesion: 0.51
Nodes (8): bareTitle(), esc(), findTarget(), head(), linkHeadings(), norm(), retitlePlan(), splitTarget()

### Community 118 - "bugreport.test.ts"
Cohesion: 0.48
Nodes (5): BugEnv, bugReportUrl(), isError(), ISSUES, osName()

### Community 119 - "implementer.md"
Cohesion: 0.40
Nodes (4): Finish, House rules, Report (your final message), Start

### Community 120 - "remind.rs"
Cohesion: 0.20
Nodes (5): notify(), once_a_day(), picks_due_reminders(), start(), tick()

### Community 121 - "footnote.ts"
Cohesion: 0.29
Nodes (8): followFootnote(), insertFootnote(), recordJump(), addFootnote(), FN, footnoteTarget(), nextLabel(), add()

### Community 122 - "clock.ts"
Cohesion: 0.31
Nodes (7): dayTime(), inactiveStamp(), naive(), p2(), parseWhen(), WeekDay, worked()

### Community 124 - "Kw"
Cohesion: 0.18
Nodes (13): cycle_priority(), edit_heading(), heading(), Kw, parse(), parse_effort(), Parts, query() (+5 more)

### Community 125 - "media-size.ts"
Cohesion: 0.27
Nodes (7): DIRS, EACH, GIFS, mb(), problems(), scan(), TOTAL

### Community 126 - "releaser.md"
Cohesion: 0.33
Nodes (5): 1. Gate (all must hold, else report which failed and stop), 2. Bump PR, 3. Tag, build, publish, Report, Rules

### Community 127 - "activity.test.ts"
Cohesion: 0.32
Nodes (6): at(), buckets, day, ev(), events, y

### Community 128 - "tray.rs"
Cohesion: 0.27
Nodes (5): refresh(), setup(), show(), tracking(), Tray

### Community 129 - "board.test.ts"
Cohesion: 0.32
Nodes (5): Column, columns(), moveTo(), done, todo

### Community 130 - "narrow.ts"
Cohesion: 0.21
Nodes (11): @codemirror/state, @codemirror/view, hidden(), hideBlock, Narrow, narrowedTo(), narrowEffect, narrowField (+3 more)

### Community 134 - "bulk.ts"
Cohesion: 0.39
Nodes (7): Loc, Mark, markKey(), orderForEdit(), outermost(), reresolve(), tagChange

### Community 137 - "super"
Cohesion: 0.20
Nodes (3): source(), start(), step()

### Community 138 - "autopilot-forever.sh"
Cohesion: 0.23
Nodes (13): AUTOPILOT_HEARTBEAT, AUTOPILOT_LIMITS, AUTOPILOT_MAX, AUTOPILOT_ROTATE, AUTOPILOT_TAIL, countdown(), disk_gate(), limit_summary() (+5 more)

### Community 139 - "whatsnew.test.ts"
Cohesion: 0.43
Nodes (4): Notes, notesText(), notesTitle(), notesToShow()

### Community 140 - "autopilot/SKILL.md"
Cohesion: 0.40
Nodes (4): Idle, Loop, Rules, Stop

### Community 141 - "OrgFile"
Cohesion: 0.17
Nodes (12): open_entries(), all_tags(), Cache, Headline, Item, org_paths(), OrgFile, overdue_scheduled() (+4 more)

### Community 142 - "gh"
Cohesion: 0.18
Nodes (8): Agent Workflow, Rules, Run, Rules, Run, Permission Mode, gh(), pollUp()

## Knowledge Gaps
- **383 isolated node(s):** `changelog.sh script`, `q`, `sched`, `late`, `habit` (+378 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 657 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **34 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `start()` connect `setup.ts` to `screenshots.ts`, `+page.svelte`, `svelte`, `lib/Agenda.svelte`?**
  _High betweenness centrality (0.129) - this node is a cross-community bridge._
- **Why does `line()` connect `level` to `svelte`, `editor.ts`, `decorate`, `+page.svelte`?**
  _High betweenness centrality (0.048) - this node is a cross-community bridge._
- **Why does `tableFormulas()` connect `editor.ts` to `+page.svelte`?**
  _High betweenness centrality (0.031) - this node is a cross-community bridge._
- **What connects `changelog.sh script`, `q`, `sched` to the rest of the system?**
  _383 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `screenshots.ts` be split into smaller, more focused modules?**
  _Cohesion score 0.11954022988505747 - nodes in this community are weakly interconnected._
- **Should `timeclock.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.08974358974358974 - nodes in this community are weakly interconnected._
- **Should `AppHandle` be split into smaller, more focused modules?**
  _Cohesion score 0.13675213675213677 - nodes in this community are weakly interconnected._