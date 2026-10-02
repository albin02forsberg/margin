---
name: verify-all
description: Run all checks and tests (TypeScript, Svelte, Rust) in one command
---

Run the full verification suite to catch issues before commit. This skill runs in sequence (type checks first, then tests) and fails on any error.

## Usage

```bash
/verify-all
```

## What it does

1. **TypeScript & Svelte checks:** `npm run check` (syncs SvelteKit and runs svelte-check in strict mode)
2. **TypeScript tests:** `npm test` (Node test runner on `src/lib/*.test.ts`)
3. **Rust tests:** `cd src-tauri && cargo test` (all backend unit and integration tests)

Stops at first failure and prints the error. On success, all tests pass.

## When to use

- Before committing (catch mistakes early)
- Before pushing a PR (verify nothing breaks CI)
- After a big refactor (ensure no regressions)
