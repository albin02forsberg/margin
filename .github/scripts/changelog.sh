#!/usr/bin/env bash
# Release notes for <from>..<to> as markdown, grouped by conventional-commit type (#254).
# The one generator for release.yml, nightly.yml and the releaser agent. Empty <from> = whole history.
# Prints commit subjects only (no hashes: the nightly finds its previous sha by the first 40-hex string).
set -euo pipefail
from=${1:-} to=${2:-HEAD}
git log --no-merges --format=%s "${from:+$from..}$to" | awk '
  /^chore: refresh graphify graph/ || /\[skip ci\]/ || /^docs(\(.*\))?!?:/ || /^chore: release v/ { next }
  n >= 50 { more++; next }
  { n++; s = $0; t = "o"
    if (match(s, /^[a-z]+(\([^)]*\))?!?: */)) { p = substr(s, 1, RLENGTH); s = substr(s, RLENGTH + 1)
      if (p ~ /^feat/) t = "f"; else if (p ~ /^fix/) t = "x" }
    l[t] = l[t] "- " s "\n" }
  END {
    if (!n) { print "No changes."; exit }
    if (!l["f"] && !l["x"]) print "Maintenance only.\n"
    if (l["f"]) printf "### Features\n%s\n", l["f"]
    if (l["x"]) printf "### Fixes\n%s\n", l["x"]
    if (l["o"]) printf "### Other\n%s\n", l["o"]
    if (more) printf "…and %d more.\n", more
  }'
