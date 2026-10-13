# Graph Report - margin  (2026-10-13)

## Corpus Check
- 91 files · ~121,533 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 7 file(s) not represented in the graph (top: (none) 3, .icns 1, .ico 1)

## Summary
- 1449 nodes · 3322 edges · 84 communities (53 shown, 31 thin omitted)
- Extraction: 98% EXTRACTED · 2% INFERRED · 0% AMBIGUOUS · INFERRED: 52 edges (avg confidence: 0.87)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `b57bcaf9`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- org.rs
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
- GitHub Releases
- level
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
- margin
- ai.rs
- releaser.md
- lib/TaskDialog.svelte
- generate
- config.rs
- CLAUDE.md
- ship/SKILL.md
- main
- day_prompt
- release/SKILL.md
- autopilot/SKILL.md
- verify-all/SKILL.md
- scout.md
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
- Glyph
- backup.rs
- settings.ts
- triager.md
- extension.js
- reviewer.md
- focus
- download
- load
- scripts
- ideator.md
- remind.rs
- merge.test.ts
- lib.rs
- v0.3.0 - 2026-10-11
- dependencies
- changelog.sh
- cfg
- App
- kw
- R
- chatctx.ts
- devDependencies
- export_linked

## God Nodes (most connected - your core abstractions)
1. `App` - 94 edges
2. `Tc` - 40 edges
3. `cfg()` - 36 edges
4. `Kw` - 26 edges
5. `OrgFile` - 24 edges
6. `Session` - 20 edges
7. `[]` - 20 edges
8. `reload_config()` - 19 edges
9. `kw()` - 18 edges
10. `Event` - 18 edges

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

## Communities (84 total, 31 thin omitted)

### Community 0 - "org.rs"
Cohesion: 0.07
Nodes (76): add_interval(), agenda(), agenda_week(), ALIAS, all_tags(), archive(), at(), Cache (+68 more)

### Community 1 - "timeclock.rs"
Cohesion: 0.05
Nodes (73): add_past_sessions(), append_diary(), apply_carry(), breaks_accumulate_and_flex(), csv(), csv_field(), csv_quotes_and_comma(), csv_skips_zero_rows() (+65 more)

### Community 2 - "AppHandle"
Cohesion: 0.15
Nodes (22): ai_chat(), ai_download(), ai_draft(), ai_model_delete(), ai_models(), ai_set(), apply_config(), capture() (+14 more)

### Community 3 - "editor.ts"
Cohesion: 0.07
Nodes (28): Block, BULLETS, clicks, codeTokens(), decorate(), hide, hooks, imageSrc() (+20 more)

### Community 4 - "[]"
Cohesion: 0.18
Nodes (11): [], dateChip(), dayInput(), fmtDay(), goToday(), key(), monday(), op() (+3 more)

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

### Community 15 - "level"
Cohesion: 0.18
Nodes (27): createState(), curLine(), drawerRange(), expandTemplate(), headingAt(), headings(), insertBlock(), insertMode() (+19 more)

### Community 16 - "Notes"
Cohesion: 0.29
Nodes (7): Links and Backlinks, Notes, Org Format, Search, Syntax Highlighting, Table Formulas, Vim Keys

### Community 20 - "default.json"
Cohesion: 0.33
Nodes (5): description, identifier, permissions, $schema, windows

### Community 21 - "setup.ts"
Cohesion: 0.05
Nodes (60): ADR-0007, ADR-0008, at(), buckets, day, ev(), events, y (+52 more)

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

### Community 32 - "releaser.md"
Cohesion: 0.33
Nodes (5): 1. Gate (all must hold, else report which failed and stop), 2. Bump PR, 3. Tag, build, publish, Report, Rules

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

### Community 37 - "ship/SKILL.md"
Cohesion: 0.25
Nodes (7): 1. Survey, 2. Dispatch, 3. Review each report, 4. Clean up, 5. Report to the owner, Rules, Running it repeatedly

### Community 38 - "main"
Cohesion: 0.33
Nodes (5): Finish, House rules, Report (your final message), Start, main()

### Community 39 - "day_prompt"
Cohesion: 0.21
Nodes (9): capped(), day_prompt(), dt(), embedded_drafts(), kept(), note_prompt(), prompts(), t() (+1 more)

### Community 40 - "release/SKILL.md"
Cohesion: 0.40
Nodes (4): Notes, Secrets required (in GitHub repo settings), Usage, What it does

### Community 41 - "autopilot/SKILL.md"
Cohesion: 0.40
Nodes (4): Idle, Loop, Rules, Stop

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
Cohesion: 0.22
Nodes (9): available_gb(), free_gb(), ollama(), ollama_chat(), parse(), read_chat(), ticket(), unreachable() (+1 more)

### Community 60 - "ics.rs"
Cohesion: 0.07
Nodes (34): clean(), LINK, names_and_links(), normalize(), target(), unused(), builds_feed(), CALENDAR (+26 more)

### Community 62 - "backup.rs"
Cohesion: 0.15
Nodes (25): BACKUP, backup_syncs_the_data_repo(), clean_env(), commit(), concurrent_backups_both_succeed(), ensure_attributes(), fail(), git() (+17 more)

### Community 64 - "settings.ts"
Cohesion: 0.21
Nodes (15): changes(), checkTemplates(), checkViews(), Field, Form, fromForm(), GROUPS, needsMacPrompt() (+7 more)

### Community 65 - "triager.md"
Cohesion: 0.40
Nodes (4): Pass, Report (ends your run), Rules for what you post, Watch mode (only when asked)

### Community 72 - "download"
Cohesion: 0.25
Nodes (7): chat_http(), download(), download_resumes_and_checks(), download_stall_times_out(), part(), serve(), sha256_file()

### Community 73 - "load"
Cohesion: 0.20
Nodes (8): cancel_draft(), cancel_hits_only_the_running_draft(), CANCELLED, load(), run(), RUNNING, TICKETS, whole()

### Community 74 - "scripts"
Cohesion: 0.13
Nodes (15): scripts, build, check, check:watch, coverage, coverage:ratchet, dev, e2e:build (+7 more)

### Community 76 - "remind.rs"
Cohesion: 0.05
Nodes (16): source(), start(), step(), notify(), once_a_day(), picks_due_reminders(), start(), tick() (+8 more)

### Community 77 - "merge.test.ts"
Cohesion: 0.42
Nodes (7): applyText(), Change, diffChange(), mergeEdits(), perKey(), apply(), merged()

### Community 78 - "lib.rs"
Cohesion: 0.07
Nodes (23): attach_bytes(), attach_file(), attach_in(), attach_in_numbers_clashes_retries_races_and_rolls_back(), attachment_files(), attachment_files_skips_trash_and_unused_ignores_linked(), CANCEL, capture_file_fills_the_pattern_under_notes() (+15 more)

### Community 79 - "v0.3.0 - 2026-10-11"
Cohesion: 0.33
Nodes (5): Changelog, Features, Fixes, Other, v0.3.0 - 2026-10-11

### Community 80 - "dependencies"
Cohesion: 0.15
Nodes (13): dependencies, @codemirror/commands, @codemirror/language, @codemirror/language-data, @codemirror/search, @codemirror/state, @codemirror/view, @lezer/highlight (+5 more)

### Community 82 - "cfg"
Cohesion: 0.18
Nodes (12): ai_note_prompt(), cfg(), clock_out_before_setup(), config(), export_open(), notes_new(), org_targets(), orphan_warning() (+4 more)

### Community 83 - "App"
Cohesion: 0.11
Nodes (41): activity_learn(), activity_rules_delete(), activity_rules_list(), App, backup_now(), capture_templates(), day_done_now(), export_report() (+33 more)

### Community 84 - "kw"
Cohesion: 0.12
Nodes (19): capture_insert(), capture_template(), context_of(), dash_fixture(), dashboard(), dashboard_and_status_show_the_clock(), dashboard_builds_the_week(), dashboard_week_runs_monday_to_sunday() (+11 more)

### Community 85 - "R"
Cohesion: 0.13
Nodes (30): activity_dismiss(), activity_suggestions(), agenda(), ai_day_prompt(), bad_meeting_pattern_and_calendar_only_warn(), capture_file(), capture_path(), check_in() (+22 more)

### Community 86 - "chatctx.ts"
Cohesion: 0.33
Nodes (6): onkeydown(), send(), chatContext(), cleanChat(), enc, level()

### Community 88 - "devDependencies"
Cohesion: 0.18
Nodes (11): devDependencies, svelte, svelte-check, @sveltejs/adapter-static, @sveltejs/kit, @sveltejs/vite-plugin-svelte, @tauri-apps/cli, @types/node (+3 more)

### Community 92 - "export_linked"
Cohesion: 0.60
Nodes (5): export_linked(), export_note(), export_path(), stem(), write_export()

## Knowledge Gaps
- **251 isolated node(s):** `changelog.sh script`, `y`, `day`, `events`, `app` (+246 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 447 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **31 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `palette()` connect `setup.ts` to `+page.svelte`?**
  _High betweenness centrality (0.099) - this node is a cross-community bridge._
- **Why does `line()` connect `level` to `svelte`, `editor.ts`, `+page.svelte`?**
  _High betweenness centrality (0.087) - this node is a cross-community bridge._
- **Why does `insertTable()` connect `level` to `editor.ts`, `+page.svelte`?**
  _High betweenness centrality (0.076) - this node is a cross-community bridge._
- **What connects `changelog.sh script`, `y`, `day` to the rest of the system?**
  _251 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `org.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.06545114539504442 - nodes in this community are weakly interconnected._
- **Should `timeclock.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.052516129032258066 - nodes in this community are weakly interconnected._
- **Should `AppHandle` be split into smaller, more focused modules?**
  _Cohesion score 0.14666666666666667 - nodes in this community are weakly interconnected._