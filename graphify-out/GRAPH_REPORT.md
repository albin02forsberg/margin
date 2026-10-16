# Graph Report - margin  (2026-10-16)

## Corpus Check
- 114 files · ~917,698 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 15 file(s) not represented in the graph (top: .css 8, (none) 4, .icns 1)

## Summary
- 1743 nodes · 3911 edges · 104 communities (68 shown, 36 thin omitted)
- Extraction: 98% EXTRACTED · 2% INFERRED · 0% AMBIGUOUS · INFERRED: 72 edges (avg confidence: 0.87)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `faf3a46f`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- at
- timeclock.rs
- AppHandle
- editor.ts
- watcher.rs
- orgtable.ts
- tauri.conf.json
- package.json
- compilerOptions
- Calendar
- ics.rs
- GitHub Releases
- devDependencies
- level
- Notes
- AppImage
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
- lib/TaskDialog.svelte
- generate
- config.rs
- main
- media.rs
- remind.rs
- day_prompt
- release/SKILL.md
- paths.test.ts
- verify-all/SKILL.md
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
- Glyph
- backup.rs
- r4.js
- settings.ts
- triager.md
- extension.js
- svelte
- session.ts
- b-variants.js
- download
- load
- scripts
- ideator.md
- merge.test.ts
- lib.rs
- v0.3.0 - 2026-10-11
- screenshots.ts
- changelog.sh
- chips.test.ts
- App
- ref_node_assert
- R
- dependencies
- clock.ts
- donelog.ts
- mock.js
- r5.js
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
- NOTE_WRITES
- lib/Agenda.svelte
- build.sh
- projects.ts
- org.rs
- kw

## God Nodes (most connected - your core abstractions)
1. `App` - 100 edges
2. `Tc` - 41 edges
3. `cfg()` - 40 edges
4. `OrgFile` - 32 edges
5. `Kw` - 27 edges
6. `reload_config()` - 19 edges
7. `kw()` - 19 edges
8. `Event` - 18 edges
9. `csv()` - 18 edges
10. `level()` - 18 edges

## Surprising Connections (you probably didn't know these)
- `1. Gate (all must hold, else report which failed and stop)` --references--> `main()`  [INFERRED]
  .claude/agents/releaser.md → scripts/coverage-ratchet.ts
- `Agent Workflow` --references--> `main()`  [INFERRED]
  CLAUDE.md → scripts/coverage-ratchet.ts
- `Finish` --references--> `main()`  [INFERRED]
  .claude/agents/implementer.md → scripts/coverage-ratchet.ts
- `House rules` --references--> `main()`  [INFERRED]
  .claude/agents/implementer.md → scripts/coverage-ratchet.ts
- `Run` --references--> `main()`  [INFERRED]
  .claude/agents/scout.md → scripts/coverage-ratchet.ts

## Import Cycles
- 1-file cycle: `src-tauri/src/ai.rs -> src-tauri/src/ai.rs`

## Communities (104 total, 36 thin omitted)

### Community 0 - "at"
Cohesion: 0.67
Nodes (3): at(), buckets, ev()

### Community 1 - "timeclock.rs"
Cohesion: 0.05
Nodes (77): add_past_sessions(), adjust_start_refuses_to_overlap_the_previous_session(), append_diary(), apply_carry(), breaks_accumulate_and_flex(), clock_out_earlier_refuses_before_the_start(), csv(), csv_field() (+69 more)

### Community 2 - "AppHandle"
Cohesion: 0.12
Nodes (26): ai_chat(), ai_download(), ai_draft(), ai_model_delete(), ai_models(), ai_set(), apply_config(), capture() (+18 more)

### Community 3 - "editor.ts"
Cohesion: 0.07
Nodes (28): Block, BULLETS, clicks, codeTokens(), decorate(), hide, hooks, imageSrc() (+20 more)

### Community 5 - "watcher.rs"
Cohesion: 0.12
Nodes (28): active(), active_window(), AFK_MINS, append(), clean(), exclusion_and_truncation_before_write(), flush(), flush_one() (+20 more)

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

### Community 12 - "ics.rs"
Cohesion: 0.07
Nodes (34): clean(), LINK, names_and_links(), normalize(), target(), unused(), builds_feed(), CALENDAR (+26 more)

### Community 14 - "devDependencies"
Cohesion: 0.18
Nodes (11): devDependencies, svelte, svelte-check, @sveltejs/adapter-static, @sveltejs/kit, @sveltejs/vite-plugin-svelte, @tauri-apps/cli, @types/node (+3 more)

### Community 15 - "level"
Cohesion: 0.18
Nodes (27): createState(), curLine(), drawerRange(), expandTemplate(), headingAt(), headings(), insertBlock(), insertMode() (+19 more)

### Community 16 - "Notes"
Cohesion: 0.29
Nodes (7): Links and Backlinks, Notes, Org Format, Search, Syntax Highlighting, Table Formulas, Vim Keys

### Community 19 - "lib/Menu.svelte"
Cohesion: 0.29
Nodes (4): active, node, show(), filter()

### Community 20 - "default.json"
Cohesion: 0.33
Nodes (5): description, identifier, permissions, $schema, windows

### Community 21 - "setup.ts"
Cohesion: 0.15
Nodes (24): day, events, y, answer(), app, browser, clickIn(), close() (+16 more)

### Community 22 - "activity.rs"
Cohesion: 0.07
Nodes (47): AwEvent, browser_urls(), Bucket, Buckets, CEST, COMMON, cut(), dismiss() (+39 more)

### Community 23 - "export.rs"
Cohesion: 0.09
Nodes (46): BLOCK, Heading, List, Para, Pre, Quote, Rule, Table (+38 more)

### Community 24 - "notes.rs"
Cohesion: 0.06
Nodes (55): adjacency(), ANY_LINK, backlinks(), ensure_id(), ensures_ids(), files(), gen_id(), Graph (+47 more)

### Community 25 - "Sync"
Cohesion: 0.67
Nodes (3): Emacs, Git, Sync

### Community 31 - "ai.rs"
Cohesion: 0.09
Nodes (11): CHAT_WAIT, CTX, IDLE, LOADED, MAX_CHAT, MAX_PROMPT, READ_TIMEOUT, RUNS (+3 more)

### Community 33 - "lib/TaskDialog.svelte"
Cohesion: 0.17
Nodes (10): svelte, active, close(), closed(), duePreview, error, key(), save() (+2 more)

### Community 34 - "generate"
Cohesion: 0.15
Nodes (15): Backend, Embedded, Ollama, body(), chat(), chat_messages(), chats(), clean() (+7 more)

### Community 35 - "config.rs"
Cohesion: 0.11
Nodes (25): add_view(), add_views(), atomic_writes(), Config, defaults_round_trip(), expand(), recreate(), recreate_only_when_missing() (+17 more)

### Community 36 - "main"
Cohesion: 0.06
Nodes (28): Agent Workflow, Finish, House rules, Report (your final message), Start, Rules, Run, Build & Test (+20 more)

### Community 37 - "media.rs"
Cohesion: 0.06
Nodes (20): source(), start(), step(), any(), AudioObjectGetPropertyData(), AudioObjectGetPropertyDataSize(), cc(), CMIOObjectGetPropertyData() (+12 more)

### Community 38 - "remind.rs"
Cohesion: 0.06
Nodes (20): app_menu(), ask(), ASKED, BUSY, GO, IDLE, quit_reply(), SEQ (+12 more)

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

### Community 45 - "coverage-ratchet.ts"
Cohesion: 0.25
Nodes (8): Baseline, Count, parseLcov(), pct(), regressions(), Report, Suite, summarize()

### Community 52 - "wlr.rs"
Cohesion: 0.13
Nodes (12): active(), apply(), Ev, App, Closed, State, Title, focused() (+4 more)

### Community 57 - "ollama"
Cohesion: 0.18
Nodes (11): available_gb(), cancel_draft(), cancel_hits_only_the_running_draft(), free_gb(), ollama(), ollama_chat(), parse(), read_chat() (+3 more)

### Community 62 - "backup.rs"
Cohesion: 0.12
Nodes (36): active_profile_is_untracked_and_survives_the_pull(), activity_files_union_merge(), attributes_are_appended_once(), BACKUP, backup_syncs_the_data_repo(), clean_env(), commit(), commit_all() (+28 more)

### Community 63 - "r4.js"
Cohesion: 0.05
Nodes (26): agendaDays, backlinks, board, byTag, DEMO, DEMOS, done3, doneWeek (+18 more)

### Community 64 - "settings.ts"
Cohesion: 0.18
Nodes (17): CATEGORIES, changes(), checkTemplates(), checkViews(), Field, Form, fromForm(), GROUPS (+9 more)

### Community 65 - "triager.md"
Cohesion: 0.40
Nodes (4): Pass, Report (ends your run), Rules for what you post, Watch mode (only when asked)

### Community 67 - "svelte"
Cohesion: 0.21
Nodes (6): chatContext(), cleanChat(), enc, level(), SvelteKit, Vite

### Community 68 - "session.ts"
Cohesion: 0.25
Nodes (7): Review, Rules, Entry, KINDS, ok(), parseSession(), Session

### Community 70 - "b-variants.js"
Cohesion: 0.14
Nodes (29): b1(), b2(), b3(), b4(), b5(), bars(), blocks, card() (+21 more)

### Community 72 - "download"
Cohesion: 0.25
Nodes (7): chat_http(), download(), download_resumes_and_checks(), download_stall_times_out(), part(), serve(), sha256_file()

### Community 73 - "load"
Cohesion: 0.25
Nodes (6): CANCELLED, load(), run(), RUNNING, TICKETS, whole()

### Community 74 - "scripts"
Cohesion: 0.13
Nodes (15): scripts, build, check, check:watch, coverage, coverage:ratchet, dev, e2e:build (+7 more)

### Community 77 - "merge.test.ts"
Cohesion: 0.42
Nodes (7): applyText(), Change, diffChange(), mergeEdits(), perKey(), apply(), merged()

### Community 78 - "lib.rs"
Cohesion: 0.06
Nodes (34): attach_file(), attach_in(), attach_in_numbers_clashes_retries_races_and_rolls_back(), attachment_files(), attachment_files_skips_trash_and_unused_ignores_linked(), CANCEL, capture_file_fills_the_pattern_under_notes(), dash_fixture() (+26 more)

### Community 79 - "v0.3.0 - 2026-10-11"
Cohesion: 0.33
Nodes (5): Changelog, Features, Fixes, Other, v0.3.0 - 2026-10-11

### Community 80 - "screenshots.ts"
Cohesion: 0.13
Nodes (18): ADR-0007, ADR-0008, app, data, day(), driver, env, fixtures() (+10 more)

### Community 82 - "chips.test.ts"
Cohesion: 0.33
Nodes (3): Chip, chips(), items

### Community 83 - "App"
Cohesion: 0.09
Nodes (51): activity_learn(), activity_rules_delete(), agenda(), ai_note_prompt(), ai_retrieve(), App, attach_bytes(), backup_now() (+43 more)

### Community 84 - "ref_node_assert"
Cohesion: 0.16
Nodes (11): BugEnv, bugReportUrl(), isError(), ISSUES, osName(), layout(), Pt, empty (+3 more)

### Community 85 - "R"
Cohesion: 0.10
Nodes (37): activity_dismiss(), activity_rules_list(), activity_suggestions(), ai_day_prompt(), append_journal_template(), bad_meeting_pattern_and_calendar_only_warn(), capture_file(), capture_path() (+29 more)

### Community 86 - "dependencies"
Cohesion: 0.15
Nodes (13): dependencies, @codemirror/commands, @codemirror/language, @codemirror/language-data, @codemirror/search, @codemirror/state, @codemirror/view, @lezer/highlight (+5 more)

### Community 89 - "clock.ts"
Cohesion: 0.27
Nodes (5): naive(), p2(), parseWhen(), WeekDay, worked()

### Community 90 - "donelog.ts"
Cohesion: 0.43
Nodes (4): at(), doneLog(), It, withLog()

### Community 91 - "mock.js"
Cohesion: 0.22
Nodes (8): habit, late, nav, q, row(), tags(), today, week

### Community 93 - "r5.js"
Cohesion: 0.32
Nodes (6): bar(), DEMOS5, layered(), P5, page(), worked

### Community 94 - "releaser.md"
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

### Community 109 - "lib/Agenda.svelte"
Cohesion: 0.16
Nodes (12): dayInput(), key(), op(), pointer(), setBoard(), shift(), Column, columns() (+4 more)

### Community 112 - "projects.ts"
Cohesion: 0.24
Nodes (10): Card, cards(), merge(), Project, ROUNDINGS, Row, Task, toProject() (+2 more)

### Community 113 - "org.rs"
Cohesion: 0.06
Nodes (82): add_interval(), agenda(), agenda_week(), ALIAS, archive(), at(), Cache, capture_insert() (+74 more)

### Community 117 - "kw"
Cohesion: 0.16
Nodes (17): capture_insert(), capture_template(), context_of(), edit(), export_linked(), export_note(), export_path(), export_report() (+9 more)

## Knowledge Gaps
- **325 isolated node(s):** `changelog.sh script`, `q`, `sched`, `late`, `habit` (+320 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 560 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **36 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `line()` connect `level` to `clock.ts`, `editor.ts`, `+page.svelte`?**
  _High betweenness centrality (0.084) - this node is a cross-community bridge._
- **Why does `insertTable()` connect `level` to `editor.ts`, `+page.svelte`?**
  _High betweenness centrality (0.057) - this node is a cross-community bridge._
- **Why does `view()` connect `frag` to `+page.svelte`, `r4.js`?**
  _High betweenness centrality (0.044) - this node is a cross-community bridge._
- **What connects `changelog.sh script`, `q`, `sched` to the rest of the system?**
  _325 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `timeclock.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.05159883720930233 - nodes in this community are weakly interconnected._
- **Should `AppHandle` be split into smaller, more focused modules?**
  _Cohesion score 0.12315270935960591 - nodes in this community are weakly interconnected._
- **Should `editor.ts` be split into smaller, more focused modules?**
  _Cohesion score 0.07394957983193277 - nodes in this community are weakly interconnected._