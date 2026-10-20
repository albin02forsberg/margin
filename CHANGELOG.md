# Changelog

Grouped by `.github/scripts/changelog.sh`, the same generator that writes the GitHub release notes. The releaser agent adds a section here in each release PR.

## v0.5.0 - 2026-10-20

### Features
- show release notes after a self-update (#513) (#527)
- pasted and dropped images go to a shared images/ folder next to the note (#429) (#521)
- public holiday setting and extra days off (#505) (#515)
- expected hours per weekday (#506) (#511)
- spell check setting for org notes (#507) (#512)
- follow [[*Heading]], [[#custom-id]] and file::search links (#504) (#509)
- weekday names look back when logging a past session (#497) (#503)
- :set number / relativenumber in the editor (#496) (#500)
- y copies a link to the selected task in Today/Tasks (#478) (#493)
- rename a task from Today/Tasks (e) and the editor (Space x e) (#477) (#491)
- selection becomes link text; paste a URL over a selection (#479) (#488)
- AI chat sends tasks, agenda and time log; source switches and ai_exclude (#468) (#484)
- 'Stop repeating' pick in the s/d date prompt (norepeat) (#474) (#486)
- Alt+J/K moves a list item with its nested lines (#472) (#487)
- log a past session from the Time view and Space t n (#476) (#485)
- SCHEDULED delay cookie (-Nd) (#482)
- word count (and selection count) in the editor status bar (#460) (#467)
- Every day / Every week quick picks in the s/d date prompt (#462) (#465)
- accept a warning cookie ('fri -3d') in date inputs (#461) (#466)
- bold / italic / code / strike-through the selection or word (#451) (#457)
- set a repeater from date inputs ('fri +1w', 'today .+1d') (#449) (#456)
- renumber an ordered list after Enter / Alt+Enter adds an item (#450) (#454)

### Fixes
- skip full refresh for attachment/noise file changes (#524) (#528)
- move-note picker hides the images folder; neutral .trash message (#522) (#525)
- last subtree no longer takes in the trailing newline (#519)
- name the real reason when a session edit ends before its start (#499) (#502)
- ai_exclude patterns accept a leading / and backslashes (#498) (#501)
- show delayed (-Nd) SCHEDULED on date+N in the agenda (#489) (#492)
- word count skips org markup and punctuation (#473) (#490)
- edit_session refuses zero, overlapping and future end times (#475) (#483)
- resolve a relative CSV name against export_dir in tc_csv (#463) (#464)
- validate the tc_csv destination path (#458) (#459)
- reject non-time words after a date in read_date (#448) (#453)

### Other
- e2e for priority keys and Enter on links (#520)
- e2e for org structure keys (#508) (#510)
- de-flake the time e2e start-earlier check (#494) (#495)
- e2e for Time break/resume, switch project, started/stopped earlier, CSV export (#455)

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
