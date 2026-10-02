# Graph Report - tool  (2026-10-02)

## Corpus Check
- Corpus is ~38,310 words - fits in a single context window. You may not need a graph.

## Summary
- 621 nodes · 1359 edges · 31 communities (23 shown, 8 thin omitted)
- Extraction: 95% EXTRACTED · 1% INFERRED · 0% AMBIGUOUS · INFERRED: 19 edges (avg confidence: 0.85)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- Core UI Components
- Tauri Backend Core
- Svelte Components
- Data Models
- Configuration & Storage
- Time & Scheduling
- Org Format Parsing
- Tauri Integrations
- ICS Calendar Export
- Frontend Routes
- Menu Components
- Dialog Components
- Tauri Plugins
- Editor Components
- Task Dialog
- System Integration
- Time Component
- Picker Component
- Shared Utilities
- Tray Management
- Backup & Sync
- Notifications
- GitHub Actions
- Community 23
- Community 24
- Community 25
- Community 26
- Community 30

## God Nodes (most connected - your core abstractions)
1. `App` - 51 edges
2. `Tc` - 21 edges
3. `Kw` - 20 edges
4. `[]` - 20 edges
5. `level()` - 18 edges
6. `Margin` - 18 edges
7. `OrgFile` - 17 edges
8. `Session` - 16 edges
9. `curLine()` - 16 edges
10. `line()` - 14 edges

## Surprising Connections (you probably didn't know these)
- `Margin` --uses--> `Rust`  [high]
  README.md, package.json → src-tauri/Cargo.toml, .github/workflows/ci.yml
- `Margin` --uses--> `App Icon`  [high]
  README.md, package.json → assets/icon.svg
- `Margin` --supports--> `Linux`  [high]
  README.md, package.json → README.md, .github/workflows/release.yml
- `Margin` --supports--> `macOS`  [high]
  README.md, package.json → README.md, .github/workflows/release.yml
- `Margin` --provides--> `Notifications`  [high]
  README.md, package.json → README.md, src-tauri/Cargo.toml

## Import Cycles
- None detected.

## Communities (31 total, 8 thin omitted)

### Community 0 - "Core UI Components"
Cohesion: 0.08
Nodes (61): add_interval(), agenda(), agenda_week(), archive(), capture_insert(), check_target(), cycle_and_repeat(), cycle_priority() (+53 more)

### Community 1 - "Tauri Backend Core"
Cohesion: 0.08
Nodes (43): append_diary(), apply_carry(), breaks_accumulate_and_flex(), csv(), csv_field(), csv_quotes_and_comma(), d(), daily_report() (+35 more)

### Community 2 - "Svelte Components"
Cohesion: 0.11
Nodes (51): agenda(), App, backup_now(), capture(), capture_insert(), capture_path(), config(), date() (+43 more)

### Community 3 - "Data Models"
Cohesion: 0.06
Nodes (57): applyText(), Block, BULLETS, clicks, codeTokens(), createState(), curLine(), decorate() (+49 more)

### Community 4 - "Configuration & Storage"
Cohesion: 0.05
Nodes (29): svelte, [], dateChip(), dayInput(), fmtDay(), goToday(), key(), monday() (+21 more)

### Community 5 - "Time & Scheduling"
Cohesion: 0.07
Nodes (22): Config, expand(), View, builds_feed(), event(), feed(), fnv(), fold() (+14 more)

### Community 6 - "Org Format Parsing"
Cohesion: 0.10
Nodes (33): align(), applyFormulas(), cells(), colRef(), create(), deleteCol(), deleteRow(), done() (+25 more)

### Community 7 - "Tauri Integrations"
Cohesion: 0.07
Nodes (26): app, security, windows, enable, scope, build, beforeBuildCommand, beforeDevCommand (+18 more)

### Community 8 - "ICS Calendar Export"
Cohesion: 0.09
Nodes (21): description, license, name, type, version, @codemirror/commands, @codemirror/language, @codemirror/language-data (+13 more)

### Community 9 - "Frontend Routes"
Cohesion: 0.14
Nodes (13): ./.svelte-kit/tsconfig.json, compilerOptions, allowImportingTsExtensions, allowJs, checkJs, esModuleInterop, forceConsistentCasingInFileNames, moduleResolution (+5 more)

### Community 10 - "Menu Components"
Cohesion: 0.15
Nodes (13): dependencies, @codemirror/commands, @codemirror/language, @codemirror/language-data, @codemirror/search, @codemirror/state, @codemirror/view, @lezer/highlight (+5 more)

### Community 11 - "Dialog Components"
Cohesion: 0.22
Nodes (11): App Icon, Calendar, Configuration, Favicon, Global Hotkey, iCalendar, Journal, Margin (+3 more)

### Community 12 - "Tauri Plugins"
Cohesion: 0.31
Nodes (6): backup(), fail(), git(), LAST_FAILURE, LOG, ok()

### Community 13 - "Editor Components"
Cohesion: 0.20
Nodes (10): GitHub Releases, Tauri Global Shortcut Plugin, Tauri Notification Plugin, Tauri Opener Plugin, Tauri Process Plugin, Tauri Single Instance Plugin, Tauri, Time Tracking (+2 more)

### Community 14 - "Task Dialog"
Cohesion: 0.20
Nodes (10): devDependencies, svelte, svelte-check, @sveltejs/adapter-static, @sveltejs/kit, @sveltejs/vite-plugin-svelte, @tauri-apps/cli, @types/node (+2 more)

### Community 15 - "System Integration"
Cohesion: 0.22
Nodes (9): scripts, build, check, check:watch, dev, prepare, preview, tauri (+1 more)

### Community 16 - "Time Component"
Cohesion: 0.29
Nodes (8): CodeMirror, Links and Backlinks, Notes, Org Format, Search, Syntax Highlighting, Table Formulas, Vim Keys

### Community 17 - "Picker Component"
Cohesion: 0.29
Nodes (7): AppImage, DMG, Windows Installer, Linux, macOS, Tauri Action, Windows

### Community 18 - "Shared Utilities"
Cohesion: 0.33
Nodes (6): GitHub Actions, Node.js, npm, Rust, SvelteKit, Vite

### Community 20 - "Backup & Sync"
Cohesion: 0.33
Nodes (5): description, identifier, permissions, $schema, windows

### Community 21 - "Notifications"
Cohesion: 0.67
Nodes (4): refresh(), setup(), show(), tracking()

### Community 22 - "GitHub Actions"
Cohesion: 0.40
Nodes (3): @sveltejs/adapter-static, @sveltejs/vite-plugin-svelte, config

### Community 25 - "Community 25"
Cohesion: 0.67
Nodes (3): Emacs, Git, Sync

## Knowledge Gaps
- **138 isolated node(s):** `name`, `version`, `description`, `type`, `dev` (+133 more)
  These have ≤1 connection - possible missing edges. (Counts symbols only; 218 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **8 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Margin` connect `Dialog Components` to `Configuration & Storage`, `Editor Components`, `Time Component`, `Picker Component`, `Shared Utilities`, `Community 25`?**
  _High betweenness centrality (0.226) - this node is a cross-community bridge._
- **Why does `Tauri` connect `Editor Components` to `Notifications`, `Svelte Components`, `Dialog Components`, `Time & Scheduling`?**
  _High betweenness centrality (0.185) - this node is a cross-community bridge._
- **Why does `svelte` connect `Configuration & Storage` to `ICS Calendar Export`?**
  _High betweenness centrality (0.080) - this node is a cross-community bridge._
- **What connects `name`, `version`, `description` to the rest of the system?**
  _138 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Core UI Components` be split into smaller, more focused modules?**
  _Cohesion score 0.07621326042378673 - nodes in this community are weakly interconnected._
- **Should `Tauri Backend Core` be split into smaller, more focused modules?**
  _Cohesion score 0.07567567567567568 - nodes in this community are weakly interconnected._
- **Should `Svelte Components` be split into smaller, more focused modules?**
  _Cohesion score 0.10865191146881288 - nodes in this community are weakly interconnected._