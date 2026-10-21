# Graph Report - margin  (2026-10-21)

## Corpus Check
- 145 files · ~1,022,577 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 15 file(s) not represented in the graph (top: .css 8, (none) 4, .icns 1)

## Summary
- 2100 nodes · 4857 edges · 133 communities (94 shown, 39 thin omitted)
- Extraction: 98% EXTRACTED · 2% INFERRED · 0% AMBIGUOUS · INFERRED: 105 edges (avg confidence: 0.86)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `751cad1f`
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
- State
- Notes
- AppImage
- ics.rs
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
- lib/TaskDialog.svelte
- generate
- config.rs
- CLAUDE.md
- State
- remind.rs
- day_prompt
- release/SKILL.md
- paths.test.ts
- verify-all/SKILL.md
- chatctx.ts
- App Icon
- dependencies
- Configuration
- DMG
- Tauri Global Shortcut Plugin
- Windows Installer
- Journal
- super
- Node.js
- Tauri Notification Plugin
- Tauri Single Instance Plugin
- Tauri Action
- ollama
- Time Tracking
- TypeScript
- cfg
- setup.ts
- backup.rs
- r4.js
- pins.test.ts
- triager.md
- extension.js
- coverage-ratchet.ts
- session.ts
- []
- b-variants.js
- notes.rs
- download
- load
- TS
- ideator.md
- OrgFile
- merge.test.ts
- lib.rs
- Changelog
- ship/SKILL.md
- changelog.sh
- ref_node_assert
- wayland_idle.rs
- .parse
- main
- releaser.md
- itemSwap
- autopilot/SKILL.md
- retrieve.rs
- mock.js
- scout.md
- r5.js
- bugreport.test.ts
- devDependencies
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
- lib/TableDialog.svelte
- projects.ts
- org.rs
- settings.ts
- createState
- App
- links.ts
- NOTE_WRITES
- media-size.ts
- svelte
- footnote.ts
- lib/Settings.svelte
- svelte.config.js
- Kw
- vite.config.js
- set_keyword
- whatsnew.test.ts
- engines
- narrow.ts
- R
- zoom.test.ts

## God Nodes (most connected - your core abstractions)
1. `App` - 114 edges
2. `cfg()` - 49 edges
3. `[]` - 44 edges
4. `Tc` - 42 edges
5. `OrgFile` - 37 edges
6. `Kw` - 34 edges
7. `kw()` - 28 edges
8. `w()` - 21 edges
9. `reload_config()` - 20 edges
10. `to_lines()` - 20 edges

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

## Communities (133 total, 39 thin omitted)

### Community 0 - "screenshots.ts"
Cohesion: 0.13
Nodes (25): ADR-0007, ADR-0008, button(), clip(), clips, config, day(), fixtures() (+17 more)

### Community 1 - "timeclock.rs"
Cohesion: 0.05
Nodes (83): add_past_sessions(), adjust_start_refuses_to_overlap_the_previous_session(), append_diary(), apply_carry(), breaks_accumulate_and_flex(), Calendar, clock_flow_on_disk(), clock_out_earlier_refuses_before_the_start() (+75 more)

### Community 2 - "AppHandle"
Cohesion: 0.14
Nodes (24): ai_chat(), ai_download(), ai_draft(), ai_model_delete(), ai_models(), ai_set(), apply_config(), apply_spellcheck() (+16 more)

### Community 3 - "editor.ts"
Cohesion: 0.06
Nodes (37): Block, BULLETS, clicks, convertToTable(), findTable(), hide, hooks, inTable() (+29 more)

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
Cohesion: 0.10
Nodes (20): description, license, name, type, version, @codemirror/commands, @codemirror/language, @codemirror/language-data (+12 more)

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
Cohesion: 0.13
Nodes (15): scripts, build, check, check:watch, coverage, coverage:ratchet, dev, e2e:build (+7 more)

### Community 16 - "Notes"
Cohesion: 0.29
Nodes (7): Links and Backlinks, Notes, Org Format, Search, Syntax Highlighting, Table Formulas, Vim Keys

### Community 18 - "ics.rs"
Cohesion: 0.16
Nodes (18): builds_feed(), CALENDAR, calendar_from_a_url(), CALENDAR_TTL, date(), event(), events_on(), feed() (+10 more)

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

### Community 24 - "Value"
Cohesion: 0.13
Nodes (16): dash_fixture(), dashboard(), dashboard_and_status_show_the_clock(), dashboard_builds_the_week(), dashboard_projects_month(), dashboard_week_runs_monday_to_sunday(), DashIn, keys() (+8 more)

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
Cohesion: 0.18
Nodes (27): curLine(), drawerRange(), expandTemplate(), headings(), insertBlock(), insertMode(), insertTable(), line() (+19 more)

### Community 33 - "lib/TaskDialog.svelte"
Cohesion: 0.18
Nodes (10): svelte, preview, active, close(), duePreview, error, key(), save() (+2 more)

### Community 34 - "generate"
Cohesion: 0.15
Nodes (15): Backend, Embedded, Ollama, body(), chat(), chat_messages(), chats(), clean() (+7 more)

### Community 35 - "config.rs"
Cohesion: 0.08
Nodes (28): add_view(), add_views(), atomic_writes(), Config, defaults_round_trip(), expand(), extra_days_off(), public_holidays() (+20 more)

### Community 36 - "CLAUDE.md"
Cohesion: 0.17
Nodes (10): Agent Workflow, Build & Test, Code Style, Permission Mode, Project, Release & CI/CD, Repo Conventions, Tauri-Specific (+2 more)

### Community 37 - "State"
Cohesion: 0.12
Nodes (36): activity_suggestions(), agenda(), ai_retrieve(), capture_insert(), capture_template(), export_html_fragment(), export_linked(), export_markdown() (+28 more)

### Community 38 - "remind.rs"
Cohesion: 0.06
Nodes (19): app_menu(), ask(), ASKED, BUSY, GO, IDLE, quit_reply(), SEQ (+11 more)

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

### Community 45 - "dependencies"
Cohesion: 0.15
Nodes (13): dependencies, @codemirror/commands, @codemirror/language, @codemirror/language-data, @codemirror/search, @codemirror/state, @codemirror/view, @lezer/highlight (+5 more)

### Community 52 - "super"
Cohesion: 0.20
Nodes (3): source(), start(), step()

### Community 57 - "ollama"
Cohesion: 0.20
Nodes (10): read_err(), available_gb(), free_gb(), ollama(), ollama_chat(), parse(), read_chat(), ticket() (+2 more)

### Community 60 - "cfg"
Cohesion: 0.15
Nodes (16): ai_excluded(), ai_note_prompt(), capture_templates(), cfg(), clock_out_before_setup(), config(), export_open(), is_daily_path() (+8 more)

### Community 61 - "setup.ts"
Cohesion: 0.12
Nodes (35): at(), buckets, day, ev(), events, y, session(), answer() (+27 more)

### Community 62 - "backup.rs"
Cohesion: 0.12
Nodes (36): active_profile_is_untracked_and_survives_the_pull(), activity_files_union_merge(), attributes_are_appended_once(), BACKUP, backup_syncs_the_data_repo(), clean_env(), commit(), commit_all() (+28 more)

### Community 63 - "r4.js"
Cohesion: 0.05
Nodes (26): agendaDays, backlinks, board, byTag, DEMO, DEMOS, done3, doneWeek (+18 more)

### Community 64 - "pins.test.ts"
Cohesion: 0.70
Nodes (3): orderNotes(), parsePins(), togglePin()

### Community 65 - "triager.md"
Cohesion: 0.40
Nodes (4): Pass, Report (ends your run), Rules for what you post, Watch mode (only when asked)

### Community 67 - "coverage-ratchet.ts"
Cohesion: 0.25
Nodes (8): Baseline, Count, parseLcov(), pct(), regressions(), Report, Suite, summarize()

### Community 68 - "session.ts"
Cohesion: 0.25
Nodes (7): Review, Rules, Entry, KINDS, ok(), parseSession(), Session

### Community 69 - "[]"
Cohesion: 0.05
Nodes (48): [], addDays(), copyList(), dateChip(), day, dayInput(), elapsed(), fmtDay() (+40 more)

### Community 70 - "b-variants.js"
Cohesion: 0.14
Nodes (29): b1(), b2(), b3(), b4(), b5(), bars(), blocks, card() (+21 more)

### Community 71 - "notes.rs"
Cohesion: 0.07
Nodes (41): adjacency(), ANY_LINK, backlinks(), ensure_id(), ensures_ids(), files(), gen_id(), Graph (+33 more)

### Community 72 - "download"
Cohesion: 0.25
Nodes (7): chat_http(), download(), download_resumes_and_checks(), download_stall_times_out(), part(), serve(), sha256_file()

### Community 73 - "load"
Cohesion: 0.20
Nodes (8): cancel_draft(), cancel_hits_only_the_running_draft(), CANCELLED, load(), run(), RUNNING, TICKETS, whole()

### Community 74 - "TS"
Cohesion: 0.29
Nodes (8): move_planning(), parse_ts(), planning(), planning_ts(), set_planning(), set_planning_keeps_repeater(), shift_planning(), TS

### Community 76 - "OrgFile"
Cohesion: 0.14
Nodes (19): open_entries(), add_interval(), all_tags(), Cache, Habit, Headline, Item, norm_time() (+11 more)

### Community 77 - "merge.test.ts"
Cohesion: 0.42
Nodes (7): applyText(), Change, diffChange(), mergeEdits(), perKey(), apply(), merged()

### Community 78 - "lib.rs"
Cohesion: 0.05
Nodes (32): attach_bytes(), attach_file(), attach_in(), attach_in_numbers_clashes_retries_races_and_rolls_back(), attachment_files(), attachment_files_skips_trash_and_unused_ignores_linked(), CANCEL, context_of() (+24 more)

### Community 79 - "Changelog"
Cohesion: 0.14
Nodes (13): Changelog, Features, Features, Features, Fixes, Fixes, Fixes, Other (+5 more)

### Community 80 - "ship/SKILL.md"
Cohesion: 0.25
Nodes (7): 1. Survey, 2. Dispatch, 3. Review each report, 4. Clean up, 5. Report to the owner, Rules, Running it repeatedly

### Community 82 - "ref_node_assert"
Cohesion: 0.11
Nodes (11): Chip, chips(), items, layout(), Pt, empty, asUrl(), linkFor() (+3 more)

### Community 84 - ".parse"
Cohesion: 0.23
Nodes (15): agenda(), agenda_week(), d(), dates(), deadline_warning_cookie(), effort_parses_org_durations(), habits(), overdue_and_move() (+7 more)

### Community 85 - "main"
Cohesion: 0.33
Nodes (5): Finish, House rules, Report (your final message), Start, main()

### Community 86 - "releaser.md"
Cohesion: 0.33
Nodes (5): 1. Gate (all must hold, else report which failed and stop), 2. Bump PR, 3. Tag, build, publish, Report, Rules

### Community 87 - "itemSwap"
Cohesion: 0.50
Nodes (6): indent(), isItem(), itemSwap(), renumber(), fix(), swap()

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

### Community 94 - "bugreport.test.ts"
Cohesion: 0.48
Nodes (5): BugEnv, bugReportUrl(), isError(), ISSUES, osName()

### Community 95 - "devDependencies"
Cohesion: 0.18
Nodes (11): devDependencies, svelte, svelte-check, @sveltejs/adapter-static, @sveltejs/kit, @sveltejs/vite-plugin-svelte, @tauri-apps/cli, @types/node (+3 more)

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
Cohesion: 0.13
Nodes (12): active(), apply(), Ev, App, Closed, State, Title, focused() (+4 more)

### Community 111 - "lib/TableDialog.svelte"
Cohesion: 0.33
Nodes (5): active, close(), [hc, hr], key(), closed()

### Community 112 - "projects.ts"
Cohesion: 0.24
Nodes (10): Card, cards(), merge(), Project, ROUNDINGS, Row, Task, toProject() (+2 more)

### Community 113 - "org.rs"
Cohesion: 0.10
Nodes (31): ALIAS, archive_done_moves_top_most_done_subtrees_in_order(), capture_template(), clone_with_time_shift(), CLOSED, COOKIE, CURSOR, DOC (+23 more)

### Community 114 - "settings.ts"
Cohesion: 0.18
Nodes (17): CATEGORIES, changes(), checkTemplates(), checkViews(), Field, Form, fromForm(), GROUPS (+9 more)

### Community 115 - "createState"
Cohesion: 0.14
Nodes (11): createState(), emphasis(), Num, numExt(), orgShiftTab(), setSpellcheck(), spellExt(), vimInsert() (+3 more)

### Community 116 - "App"
Cohesion: 0.10
Nodes (35): activity_dismiss(), activity_learn(), activity_rules_delete(), activity_rules_list(), App, backup_now(), day_done_now(), dismissed() (+27 more)

### Community 117 - "links.ts"
Cohesion: 0.47
Nodes (7): bareTitle(), esc(), findTarget(), head(), linkHeadings(), norm(), splitTarget()

### Community 119 - "media-size.ts"
Cohesion: 0.17
Nodes (8): DIRS, EACH, GIFS, mb(), problems(), scan(), TOTAL, script

### Community 120 - "svelte"
Cohesion: 0.33
Nodes (3): title(), SvelteKit, Vite

### Community 121 - "footnote.ts"
Cohesion: 0.29
Nodes (8): followFootnote(), insertFootnote(), recordJump(), addFootnote(), FN, footnoteTarget(), nextLabel(), add()

### Community 123 - "svelte.config.js"
Cohesion: 0.40
Nodes (3): @sveltejs/adapter-static, @sveltejs/vite-plugin-svelte, config

### Community 124 - "Kw"
Cohesion: 0.18
Nodes (13): cycle_priority(), edit_heading(), heading(), Kw, parse(), parse_effort(), Parts, set_priority() (+5 more)

### Community 126 - "set_keyword"
Cohesion: 0.31
Nodes (9): at(), closed_stamp(), cycle_and_repeat(), cycle_todo(), fmt_time(), heading_end(), notes(), repeat_with_unclosed_properties_stays_put() (+1 more)

### Community 127 - "whatsnew.test.ts"
Cohesion: 0.43
Nodes (4): Notes, notesText(), notesTitle(), notesToShow()

### Community 130 - "narrow.ts"
Cohesion: 0.22
Nodes (10): @codemirror/state, @codemirror/view, hidden(), hideBlock, Narrow, narrowedTo(), narrowEffect, narrowField (+2 more)

### Community 132 - "R"
Cohesion: 0.08
Nodes (42): ai_day_prompt(), append_journal_template(), bad_meeting_pattern_and_calendar_only_warn(), capture_file(), capture_file_fills_the_pattern_under_notes(), capture_path(), check_in(), csv_dest() (+34 more)

### Community 133 - "zoom.test.ts"
Cohesion: 0.67
Nodes (4): clamp(), nextZoom(), parseZoom(), wheelZoom()

## Knowledge Gaps
- **371 isolated node(s):** `changelog.sh script`, `q`, `sched`, `late`, `habit` (+366 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 641 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **39 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `start()` connect `setup.ts` to `lib/Settings.svelte`, `+page.svelte`, `[]`?**
  _High betweenness centrality (0.118) - this node is a cross-community bridge._
- **Why does `line()` connect `level` to `+page.svelte`, `editor.ts`, `decorate`, `[]`?**
  _High betweenness centrality (0.054) - this node is a cross-community bridge._
- **Why does `[]` connect `[]` to `orgtable.ts`, `ref_node_assert`, `setup.ts`, `svelte`, `lib/Settings.svelte`, `+page.svelte`?**
  _High betweenness centrality (0.040) - this node is a cross-community bridge._
- **Are the 7 inferred relationships involving `[]` (e.g. with `start()` and `loadClock()`) actually correct?**
  _`[]` has 7 INFERRED edges - model-reasoned connections that need verification._
- **What connects `changelog.sh script`, `q`, `sched` to the rest of the system?**
  _371 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `screenshots.ts` be split into smaller, more focused modules?**
  _Cohesion score 0.13105413105413105 - nodes in this community are weakly interconnected._
- **Should `timeclock.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.051412250079339895 - nodes in this community are weakly interconnected._