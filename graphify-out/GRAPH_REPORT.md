# Graph Report - margin  (2026-10-12)

## Corpus Check
- 89 files · ~117,954 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 7 file(s) not represented in the graph (top: (none) 3, .icns 1, .ico 1)

## Summary
- 1405 nodes · 3207 edges · 87 communities (57 shown, 30 thin omitted)
- Extraction: 98% EXTRACTED · 2% INFERRED · 0% AMBIGUOUS · INFERRED: 49 edges (avg confidence: 0.87)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `e89c41f2`
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
- activity_suggestions
- Calendar
- GitHub Releases
- level
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
- decorate
- backup.rs
- idle.rs
- settings.ts
- triager.md
- extension.js
- gnome.rs
- reviewer.md
- tray.rs
- path
- focus
- download
- load
- ideator.md
- super
- merge.test.ts
- lib.rs
- v0.3.0 - 2026-10-11
- scripts
- changelog.sh
- kw
- App
- Value
- R
- devDependencies

## God Nodes (most connected - your core abstractions)
1. `App` - 94 edges
2. `Tc` - 38 edges
3. `cfg()` - 35 edges
4. `Kw` - 26 edges
5. `OrgFile` - 24 edges
6. `[]` - 20 edges
7. `reload_config()` - 19 edges
8. `Session` - 19 edges
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

## Communities (87 total, 30 thin omitted)

### Community 0 - "org.rs"
Cohesion: 0.07
Nodes (76): add_interval(), agenda(), agenda_week(), ALIAS, all_tags(), archive(), at(), Cache (+68 more)

### Community 1 - "timeclock.rs"
Cohesion: 0.06
Nodes (61): add_past_sessions(), append_diary(), apply_carry(), breaks_accumulate_and_flex(), csv(), csv_field(), csv_quotes_and_comma(), csv_skips_zero_rows() (+53 more)

### Community 2 - "AppHandle"
Cohesion: 0.15
Nodes (21): ai_chat(), ai_download(), ai_draft(), ai_model_delete(), ai_models(), ai_set(), apply_config(), capture() (+13 more)

### Community 3 - "editor.ts"
Cohesion: 0.09
Nodes (20): Block, BULLETS, clicks, hide, hooks, LANG_ALIAS, langLoaded, langs (+12 more)

### Community 4 - "[]"
Cohesion: 0.18
Nodes (12): [], dateChip(), dayInput(), fmtDay(), goToday(), key(), monday(), op() (+4 more)

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

### Community 10 - "activity_suggestions"
Cohesion: 0.23
Nodes (16): activity_suggestions(), agenda(), capture_file(), date(), date_at(), date_preview(), date_preview_at(), opt_date_at() (+8 more)

### Community 11 - "Calendar"
Cohesion: 0.67
Nodes (3): Calendar, iCalendar, Tasks

### Community 14 - "level"
Cohesion: 0.41
Nodes (13): drawerRange(), headingAt(), headings(), level(), line(), moveSubtree(), newHeading(), orgFold (+5 more)

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
Cohesion: 0.05
Nodes (61): ADR-0007, ADR-0008, at(), buckets, day, ev(), events, y (+53 more)

### Community 22 - "activity.rs"
Cohesion: 0.07
Nodes (47): AwEvent, browser_urls(), Bucket, Buckets, CEST, COMMON, cut(), dismiss() (+39 more)

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
Cohesion: 0.11
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
Cohesion: 0.07
Nodes (15): active(), apply(), Ev, App, Closed, State, Title, focused() (+7 more)

### Community 57 - "ollama"
Cohesion: 0.17
Nodes (12): read_err(), available_gb(), cancel_draft(), cancel_hits_only_the_running_draft(), free_gb(), ollama(), ollama_chat(), parse() (+4 more)

### Community 60 - "ics.rs"
Cohesion: 0.16
Nodes (18): builds_feed(), CALENDAR, calendar_from_a_url(), CALENDAR_TTL, date(), event(), events_on(), feed() (+10 more)

### Community 61 - "decorate"
Cohesion: 0.13
Nodes (9): codeTokens(), decorate(), Glyph, imageSrc(), Img, inline(), langFor(), mark() (+1 more)

### Community 62 - "backup.rs"
Cohesion: 0.24
Nodes (13): BACKUP, clean_env(), concurrent_backups_both_succeed(), fail(), git(), LAST_FAILURE, leaves_foreign_values_and_prefix_lookalikes(), LOG (+5 more)

### Community 63 - "idle.rs"
Cohesion: 0.22
Nodes (3): source(), start(), step()

### Community 64 - "settings.ts"
Cohesion: 0.21
Nodes (14): changes(), checkTemplates(), checkViews(), Field, Form, fromForm(), GROUPS, needsMacPrompt() (+6 more)

### Community 65 - "triager.md"
Cohesion: 0.40
Nodes (4): Pass, Report (ends your run), Rules for what you post, Watch mode (only when asked)

### Community 67 - "gnome.rs"
Cohesion: 0.20
Nodes (10): active(), BUS, FILES, get(), install(), install_copies_the_extension(), parse(), session() (+2 more)

### Community 69 - "tray.rs"
Cohesion: 0.22
Nodes (5): refresh(), setup(), show(), tracking(), Tray

### Community 70 - "path"
Cohesion: 0.24
Nodes (6): clean(), LINK, names_and_links(), normalize(), target(), unused()

### Community 72 - "download"
Cohesion: 0.25
Nodes (7): chat_http(), download(), download_resumes_and_checks(), download_stall_times_out(), part(), serve(), sha256_file()

### Community 73 - "load"
Cohesion: 0.25
Nodes (6): CANCELLED, load(), run(), RUNNING, TICKETS, whole()

### Community 76 - "super"
Cohesion: 0.29
Nodes (4): notify(), picks_due_reminders(), start(), tick()

### Community 77 - "merge.test.ts"
Cohesion: 0.42
Nodes (7): applyText(), Change, diffChange(), mergeEdits(), perKey(), apply(), merged()

### Community 78 - "lib.rs"
Cohesion: 0.07
Nodes (23): bad_meeting_pattern_and_calendar_only_warn(), CANCEL, capture_file_fills_the_pattern_under_notes(), clock_out_before_setup(), config(), dates_read_relative_to_a_given_today(), DOWNLOADING, iso() (+15 more)

### Community 79 - "v0.3.0 - 2026-10-11"
Cohesion: 0.33
Nodes (5): Changelog, Features, Fixes, Other, v0.3.0 - 2026-10-11

### Community 80 - "scripts"
Cohesion: 0.07
Nodes (28): dependencies, @codemirror/commands, @codemirror/language, @codemirror/language-data, @codemirror/search, @codemirror/state, @codemirror/view, @lezer/highlight (+20 more)

### Community 82 - "kw"
Cohesion: 0.21
Nodes (13): capture_insert(), capture_template(), edit(), export_linked(), export_note(), export_path(), kw(), org_edit() (+5 more)

### Community 83 - "App"
Cohesion: 0.11
Nodes (40): activity_learn(), activity_rules_delete(), activity_rules_list(), ai_day_prompt(), App, backup_now(), export_report(), leave_time() (+32 more)

### Community 84 - "Value"
Cohesion: 0.19
Nodes (11): context_of(), dash_fixture(), dashboard(), dashboard_and_status_show_the_clock(), dashboard_builds_the_week(), dashboard_week_runs_monday_to_sunday(), DashIn, keys() (+3 more)

### Community 85 - "R"
Cohesion: 0.13
Nodes (24): activity_dismiss(), ai_note_prompt(), attach(), attach_bytes(), attach_file(), capture_path(), capture_templates(), cfg() (+16 more)

### Community 86 - "devDependencies"
Cohesion: 0.18
Nodes (11): devDependencies, svelte, svelte-check, @sveltejs/adapter-static, @sveltejs/kit, @sveltejs/vite-plugin-svelte, @tauri-apps/cli, @types/node (+3 more)

## Knowledge Gaps
- **250 isolated node(s):** `changelog.sh script`, `y`, `day`, `events`, `app` (+245 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 440 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **30 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `palette()` connect `setup.ts` to `+page.svelte`?**
  _High betweenness centrality (0.097) - this node is a cross-community bridge._
- **Why does `line()` connect `level` to `editor.ts`, `curLine`, `svelte`, `decorate`, `+page.svelte`?**
  _High betweenness centrality (0.082) - this node is a cross-community bridge._
- **Why does `insertTable()` connect `curLine` to `editor.ts`, `+page.svelte`?**
  _High betweenness centrality (0.081) - this node is a cross-community bridge._
- **What connects `changelog.sh script`, `y`, `day` to the rest of the system?**
  _250 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `org.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.06545114539504442 - nodes in this community are weakly interconnected._
- **Should `timeclock.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.06031260618416582 - nodes in this community are weakly interconnected._
- **Should `editor.ts` be split into smaller, more focused modules?**
  _Cohesion score 0.09057971014492754 - nodes in this community are weakly interconnected._