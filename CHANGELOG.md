# Changelog

Grouped by `.github/scripts/changelog.sh`, the same generator that writes the GitHub release notes. The releaser agent adds a section here in each release PR.

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
