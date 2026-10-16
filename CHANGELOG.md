# Changelog

Grouped by `.github/scripts/changelog.sh`, the same generator that writes the GitHub release notes. The releaser agent adds a section here in each release PR.

## v0.4.0 - 2026-10-16

### Features
- date ranges on every agenda day, and time-range ends (#423) (#435)
- skip subtrees tagged :ARCHIVE: in Today, Tasks and the calendar feed (#426) (#433)
- Done log in Today, a group per day of tasks closed that day (c) (#424) (#431)
- inherit tags from parent headings and #+FILETAGS (#422) (#430)
- move a note to another folder, keeping links to and from it working (#412) (#420)
- honour a deadline's own warning period (DEADLINE: <... -3d>) (#411) (#417)
- closed: term in task search for a Done-this-week view (#410) (#415)
- AI chat answers from your notes via keyword + graph retrieval (#304) (#407)
- mark calls as meetings from mic/camera use (opt-in, macOS) (#125) (#408)
- remember the UI zoom level across restarts (#332) (#401)
- stamp CLOSED: when tasks are done; board Done column shows this week's (#393) (#400)
- ask before quitting with edits that clash with the disk (#281) (#313)
- go to a date in the agenda (#380) (#387)
- copy [[id:]] link to heading at cursor (#382) (#388)
- Time: I stopped earlier… (#379) (#384)
- journal template for new journal entries (#381) (#385)
- Settings category chips and filter field (#357) (#378)
- project cards with open tasks on the Projects page (#357) (#377)
- Agenda chips strip on Today (#357) (#376)
- Tasks List/Board toggle (#357) (#374)
- Time view extras: worked today, balance with reports, project cards (#357) (#373)
- Today clock column (running clock, worked today, week bars) (#357) (#371)
- reopen the last session's tabs on startup (#337)
- view-change fade and sliding top-bar highlight (#356) (#367)
- fold saved views into More, About opens Settings, help above the pill (#355) (#366)
- editor, tab strip and side panels as cards (#354) (#365)
- top bar replaces the sidebar (#353) (#364)

### Fixes
- step Ctrl+wheel zoom by accumulated delta, not per event (#439) (#445)
- filter the Tasks board Done column on the backend (#437) (#442)
- cheaper note scoring for 'Use my notes' retrieval (#438) (#441)
- wait for the previous picker to close before Start tracking (#436) (#440)
- resolve '..' in relative file links before opening (#432) (#434)
- deadline cookies in reminders, keep repeater on set_planning, parse timestamps without a day name (#421) (#427)
- macOS Cmd+Q runs the quit handshake (#409) (#416)
- stop the project-settings e2e deleting its fixture mid-edit (#405) (#406)
- log time off through a palette flow; CSV export says why time off wrote no row (#250) (#403)
- take the time-log lock for activity appends (#346) (#402)
- blend the Vim status line into the editor card, frame and shrink screenshots (#399)
- apply the journal template to any new note in the daily folder (#392) (#397)
- refuse 'stop earlier' before the session start (#391) (#396)
- no hover lift on Agenda group cards (#370) (#372)

### Other
- enforce clippy in the macOS job; fix non-Linux unused Emitter import (#443) (#444)
- cargo check on macOS for PRs touching src-tauri (#419)
- e2e for restore from .trash, Ctrl+L, go to heading, Inbox (#428)
- e2e for table Tab/TBLFM and attachment cleanup (#414)
- Arch Linux package (margin-bin) on releases (#127) (#404)
- let a second pass before stopping the timer (#375) (#398)
- focus the agenda before D in the task-keys e2e (#394) (#395)
- e2e for the journal (Ctrl+J, Space n e, n [ / n ]) and task archive / move (#383) (#386)
- harden e2e palette() and guard startup sync before config loads (#389) (#390)

…and 56 more.

## v0.3.0 - 2026-10-11

### Features
- export codes for time off, in their own leave.toml (#221) (#230)
- opt-in nightly builds and update channel (#213) (#220)
- stream chat answers from Ollama and the built-in model, cancel both (#210) (#224)
- chat sends the selection or section around the cursor of long notes (#211) (#217)
- insert or copy a chat answer (#209) (#215)
- time off (vacation, sick, other) lowers expected hours (#208) (#212)

### Fixes
- cut an over-long chat selection to the byte limit, and say so (#227) (#233)
- refuse time off in Hours-a-day mode with an empty hours field (#228) (#232)
- clean cut-off and streaming chat answers like finished ones (#226) (#231)
- link clang compiler-rt on macOS for the embedded AI build (#223) (#225)
- skip kdotool on GNOME Wayland when polling the active window (#206) (#229)
- cap Ollama chat answers at 512 tokens and wait up to 5 min (#210) (#216)

### Other
- widen approval gate to S/M, add ideator agent, cap scout (#214) (#219)
- releaser agent auto-publishes stable releases behind a gate (#218) (#222)
