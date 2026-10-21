# Graph Report - margin  (2026-10-21)

## Corpus Check
- 148 files · ~1,027,826 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 15 file(s) not represented in the graph (top: .css 8, (none) 4, .icns 1)

## Summary
- 2121 nodes · 4916 edges · 139 communities (104 shown, 35 thin omitted)
- Extraction: 98% EXTRACTED · 2% INFERRED · 0% AMBIGUOUS · INFERRED: 107 edges (avg confidence: 0.86)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `9915ced0`
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
- gnome.rs
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
- main
- State
- super
- day_prompt
- release/SKILL.md
- paths.test.ts
- verify-all/SKILL.md
- chatctx.ts
- App Icon
- attach.rs
- Configuration
- DMG
- Tauri Global Shortcut Plugin
- Windows Installer
- Journal
- Calendar
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
- NaiveDate
- changelog.sh
- ref_node_assert
- agendatext.ts
- .parse
- activity.test.ts
- read_or_empty
- itemSwap
- State
- retrieve.rs
- mock.js
- chrono
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
- .event
- projects.ts
- org.rs
- settings.ts
- createState
- App
- links.ts
- bugreport.test.ts
- media-size.ts
- remind.rs
- footnote.ts
- svelte
- set_keyword
- Kw
- export_linked
- TS
- time.test.ts
- tray.rs
- board.test.ts
- narrow.ts
- donelog.ts
- whatsnew.test.ts
- zoom.test.ts
- Cache
- pins.test.ts
- bulk.ts
- idle.rs

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
- `Finish` --references--> `main()`  [INFERRED]
  .claude/agents/implementer.md → scripts/coverage-ratchet.ts
- `House rules` --references--> `main()`  [INFERRED]
  .claude/agents/implementer.md → scripts/coverage-ratchet.ts
- `1. Gate (all must hold, else report which failed and stop)` --references--> `main()`  [INFERRED]
  .claude/agents/releaser.md → scripts/coverage-ratchet.ts
- `Run` --references--> `main()`  [INFERRED]
  .claude/agents/scout.md → scripts/coverage-ratchet.ts
- `Repo Conventions` --references--> `main()`  [INFERRED]
  CLAUDE.md → scripts/coverage-ratchet.ts

## Import Cycles
- 1-file cycle: `src-tauri/src/ai.rs -> src-tauri/src/ai.rs`

## Communities (139 total, 35 thin omitted)

### Community 0 - "screenshots.ts"
Cohesion: 0.14
Nodes (24): ADR-0007, ADR-0008, clip(), clips, config, day(), fixtures(), frame() (+16 more)

### Community 1 - "timeclock.rs"
Cohesion: 0.09
Nodes (36): adjust_start_refuses_to_overlap_the_previous_session(), breaks_accumulate_and_flex(), clock_flow_on_disk(), clock_out_earlier_refuses_before_the_start(), csv_field(), csv_quotes_and_comma(), csv_says_why_time_off_wrote_no_row(), csv_skips_zero_rows() (+28 more)

### Community 2 - "AppHandle"
Cohesion: 0.14
Nodes (24): ai_chat(), ai_download(), ai_draft(), ai_model_delete(), ai_models(), ai_set(), apply_config(), apply_spellcheck() (+16 more)

### Community 3 - "editor.ts"
Cohesion: 0.06
Nodes (34): Block, BULLETS, clicks, convertToTable(), hide, hooks, inTable(), LANG_ALIAS (+26 more)

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
Nodes (26): description, engines, node, license, name, type, version, @codemirror/commands (+18 more)

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
Cohesion: 0.05
Nodes (39): dependencies, @codemirror/commands, @codemirror/language, @codemirror/language-data, @codemirror/search, @codemirror/state, @codemirror/view, @lezer/highlight (+31 more)

### Community 15 - "gnome.rs"
Cohesion: 0.20
Nodes (10): active(), BUS, FILES, get(), install(), install_copies_the_extension(), parse(), session() (+2 more)

### Community 16 - "Notes"
Cohesion: 0.29
Nodes (7): Links and Backlinks, Notes, Org Format, Search, Syntax Highlighting, Table Formulas, Vim Keys

### Community 18 - "lib/TaskDialog.svelte"
Cohesion: 0.12
Nodes (15): svelte, preview, active, close(), [hc, hr], key(), active, close() (+7 more)

### Community 19 - "lib/Menu.svelte"
Cohesion: 0.29
Nodes (4): active, node, show(), filter()

### Community 20 - "default.json"
Cohesion: 0.33
Nodes (5): description, identifier, permissions, $schema, windows

### Community 21 - "media.rs"
Cohesion: 0.19
Nodes (14): any(), AudioObjectGetPropertyData(), AudioObjectGetPropertyDataSize(), cc(), CMIOObjectGetPropertyData(), CMIOObjectGetPropertyDataSize(), GLOBAL, in_use() (+6 more)

### Community 22 - "activity.rs"
Cohesion: 0.07
Nodes (48): AwEvent, browser_urls(), Bucket, Buckets, CEST, COMMON, cut(), dismiss() (+40 more)

### Community 23 - "export.rs"
Cohesion: 0.09
Nodes (48): BLOCK, Heading, List, Para, Pre, Quote, Rule, Table (+40 more)

### Community 24 - "Value"
Cohesion: 0.16
Nodes (14): context_of(), dash_fixture(), dashboard(), dashboard_and_status_show_the_clock(), dashboard_builds_the_week(), dashboard_projects_month(), dashboard_week_runs_monday_to_sunday(), expected_after_leave_subtracts_leave() (+6 more)

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
Cohesion: 0.14
Nodes (31): curLine(), drawerRange(), expandTemplate(), findTable(), headings(), insertBlock(), insertMode(), insertTable() (+23 more)

### Community 33 - "ics.rs"
Cohesion: 0.16
Nodes (18): builds_feed(), CALENDAR, calendar_from_a_url(), CALENDAR_TTL, date(), event(), events_on(), feed() (+10 more)

### Community 34 - "generate"
Cohesion: 0.15
Nodes (15): Backend, Embedded, Ollama, body(), chat(), chat_messages(), chats(), clean() (+7 more)

### Community 35 - "config.rs"
Cohesion: 0.09
Nodes (28): add_view(), add_views(), atomic_writes(), Config, defaults_round_trip(), expand(), extra_days_off(), public_holidays() (+20 more)

### Community 36 - "main"
Cohesion: 0.05
Nodes (33): Agent Workflow, Finish, House rules, Report (your final message), Start, 1. Gate (all must hold, else report which failed and stop), 2. Bump PR, 3. Tag, build, publish (+25 more)

### Community 37 - "State"
Cohesion: 0.11
Nodes (42): activity_suggestions(), agenda(), ai_excluded(), ai_note_prompt(), ai_retrieve(), capture_insert(), capture_template(), capture_templates() (+34 more)

### Community 38 - "super"
Cohesion: 0.12
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

### Community 45 - "attach.rs"
Cohesion: 0.24
Nodes (8): clean(), images_share_the_note_folders_images_dir(), is_image(), LINK, names_and_links(), normalize(), target(), unused()

### Community 52 - "Calendar"
Cohesion: 0.16
Nodes (28): DashIn, apply_carry(), Calendar, csv(), daily_report(), day_done(), day_off_rules(), easter() (+20 more)

### Community 57 - "ollama"
Cohesion: 0.18
Nodes (11): available_gb(), cancel_draft(), cancel_hits_only_the_running_draft(), free_gb(), ollama(), ollama_chat(), parse(), read_chat() (+3 more)

### Community 60 - "Event"
Cohesion: 0.20
Nodes (12): add_past_sessions(), append_diary(), Event, Break, In, Out, hours(), normalize_log_sorts_and_dedupes() (+4 more)

### Community 61 - "setup.ts"
Cohesion: 0.17
Nodes (20): session(), answer(), app, browser, close(), Dirs, fileMatches(), isoDay() (+12 more)

### Community 62 - "backup.rs"
Cohesion: 0.12
Nodes (36): active_profile_is_untracked_and_survives_the_pull(), activity_files_union_merge(), attributes_are_appended_once(), BACKUP, backup_syncs_the_data_repo(), clean_env(), commit(), commit_all() (+28 more)

### Community 63 - "r4.js"
Cohesion: 0.05
Nodes (26): agendaDays, backlinks, board, byTag, DEMO, DEMOS, done3, doneWeek (+18 more)

### Community 64 - "Tc"
Cohesion: 0.20
Nodes (6): discard_idle_sessions(), edit_session_bounds(), locked_keeps_appends_during_a_rewrite(), now(), Tc, unreadable_files_are_not_overwritten()

### Community 65 - "triager.md"
Cohesion: 0.40
Nodes (4): Pass, Report (ends your run), Rules for what you post, Watch mode (only when asked)

### Community 67 - "coverage-ratchet.ts"
Cohesion: 0.17
Nodes (9): Baseline, Count, parseLcov(), pct(), regressions(), Report, Suite, summarize() (+1 more)

### Community 68 - "session.ts"
Cohesion: 0.25
Nodes (7): Review, Rules, Entry, KINDS, ok(), parseSession(), Session

### Community 69 - "lib/Agenda.svelte"
Cohesion: 0.28
Nodes (14): start(), copyList(), dayInput(), goTo(), key(), op(), pointer(), refresh() (+6 more)

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
Cohesion: 0.10
Nodes (35): activity_rules_list(), ai_day_prompt(), append_journal_template(), capture_file(), capture_path(), check_in(), csv_dest(), csv_target() (+27 more)

### Community 76 - "OrgFile"
Cohesion: 0.17
Nodes (18): open_entries(), add_interval(), all_tags(), Habit, Headline, Item, norm_time(), occurrences() (+10 more)

### Community 77 - "merge.test.ts"
Cohesion: 0.42
Nodes (7): applyText(), Change, diffChange(), mergeEdits(), perKey(), apply(), merged()

### Community 78 - "lib.rs"
Cohesion: 0.04
Nodes (35): attach_bytes(), attach_file(), attach_in(), attach_in_numbers_clashes_retries_races_and_rolls_back(), attachment_files(), attachment_files_skips_trash_and_unused_ignores_linked(), bad_meeting_pattern_and_calendar_only_warn(), CANCEL (+27 more)

### Community 79 - "Changelog"
Cohesion: 0.14
Nodes (13): Changelog, Features, Features, Features, Fixes, Fixes, Fixes, Other (+5 more)

### Community 80 - "NaiveDate"
Cohesion: 0.13
Nodes (20): capture_file_fills_the_pattern_under_notes(), date_preview(), date_preview_at(), date_stamp(), date_stamp_at(), dates_read_relative_to_a_given_today(), expected_after_leave(), iso() (+12 more)

### Community 82 - "ref_node_assert"
Cohesion: 0.16
Nodes (8): layout(), Pt, empty, asUrl(), linkFor(), windowTitle(), wordCount(), wordsLabel()

### Community 83 - "agendatext.ts"
Cohesion: 0.24
Nodes (10): agendaText(), DAYS, heading(), MONTHS, Row, r(), fmtEffort(), overbooked() (+2 more)

### Community 84 - ".parse"
Cohesion: 0.23
Nodes (15): agenda(), agenda_week(), d(), dates(), deadline_warning_cookie(), effort_parses_org_durations(), habits(), overdue_and_move() (+7 more)

### Community 85 - "activity.test.ts"
Cohesion: 0.21
Nodes (10): at(), buckets, day, ev(), events, y, button(), clickIn() (+2 more)

### Community 86 - "read_or_empty"
Cohesion: 0.24
Nodes (4): leave_on_disk(), malformed_projects_are_reported(), read_or_empty(), rewrite_with()

### Community 87 - "itemSwap"
Cohesion: 0.50
Nodes (6): indent(), isItem(), itemSwap(), renumber(), fix(), swap()

### Community 90 - "retrieve.rs"
Cohesion: 0.11
Nodes (27): APP, app_source(), app_sources(), app_sources_and_mixed_packing(), BUDGET, cost(), cut(), EXCERPT (+19 more)

### Community 91 - "mock.js"
Cohesion: 0.22
Nodes (8): habit, late, nav, q, row(), tags(), today, week

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
Cohesion: 0.18
Nodes (11): active(), apply(), Ev, App, Closed, State, Title, focused() (+3 more)

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
Cohesion: 0.16
Nodes (9): createState(), emphasis(), Num, numExt(), orgShiftTab(), vimInsert(), EMPH, run() (+1 more)

### Community 116 - "App"
Cohesion: 0.12
Nodes (28): activity_dismiss(), activity_learn(), activity_rules_delete(), App, backup_now(), day_done_now(), export_report(), leave_time() (+20 more)

### Community 117 - "links.ts"
Cohesion: 0.51
Nodes (8): bareTitle(), esc(), findTarget(), head(), linkHeadings(), norm(), retitlePlan(), splitTarget()

### Community 118 - "bugreport.test.ts"
Cohesion: 0.48
Nodes (5): BugEnv, bugReportUrl(), isError(), ISSUES, osName()

### Community 119 - "media-size.ts"
Cohesion: 0.27
Nodes (7): DIRS, EACH, GIFS, mb(), problems(), scan(), TOTAL

### Community 120 - "remind.rs"
Cohesion: 0.20
Nodes (5): notify(), once_a_day(), picks_due_reminders(), start(), tick()

### Community 121 - "footnote.ts"
Cohesion: 0.29
Nodes (8): followFootnote(), insertFootnote(), recordJump(), addFootnote(), FN, footnoteTarget(), nextLabel(), add()

### Community 122 - "svelte"
Cohesion: 0.12
Nodes (3): title(), SvelteKit, Vite

### Community 123 - "set_keyword"
Cohesion: 0.31
Nodes (9): at(), closed_stamp(), cycle_and_repeat(), cycle_todo(), fmt_time(), heading_end(), notes(), repeat_with_unclosed_properties_stays_put() (+1 more)

### Community 124 - "Kw"
Cohesion: 0.20
Nodes (11): cycle_priority(), edit_heading(), heading(), Kw, Parts, set_priority(), set_tags(), set_title() (+3 more)

### Community 125 - "export_linked"
Cohesion: 0.38
Nodes (7): export_html_fragment(), export_linked(), export_markdown(), export_note(), export_path(), stem(), write_export()

### Community 126 - "TS"
Cohesion: 0.29
Nodes (8): move_planning(), parse_ts(), planning(), planning_ts(), set_planning(), set_planning_keeps_repeater(), shift_planning(), TS

### Community 127 - "time.test.ts"
Cohesion: 0.32
Nodes (6): pad(), timeFocused(), Ev, events(), evs(), hm()

### Community 128 - "tray.rs"
Cohesion: 0.22
Nodes (5): refresh(), setup(), show(), tracking(), Tray

### Community 129 - "board.test.ts"
Cohesion: 0.32
Nodes (5): Column, columns(), moveTo(), done, todo

### Community 130 - "narrow.ts"
Cohesion: 0.20
Nodes (12): @codemirror/state, @codemirror/view, headingAt(), hidden(), hideBlock, Narrow, narrowedTo(), narrowEffect (+4 more)

### Community 131 - "donelog.ts"
Cohesion: 0.43
Nodes (4): at(), doneLog(), It, withLog()

### Community 132 - "whatsnew.test.ts"
Cohesion: 0.43
Nodes (4): Notes, notesText(), notesTitle(), notesToShow()

### Community 133 - "zoom.test.ts"
Cohesion: 0.67
Nodes (4): clamp(), nextZoom(), parseZoom(), wheelZoom()

### Community 134 - "Cache"
Cohesion: 0.40
Nodes (3): Cache, org_paths(), scan()

### Community 135 - "pins.test.ts"
Cohesion: 0.70
Nodes (3): orderNotes(), parsePins(), togglePin()

### Community 136 - "bulk.ts"
Cohesion: 0.53
Nodes (4): Loc, orderForEdit(), outermost(), tagChange

### Community 137 - "idle.rs"
Cohesion: 0.24
Nodes (3): source(), start(), step()

## Knowledge Gaps
- **371 isolated node(s):** `changelog.sh script`, `q`, `sched`, `late`, `habit` (+366 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 641 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **35 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `start()` connect `lib/Agenda.svelte` to `setup.ts`, `activity.test.ts`, `svelte`, `+page.svelte`, `time.test.ts`?**
  _High betweenness centrality (0.125) - this node is a cross-community bridge._
- **Why does `line()` connect `level` to `svelte`, `editor.ts`, `decorate`, `+page.svelte`?**
  _High betweenness centrality (0.058) - this node is a cross-community bridge._
- **Why does `insertTable()` connect `level` to `editor.ts`, `+page.svelte`?**
  _High betweenness centrality (0.035) - this node is a cross-community bridge._
- **What connects `changelog.sh script`, `q`, `sched` to the rest of the system?**
  _371 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `screenshots.ts` be split into smaller, more focused modules?**
  _Cohesion score 0.13846153846153847 - nodes in this community are weakly interconnected._
- **Should `timeclock.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.08603145235892692 - nodes in this community are weakly interconnected._
- **Should `AppHandle` be split into smaller, more focused modules?**
  _Cohesion score 0.13675213675213677 - nodes in this community are weakly interconnected._