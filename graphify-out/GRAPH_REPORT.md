# Graph Report - margin  (2026-10-21)

## Corpus Check
- 148 files · ~1,027,824 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 15 file(s) not represented in the graph (top: .css 8, (none) 4, .icns 1)

## Summary
- 2124 nodes · 4916 edges · 141 communities (104 shown, 37 thin omitted)
- Extraction: 98% EXTRACTED · 2% INFERRED · 0% AMBIGUOUS · INFERRED: 107 edges (avg confidence: 0.86)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `890474c4`
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
- Value
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
- Calendar
- Configuration
- DMG
- Tauri Global Shortcut Plugin
- Windows Installer
- Journal
- .new
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
- OrgFile
- merge.test.ts
- lib.rs
- Changelog
- dependencies
- changelog.sh
- ref_node_assert
- agendatext.ts
- .parse
- devDependencies
- ship/SKILL.md
- itemSwap
- main
- retrieve.rs
- mock.js
- releaser.md
- r5.js
- clock.ts
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
- lib/TableDialog.svelte
- projects.ts
- org.rs
- settings.ts
- createState
- App
- links.ts
- autopilot/SKILL.md
- media-size.ts
- remind.rs
- footnote.ts
- svelte
- set_keyword
- Kw
- scout.md
- TS
- time.test.ts
- tray.rs
- board.test.ts
- narrow.ts
- donelog.ts
- whatsnew.test.ts
- zoom.test.ts
- NOTE_WRITES
- pins.test.ts
- super

## God Nodes (most connected - your core abstractions)
1. `App` - 116 edges
2. `cfg()` - 50 edges
3. `Tc` - 42 edges
4. `OrgFile` - 38 edges
5. `Kw` - 34 edges
6. `kw()` - 28 edges
7. `w()` - 21 edges
8. `reload_config()` - 20 edges
9. `to_lines()` - 20 edges
10. `join()` - 20 edges

## Surprising Connections (you probably didn't know these)
- `1. Gate (all must hold, else report which failed and stop)` --references--> `main()`  [INFERRED]
  .claude/agents/releaser.md → scripts/coverage-ratchet.ts
- `Run` --references--> `main()`  [INFERRED]
  .claude/agents/scout.md → scripts/coverage-ratchet.ts
- `Repo Conventions` --references--> `main()`  [INFERRED]
  CLAUDE.md → scripts/coverage-ratchet.ts
- `Idle` --references--> `main()`  [INFERRED]
  .claude/skills/autopilot/SKILL.md → scripts/coverage-ratchet.ts
- `3. Review each report` --references--> `main()`  [INFERRED]
  .claude/skills/ship/SKILL.md → scripts/coverage-ratchet.ts

## Import Cycles
- 1-file cycle: `src-tauri/src/ai.rs -> src-tauri/src/ai.rs`

## Communities (141 total, 37 thin omitted)

### Community 0 - "screenshots.ts"
Cohesion: 0.14
Nodes (24): ADR-0007, ADR-0008, clip(), clips, config, day(), fixtures(), frame() (+16 more)

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
Nodes (22): active, close(), col, commit(), draft, error, expr, fm (+14 more)

### Community 11 - "Calendar"
Cohesion: 0.67
Nodes (3): Calendar, iCalendar, Tasks

### Community 12 - "join"
Cohesion: 0.20
Nodes (32): add_note(), append_archived(), archive(), archive_done(), capture_insert(), check_target(), clone_shift(), copy_subtree() (+24 more)

### Community 14 - "scripts"
Cohesion: 0.13
Nodes (15): scripts, build, check, check:watch, coverage, coverage:ratchet, dev, e2e:build (+7 more)

### Community 15 - "cfg"
Cohesion: 0.11
Nodes (19): ai_excluded(), ai_note_prompt(), attach_bytes(), attach_file(), attach_in(), attach_in_numbers_clashes_retries_races_and_rolls_back(), capture_templates(), cfg() (+11 more)

### Community 16 - "Notes"
Cohesion: 0.29
Nodes (7): Links and Backlinks, Notes, Org Format, Search, Syntax Highlighting, Table Formulas, Vim Keys

### Community 18 - "lib/TaskDialog.svelte"
Cohesion: 0.17
Nodes (11): svelte, preview, active, close(), closed(), duePreview, error, key() (+3 more)

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

### Community 24 - "Value"
Cohesion: 0.16
Nodes (14): activity_rules_list(), context_of(), dash_fixture(), dashboard(), dashboard_and_status_show_the_clock(), dashboard_builds_the_week(), dashboard_projects_month(), dashboard_week_runs_monday_to_sunday() (+6 more)

### Community 25 - "Sync"
Cohesion: 0.67
Nodes (3): Emacs, Git, Sync

### Community 29 - "+page.svelte"
Cohesion: 0.06
Nodes (7): cmd(), edit(), done(), effort(), note(), priority(), state()

### Community 31 - "ai.rs"
Cohesion: 0.09
Nodes (11): CHAT_WAIT, CTX, IDLE, LOADED, MAX_CHAT, MAX_PROMPT, READ_TIMEOUT, RUNS (+3 more)

### Community 32 - "level"
Cohesion: 0.18
Nodes (27): curLine(), drawerRange(), expandTemplate(), headings(), insertBlock(), insertMode(), insertTable(), line() (+19 more)

### Community 33 - "ics.rs"
Cohesion: 0.17
Nodes (17): builds_feed(), CALENDAR, calendar_from_a_url(), CALENDAR_TTL, date(), event(), events_on(), fetches_over_https() (+9 more)

### Community 34 - "generate"
Cohesion: 0.15
Nodes (15): Backend, Embedded, Ollama, body(), chat(), chat_messages(), chats(), clean() (+7 more)

### Community 35 - "config.rs"
Cohesion: 0.08
Nodes (28): add_view(), add_views(), atomic_writes(), Config, defaults_round_trip(), expand(), extra_days_off(), public_holidays() (+20 more)

### Community 36 - "CLAUDE.md"
Cohesion: 0.17
Nodes (10): Agent Workflow, Build & Test, Code Style, Permission Mode, Project, Release & CI/CD, Repo Conventions, Tauri-Specific (+2 more)

### Community 37 - "State"
Cohesion: 0.11
Nodes (40): activity_suggestions(), agenda(), ai_retrieve(), capture_insert(), capture_template(), expected_hours_for(), export_html_fragment(), export_linked() (+32 more)

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

### Community 45 - "Calendar"
Cohesion: 0.27
Nodes (14): Calendar, day_done(), day_off_rules(), easter(), expected_hours(), flex(), holidays(), holidays_report() (+6 more)

### Community 52 - ".new"
Cohesion: 0.16
Nodes (23): add_past_sessions(), apply_carry(), breaks_accumulate_and_flex(), csv(), csv_field(), daily_report(), export_code(), import_timelog() (+15 more)

### Community 57 - "ollama"
Cohesion: 0.18
Nodes (11): available_gb(), cancel_draft(), cancel_hits_only_the_running_draft(), free_gb(), ollama(), ollama_chat(), parse(), read_chat() (+3 more)

### Community 60 - "Event"
Cohesion: 0.16
Nodes (12): append_diary(), clock_out_earlier_refuses_before_the_start(), edit_session_bounds(), Event, Break, In, Out, hours() (+4 more)

### Community 61 - "setup.ts"
Cohesion: 0.12
Nodes (28): at(), buckets, day, ev(), events, y, button(), session() (+20 more)

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
Cohesion: 0.20
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
Cohesion: 0.08
Nodes (45): ai_day_prompt(), append_journal_template(), capture_file(), capture_file_fills_the_pattern_under_notes(), capture_path(), csv_dest(), csv_target(), date() (+37 more)

### Community 76 - "OrgFile"
Cohesion: 0.15
Nodes (19): feed(), open_entries(), add_interval(), all_tags(), Cache, Habit, Headline, Item (+11 more)

### Community 77 - "merge.test.ts"
Cohesion: 0.42
Nodes (7): applyText(), Change, diffChange(), mergeEdits(), perKey(), apply(), merged()

### Community 78 - "lib.rs"
Cohesion: 0.05
Nodes (29): attachment_files(), attachment_files_skips_trash_and_unused_ignores_linked(), CANCEL, DOWNLOADING, image_dirs(), image_dirs_and_trash_roots(), may_overwrite(), needs_refresh() (+21 more)

### Community 79 - "Changelog"
Cohesion: 0.14
Nodes (13): Changelog, Features, Features, Features, Fixes, Fixes, Fixes, Other (+5 more)

### Community 80 - "dependencies"
Cohesion: 0.15
Nodes (13): dependencies, @codemirror/commands, @codemirror/language, @codemirror/language-data, @codemirror/search, @codemirror/state, @codemirror/view, @lezer/highlight (+5 more)

### Community 82 - "ref_node_assert"
Cohesion: 0.12
Nodes (13): BugEnv, bugReportUrl(), isError(), ISSUES, osName(), layout(), Pt, empty (+5 more)

### Community 83 - "agendatext.ts"
Cohesion: 0.29
Nodes (9): agendaText(), DAYS, heading(), MONTHS, Row, fmtEffort(), overbooked(), planned() (+1 more)

### Community 84 - ".parse"
Cohesion: 0.19
Nodes (17): agenda(), agenda_week(), d(), dates(), deadline_warning_cookie(), effort_parses_org_durations(), habits(), overdue_and_move() (+9 more)

### Community 85 - "devDependencies"
Cohesion: 0.18
Nodes (11): devDependencies, svelte, svelte-check, @sveltejs/adapter-static, @sveltejs/kit, @sveltejs/vite-plugin-svelte, @tauri-apps/cli, @types/node (+3 more)

### Community 86 - "ship/SKILL.md"
Cohesion: 0.25
Nodes (7): 1. Survey, 2. Dispatch, 3. Review each report, 4. Clean up, 5. Report to the owner, Rules, Running it repeatedly

### Community 87 - "itemSwap"
Cohesion: 0.50
Nodes (6): indent(), isItem(), itemSwap(), renumber(), fix(), swap()

### Community 89 - "main"
Cohesion: 0.33
Nodes (5): Finish, House rules, Report (your final message), Start, main()

### Community 90 - "retrieve.rs"
Cohesion: 0.11
Nodes (27): APP, app_source(), app_sources(), app_sources_and_mixed_packing(), BUDGET, cost(), cut(), EXCERPT (+19 more)

### Community 91 - "mock.js"
Cohesion: 0.22
Nodes (8): habit, late, nav, q, row(), tags(), today, week

### Community 92 - "releaser.md"
Cohesion: 0.33
Nodes (5): 1. Gate (all must hold, else report which failed and stop), 2. Bump PR, 3. Tag, build, publish, Report, Rules

### Community 93 - "r5.js"
Cohesion: 0.32
Nodes (6): bar(), DEMOS5, layered(), P5, page(), worked

### Community 94 - "clock.ts"
Cohesion: 0.31
Nodes (7): dayTime(), inactiveStamp(), naive(), p2(), parseWhen(), WeekDay, worked()

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
Cohesion: 0.07
Nodes (15): active(), apply(), Ev, App, Closed, State, Title, focused() (+7 more)

### Community 111 - "lib/TableDialog.svelte"
Cohesion: 0.40
Nodes (4): active, close(), [hc, hr], key()

### Community 112 - "projects.ts"
Cohesion: 0.22
Nodes (13): budgetUsage(), Card, cards(), merge(), period(), Project, ROUNDINGS, Row (+5 more)

### Community 113 - "org.rs"
Cohesion: 0.10
Nodes (31): ALIAS, archive_done_moves_top_most_done_subtrees_in_order(), capture_template(), clone_with_time_shift(), CLOSED, COOKIE, CURSOR, DOC (+23 more)

### Community 114 - "settings.ts"
Cohesion: 0.18
Nodes (17): CATEGORIES, changes(), checkTemplates(), checkViews(), Field, Form, fromForm(), GROUPS (+9 more)

### Community 115 - "createState"
Cohesion: 0.14
Nodes (11): createState(), emphasis(), Num, numExt(), orgShiftTab(), setSpellcheck(), spellExt(), vimInsert() (+3 more)

### Community 116 - "App"
Cohesion: 0.12
Nodes (28): activity_dismiss(), activity_learn(), activity_rules_delete(), App, backup_now(), day_done_now(), export_report(), leave_time() (+20 more)

### Community 117 - "links.ts"
Cohesion: 0.51
Nodes (8): bareTitle(), esc(), findTarget(), head(), linkHeadings(), norm(), retitlePlan(), splitTarget()

### Community 118 - "autopilot/SKILL.md"
Cohesion: 0.40
Nodes (4): Idle, Loop, Rules, Stop

### Community 119 - "media-size.ts"
Cohesion: 0.31
Nodes (7): DIRS, EACH, GIFS, mb(), problems(), scan(), TOTAL

### Community 120 - "remind.rs"
Cohesion: 0.20
Nodes (5): notify(), once_a_day(), picks_due_reminders(), start(), tick()

### Community 121 - "footnote.ts"
Cohesion: 0.29
Nodes (8): followFootnote(), insertFootnote(), recordJump(), addFootnote(), FN, footnoteTarget(), nextLabel(), add()

### Community 122 - "svelte"
Cohesion: 0.12
Nodes (4): r(), title(), SvelteKit, Vite

### Community 123 - "set_keyword"
Cohesion: 0.31
Nodes (9): at(), closed_stamp(), cycle_and_repeat(), cycle_todo(), fmt_time(), heading_end(), notes(), repeat_with_unclosed_properties_stays_put() (+1 more)

### Community 124 - "Kw"
Cohesion: 0.18
Nodes (12): cycle_priority(), edit_heading(), heading(), Kw, parse(), Parts, set_priority(), set_tags() (+4 more)

### Community 126 - "TS"
Cohesion: 0.29
Nodes (8): move_planning(), parse_ts(), planning(), planning_ts(), set_planning(), set_planning_keeps_repeater(), shift_planning(), TS

### Community 127 - "time.test.ts"
Cohesion: 0.29
Nodes (9): noPicker(), pad(), texts(), timeFocused(), Ev, events(), evs(), hm() (+1 more)

### Community 128 - "tray.rs"
Cohesion: 0.27
Nodes (5): refresh(), setup(), show(), tracking(), Tray

### Community 129 - "board.test.ts"
Cohesion: 0.32
Nodes (5): Column, columns(), moveTo(), done, todo

### Community 130 - "narrow.ts"
Cohesion: 0.23
Nodes (10): @codemirror/state, hidden(), hideBlock, Narrow, narrowedTo(), narrowEffect, narrowField, narrowRange() (+2 more)

### Community 131 - "donelog.ts"
Cohesion: 0.36
Nodes (5): setNow(), at(), doneLog(), It, withLog()

### Community 132 - "whatsnew.test.ts"
Cohesion: 0.43
Nodes (4): Notes, notesText(), notesTitle(), notesToShow()

### Community 133 - "zoom.test.ts"
Cohesion: 0.67
Nodes (4): clamp(), nextZoom(), parseZoom(), wheelZoom()

### Community 135 - "pins.test.ts"
Cohesion: 0.70
Nodes (3): orderNotes(), parsePins(), togglePin()

### Community 137 - "super"
Cohesion: 0.18
Nodes (3): source(), start(), step()

## Knowledge Gaps
- **371 isolated node(s):** `changelog.sh script`, `q`, `sched`, `late`, `habit` (+366 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 644 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **37 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `start()` connect `time.test.ts` to `donelog.ts`, `lib/Agenda.svelte`, `setup.ts`, `svelte`, `+page.svelte`?**
  _High betweenness centrality (0.130) - this node is a cross-community bridge._
- **Why does `line()` connect `level` to `svelte`, `editor.ts`, `decorate`, `+page.svelte`?**
  _High betweenness centrality (0.058) - this node is a cross-community bridge._
- **Why does `insertTable()` connect `level` to `editor.ts`, `+page.svelte`?**
  _High betweenness centrality (0.035) - this node is a cross-community bridge._
- **What connects `changelog.sh script`, `q`, `sched` to the rest of the system?**
  _371 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `screenshots.ts` be split into smaller, more focused modules?**
  _Cohesion score 0.13846153846153847 - nodes in this community are weakly interconnected._
- **Should `timeclock.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.08974358974358974 - nodes in this community are weakly interconnected._
- **Should `AppHandle` be split into smaller, more focused modules?**
  _Cohesion score 0.10160427807486631 - nodes in this community are weakly interconnected._