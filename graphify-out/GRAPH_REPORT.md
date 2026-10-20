# Graph Report - margin  (2026-10-20)

## Corpus Check
- 135 files · ~1,011,478 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 15 file(s) not represented in the graph (top: .css 8, (none) 4, .icns 1)

## Summary
- 1983 nodes · 4507 edges · 128 communities (91 shown, 37 thin omitted)
- Extraction: 98% EXTRACTED · 2% INFERRED · 0% AMBIGUOUS · INFERRED: 98 edges (avg confidence: 0.86)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `d77521c3`
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
- cfg
- GitHub Releases
- Value
- scripts
- Notes
- AppImage
- settings.ts
- lib/Menu.svelte
- default.json
- media.rs
- activity.rs
- export.rs
- notes.rs
- Sync
- +layout.ts
- build.rs
- +page.svelte
- margin
- ai.rs
- line
- lib/TaskDialog.svelte
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
- board.test.ts
- Configuration
- DMG
- Tauri Global Shortcut Plugin
- Windows Installer
- Journal
- ics.rs
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
- super
- triager.md
- extension.js
- coverage-ratchet.ts
- session.ts
- clock.ts
- b-variants.js
- devDependencies
- download
- load
- OrgFile
- ideator.md
- State
- merge.test.ts
- lib.rs
- Changelog
- ship/SKILL.md
- changelog.sh
- ref_node_assert
- dependencies
- chrono
- main
- releaser.md
- itemSwap
- autopilot/SKILL.md
- retrieve.rs
- mock.js
- scout.md
- r5.js
- NOTE_WRITES
- tray.rs
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
- Num
- decorate
- wlr.rs
- build.sh
- lib/Agenda.svelte
- projects.ts
- org.rs
- R
- donelog.ts
- App
- links.ts
- zoom.test.ts
- media-size.ts
- curLine
- footnote.ts
- whatsnew.test.ts
- activity.test.ts
- emphasis.test.ts
- chips.test.ts
- setopt.test.ts

## God Nodes (most connected - your core abstractions)
1. `App` - 104 edges
2. `cfg()` - 43 edges
3. `Tc` - 42 edges
4. `OrgFile` - 31 edges
5. `Kw` - 29 edges
6. `kw()` - 23 edges
7. `w()` - 21 edges
8. `reload_config()` - 20 edges
9. `line()` - 20 edges
10. `Calendar` - 19 edges

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

## Communities (128 total, 37 thin omitted)

### Community 0 - "screenshots.ts"
Cohesion: 0.13
Nodes (26): ADR-0007, ADR-0008, button(), clip(), clips, config, day(), fixtures() (+18 more)

### Community 1 - "timeclock.rs"
Cohesion: 0.05
Nodes (83): add_past_sessions(), adjust_start_refuses_to_overlap_the_previous_session(), append_diary(), apply_carry(), breaks_accumulate_and_flex(), Calendar, clock_flow_on_disk(), clock_out_earlier_refuses_before_the_start() (+75 more)

### Community 2 - "AppHandle"
Cohesion: 0.14
Nodes (24): ai_chat(), ai_download(), ai_draft(), ai_model_delete(), ai_models(), ai_set(), apply_config(), apply_spellcheck() (+16 more)

### Community 3 - "editor.ts"
Cohesion: 0.06
Nodes (29): Block, BULLETS, clicks, hide, hooks, LANG_ALIAS, langLoaded, langs (+21 more)

### Community 5 - "watcher.rs"
Cohesion: 0.12
Nodes (28): active(), active_window(), AFK_MINS, append(), clean(), exclusion_and_truncation_before_write(), flush(), flush_one() (+20 more)

### Community 6 - "orgtable.ts"
Cohesion: 0.10
Nodes (38): align(), applyFormulas(), cells(), colRef(), create(), deleteCol(), deleteRow(), done() (+30 more)

### Community 7 - "tauri.conf.json"
Cohesion: 0.06
Nodes (34): app, security, windows, enable, scope, build, beforeBuildCommand, beforeDevCommand (+26 more)

### Community 8 - "package.json"
Cohesion: 0.06
Nodes (29): description, engines, node, license, name, type, version, @codemirror/commands (+21 more)

### Community 9 - "compilerOptions"
Cohesion: 0.14
Nodes (13): ./.svelte-kit/tsconfig.json, compilerOptions, allowImportingTsExtensions, allowJs, checkJs, esModuleInterop, forceConsistentCasingInFileNames, moduleResolution (+5 more)

### Community 10 - "lib/FormulaDialog.svelte"
Cohesion: 0.10
Nodes (22): active, close(), col, commit(), draft, error, expr, fm (+14 more)

### Community 11 - "Calendar"
Cohesion: 0.67
Nodes (3): Calendar, iCalendar, Tasks

### Community 12 - "cfg"
Cohesion: 0.10
Nodes (18): ai_excluded(), ai_note_prompt(), attach_bytes(), attach_file(), attach_in(), attach_in_numbers_clashes_retries_races_and_rolls_back(), bad_meeting_pattern_and_calendar_only_warn(), capture_templates() (+10 more)

### Community 14 - "Value"
Cohesion: 0.17
Nodes (12): context_of(), dash_fixture(), dashboard(), dashboard_and_status_show_the_clock(), dashboard_builds_the_week(), dashboard_projects_month(), dashboard_week_runs_monday_to_sunday(), DashIn (+4 more)

### Community 15 - "scripts"
Cohesion: 0.13
Nodes (15): scripts, build, check, check:watch, coverage, coverage:ratchet, dev, e2e:build (+7 more)

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

### Community 21 - "media.rs"
Cohesion: 0.07
Nodes (32): clean(), images_share_the_note_folders_images_dir(), is_image(), LINK, names_and_links(), normalize(), target(), unused() (+24 more)

### Community 22 - "activity.rs"
Cohesion: 0.07
Nodes (47): AwEvent, browser_urls(), Bucket, Buckets, CEST, COMMON, cut(), dismiss() (+39 more)

### Community 23 - "export.rs"
Cohesion: 0.09
Nodes (48): BLOCK, Heading, List, Para, Pre, Quote, Rule, Table (+40 more)

### Community 24 - "notes.rs"
Cohesion: 0.11
Nodes (19): ANY_LINK, backlinks(), ensure_id(), ensures_ids(), gen_id(), Hit, links_and_backlinks(), MAX_NODES (+11 more)

### Community 25 - "Sync"
Cohesion: 0.67
Nodes (3): Emacs, Git, Sync

### Community 29 - "+page.svelte"
Cohesion: 0.05
Nodes (6): cmd(), edit(), done(), note(), priority(), state()

### Community 31 - "ai.rs"
Cohesion: 0.09
Nodes (11): CHAT_WAIT, CTX, IDLE, LOADED, MAX_CHAT, MAX_PROMPT, READ_TIMEOUT, RUNS (+3 more)

### Community 32 - "line"
Cohesion: 0.25
Nodes (18): drawerRange(), findTable(), headingAt(), headings(), level(), line(), moveSubtree(), newHeading() (+10 more)

### Community 33 - "lib/TaskDialog.svelte"
Cohesion: 0.12
Nodes (15): svelte, preview, active, close(), [hc, hr], key(), active, close() (+7 more)

### Community 34 - "generate"
Cohesion: 0.15
Nodes (15): Backend, Embedded, Ollama, body(), chat(), chat_messages(), chats(), clean() (+7 more)

### Community 35 - "config.rs"
Cohesion: 0.09
Nodes (28): add_view(), add_views(), atomic_writes(), Config, defaults_round_trip(), expand(), extra_days_off(), public_holidays() (+20 more)

### Community 36 - "CLAUDE.md"
Cohesion: 0.17
Nodes (10): Agent Workflow, Build & Test, Code Style, Permission Mode, Project, Release & CI/CD, Repo Conventions, Tauri-Specific (+2 more)

### Community 37 - "kw"
Cohesion: 0.28
Nodes (15): activity_suggestions(), agenda(), ai_retrieve(), export_html_fragment(), export_linked(), export_markdown(), export_note(), export_path() (+7 more)

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

### Community 45 - "board.test.ts"
Cohesion: 0.32
Nodes (5): Column, columns(), moveTo(), done, todo

### Community 52 - "ics.rs"
Cohesion: 0.16
Nodes (18): builds_feed(), CALENDAR, calendar_from_a_url(), CALENDAR_TTL, date(), event(), events_on(), feed() (+10 more)

### Community 57 - "ollama"
Cohesion: 0.18
Nodes (11): available_gb(), cancel_draft(), cancel_hits_only_the_running_draft(), free_gb(), ollama(), ollama_chat(), parse(), read_chat() (+3 more)

### Community 60 - "remind.rs"
Cohesion: 0.17
Nodes (6): Due, notify(), once_a_day(), picks_due_reminders(), start(), tick()

### Community 61 - "setup.ts"
Cohesion: 0.16
Nodes (27): answer(), app, browser, clickIn(), close(), Dirs, fileMatches(), find() (+19 more)

### Community 62 - "backup.rs"
Cohesion: 0.12
Nodes (36): active_profile_is_untracked_and_survives_the_pull(), activity_files_union_merge(), attributes_are_appended_once(), BACKUP, backup_syncs_the_data_repo(), clean_env(), commit(), commit_all() (+28 more)

### Community 63 - "r4.js"
Cohesion: 0.05
Nodes (26): agendaDays, backlinks, board, byTag, DEMO, DEMOS, done3, doneWeek (+18 more)

### Community 64 - "super"
Cohesion: 0.20
Nodes (3): source(), start(), step()

### Community 65 - "triager.md"
Cohesion: 0.40
Nodes (4): Pass, Report (ends your run), Rules for what you post, Watch mode (only when asked)

### Community 67 - "coverage-ratchet.ts"
Cohesion: 0.25
Nodes (8): Baseline, Count, parseLcov(), pct(), regressions(), Report, Suite, summarize()

### Community 68 - "session.ts"
Cohesion: 0.25
Nodes (7): Review, Rules, Entry, KINDS, ok(), parseSession(), Session

### Community 69 - "clock.ts"
Cohesion: 0.31
Nodes (7): dayTime(), inactiveStamp(), naive(), p2(), parseWhen(), WeekDay, worked()

### Community 70 - "b-variants.js"
Cohesion: 0.14
Nodes (29): b1(), b2(), b3(), b4(), b5(), bars(), blocks, card() (+21 more)

### Community 71 - "devDependencies"
Cohesion: 0.18
Nodes (11): devDependencies, svelte, svelte-check, @sveltejs/adapter-static, @sveltejs/kit, @sveltejs/vite-plugin-svelte, @tauri-apps/cli, @types/node (+3 more)

### Community 72 - "download"
Cohesion: 0.25
Nodes (7): chat_http(), download(), download_resumes_and_checks(), download_stall_times_out(), part(), serve(), sha256_file()

### Community 73 - "load"
Cohesion: 0.25
Nodes (6): CANCELLED, load(), run(), RUNNING, TICKETS, whole()

### Community 74 - "OrgFile"
Cohesion: 0.16
Nodes (18): adjacency(), files(), Graph, graph_neighbourhood(), GraphNode, Mention, move_note(), move_to() (+10 more)

### Community 77 - "merge.test.ts"
Cohesion: 0.42
Nodes (7): applyText(), Change, diffChange(), mergeEdits(), perKey(), apply(), merged()

### Community 78 - "lib.rs"
Cohesion: 0.05
Nodes (32): attachment_files(), attachment_files_skips_trash_and_unused_ignores_linked(), CANCEL, capture_file_fills_the_pattern_under_notes(), csv_dest(), csv_target(), dates_read_relative_to_a_given_today(), DOWNLOADING (+24 more)

### Community 79 - "Changelog"
Cohesion: 0.14
Nodes (13): Changelog, Features, Features, Features, Fixes, Fixes, Fixes, Other (+5 more)

### Community 80 - "ship/SKILL.md"
Cohesion: 0.25
Nodes (7): 1. Survey, 2. Dispatch, 3. Review each report, 4. Clean up, 5. Report to the owner, Rules, Running it repeatedly

### Community 82 - "ref_node_assert"
Cohesion: 0.11
Nodes (13): BugEnv, bugReportUrl(), isError(), ISSUES, osName(), script, layout(), Pt (+5 more)

### Community 83 - "dependencies"
Cohesion: 0.15
Nodes (13): dependencies, @codemirror/commands, @codemirror/language, @codemirror/language-data, @codemirror/search, @codemirror/state, @codemirror/view, @lezer/highlight (+5 more)

### Community 85 - "main"
Cohesion: 0.33
Nodes (5): Finish, House rules, Report (your final message), Start, main()

### Community 86 - "releaser.md"
Cohesion: 0.33
Nodes (5): 1. Gate (all must hold, else report which failed and stop), 2. Bump PR, 3. Tag, build, publish, Report, Rules

### Community 87 - "itemSwap"
Cohesion: 0.42
Nodes (7): moveItem(), indent(), isItem(), itemSwap(), renumber(), fix(), swap()

### Community 89 - "autopilot/SKILL.md"
Cohesion: 0.40
Nodes (4): Idle, Loop, Rules, Stop

### Community 90 - "retrieve.rs"
Cohesion: 0.11
Nodes (27): APP, app_source(), app_sources(), app_sources_and_mixed_packing(), BUDGET, cost(), cut(), EXCERPT (+19 more)

### Community 91 - "mock.js"
Cohesion: 0.22
Nodes (8): habit, late, nav, q, row(), tags(), today, week

### Community 93 - "r5.js"
Cohesion: 0.32
Nodes (6): bar(), DEMOS5, layered(), P5, page(), worked

### Community 95 - "tray.rs"
Cohesion: 0.27
Nodes (5): refresh(), setup(), show(), tracking(), Tray

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

### Community 108 - "decorate"
Cohesion: 0.13
Nodes (9): codeTokens(), decorate(), Glyph, imageSrc(), Img, inline(), langFor(), mark() (+1 more)

### Community 109 - "wlr.rs"
Cohesion: 0.12
Nodes (12): active(), apply(), Ev, App, Closed, State, Title, focused() (+4 more)

### Community 111 - "lib/Agenda.svelte"
Cohesion: 0.15
Nodes (10): dayInput(), key(), op(), pointer(), setBoard(), shift(), refresh(), title() (+2 more)

### Community 112 - "projects.ts"
Cohesion: 0.24
Nodes (10): Card, cards(), merge(), Project, ROUNDINGS, Row, Task, toProject() (+2 more)

### Community 113 - "org.rs"
Cohesion: 0.05
Nodes (100): add_interval(), add_note(), agenda(), agenda_week(), ALIAS, archive(), at(), Cache (+92 more)

### Community 114 - "R"
Cohesion: 0.10
Nodes (38): append_journal_template(), capture_file(), capture_path(), check_in(), date(), date_at(), date_preview(), date_preview_at() (+30 more)

### Community 115 - "donelog.ts"
Cohesion: 0.43
Nodes (4): at(), doneLog(), It, withLog()

### Community 116 - "App"
Cohesion: 0.10
Nodes (45): activity_dismiss(), activity_learn(), activity_rules_delete(), activity_rules_list(), ai_day_prompt(), App, backup_now(), capture_insert() (+37 more)

### Community 117 - "links.ts"
Cohesion: 0.47
Nodes (7): bareTitle(), esc(), findTarget(), head(), linkHeadings(), norm(), splitTarget()

### Community 118 - "zoom.test.ts"
Cohesion: 0.67
Nodes (4): clamp(), nextZoom(), parseZoom(), wheelZoom()

### Community 119 - "media-size.ts"
Cohesion: 0.31
Nodes (7): DIRS, EACH, GIFS, mb(), problems(), scan(), TOTAL

### Community 120 - "curLine"
Cohesion: 0.18
Nodes (17): createState(), curLine(), emphasis(), expandTemplate(), insertBlock(), insertMode(), insertTable(), inTable() (+9 more)

### Community 121 - "footnote.ts"
Cohesion: 0.29
Nodes (8): followFootnote(), insertFootnote(), recordJump(), addFootnote(), FN, footnoteTarget(), nextLabel(), add()

### Community 122 - "whatsnew.test.ts"
Cohesion: 0.43
Nodes (4): Notes, notesText(), notesTitle(), notesToShow()

### Community 123 - "activity.test.ts"
Cohesion: 0.28
Nodes (6): at(), buckets, day, ev(), events, y

### Community 124 - "emphasis.test.ts"
Cohesion: 0.60
Nodes (3): EMPH, run(), toggleEmphasis()

### Community 125 - "chips.test.ts"
Cohesion: 0.33
Nodes (3): Chip, chips(), items

### Community 126 - "setopt.test.ts"
Cohesion: 0.50
Nodes (3): applySet(), NumOpts, off

## Knowledge Gaps
- **364 isolated node(s):** `changelog.sh script`, `q`, `sched`, `late`, `habit` (+359 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 626 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **37 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `start()` connect `setup.ts` to `+page.svelte`, `lib/Agenda.svelte`?**
  _High betweenness centrality (0.137) - this node is a cross-community bridge._
- **Why does `line()` connect `line` to `editor.ts`, `decorate`, `lib/Agenda.svelte`, `itemSwap`, `curLine`, `+page.svelte`?**
  _High betweenness centrality (0.063) - this node is a cross-community bridge._
- **Why does `view()` connect `frag` to `+page.svelte`, `r4.js`?**
  _High betweenness centrality (0.043) - this node is a cross-community bridge._
- **What connects `changelog.sh script`, `q`, `sched` to the rest of the system?**
  _364 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `screenshots.ts` be split into smaller, more focused modules?**
  _Cohesion score 0.12962962962962962 - nodes in this community are weakly interconnected._
- **Should `timeclock.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.051412250079339895 - nodes in this community are weakly interconnected._
- **Should `AppHandle` be split into smaller, more focused modules?**
  _Cohesion score 0.13675213675213677 - nodes in this community are weakly interconnected._