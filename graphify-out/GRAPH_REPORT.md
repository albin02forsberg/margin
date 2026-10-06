# Graph Report - margin  (2026-10-06)

## Corpus Check
- 73 files · ~97,611 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 6 file(s) not represented in the graph (top: (none) 2, .icns 1, .ico 1)

## Summary
- 1229 nodes · 2719 edges · 64 communities (37 shown, 27 thin omitted)
- Extraction: 98% EXTRACTED · 1% INFERRED · 0% AMBIGUOUS · INFERRED: 34 edges (avg confidence: 0.86)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `8845aa22`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- org.rs
- timeclock.rs
- lib.rs
- editor.ts
- []
- watcher.rs
- orgtable.ts
- tauri.conf.json
- package.json
- compilerOptions
- dependencies
- Calendar
- GitHub Releases
- scripts
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
- margin
- ai.rs
- devDependencies
- lib/TaskDialog.svelte
- config.rs
- CLAUDE.md
- ship/SKILL.md
- implementer.md
- release/SKILL.md
- autopilot/SKILL.md
- verify-all/SKILL.md
- scout.md
- App Icon
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
- Time Tracking
- TypeScript
- ics.rs
- decorate
- curLine
- gnome.rs
- settings.ts
- extension.js

## God Nodes (most connected - your core abstractions)
1. `App` - 84 edges
2. `Tc` - 31 edges
3. `cfg()` - 30 edges
4. `Kw` - 25 edges
5. `OrgFile` - 22 edges
6. `reload_config()` - 19 edges
7. `[]` - 19 edges
8. `Event` - 18 edges
9. `Session` - 18 edges
10. `level()` - 18 edges

## Surprising Connections (you probably didn't know these)
- `Review` --references--> `done()`  [INFERRED]
  .claude/agents/reviewer.md → src/lib/orgtable.ts
- `Pass` --references--> `done()`  [INFERRED]
  .claude/agents/triager.md → src/lib/orgtable.ts
- `calendar_from_a_url()` --calls--> `at()`  [INFERRED]
  src-tauri/src/ics.rs → src-tauri/src/org.rs
- `picks_due_reminders()` --calls--> `at()`  [INFERRED]
  src-tauri/src/remind.rs → src-tauri/src/org.rs
- `exclusion_and_truncation_before_write()` --calls--> `exclude_rules()`  [INFERRED]
  src-tauri/src/watcher.rs → src-tauri/src/activity.rs

## Import Cycles
- 1-file cycle: `src-tauri/src/ai.rs -> src-tauri/src/ai.rs`

## Communities (64 total, 27 thin omitted)

### Community 0 - "org.rs"
Cohesion: 0.07
Nodes (74): add_interval(), agenda(), agenda_week(), ALIAS, archive(), at(), Cache, capture_insert() (+66 more)

### Community 1 - "timeclock.rs"
Cohesion: 0.07
Nodes (50): add_past_sessions(), append_diary(), apply_carry(), breaks_accumulate_and_flex(), csv(), csv_field(), csv_quotes_and_comma(), csv_skips_zero_rows() (+42 more)

### Community 2 - "lib.rs"
Cohesion: 0.06
Nodes (102): activity_dismiss(), activity_learn(), activity_rules_delete(), activity_rules_list(), activity_suggestions(), agenda(), ai_day_prompt(), ai_download() (+94 more)

### Community 3 - "editor.ts"
Cohesion: 0.09
Nodes (21): applyText(), Block, BULLETS, clicks, diffChange(), hide, hooks, LANG_ALIAS (+13 more)

### Community 4 - "[]"
Cohesion: 0.20
Nodes (10): [], dateChip(), dayInput(), fmtDay(), goToday(), key(), monday(), op() (+2 more)

### Community 5 - "watcher.rs"
Cohesion: 0.13
Nodes (23): active(), active_window(), AFK_MINS, append(), clean(), flush(), flush_writes_the_open_record_once(), gnome_extension() (+15 more)

### Community 6 - "orgtable.ts"
Cohesion: 0.09
Nodes (39): Review, Rules, Pass, Report (ends your run), Rules for what you post, Watch mode (only when asked), align(), applyFormulas() (+31 more)

### Community 7 - "tauri.conf.json"
Cohesion: 0.06
Nodes (34): app, security, windows, enable, scope, build, beforeBuildCommand, beforeDevCommand (+26 more)

### Community 8 - "package.json"
Cohesion: 0.07
Nodes (26): description, license, name, type, version, @codemirror/commands, @codemirror/language, @codemirror/language-data (+18 more)

### Community 9 - "compilerOptions"
Cohesion: 0.14
Nodes (13): ./.svelte-kit/tsconfig.json, compilerOptions, allowImportingTsExtensions, allowJs, checkJs, esModuleInterop, forceConsistentCasingInFileNames, moduleResolution (+5 more)

### Community 10 - "dependencies"
Cohesion: 0.15
Nodes (13): dependencies, @codemirror/commands, @codemirror/language, @codemirror/language-data, @codemirror/search, @codemirror/state, @codemirror/view, @lezer/highlight (+5 more)

### Community 11 - "Calendar"
Cohesion: 0.67
Nodes (3): Calendar, iCalendar, Tasks

### Community 14 - "scripts"
Cohesion: 0.15
Nodes (13): scripts, build, check, check:watch, dev, e2e:build, e2e:run, prepare (+5 more)

### Community 15 - "level"
Cohesion: 0.37
Nodes (13): drawerRange(), headingAt(), headings(), level(), line(), moveSubtree(), orgFold, scan() (+5 more)

### Community 16 - "Notes"
Cohesion: 0.29
Nodes (7): Links and Backlinks, Notes, Org Format, Search, Syntax Highlighting, Table Formulas, Vim Keys

### Community 20 - "default.json"
Cohesion: 0.33
Nodes (5): description, identifier, permissions, $schema, windows

### Community 21 - "setup.ts"
Cohesion: 0.06
Nodes (50): ADR-0007, ADR-0008, at(), buckets, day, ev(), events, y (+42 more)

### Community 22 - "activity.rs"
Cohesion: 0.07
Nodes (45): AwEvent, browser_urls(), Bucket, Buckets, CEST, COMMON, cut(), dismiss() (+37 more)

### Community 23 - "export.rs"
Cohesion: 0.09
Nodes (46): BLOCK, Heading, List, Para, Pre, Quote, Rule, Table (+38 more)

### Community 24 - "notes.rs"
Cohesion: 0.13
Nodes (24): ANY_LINK, backlinks(), ensure_id(), ensures_ids(), files(), gen_id(), Graph, graph_neighbourhood() (+16 more)

### Community 25 - "Sync"
Cohesion: 0.67
Nodes (3): Emacs, Git, Sync

### Community 31 - "ai.rs"
Cohesion: 0.05
Nodes (47): request(), available_gb(), Backend, Embedded, Ollama, body(), cancel_draft(), cancel_hits_only_the_running_draft() (+39 more)

### Community 32 - "devDependencies"
Cohesion: 0.18
Nodes (11): devDependencies, svelte, svelte-check, @sveltejs/adapter-static, @sveltejs/kit, @sveltejs/vite-plugin-svelte, @tauri-apps/cli, @types/node (+3 more)

### Community 33 - "lib/TaskDialog.svelte"
Cohesion: 0.17
Nodes (10): svelte, active, close(), closed(), duePreview, error, key(), save() (+2 more)

### Community 35 - "config.rs"
Cohesion: 0.10
Nodes (25): add_view(), add_views(), atomic_writes(), Config, defaults_round_trip(), expand(), recreate(), recreate_only_when_missing() (+17 more)

### Community 36 - "CLAUDE.md"
Cohesion: 0.17
Nodes (10): Agent Workflow, Build & Test, Code Style, Permission Mode, Project, Release & CI/CD, Repo Conventions, Tauri-Specific (+2 more)

### Community 37 - "ship/SKILL.md"
Cohesion: 0.25
Nodes (7): 1. Survey, 2. Dispatch, 3. Review each report, 4. Clean up, 5. Report to the owner, Rules, Running it repeatedly

### Community 38 - "implementer.md"
Cohesion: 0.40
Nodes (4): Finish, House rules, Report (your final message), Start

### Community 40 - "release/SKILL.md"
Cohesion: 0.40
Nodes (4): Notes, Secrets required (in GitHub repo settings), Usage, What it does

### Community 41 - "autopilot/SKILL.md"
Cohesion: 0.40
Nodes (4): Idle (`--overnight`), Loop, Rules, Stop

### Community 42 - "verify-all/SKILL.md"
Cohesion: 0.50
Nodes (3): Usage, What it does, When to use

### Community 52 - "wlr.rs"
Cohesion: 0.07
Nodes (15): active(), apply(), Ev, App, Closed, State, Title, focused() (+7 more)

### Community 60 - "ics.rs"
Cohesion: 0.06
Nodes (30): builds_feed(), CALENDAR, calendar_from_a_url(), CALENDAR_TTL, date(), event(), events_on(), feed() (+22 more)

### Community 61 - "decorate"
Cohesion: 0.13
Nodes (9): codeTokens(), decorate(), Glyph, imageSrc(), Img, inline(), langFor(), mark() (+1 more)

### Community 62 - "curLine"
Cohesion: 0.21
Nodes (15): createState(), curLine(), expandTemplate(), insertBlock(), insertMode(), insertTable(), inTable(), newHeading() (+7 more)

### Community 63 - "gnome.rs"
Cohesion: 0.08
Nodes (22): clean(), LINK, names_and_links(), normalize(), target(), unused(), backup(), fail() (+14 more)

### Community 64 - "settings.ts"
Cohesion: 0.21
Nodes (15): changes(), checkTemplates(), checkViews(), Field, Form, fromForm(), GROUPS, needsMacPrompt() (+7 more)

## Knowledge Gaps
- **233 isolated node(s):** `y`, `day`, `events`, `app`, `out` (+228 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 404 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **27 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `insertTable()` connect `curLine` to `editor.ts`, `+page.svelte`?**
  _High betweenness centrality (0.136) - this node is a cross-community bridge._
- **Why does `palette()` connect `setup.ts` to `+page.svelte`?**
  _High betweenness centrality (0.049) - this node is a cross-community bridge._
- **Why does `format()` connect `orgtable.ts` to `+page.svelte`?**
  _High betweenness centrality (0.047) - this node is a cross-community bridge._
- **What connects `y`, `day`, `events` to the rest of the system?**
  _233 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `org.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.06544566544566545 - nodes in this community are weakly interconnected._
- **Should `timeclock.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.06813186813186813 - nodes in this community are weakly interconnected._
- **Should `lib.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.061531343974092066 - nodes in this community are weakly interconnected._