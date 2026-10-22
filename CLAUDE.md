# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

**Margin** — Cross-platform desktop app for notes (org files), tasks, journal, and time tracking.
- Frontend: SvelteKit + TypeScript + Svelte 5 + CodeMirror
- Backend: Rust + Tauri (single-package repo: `/src` frontend, `/src-tauri` backend)
- Multi-platform: Linux (AppImage/deb/rpm), macOS (DMG), Windows (exe) via GitHub Actions

## Build & Test

**Frontend + Backend (dev mode):**
```bash
npm run tauri dev  # Starts Vite dev server + Rust backend together
```

**Frontend only:**
```bash
npm run dev        # Vite dev server (port 5173 by default)
npm run build      # Build frontend bundle
```

**Checks & Tests:**
```bash
npm run check      # Type checking (svelte-kit sync + svelte-check)
npm test           # Run TypeScript tests (node --test src/lib/*.test.ts)
cd src-tauri && cargo test  # Rust tests
```

**Release build:**
```bash
npm run tauri build  # Release binary for current OS only
```

## Release & CI/CD

Version bumping and release process:
1. Edit `src-tauri/tauri.conf.json` to bump version
2. `git tag v<version> && git push --tags`
3. GitHub Actions CI/CD runs, builds for all three OS, creates draft release
4. Manual publish makes live (installed copies auto-update via TAURI_SIGNING_PRIVATE_KEY)

CI/CD secrets needed: `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

## Code Style

- TypeScript strict mode, ESM modules
- Svelte 5 (latest), SvelteKit static adapter (SPA with index.html fallback)
- CodeMirror 6 for editor, Vim bindings enabled
- No linter/formatter configured (type checking via svelte-check only)
- Rust uses Cargo release optimizations (LTO, single codegen unit, stripped)

## Testing

- TypeScript: `node --test` (native Node test runner, no framework)
- Rust: `cargo test`
- Run single TS test: `npm test 2>&1 | grep -A 20 "test_name"`

## Tauri-Specific

- Plugins: global-shortcut, notification, updater, process, tray-icon, single-instance, opener, user-idle
- Backend API: Tauri commands expose Rust functions to frontend (invoke pattern)
- File system: app uses user config dir (TOML: `config.toml`) and data dirs (notes, timeclock)
- Sync: file changes detected via notify-debouncer-mini, triggers frontend refresh
- Tray: app can close to tray or quit, controlled by `close_to_tray` setting

## Repo Conventions

- Main branch: `main` (PRs / CI required before merge)
- Commits: Conventional or clear description (no strict format enforced)
- Conflict resolution: When files sync (Emacs, git, or app editing), non-overlapping changes auto-merge; conflicts flagged

## Agent Workflow

Parallel work runs through checked-in agents and skills (see `.claude/`):
- `/triage [watch]` — the `triager` agent plans issues (`## Plan` comment), labels them, answers comments, closes exact duplicates, already-merged work and issues the owner explicitly says to close (unsure ones get a comment and stay open). Comments it posts start with "🤖 *Claude:*".
- `/ship [issues…] [--max N]` — coordinator: runs `implementer` agents (one worktree per issue, PR → CI → squash-merge) and `pr-shepherd` agents for open PRs, files follow-ups, cleans up worktrees.
- Models: `reviewer`, `scout`, `ideator`, `implementer`, `releaser` run on opus, `triager` and `pr-shepherd` on sonnet (agent frontmatter); `/ship` runs implementers for plain size-`S` plans on sonnet.
- Implementers assign the issue to the owner (`albin02forsberg`) when they start, unless it is already assigned to someone else, and unassign it if they give up.
- Approval gate: defined once in `.claude/skills/ship/SKILL.md` → Survey (`approved` label, issues named explicitly, or a triager + reviewer "ready" comment for size S or M on any issue, owner-written included; L/`epic` is split first; `hold` always blocks; releases, signing, secrets and repo settings are never covered).
- Agents run `cargo test`/`clippy` with a shared `CARGO_TARGET_DIR=~/.cache/margin-agents-target` (safe to `rm -rf`) and skip local checks for docs-only changes (CI's rule).
- `/autopilot [issues…] [--max N] [--hours H] [--all-ready] [--overnight]` — one command for continuous runs: triager in watch mode plus `/ship` rounds, running until stopped (idles on cheap `gh` polling when there is nothing to do; `--hours H` caps it, `--overnight` is an alias for the default). Same approval gate. While waiting, the coordinator fills free slots (next issue, `pr-shepherd`, scout/ideator, triager, worktree cleanup) instead of idling. On start it adopts unfinished worktrees/issues instead of redoing them. Usage limits: `scripts/autopilot-forever.sh` (optionally via `scripts/com.margin.autopilot.plist` under launchd) restarts `/autopilot` after limits and crashes, sleeping until the reset; stop it with `touch ~/.cache/margin-autopilot/STOP`.
- **Autonomous mode:** agents make the decisions. The triager's decider step resolves `needs-decision` (comment `Decision (autonomous)`, overrule by commenting), owner-written issues take the triager + reviewer "ready" route, and L/`epic` issues are split into S/M children. The owner's veto is the `hold` label. Still never autonomous: releases outside the `releaser` gate, signing/secrets, repo settings, user-file format changes without migration + backup.
- **Feature discovery:** the `ideator` rotates lenses (product scan, roadmap, Emacs inspiration, borrowed from other tools, original, all-in-one modules, redesign) and runs whenever the ready queue is below 2×`--max` (at most hourly, max 8 issues).
- Repeat with `/loop /ship`, or `/schedule` a cloud routine for unattended runs. The `scout` agent runs the test suites on `main` and checks CI/nightly (failures become `bug` issues), and scans changed code every ~3 h for bugs, performance and missing features and files `scout` issues (max 5 per run). When the queue is empty, `/autopilot` runs the `ideator` agent, which scans the whole product and files up to 5 `idea` issues (S/M improvements, features, tests, bugfixes). Both go through the triager + reviewer gate like any other issue.
- Releases: `/autopilot` runs the `releaser` agent after ship rounds. It cuts and publishes a stable release on its own (#218) only when CI is green, the nightly built clean for the same commit, the last release is ≥3 days old with ≥3 `feat`/`fix` commits since (or it is a bug hotfix), and no open issue/PR has the `no-release` label (the kill switch). Minor bump for `feat`, else patch, never major; it logs to the `Release log` issue. `/release <version>` stays the owner's manual path.

## Tools

- **graphify** (`graphify-out/`): for architecture / "where is X" questions, read `graphify-out/GRAPH_REPORT.md` and use `graphify query|path|explain` before grepping. The `graphify` workflow refreshes the graph on `main` after each merge; do not commit `graphify-out/` changes in feature PRs.
- **ponytail** ([plugin](https://github.com/dietrichgebert/ponytail), enabled in `.claude/settings.json`): write the minimum code that works — reuse what exists, stdlib before dependencies, no speculative abstractions, one runnable check for non-trivial logic.

## Permission Mode

Always run in auto mode: proceed without asking for confirmation on routine, reversible actions (edits, tests, `gh` reads, branches/PRs per the workflow above). Still confirm destructive or hard-to-reverse actions (force-push, deleting branches/data, releases outside the `releaser` gate) and anything outside the approval gate.
