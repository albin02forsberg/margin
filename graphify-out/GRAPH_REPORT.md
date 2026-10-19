# Graph Report - margin  (2026-10-19)

## Corpus Check
- 127 files · ~928,388 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 15 file(s) not represented in the graph (top: .css 8, (none) 4, .icns 1)

## Summary
- 1841 nodes · 4163 edges · 119 communities (79 shown, 40 thin omitted)
- Extraction: 98% EXTRACTED · 2% INFERRED · 0% AMBIGUOUS · INFERRED: 82 edges (avg confidence: 0.87)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `b645a4f2`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- State
- timeclock.rs
- R
- editor.ts
- watcher.rs
- orgtable.ts
- tauri.conf.json
- package.json
- compilerOptions
- media.rs
- Calendar
- ics.rs
- GitHub Releases
- today
- scripts
- Notes
- AppImage
- settings.ts
- lib/Menu.svelte
- default.json
- retrieve.rs
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
- main
- tray.rs
- quit.rs
- day_prompt
- release/SKILL.md
- paths.test.ts
- verify-all/SKILL.md
- chatctx.ts
- App Icon
- Value
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
- remind.rs
- setup.ts
- backup.rs
- r4.js
- lib/Agenda.svelte
- triager.md
- extension.js
- super
- session.ts
- clock.ts
- b-variants.js
- gnome.rs
- download
- load
- path
- ideator.md
- chrono
- merge.test.ts
- lib.rs
- v0.3.0 - 2026-10-11
- board.test.ts
- changelog.sh
- ref_node_assert
- dependencies
- bugreport.test.ts
- Glyph
- graph.test.ts
- itemSwap
- curLine
- zoom.test.ts
- mock.js
- links.test.ts
- r5.js
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
- emphasis.test.ts
- Img
- build.sh
- projects.ts
- org.rs
- cfg
- donelog.ts
- App
- .files
- chips.test.ts
- Num
- setopt.test.ts

## God Nodes (most connected - your core abstractions)
1. `App` - 101 edges
2. `cfg()` - 43 edges
3. `Tc` - 41 edges
4. `OrgFile` - 31 edges
5. `Kw` - 28 edges
6. `kw()` - 20 edges
7. `reload_config()` - 19 edges
8. `Event` - 18 edges
9. `csv()` - 18 edges
10. `level()` - 18 edges

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

## Communities (119 total, 40 thin omitted)

### Community 1 - "timeclock.rs"
Cohesion: 0.05
Nodes (78): add_past_sessions(), adjust_start_refuses_to_overlap_the_previous_session(), append_diary(), apply_carry(), breaks_accumulate_and_flex(), clock_out_earlier_refuses_before_the_start(), csv(), csv_field() (+70 more)

### Community 2 - "R"
Cohesion: 0.18
Nodes (23): ai_chat(), ai_download(), ai_draft(), ai_model_delete(), ai_models(), ai_set(), apply_config(), capture() (+15 more)

### Community 3 - "editor.ts"
Cohesion: 0.07
Nodes (23): Block, BULLETS, clicks, hide, hooks, LANG_ALIAS, langLoaded, langs (+15 more)

### Community 5 - "watcher.rs"
Cohesion: 0.13
Nodes (27): active(), active_window(), AFK_MINS, append(), clean(), flush(), flush_one(), flush_writes_the_open_record_once() (+19 more)

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

### Community 10 - "media.rs"
Cohesion: 0.19
Nodes (14): any(), AudioObjectGetPropertyData(), AudioObjectGetPropertyDataSize(), cc(), CMIOObjectGetPropertyData(), CMIOObjectGetPropertyDataSize(), GLOBAL, in_use() (+6 more)

### Community 11 - "Calendar"
Cohesion: 0.67
Nodes (3): Calendar, iCalendar, Tasks

### Community 12 - "ics.rs"
Cohesion: 0.16
Nodes (18): builds_feed(), CALENDAR, calendar_from_a_url(), CALENDAR_TTL, date(), event(), events_on(), feed() (+10 more)

### Community 14 - "today"
Cohesion: 0.14
Nodes (23): activity_suggestions(), append_journal_template(), bad_meeting_pattern_and_calendar_only_warn(), capture_file(), capture_path(), date(), date_at(), date_preview() (+15 more)

### Community 15 - "scripts"
Cohesion: 0.14
Nodes (14): scripts, build, check, check:watch, coverage, coverage:ratchet, dev, e2e:build (+6 more)

### Community 16 - "Notes"
Cohesion: 0.29
Nodes (7): Links and Backlinks, Notes, Org Format, Search, Syntax Highlighting, Table Formulas, Vim Keys

### Community 18 - "settings.ts"
Cohesion: 0.18
Nodes (16): CATEGORIES, changes(), checkTemplates(), checkViews(), Field, Form, fromForm(), GROUPS (+8 more)

### Community 19 - "lib/Menu.svelte"
Cohesion: 0.29
Nodes (4): active, node, show(), filter()

### Community 20 - "default.json"
Cohesion: 0.33
Nodes (5): description, identifier, permissions, $schema, windows

### Community 21 - "retrieve.rs"
Cohesion: 0.11
Nodes (27): APP, app_source(), app_sources(), app_sources_and_mixed_packing(), BUDGET, cost(), cut(), EXCERPT (+19 more)

### Community 22 - "activity.rs"
Cohesion: 0.07
Nodes (48): AwEvent, browser_urls(), Bucket, Buckets, CEST, COMMON, cut(), dismiss() (+40 more)

### Community 23 - "export.rs"
Cohesion: 0.09
Nodes (46): BLOCK, Heading, List, Para, Pre, Quote, Rule, Table (+38 more)

### Community 24 - "notes.rs"
Cohesion: 0.08
Nodes (34): adjacency(), ANY_LINK, backlinks(), ensure_id(), ensures_ids(), files(), gen_id(), Graph (+26 more)

### Community 25 - "Sync"
Cohesion: 0.67
Nodes (3): Emacs, Git, Sync

### Community 31 - "ai.rs"
Cohesion: 0.09
Nodes (11): CHAT_WAIT, CTX, IDLE, LOADED, MAX_CHAT, MAX_PROMPT, READ_TIMEOUT, RUNS (+3 more)

### Community 32 - "level"
Cohesion: 0.25
Nodes (18): codeTokens(), decorate(), drawerRange(), headingAt(), headings(), langFor(), level(), line() (+10 more)

### Community 33 - "lib/TaskDialog.svelte"
Cohesion: 0.17
Nodes (11): preview, svelte, active, close(), closed(), duePreview, error, key() (+3 more)

### Community 34 - "generate"
Cohesion: 0.15
Nodes (15): Backend, Embedded, Ollama, body(), chat(), chat_messages(), chats(), clean() (+7 more)

### Community 35 - "config.rs"
Cohesion: 0.10
Nodes (25): add_view(), add_views(), atomic_writes(), Config, defaults_round_trip(), expand(), recreate(), recreate_only_when_missing() (+17 more)

### Community 36 - "main"
Cohesion: 0.05
Nodes (33): Agent Workflow, Finish, House rules, Report (your final message), Start, 1. Gate (all must hold, else report which failed and stop), 2. Bump PR, 3. Tag, build, publish (+25 more)

### Community 37 - "tray.rs"
Cohesion: 0.27
Nodes (5): refresh(), setup(), show(), tracking(), Tray

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
Cohesion: 0.44
Nodes (7): changed(), folderName(), journalNeighbour(), mergeChanged(), noteFolders(), relativeTo(), resolvePath()

### Community 42 - "verify-all/SKILL.md"
Cohesion: 0.50
Nodes (3): Usage, What it does, When to use

### Community 43 - "chatctx.ts"
Cohesion: 0.43
Nodes (4): chatContext(), cleanChat(), enc, level()

### Community 45 - "Value"
Cohesion: 0.15
Nodes (13): context_of(), dash_fixture(), dashboard(), dashboard_and_status_show_the_clock(), dashboard_builds_the_week(), dashboard_projects_month(), dashboard_week_runs_monday_to_sunday(), DashIn (+5 more)

### Community 52 - "wlr.rs"
Cohesion: 0.12
Nodes (12): active(), apply(), Ev, App, Closed, State, Title, focused() (+4 more)

### Community 57 - "ollama"
Cohesion: 0.18
Nodes (11): available_gb(), cancel_draft(), cancel_hits_only_the_running_draft(), free_gb(), ollama(), ollama_chat(), parse(), read_chat() (+3 more)

### Community 60 - "remind.rs"
Cohesion: 0.20
Nodes (5): notify(), once_a_day(), picks_due_reminders(), start(), tick()

### Community 61 - "setup.ts"
Cohesion: 0.06
Nodes (58): ADR-0007, ADR-0008, at(), buckets, day, ev(), events, y (+50 more)

### Community 62 - "backup.rs"
Cohesion: 0.12
Nodes (36): active_profile_is_untracked_and_survives_the_pull(), activity_files_union_merge(), attributes_are_appended_once(), BACKUP, backup_syncs_the_data_repo(), clean_env(), commit(), commit_all() (+28 more)

### Community 63 - "r4.js"
Cohesion: 0.05
Nodes (26): agendaDays, backlinks, board, byTag, DEMO, DEMOS, done3, doneWeek (+18 more)

### Community 64 - "lib/Agenda.svelte"
Cohesion: 0.14
Nodes (11): dayInput(), key(), op(), pointer(), setBoard(), shift(), refresh(), priority() (+3 more)

### Community 65 - "triager.md"
Cohesion: 0.40
Nodes (4): Pass, Report (ends your run), Rules for what you post, Watch mode (only when asked)

### Community 67 - "super"
Cohesion: 0.20
Nodes (3): source(), start(), step()

### Community 68 - "session.ts"
Cohesion: 0.25
Nodes (7): Review, Rules, Entry, KINDS, ok(), parseSession(), Session

### Community 69 - "clock.ts"
Cohesion: 0.31
Nodes (6): dayTime(), naive(), p2(), parseWhen(), WeekDay, worked()

### Community 70 - "b-variants.js"
Cohesion: 0.14
Nodes (29): b1(), b2(), b3(), b4(), b5(), bars(), blocks, card() (+21 more)

### Community 71 - "gnome.rs"
Cohesion: 0.20
Nodes (10): active(), BUS, FILES, get(), install(), install_copies_the_extension(), parse(), session() (+2 more)

### Community 72 - "download"
Cohesion: 0.25
Nodes (7): chat_http(), download(), download_resumes_and_checks(), download_stall_times_out(), part(), serve(), sha256_file()

### Community 73 - "load"
Cohesion: 0.25
Nodes (6): CANCELLED, load(), run(), RUNNING, TICKETS, whole()

### Community 74 - "path"
Cohesion: 0.24
Nodes (6): clean(), LINK, names_and_links(), normalize(), target(), unused()

### Community 77 - "merge.test.ts"
Cohesion: 0.42
Nodes (7): applyText(), Change, diffChange(), mergeEdits(), perKey(), apply(), merged()

### Community 78 - "lib.rs"
Cohesion: 0.05
Nodes (29): attach_bytes(), attach_file(), attach_in(), attach_in_numbers_clashes_retries_races_and_rolls_back(), attachment_files(), attachment_files_skips_trash_and_unused_ignores_linked(), CANCEL, capture_file_fills_the_pattern_under_notes() (+21 more)

### Community 79 - "v0.3.0 - 2026-10-11"
Cohesion: 0.20
Nodes (9): Changelog, Features, Features, Fixes, Fixes, Other, Other, v0.3.0 - 2026-10-11 (+1 more)

### Community 80 - "board.test.ts"
Cohesion: 0.32
Nodes (5): Column, columns(), moveTo(), done, todo

### Community 82 - "ref_node_assert"
Cohesion: 0.24
Nodes (5): empty, asUrl(), linkFor(), wordCount(), wordsLabel()

### Community 83 - "dependencies"
Cohesion: 0.08
Nodes (24): dependencies, @codemirror/commands, @codemirror/language, @codemirror/language-data, @codemirror/search, @codemirror/state, @codemirror/view, @lezer/highlight (+16 more)

### Community 84 - "bugreport.test.ts"
Cohesion: 0.48
Nodes (5): BugEnv, bugReportUrl(), isError(), ISSUES, osName()

### Community 87 - "itemSwap"
Cohesion: 0.50
Nodes (6): indent(), isItem(), itemSwap(), renumber(), fix(), swap()

### Community 89 - "curLine"
Cohesion: 0.16
Nodes (19): createState(), curLine(), emphasis(), expandTemplate(), insertBlock(), insertMode(), insertTable(), inTable() (+11 more)

### Community 90 - "zoom.test.ts"
Cohesion: 0.67
Nodes (4): clamp(), nextZoom(), parseZoom(), wheelZoom()

### Community 91 - "mock.js"
Cohesion: 0.22
Nodes (8): habit, late, nav, q, row(), tags(), today, week

### Community 93 - "r5.js"
Cohesion: 0.32
Nodes (6): bar(), DEMOS5, layered(), P5, page(), worked

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

### Community 107 - "emphasis.test.ts"
Cohesion: 0.60
Nodes (3): EMPH, run(), toggleEmphasis()

### Community 108 - "Img"
Cohesion: 0.29
Nodes (4): imageSrc(), Img, inline(), mark()

### Community 112 - "projects.ts"
Cohesion: 0.24
Nodes (10): Card, cards(), merge(), Project, ROUNDINGS, Row, Task, toProject() (+2 more)

### Community 113 - "org.rs"
Cohesion: 0.05
Nodes (95): add_interval(), agenda(), agenda_week(), ALIAS, all_tags(), archive(), at(), Cache (+87 more)

### Community 114 - "cfg"
Cohesion: 0.10
Nodes (24): activity_dismiss(), ai_note_prompt(), cfg(), check_in(), config(), dismissed(), file_exists(), in_notes() (+16 more)

### Community 115 - "donelog.ts"
Cohesion: 0.43
Nodes (4): at(), doneLog(), It, withLog()

### Community 116 - "App"
Cohesion: 0.12
Nodes (37): activity_learn(), activity_rules_delete(), activity_rules_list(), ai_day_prompt(), ai_excluded(), App, backup_now(), capture_template() (+29 more)

### Community 117 - ".files"
Cohesion: 0.12
Nodes (25): agenda(), ai_retrieve(), capture_insert(), edit(), export_linked(), export_note(), export_path(), kw() (+17 more)

### Community 118 - "chips.test.ts"
Cohesion: 0.33
Nodes (3): Chip, chips(), items

### Community 121 - "setopt.test.ts"
Cohesion: 0.50
Nodes (3): applySet(), NumOpts, off

## Knowledge Gaps
- **335 isolated node(s):** `changelog.sh script`, `q`, `sched`, `late`, `habit` (+330 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 585 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **40 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `start()` connect `setup.ts` to `lib/Agenda.svelte`, `+page.svelte`?**
  _High betweenness centrality (0.098) - this node is a cross-community bridge._
- **Why does `line()` connect `level` to `lib/Agenda.svelte`, `curLine`, `editor.ts`, `+page.svelte`?**
  _High betweenness centrality (0.080) - this node is a cross-community bridge._
- **Why does `insertTable()` connect `curLine` to `editor.ts`, `+page.svelte`?**
  _High betweenness centrality (0.061) - this node is a cross-community bridge._
- **What connects `changelog.sh script`, `q`, `sched` to the rest of the system?**
  _335 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `timeclock.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.0518783542039356 - nodes in this community are weakly interconnected._
- **Should `editor.ts` be split into smaller, more focused modules?**
  _Cohesion score 0.07389162561576355 - nodes in this community are weakly interconnected._
- **Should `watcher.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.12941176470588237 - nodes in this community are weakly interconnected._