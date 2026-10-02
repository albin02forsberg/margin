---
name: release
description: Automate version bump, git tag, and push for release
disable-model-invocation: true
---

Automate the release process: bump version in `src-tauri/tauri.conf.json`, commit, create a git tag, and push to GitHub. This triggers the CI/CD pipeline to build and publish installers.

## Usage

```bash
/release <version>
```

Example: `/release 0.2.0`

## What it does

1. Edits `src-tauri/tauri.conf.json` to set `version = "<version>"`
2. Commits with message: `Release v<version>`
3. Creates git tag: `git tag v<version>`
4. Pushes branch and tag: `git push && git push --tags`
5. GitHub Actions CI/CD runs automatically, builds for all OSes, creates draft release

## Notes

- New version must be semantic (e.g., 0.2.0, 1.0.0)
- Verify `npm run check && npm test && cargo test` pass before releasing
- After push, check GitHub Actions to ensure builds complete
- Publish the draft release manually when ready (auto-update will offer it to installed copies)

## Secrets required (in GitHub repo settings)

- `TAURI_SIGNING_PRIVATE_KEY` — Private key for release signing
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` — Password for the key

Run `/release 0.2.0` to bump to 0.2.0 and trigger the release workflow.
