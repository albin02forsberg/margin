#!/bin/bash
# Bug/Improvement Detector
# Scans the codebase for issues and creates GitHub issues

set -e

cd /home/albin/code/tool

echo "[$(date)] Bug Detector started"

# Function to check if an issue already exists (rough dedup)
issue_exists() {
  local query="$1"
  if gh issue list --search "$query" --state=all --json number | grep -q "number"; then
    return 0
  fi
  return 1
}

# Track issues created this run
declare -A created_issues

# 1. Check for TypeScript errors
echo "Scanning for TypeScript errors..."
if npm run check 2>&1 | grep -i "error TS" > /tmp/ts-errors.txt; then
  while IFS= read -r line; do
    # Extract file path and error
    if [[ $line =~ ([^:]+):([0-9]+):([0-9]+) ]]; then
      file="${BASH_REMATCH[1]}"
      lineno="${BASH_REMATCH[2]}"

      title="TypeScript error in $file:$lineno"

      if ! issue_exists "TypeScript $file"; then
        body="Found TypeScript error in \`$file\` at line $lineno

\`\`\`
$line
\`\`\`

Category: bug
Auto-detected by bug detector."

        gh issue create --title "$title" --body "$body" --label bug,detector-generated
        created_issues["ts-$file-$lineno"]=1
        echo "  Created issue: $title"
      fi
    fi
  done < /tmp/ts-errors.txt
fi

# 2. Find TODO/FIXME comments
echo "Scanning for TODO/FIXME comments..."
if grep -r "TODO\|FIXME" src src-tauri --include="*.ts" --include="*.tsx" --include="*.rs" 2>/dev/null | head -20 > /tmp/todos.txt; then
  while IFS= read -r line; do
    # Extract file and comment
    if [[ $line =~ ([^:]+):.*\((TODO|FIXME)\)(.*)$ ]]; then
      file="${BASH_REMATCH[1]}"
      marker="${BASH_REMATCH[2]}"
      comment="${BASH_REMATCH[3]}"

      title="${marker}: $comment"

      if ! issue_exists "$marker $comment"; then
        body="Found $marker in \`$file\`

\`\`\`
$line
\`\`\`

Category: improvement
Auto-detected by bug detector."

        gh issue create --title "$title" --body "$body" --label improvement,detector-generated
        created_issues["todo-$file"]=1
        echo "  Created issue: $title"
      fi
    fi
  done < /tmp/todos.txt
fi

# 3. Find large files (> 500 lines)
echo "Scanning for large files..."
find src src-tauri -type f \( -name "*.ts" -o -name "*.tsx" -o -name "*.rs" \) | while read -r file; do
  lines=$(wc -l < "$file")
  if [ $lines -gt 500 ]; then
    title="Large file: $file ($lines lines)"

    if ! issue_exists "Large file $file"; then
      body="File \`$file\` has $lines lines and may benefit from refactoring.

Consider splitting into smaller modules.

Category: improvement
Auto-detected by bug detector."

      gh issue create --title "$title" --body "$body" --label refactor,detector-generated
      created_issues["large-$file"]=1
      echo "  Created issue: $title"
    fi
  fi
done

# 4. Summary
echo ""
echo "[$(date)] Bug Detector complete"
echo "  Issues created: ${#created_issues[@]}"
echo "  Total issues found in repo: $(gh issue list --state=open --json number | jq length)"
