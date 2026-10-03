# Graph Report - margin  (2026-10-03)

## Corpus Check
- 62 files · ~77,953 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 6 file(s) not represented in the graph (top: (none) 2, .icns 1, .ico 1)

## Summary
- 989 nodes · 2184 edges · 63 communities (35 shown, 28 thin omitted)
- Extraction: 98% EXTRACTED · 1% INFERRED · 0% AMBIGUOUS · INFERRED: 25 edges (avg confidence: 0.86)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `e3bb5b8f`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- org.rs
- timeclock.rs
- lib.rs
- editor.ts
- []
- idle.rs
- orgtable.ts
- tauri.conf.json
- package.json
- compilerOptions
- dependencies
- Calendar
- remind.rs
- GitHub Releases
- svelte.config.js
- attach.rs
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
- config.rs
- vite.config.js
- lib/TaskDialog.svelte
- CLAUDE.md
- ship/SKILL.md
- implementer.md
- scripts
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
- tray.rs
- Node.js
- Tauri Notification Plugin
- Tauri Single Instance Plugin
- Tauri Action
- wayland_idle.rs
- Time Tracking
- TypeScript
- super
- backup.rs
- devDependencies

## God Nodes (most connected - your core abstractions)
1. `App` - 77 edges
2. `State` - 69 edges
3. `Tc` - 31 edges
4. `cfg()` - 29 edges
5. `Kw` - 25 edges
6. `OrgFile` - 22 edges
7. `Session` - 18 edges
8. `[]` - 18 edges
9. `level()` - 18 edges
10. `BLOCK` - 16 edges

## Surprising Connections (you probably didn't know these)
- `Review` --references--> `done()`  [INFERRED]
  .claude/agents/reviewer.md → src/lib/orgtable.ts
- `Pass` --references--> `done()`  [INFERRED]
  .claude/agents/triager.md → src/lib/orgtable.ts
- `picks_due_reminders()` --calls--> `at()`  [INFERRED]
  src-tauri/src/remind.rs → src-tauri/src/org.rs
- `activity_suggestions()` --references--> `Suggestion`  [EXTRACTED]
  src-tauri/src/lib.rs → src-tauri/src/activity.rs
- `generate()` --calls--> `request()`  [INFERRED]
  src-tauri/src/ai.rs → src-tauri/src/activity.rs

## Import Cycles
- None detected.

## Communities (63 total, 28 thin omitted)

### Community 0 - "org.rs"
Cohesion: 0.07
Nodes (75): nodes(), add_interval(), agenda(), agenda_week(), ALIAS, all_tags(), archive(), at() (+67 more)

### Community 1 - "timeclock.rs"
Cohesion: 0.07
Nodes (48): add_past_sessions(), append_diary(), apply_carry(), breaks_accumulate_and_flex(), csv(), csv_field(), csv_quotes_and_comma(), d() (+40 more)

### Community 2 - "lib.rs"
Cohesion: 0.07
Nodes (85): activity_dismiss(), activity_suggestions(), agenda(), ai_day_prompt(), ai_draft(), ai_note_prompt(), App, attach() (+77 more)

### Community 3 - "editor.ts"
Cohesion: 0.06
Nodes (58): applyText(), Block, BULLETS, clicks, codeTokens(), createState(), curLine(), decorate() (+50 more)

### Community 4 - "[]"
Cohesion: 0.20
Nodes (10): [], dateChip(), dayInput(), fmtDay(), goToday(), key(), monday(), op() (+2 more)

### Community 5 - "idle.rs"
Cohesion: 0.22
Nodes (3): source(), start(), step()

### Community 6 - "orgtable.ts"
Cohesion: 0.09
Nodes (39): Review, Rules, Pass, Report (ends your run), Rules for what you post, Watch mode (only when asked), align(), applyFormulas() (+31 more)

### Community 7 - "tauri.conf.json"
Cohesion: 0.06
Nodes (33): app, security, windows, enable, scope, build, beforeBuildCommand, beforeDevCommand (+25 more)

### Community 8 - "package.json"
Cohesion: 0.09
Nodes (21): description, license, name, type, version, @codemirror/commands, @codemirror/language, @codemirror/language-data (+13 more)

### Community 9 - "compilerOptions"
Cohesion: 0.14
Nodes (13): ./.svelte-kit/tsconfig.json, compilerOptions, allowImportingTsExtensions, allowJs, checkJs, esModuleInterop, forceConsistentCasingInFileNames, moduleResolution (+5 more)

### Community 10 - "dependencies"
Cohesion: 0.15
Nodes (13): dependencies, @codemirror/commands, @codemirror/language, @codemirror/language-data, @codemirror/search, @codemirror/state, @codemirror/view, @lezer/highlight (+5 more)

### Community 11 - "Calendar"
Cohesion: 0.67
Nodes (3): Calendar, iCalendar, Tasks

### Community 12 - "remind.rs"
Cohesion: 0.27
Nodes (5): Due, notify(), picks_due_reminders(), start(), tick()

### Community 14 - "svelte.config.js"
Cohesion: 0.40
Nodes (3): @sveltejs/adapter-static, @sveltejs/vite-plugin-svelte, config

### Community 15 - "attach.rs"
Cohesion: 0.24
Nodes (6): clean(), LINK, names_and_links(), normalize(), target(), unused()

### Community 16 - "Notes"
Cohesion: 0.29
Nodes (7): Links and Backlinks, Notes, Org Format, Search, Syntax Highlighting, Table Formulas, Vim Keys

### Community 20 - "default.json"
Cohesion: 0.33
Nodes (5): description, identifier, permissions, $schema, windows

### Community 21 - "setup.ts"
Cohesion: 0.06
Nodes (43): ADR-0007, ADR-0008, at(), buckets, day, ev(), events, y (+35 more)

### Community 22 - "activity.rs"
Cohesion: 0.07
Nodes (38): AwEvent, browser_urls(), Bucket, Buckets, CEST, cut(), dismiss(), dismissals() (+30 more)

### Community 23 - "export.rs"
Cohesion: 0.08
Nodes (48): BLOCK, Heading, List, Para, Pre, Quote, Rule, Table (+40 more)

### Community 24 - "notes.rs"
Cohesion: 0.12
Nodes (20): ANY_LINK, backlinks(), ensure_id(), ensures_ids(), files(), gen_id(), Graph, graph_neighbourhood() (+12 more)

### Community 25 - "Sync"
Cohesion: 0.67
Nodes (3): Emacs, Git, Sync

### Community 31 - "config.rs"
Cohesion: 0.13
Nodes (16): add_view(), add_views(), atomic_writes(), Config, defaults_round_trip(), expand(), rename_view(), rename_views() (+8 more)

### Community 33 - "lib/TaskDialog.svelte"
Cohesion: 0.17
Nodes (10): svelte, active, close(), closed(), duePreview, error, key(), save() (+2 more)

### Community 36 - "CLAUDE.md"
Cohesion: 0.17
Nodes (10): Agent Workflow, Build & Test, Code Style, Permission Mode, Project, Release & CI/CD, Repo Conventions, Tauri-Specific (+2 more)

### Community 37 - "ship/SKILL.md"
Cohesion: 0.25
Nodes (7): 1. Survey, 2. Dispatch, 3. Review each report, 4. Clean up, 5. Report to the owner, Rules, Running it repeatedly

### Community 38 - "implementer.md"
Cohesion: 0.40
Nodes (4): Finish, House rules, Report (your final message), Start

### Community 39 - "scripts"
Cohesion: 0.15
Nodes (13): scripts, build, check, check:watch, dev, e2e:build, e2e:run, prepare (+5 more)

### Community 40 - "release/SKILL.md"
Cohesion: 0.40
Nodes (4): Notes, Secrets required (in GitHub repo settings), Usage, What it does

### Community 41 - "autopilot/SKILL.md"
Cohesion: 0.40
Nodes (4): Idle (`--overnight`), Loop, Rules, Stop

### Community 42 - "verify-all/SKILL.md"
Cohesion: 0.50
Nodes (3): Usage, What it does, When to use

### Community 52 - "tray.rs"
Cohesion: 0.27
Nodes (5): refresh(), setup(), show(), tracking(), Tray

### Community 60 - "super"
Cohesion: 0.31
Nodes (5): builds_feed(), event(), feed(), fnv(), fold()

### Community 61 - "backup.rs"
Cohesion: 0.31
Nodes (6): backup(), fail(), git(), LAST_FAILURE, LOG, ok()

### Community 62 - "devDependencies"
Cohesion: 0.18
Nodes (11): devDependencies, svelte, svelte-check, @sveltejs/adapter-static, @sveltejs/kit, @sveltejs/vite-plugin-svelte, @tauri-apps/cli, @types/node (+3 more)

## Knowledge Gaps
- **210 isolated node(s):** `y`, `day`, `events`, `app`, `out` (+205 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 346 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **28 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `insertTable()` connect `editor.ts` to `+page.svelte`?**
  _High betweenness centrality (0.146) - this node is a cross-community bridge._
- **Why does `format()` connect `orgtable.ts` to `+page.svelte`?**
  _High betweenness centrality (0.057) - this node is a cross-community bridge._
- **Why does `palette()` connect `setup.ts` to `+page.svelte`?**
  _High betweenness centrality (0.056) - this node is a cross-community bridge._
- **What connects `y`, `day`, `events` to the rest of the system?**
  _210 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `org.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.06791510611735331 - nodes in this community are weakly interconnected._
- **Should `timeclock.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.06971399387129724 - nodes in this community are weakly interconnected._
- **Should `lib.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.06704980842911877 - nodes in this community are weakly interconnected._