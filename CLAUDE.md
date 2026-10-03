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
- Models: `reviewer`, `scout`, `implementer` run on opus, `triager` and `pr-shepherd` on sonnet (agent frontmatter); `/ship` runs implementers for plain size-`S` plans on sonnet.
- Implementers assign the issue to the owner (`albin02forsberg`) when they start, unless it is already assigned to someone else, and unassign it if they give up.
- Approval gate: `/ship` builds issues labelled `approved` (owner adds it), ones named explicitly, or ones the `triager` and `reviewer` agents agree are small and clear (a comment naming both agents and why it is small; no label needed). `needs-decision` = waiting on the owner and always blocks. Releases, secrets and repo settings are never covered.
- `/autopilot [issues…] [--max N] [--hours H] [--all-ready] [--overnight]` — one command for continuous runs: triager in watch mode plus `/ship` rounds, until the queue is empty or time is up (`--overnight`: 12 h, idles on cheap `gh` polling instead of stopping). Same approval gate.
- Repeat with `/loop /ship`, or `/schedule` a cloud routine for unattended runs. The `scout` agent scans changed code every ~3 h for bugs, performance and missing features and files `scout` + `needs-decision` issues (no per-run cap). Releases stay manual (`/release`).

## Tools

- **graphify** (`graphify-out/`): for architecture / "where is X" questions, read `graphify-out/GRAPH_REPORT.md` and use `graphify query|path|explain` before grepping. The `graphify` workflow refreshes the graph on `main` after each merge; do not commit `graphify-out/` changes in feature PRs.
- **ponytail** ([plugin](https://github.com/dietrichgebert/ponytail), enabled in `.claude/settings.json`): write the minimum code that works — reuse what exists, stdlib before dependencies, no speculative abstractions, one runnable check for non-trivial logic.

## Permission Mode

Always run in auto mode: proceed without asking for confirmation on routine, reversible actions (edits, tests, `gh` reads, branches/PRs per the workflow above). Still confirm destructive or hard-to-reverse actions (force-push, deleting branches/data, releases) and anything outside the approval gate.
