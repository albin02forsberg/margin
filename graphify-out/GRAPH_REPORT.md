# Graph Report - margin  (2026-10-14)

## Corpus Check
- 106 files · ~902,849 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 14 file(s) not represented in the graph (top: .css 8, (none) 3, .icns 1)

## Summary
- 1640 nodes · 3644 edges · 120 communities (81 shown, 39 thin omitted)
- Extraction: 98% EXTRACTED · 2% INFERRED · 0% AMBIGUOUS · INFERRED: 66 edges (avg confidence: 0.87)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `d3012b83`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- Kw
- timeclock.rs
- AppHandle
- editor.ts
- []
- watcher.rs
- orgtable.ts
- tauri.conf.json
- package.json
- compilerOptions
- Calendar
- gnome.rs
- GitHub Releases
- tray.rs
- curLine
- Notes
- AppImage
- svelte
- lib/Menu.svelte
- default.json
- setup.ts
- activity.rs
- export.rs
- notes.rs
- Sync
- +layout.ts
- build.rs
- +page.svelte
- margin
- ai.rs
- level
- lib/TaskDialog.svelte
- generate
- config.rs
- CLAUDE.md
- screenshots.ts
- export_linked
- day_prompt
- release/SKILL.md
- OrgFile
- verify-all/SKILL.md
- org.rs
- App Icon
- coverage-ratchet.ts
- Configuration
- DMG
- Tauri Global Shortcut Plugin
- Windows Installer
- Journal
- wlr.rs
- Node.js
- Tauri Notification Plugin
- Tauri Single Instance Plugin
- Tauri Action
- ollama
- Time Tracking
- TypeScript
- ics.rs
- decorate
- backup.rs
- r4.js
- settings.ts
- triager.md
- extension.js
- remind.rs
- session.ts
- super
- b-variants.js
- focus
- download
- load
- scripts
- ideator.md
- path
- merge.test.ts
- lib.rs
- v0.3.0 - 2026-10-11
- clock.ts
- changelog.sh
- R
- App
- dashboard
- date
- chatctx.ts
- ship/SKILL.md
- join
- ref_node_assert
- mock.js
- kw
- r5.js
- main
- releaser.md
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
- autopilot/SKILL.md
- scout.md
- board.test.ts
- activity.test.ts
- AtomicU64
- projects.ts
- chips.test.ts
- paths.test.ts
- Cache

## God Nodes (most connected - your core abstractions)
1. `App` - 96 edges
2. `Tc` - 40 edges
3. `cfg()` - 38 edges
4. `[]` - 34 edges
5. `Kw` - 26 edges
6. `OrgFile` - 24 edges
7. `reload_config()` - 19 edges
8. `kw()` - 18 edges
9. `Event` - 18 edges
10. `level()` - 18 edges

## Surprising Connections (you probably didn't know these)
- `Agent Workflow` --references--> `main()`  [INFERRED]
  CLAUDE.md → scripts/coverage-ratchet.ts
- `1. Gate (all must hold, else report which failed and stop)` --references--> `main()`  [INFERRED]
  .claude/agents/releaser.md → scripts/coverage-ratchet.ts
- `Run` --references--> `main()`  [INFERRED]
  .claude/agents/scout.md → scripts/coverage-ratchet.ts
- `Repo Conventions` --references--> `main()`  [INFERRED]
  CLAUDE.md → scripts/coverage-ratchet.ts
- `Idle` --references--> `main()`  [INFERRED]
  .claude/skills/autopilot/SKILL.md → scripts/coverage-ratchet.ts

## Import Cycles
- 1-file cycle: `src-tauri/src/ai.rs -> src-tauri/src/ai.rs`

## Communities (120 total, 39 thin omitted)

### Community 0 - "Kw"
Cohesion: 0.16
Nodes (19): agenda_week(), at(), cycle_and_repeat(), cycle_priority(), cycle_todo(), d(), dates(), edit_heading() (+11 more)

### Community 1 - "timeclock.rs"
Cohesion: 0.05
Nodes (74): add_past_sessions(), append_diary(), apply_carry(), breaks_accumulate_and_flex(), csv(), csv_field(), csv_quotes_and_comma(), csv_skips_zero_rows() (+66 more)

### Community 2 - "AppHandle"
Cohesion: 0.15
Nodes (21): ai_chat(), ai_download(), ai_draft(), ai_model_delete(), ai_models(), ai_set(), apply_config(), capture() (+13 more)

### Community 3 - "editor.ts"
Cohesion: 0.09
Nodes (20): Block, BULLETS, clicks, hide, hooks, LANG_ALIAS, langLoaded, langs (+12 more)

### Community 4 - "[]"
Cohesion: 0.14
Nodes (21): [], addDays(), dateChip(), day, dayInput(), fmtDay(), goToday(), key() (+13 more)

### Community 5 - "watcher.rs"
Cohesion: 0.13
Nodes (23): active(), active_window(), AFK_MINS, append(), clean(), flush(), flush_writes_the_open_record_once(), gnome_extension() (+15 more)

### Community 6 - "orgtable.ts"
Cohesion: 0.12
Nodes (33): align(), applyFormulas(), cells(), colRef(), create(), deleteCol(), deleteRow(), done() (+25 more)

### Community 7 - "tauri.conf.json"
Cohesion: 0.06
Nodes (34): app, security, windows, enable, scope, build, beforeBuildCommand, beforeDevCommand (+26 more)

### Community 8 - "package.json"
Cohesion: 0.06
Nodes (28): description, engines, node, license, name, type, version, @codemirror/commands (+20 more)

### Community 9 - "compilerOptions"
Cohesion: 0.14
Nodes (13): ./.svelte-kit/tsconfig.json, compilerOptions, allowImportingTsExtensions, allowJs, checkJs, esModuleInterop, forceConsistentCasingInFileNames, moduleResolution (+5 more)

### Community 11 - "Calendar"
Cohesion: 0.67
Nodes (3): Calendar, iCalendar, Tasks

### Community 12 - "gnome.rs"
Cohesion: 0.20
Nodes (10): active(), BUS, FILES, get(), install(), install_copies_the_extension(), parse(), session() (+2 more)

### Community 14 - "tray.rs"
Cohesion: 0.27
Nodes (5): refresh(), setup(), show(), tracking(), Tray

### Community 15 - "curLine"
Cohesion: 0.21
Nodes (14): createState(), curLine(), expandTemplate(), insertBlock(), insertMode(), insertTable(), inTable(), newItem() (+6 more)

### Community 16 - "Notes"
Cohesion: 0.29
Nodes (7): Links and Backlinks, Notes, Org Format, Search, Syntax Highlighting, Table Formulas, Vim Keys

### Community 20 - "default.json"
Cohesion: 0.33
Nodes (5): description, identifier, permissions, $schema, windows

### Community 21 - "setup.ts"
Cohesion: 0.17
Nodes (20): answer(), app, browser, clickIn(), close(), Dirs, fileMatches(), find() (+12 more)

### Community 22 - "activity.rs"
Cohesion: 0.07
Nodes (48): AwEvent, browser_urls(), Bucket, Buckets, CEST, COMMON, cut(), dismiss() (+40 more)

### Community 23 - "export.rs"
Cohesion: 0.09
Nodes (46): BLOCK, Heading, List, Para, Pre, Quote, Rule, Table (+38 more)

### Community 24 - "notes.rs"
Cohesion: 0.10
Nodes (29): ANY_LINK, backlinks(), ensure_id(), ensures_ids(), files(), gen_id(), Graph, graph_neighbourhood() (+21 more)

### Community 25 - "Sync"
Cohesion: 0.67
Nodes (3): Emacs, Git, Sync

### Community 31 - "ai.rs"
Cohesion: 0.09
Nodes (11): CHAT_WAIT, CTX, IDLE, LOADED, MAX_CHAT, MAX_PROMPT, READ_TIMEOUT, RUNS (+3 more)

### Community 32 - "level"
Cohesion: 0.41
Nodes (13): drawerRange(), headingAt(), headings(), level(), line(), moveSubtree(), newHeading(), orgFold (+5 more)

### Community 33 - "lib/TaskDialog.svelte"
Cohesion: 0.17
Nodes (10): svelte, active, close(), closed(), duePreview, error, key(), save() (+2 more)

### Community 34 - "generate"
Cohesion: 0.16
Nodes (15): Backend, Embedded, Ollama, body(), chat(), chat_messages(), chats(), clean() (+7 more)

### Community 35 - "config.rs"
Cohesion: 0.10
Nodes (25): add_view(), add_views(), atomic_writes(), Config, defaults_round_trip(), expand(), recreate(), recreate_only_when_missing() (+17 more)

### Community 36 - "CLAUDE.md"
Cohesion: 0.17
Nodes (10): Agent Workflow, Build & Test, Code Style, Permission Mode, Project, Release & CI/CD, Repo Conventions, Tauri-Specific (+2 more)

### Community 37 - "screenshots.ts"
Cohesion: 0.12
Nodes (18): ADR-0007, ADR-0008, app, data, day(), driver, env, fixtures() (+10 more)

### Community 38 - "export_linked"
Cohesion: 0.60
Nodes (5): export_linked(), export_note(), export_path(), stem(), write_export()

### Community 39 - "day_prompt"
Cohesion: 0.19
Nodes (9): capped(), day_prompt(), dt(), embedded_drafts(), kept(), note_prompt(), prompts(), t() (+1 more)

### Community 40 - "release/SKILL.md"
Cohesion: 0.40
Nodes (4): Notes, Secrets required (in GitHub repo settings), Usage, What it does

### Community 41 - "OrgFile"
Cohesion: 0.18
Nodes (18): add_interval(), agenda(), all_tags(), Habit, Headline, Item, occurrences(), OrgFile (+10 more)

### Community 42 - "verify-all/SKILL.md"
Cohesion: 0.50
Nodes (3): Usage, What it does, When to use

### Community 43 - "org.rs"
Cohesion: 0.17
Nodes (16): ALIAS, capture_template(), CURSOR, DOC, expand_template(), HABIT_DAYS, LINK, PROP (+8 more)

### Community 45 - "coverage-ratchet.ts"
Cohesion: 0.25
Nodes (8): Baseline, Count, parseLcov(), pct(), regressions(), Report, Suite, summarize()

### Community 52 - "wlr.rs"
Cohesion: 0.07
Nodes (15): active(), apply(), Ev, App, Closed, State, Title, focused() (+7 more)

### Community 57 - "ollama"
Cohesion: 0.22
Nodes (9): read_err(), available_gb(), free_gb(), ollama(), ollama_chat(), parse(), read_chat(), unreachable() (+1 more)

### Community 60 - "ics.rs"
Cohesion: 0.19
Nodes (17): builds_feed(), calendar_from_a_url(), CALENDAR_TTL, date(), event(), events_on(), feed(), fetches_over_https() (+9 more)

### Community 61 - "decorate"
Cohesion: 0.13
Nodes (9): codeTokens(), decorate(), Glyph, imageSrc(), Img, inline(), langFor(), mark() (+1 more)

### Community 62 - "backup.rs"
Cohesion: 0.12
Nodes (36): active_profile_is_untracked_and_survives_the_pull(), activity_files_union_merge(), attributes_are_appended_once(), BACKUP, backup_syncs_the_data_repo(), clean_env(), commit(), commit_all() (+28 more)

### Community 63 - "r4.js"
Cohesion: 0.05
Nodes (26): agendaDays, backlinks, board, byTag, DEMO, DEMOS, done3, doneWeek (+18 more)

### Community 64 - "settings.ts"
Cohesion: 0.21
Nodes (15): changes(), checkTemplates(), checkViews(), Field, Form, fromForm(), GROUPS, needsMacPrompt() (+7 more)

### Community 65 - "triager.md"
Cohesion: 0.40
Nodes (4): Pass, Report (ends your run), Rules for what you post, Watch mode (only when asked)

### Community 67 - "remind.rs"
Cohesion: 0.22
Nodes (5): notify(), once_a_day(), picks_due_reminders(), start(), tick()

### Community 68 - "session.ts"
Cohesion: 0.25
Nodes (7): Review, Rules, Entry, KINDS, ok(), parseSession(), Session

### Community 69 - "super"
Cohesion: 0.18
Nodes (3): source(), start(), step()

### Community 70 - "b-variants.js"
Cohesion: 0.14
Nodes (29): b1(), b2(), b3(), b4(), b5(), bars(), blocks, card() (+21 more)

### Community 72 - "download"
Cohesion: 0.25
Nodes (7): chat_http(), download(), download_resumes_and_checks(), download_stall_times_out(), part(), serve(), sha256_file()

### Community 73 - "load"
Cohesion: 0.29
Nodes (6): cancel_draft(), cancel_hits_only_the_running_draft(), load(), run(), ticket(), whole()

### Community 74 - "scripts"
Cohesion: 0.05
Nodes (39): dependencies, @codemirror/commands, @codemirror/language, @codemirror/language-data, @codemirror/search, @codemirror/state, @codemirror/view, @lezer/highlight (+31 more)

### Community 76 - "path"
Cohesion: 0.24
Nodes (6): clean(), LINK, names_and_links(), normalize(), target(), unused()

### Community 77 - "merge.test.ts"
Cohesion: 0.42
Nodes (7): applyText(), Change, diffChange(), mergeEdits(), perKey(), apply(), merged()

### Community 78 - "lib.rs"
Cohesion: 0.06
Nodes (29): attachment_files(), attachment_files_skips_trash_and_unused_ignores_linked(), bad_meeting_pattern_and_calendar_only_warn(), CANCEL, capture_file_fills_the_pattern_under_notes(), clock_out_before_setup(), config(), dates_read_relative_to_a_given_today() (+21 more)

### Community 79 - "v0.3.0 - 2026-10-11"
Cohesion: 0.33
Nodes (5): Changelog, Features, Fixes, Other, v0.3.0 - 2026-10-11

### Community 80 - "clock.ts"
Cohesion: 0.28
Nodes (4): elapsed(), hm(), WeekDay, worked()

### Community 82 - "R"
Cohesion: 0.12
Nodes (27): activity_dismiss(), ai_note_prompt(), attach_bytes(), attach_file(), attach_in(), attach_in_numbers_clashes_retries_races_and_rolls_back(), capture_path(), capture_templates() (+19 more)

### Community 83 - "App"
Cohesion: 0.11
Nodes (42): activity_learn(), activity_rules_delete(), activity_rules_list(), ai_day_prompt(), App, backup_now(), day_done_now(), export_report() (+34 more)

### Community 84 - "dashboard"
Cohesion: 0.24
Nodes (8): dash_fixture(), dashboard(), dashboard_and_status_show_the_clock(), dashboard_builds_the_week(), dashboard_projects_month(), dashboard_week_runs_monday_to_sunday(), DashIn, status_json()

### Community 85 - "date"
Cohesion: 0.27
Nodes (13): agenda(), capture_file(), date(), date_at(), date_preview(), date_preview_at(), opt_date_at(), org_planning() (+5 more)

### Community 86 - "chatctx.ts"
Cohesion: 0.33
Nodes (6): onkeydown(), send(), chatContext(), cleanChat(), enc, level()

### Community 87 - "ship/SKILL.md"
Cohesion: 0.25
Nodes (7): 1. Survey, 2. Dispatch, 3. Review each report, 4. Clean up, 5. Report to the owner, Rules, Running it repeatedly

### Community 89 - "join"
Cohesion: 0.22
Nodes (20): archive(), capture_insert(), check_target(), heading_level(), headline_at(), insert_subtree(), is_planning(), join() (+12 more)

### Community 90 - "ref_node_assert"
Cohesion: 0.19
Nodes (8): BugEnv, bugReportUrl(), isError(), ISSUES, osName(), layout(), Pt, empty

### Community 91 - "mock.js"
Cohesion: 0.22
Nodes (8): habit, late, nav, q, row(), tags(), today, week

### Community 92 - "kw"
Cohesion: 0.19
Nodes (14): activity_suggestions(), capture_insert(), capture_template(), context_of(), edit(), keys(), kw(), org_context() (+6 more)

### Community 93 - "r5.js"
Cohesion: 0.32
Nodes (6): bar(), DEMOS5, layered(), P5, page(), worked

### Community 94 - "main"
Cohesion: 0.33
Nodes (5): Finish, House rules, Report (your final message), Start, main()

### Community 95 - "releaser.md"
Cohesion: 0.33
Nodes (5): 1. Gate (all must hold, else report which failed and stop), 2. Bump PR, 3. Tag, build, publish, Report, Rules

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

### Community 107 - "autopilot/SKILL.md"
Cohesion: 0.40
Nodes (4): Idle, Loop, Rules, Stop

### Community 109 - "board.test.ts"
Cohesion: 0.32
Nodes (5): Column, columns(), moveTo(), done, todo

### Community 110 - "activity.test.ts"
Cohesion: 0.32
Nodes (6): at(), buckets, day, ev(), events, y

### Community 111 - "AtomicU64"
Cohesion: 0.50
Nodes (3): CANCELLED, RUNNING, TICKETS

### Community 112 - "projects.ts"
Cohesion: 0.39
Nodes (6): merge(), Project, ROUNDINGS, Row, toProject(), toRow()

### Community 113 - "chips.test.ts"
Cohesion: 0.33
Nodes (3): Chip, chips(), items

### Community 114 - "paths.test.ts"
Cohesion: 0.60
Nodes (4): changed(), journalNeighbour(), mergeChanged(), relativeTo()

### Community 115 - "Cache"
Cohesion: 0.40
Nodes (3): Cache, org_paths(), scan()

## Knowledge Gaps
- **310 isolated node(s):** `changelog.sh script`, `q`, `sched`, `late`, `habit` (+305 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 533 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **39 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `at()` connect `activity.test.ts` to `+page.svelte`?**
  _High betweenness centrality (0.126) - this node is a cross-community bridge._
- **Why does `line()` connect `level` to `editor.ts`, `curLine`, `clock.ts`, `decorate`, `+page.svelte`?**
  _High betweenness centrality (0.088) - this node is a cross-community bridge._
- **Why does `insertTable()` connect `curLine` to `editor.ts`, `+page.svelte`?**
  _High betweenness centrality (0.070) - this node is a cross-community bridge._
- **What connects `changelog.sh script`, `q`, `sched` to the rest of the system?**
  _310 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `timeclock.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.05303225806451613 - nodes in this community are weakly interconnected._
- **Should `editor.ts` be split into smaller, more focused modules?**
  _Cohesion score 0.09057971014492754 - nodes in this community are weakly interconnected._
- **Should `[]` be split into smaller, more focused modules?**
  _Cohesion score 0.14492753623188406 - nodes in this community are weakly interconnected._