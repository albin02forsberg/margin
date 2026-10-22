# Graph Report - margin  (2026-10-22)

## Corpus Check
- 150 files · ~1,031,383 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 16 file(s) not represented in the graph (top: .css 8, (none) 4, .plist 1)

## Summary
- 2150 nodes · 4974 edges · 144 communities (109 shown, 35 thin omitted)
- Extraction: 98% EXTRACTED · 2% INFERRED · 0% AMBIGUOUS · INFERRED: 106 edges (avg confidence: 0.86)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `e2095a9e`
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
- ics.rs
- Notes
- AppImage
- lib/TaskDialog.svelte
- lib/Menu.svelte
- default.json
- media.rs
- activity.rs
- export.rs
- wayland_idle.rs
- Sync
- +layout.ts
- build.rs
- +page.svelte
- margin
- ai.rs
- level
- attach_bytes
- generate
- config.rs
- CLAUDE.md
- kw
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
- svelte
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
- coverage-ratchet.ts
- session.ts
- lib/Agenda.svelte
- b-variants.js
- notes.rs
- download
- load
- R
- ideator.md
- Item
- merge.test.ts
- lib.rs
- Changelog
- lib/Projects.svelte
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
- setopt.test.ts
- decorate
- wlr.rs
- build.sh
- OrgFile
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
- zoom.test.ts
- Kw
- ref_node_fs
- releaser.md
- activity.test.ts
- tray.rs
- board.test.ts
- narrow.ts
- search
- set_keyword
- Regex
- bulk.ts
- donelog.ts
- NOTE_WRITES
- super
- autopilot-forever.sh
- whatsnew.test.ts
- autopilot/SKILL.md
- Cache
- main

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
- `Idle` --references--> `main()`  [INFERRED]
  .claude/skills/autopilot/SKILL.md → scripts/coverage-ratchet.ts
- `3. Review each report` --references--> `main()`  [INFERRED]
  .claude/skills/ship/SKILL.md → scripts/coverage-ratchet.ts
- `Rules` --references--> `main()`  [INFERRED]
  .claude/skills/ship/SKILL.md → scripts/coverage-ratchet.ts

## Import Cycles
- 1-file cycle: `src-tauri/src/ai.rs -> src-tauri/src/ai.rs`

## Communities (144 total, 35 thin omitted)

### Community 0 - "screenshots.ts"
Cohesion: 0.13
Nodes (25): ADR-0007, ADR-0008, button(), clip(), clips, config, day(), fixtures() (+17 more)

### Community 1 - "timeclock.rs"
Cohesion: 0.09
Nodes (30): adjust_start_refuses_to_overlap_the_previous_session(), clock_flow_on_disk(), csv_quotes_and_comma(), csv_says_why_time_off_wrote_no_row(), csv_skips_zero_rows(), csv_time_off_rows(), d(), dedupe_lines() (+22 more)

### Community 2 - "AppHandle"
Cohesion: 0.10
Nodes (30): ai_chat(), ai_download(), ai_draft(), ai_model_delete(), ai_models(), ai_set(), apply_config(), apply_spellcheck() (+22 more)

### Community 3 - "editor.ts"
Cohesion: 0.06
Nodes (37): Block, BULLETS, clicks, convertToTable(), findTable(), hide, hooks, inTable() (+29 more)

### Community 5 - "watcher.rs"
Cohesion: 0.12
Nodes (28): active(), active_window(), AFK_MINS, append(), clean(), exclusion_and_truncation_before_write(), flush(), flush_one() (+20 more)

### Community 6 - "orgtable.ts"
Cohesion: 0.09
Nodes (42): align(), applyFormulas(), cells(), colRef(), create(), deleteCol(), deleteRow(), done() (+34 more)

### Community 7 - "tauri.conf.json"
Cohesion: 0.06
Nodes (34): app, security, windows, enable, scope, build, beforeBuildCommand, beforeDevCommand (+26 more)

### Community 8 - "package.json"
Cohesion: 0.06
Nodes (28): description, engines, node, license, name, type, version, @codemirror/commands (+20 more)

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
Cohesion: 0.16
Nodes (38): add_note(), append_archived(), archive(), archive_done(), archive_many(), capture_insert(), check_target(), clone_shift() (+30 more)

### Community 14 - "scripts"
Cohesion: 0.13
Nodes (15): scripts, build, check, check:watch, coverage, coverage:ratchet, dev, e2e:build (+7 more)

### Community 15 - "ics.rs"
Cohesion: 0.16
Nodes (18): builds_feed(), CALENDAR, calendar_from_a_url(), CALENDAR_TTL, date(), event(), events_on(), feed() (+10 more)

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
Nodes (47): AwEvent, browser_urls(), Bucket, Buckets, CEST, COMMON, cut(), dismiss() (+39 more)

### Community 23 - "export.rs"
Cohesion: 0.09
Nodes (48): BLOCK, Heading, List, Para, Pre, Quote, Rule, Table (+40 more)

### Community 24 - "wayland_idle.rs"
Cohesion: 0.14
Nodes (3): start(), State, wayland_reports_idle()

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
Cohesion: 0.18
Nodes (27): curLine(), drawerRange(), expandTemplate(), headings(), insertBlock(), insertMode(), insertTable(), line() (+19 more)

### Community 33 - "attach_bytes"
Cohesion: 0.40
Nodes (4): attach_bytes(), attach_file(), attach_in(), attach_in_numbers_clashes_retries_races_and_rolls_back()

### Community 34 - "generate"
Cohesion: 0.15
Nodes (15): Backend, Embedded, Ollama, body(), chat(), chat_messages(), chats(), clean() (+7 more)

### Community 35 - "config.rs"
Cohesion: 0.09
Nodes (28): add_view(), add_views(), atomic_writes(), Config, defaults_round_trip(), expand(), extra_days_off(), public_holidays() (+20 more)

### Community 36 - "CLAUDE.md"
Cohesion: 0.20
Nodes (8): Build & Test, Code Style, Permission Mode, Project, Release & CI/CD, Tauri-Specific, Testing, Tools

### Community 37 - "kw"
Cohesion: 0.13
Nodes (25): activity_suggestions(), agenda(), ai_retrieve(), capture_insert(), capture_template(), context_of(), export_html_fragment(), export_linked() (+17 more)

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
Cohesion: 0.43
Nodes (4): chatContext(), cleanChat(), enc, level()

### Community 45 - ".new"
Cohesion: 0.16
Nodes (23): add_past_sessions(), apply_carry(), breaks_accumulate_and_flex(), csv(), csv_field(), daily_report(), export_code(), import_timelog() (+15 more)

### Community 52 - "svelte"
Cohesion: 0.18
Nodes (7): active, close(), [hc, hr], key(), title(), SvelteKit, Vite

### Community 57 - "ollama"
Cohesion: 0.22
Nodes (9): available_gb(), cancel_draft(), cancel_hits_only_the_running_draft(), ollama(), ollama_chat(), parse(), read_chat(), ticket() (+1 more)

### Community 60 - "Event"
Cohesion: 0.16
Nodes (12): append_diary(), clock_out_earlier_refuses_before_the_start(), edit_session_bounds(), Event, Break, In, Out, hours() (+4 more)

### Community 61 - "setup.ts"
Cohesion: 0.14
Nodes (29): session(), answer(), app, browser, clickIn(), close(), diag(), Dirs (+21 more)

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

### Community 67 - "coverage-ratchet.ts"
Cohesion: 0.25
Nodes (8): Baseline, Count, parseLcov(), pct(), regressions(), Report, Suite, summarize()

### Community 68 - "session.ts"
Cohesion: 0.25
Nodes (7): Review, Rules, Entry, KINDS, ok(), parseSession(), Session

### Community 69 - "lib/Agenda.svelte"
Cohesion: 0.29
Nodes (12): copyList(), dayInput(), goTo(), key(), op(), pointer(), refresh(), setBoard() (+4 more)

### Community 70 - "b-variants.js"
Cohesion: 0.14
Nodes (29): b1(), b2(), b3(), b4(), b5(), bars(), blocks, card() (+21 more)

### Community 71 - "notes.rs"
Cohesion: 0.11
Nodes (24): ensure_id(), ensures_ids(), gen_id(), heading_norm(), MAX_NODES, move_note(), move_to(), moves_notes() (+16 more)

### Community 72 - "download"
Cohesion: 0.25
Nodes (7): chat_http(), download(), download_resumes_and_checks(), download_stall_times_out(), part(), serve(), sha256_file()

### Community 73 - "load"
Cohesion: 0.20
Nodes (8): CANCELLED, free_gb(), load(), run(), RUNNING, TICKETS, vm_stat_gb(), whole()

### Community 74 - "R"
Cohesion: 0.07
Nodes (53): activity_dismiss(), append_journal_template(), capture_file(), capture_path(), check_in(), csv_dest(), csv_target(), date() (+45 more)

### Community 76 - "Item"
Cohesion: 0.16
Nodes (20): add_interval(), Habit, Headline, Item, move_planning(), norm_time(), occurrences(), overdue_scheduled() (+12 more)

### Community 77 - "merge.test.ts"
Cohesion: 0.42
Nodes (7): applyText(), Change, diffChange(), mergeEdits(), perKey(), apply(), merged()

### Community 78 - "lib.rs"
Cohesion: 0.05
Nodes (35): attachment_files(), attachment_files_skips_trash_and_unused_ignores_linked(), CANCEL, capture_file_fills_the_pattern_under_notes(), dash_fixture(), dashboard(), dashboard_and_status_show_the_clock(), dashboard_builds_the_week() (+27 more)

### Community 79 - "Changelog"
Cohesion: 0.14
Nodes (13): Changelog, Features, Features, Features, Fixes, Fixes, Fixes, Other (+5 more)

### Community 82 - "ref_node_assert"
Cohesion: 0.13
Nodes (11): layout(), Pt, orderNotes(), parsePins(), togglePin(), empty, asUrl(), linkFor() (+3 more)

### Community 83 - "agendatext.ts"
Cohesion: 0.29
Nodes (9): agendaText(), DAYS, heading(), MONTHS, Row, fmtEffort(), overbooked(), planned() (+1 more)

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
Cohesion: 0.19
Nodes (14): agents, [bold, dim, red, green, yellow, cyan], clock(), count(), cut(), describe(), dur(), note() (+6 more)

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

### Community 107 - "setopt.test.ts"
Cohesion: 0.50
Nodes (3): applySet(), NumOpts, off

### Community 108 - "decorate"
Cohesion: 0.13
Nodes (9): codeTokens(), decorate(), Glyph, imageSrc(), Img, inline(), langFor(), mark() (+1 more)

### Community 109 - "wlr.rs"
Cohesion: 0.13
Nodes (12): active(), apply(), Ev, App, Closed, State, Title, focused() (+4 more)

### Community 111 - "OrgFile"
Cohesion: 0.15
Nodes (18): adjacency(), files(), Graph, graph_neighbourhood(), GraphNode, Mention, notes_tags_and_tagged(), orphans() (+10 more)

### Community 112 - "projects.ts"
Cohesion: 0.22
Nodes (13): budgetUsage(), Card, cards(), merge(), period(), Project, ROUNDINGS, Row (+5 more)

### Community 113 - "org.rs"
Cohesion: 0.10
Nodes (30): ALIAS, archive_done_moves_top_most_done_subtrees_in_order(), capture_template(), CLOSED, COOKIE, CURSOR, DOC, duplicate_body_drops_ids_and_title() (+22 more)

### Community 114 - "settings.ts"
Cohesion: 0.18
Nodes (17): CATEGORIES, changes(), checkTemplates(), checkViews(), Field, Form, fromForm(), GROUPS (+9 more)

### Community 115 - "createState"
Cohesion: 0.14
Nodes (11): createState(), emphasis(), Num, numExt(), orgShiftTab(), setSpellcheck(), spellExt(), vimInsert() (+3 more)

### Community 116 - "App"
Cohesion: 0.08
Nodes (60): activity_learn(), activity_rules_delete(), activity_rules_list(), ai_day_prompt(), ai_excluded(), ai_note_prompt(), App, backup_now() (+52 more)

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
Cohesion: 0.18
Nodes (5): notify(), once_a_day(), picks_due_reminders(), start(), tick()

### Community 121 - "footnote.ts"
Cohesion: 0.29
Nodes (8): followFootnote(), insertFootnote(), recordJump(), addFootnote(), FN, footnoteTarget(), nextLabel(), add()

### Community 122 - "clock.ts"
Cohesion: 0.21
Nodes (7): dayTime(), inactiveStamp(), naive(), p2(), parseWhen(), WeekDay, worked()

### Community 123 - "zoom.test.ts"
Cohesion: 0.67
Nodes (4): clamp(), nextZoom(), parseZoom(), wheelZoom()

### Community 124 - "Kw"
Cohesion: 0.15
Nodes (14): cycle_priority(), edit_heading(), heading(), Kw, parse(), parse_effort(), Parts, query() (+6 more)

### Community 125 - "ref_node_fs"
Cohesion: 0.17
Nodes (8): DIRS, EACH, GIFS, mb(), problems(), scan(), TOTAL, script

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
Cohesion: 0.23
Nodes (10): @codemirror/state, hidden(), hideBlock, Narrow, narrowedTo(), narrowEffect, narrowField, narrowRange() (+2 more)

### Community 131 - "search"
Cohesion: 0.25
Nodes (8): backlinks(), Hit, links_and_backlinks(), parse_query(), Query, search(), search_filters_by_folder(), search_matches_every_term()

### Community 132 - "set_keyword"
Cohesion: 0.36
Nodes (8): at(), closed_stamp(), cycle_and_repeat(), cycle_todo(), heading_end(), notes(), repeat_with_unclosed_properties_stays_put(), set_keyword()

### Community 133 - "Regex"
Cohesion: 0.60
Nodes (3): ANY_LINK, HEAD_LINK, STATS

### Community 134 - "bulk.ts"
Cohesion: 0.39
Nodes (7): Loc, Mark, markKey(), orderForEdit(), outermost(), reresolve(), tagChange

### Community 135 - "donelog.ts"
Cohesion: 0.43
Nodes (4): at(), doneLog(), It, withLog()

### Community 137 - "super"
Cohesion: 0.20
Nodes (3): source(), start(), step()

### Community 139 - "whatsnew.test.ts"
Cohesion: 0.43
Nodes (4): Notes, notesText(), notesTitle(), notesToShow()

### Community 140 - "autopilot/SKILL.md"
Cohesion: 0.40
Nodes (4): Idle, Loop, Rules, Stop

### Community 141 - "Cache"
Cohesion: 0.40
Nodes (3): Cache, org_paths(), scan()

### Community 142 - "main"
Cohesion: 0.29
Nodes (5): Agent Workflow, Rules, Run, Repo Conventions, main()

## Knowledge Gaps
- **378 isolated node(s):** `changelog.sh script`, `q`, `sched`, `late`, `habit` (+373 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 651 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **35 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `start()` connect `setup.ts` to `lib/Projects.svelte`, `+page.svelte`, `lib/Agenda.svelte`?**
  _High betweenness centrality (0.139) - this node is a cross-community bridge._
- **Why does `line()` connect `level` to `clock.ts`, `editor.ts`, `decorate`, `+page.svelte`?**
  _High betweenness centrality (0.045) - this node is a cross-community bridge._
- **Why does `insertTable()` connect `level` to `editor.ts`, `+page.svelte`?**
  _High betweenness centrality (0.035) - this node is a cross-community bridge._
- **What connects `changelog.sh script`, `q`, `sched` to the rest of the system?**
  _378 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `screenshots.ts` be split into smaller, more focused modules?**
  _Cohesion score 0.13105413105413105 - nodes in this community are weakly interconnected._
- **Should `timeclock.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.08974358974358974 - nodes in this community are weakly interconnected._
- **Should `AppHandle` be split into smaller, more focused modules?**
  _Cohesion score 0.09982174688057041 - nodes in this community are weakly interconnected._