# Agent Workflow System Setup

This document explains the multi-agent workflow system for the Margin project.

## Overview

The workflow consists of 4 components:

1. **Issue Watcher** — Monitors new issues, generates plans, manages approvals
2. **Pool Manager** — Maintains a 3-agent pool, spawns implementation agents
3. **Bug Detector** — Scans codebase, creates issues automatically
4. **PR Reviewer** — Reviews PRs, handles conflicts, auto-merges when ready

## Quick Start

### 1. State File

The workflow state is stored in `.claude/workflow-state.json`:
```json
{
  "awaiting_approval": [],    // Issues awaiting your approval
  "approved_issues": [],       // Issues ready for implementation
  "in_progress": [],           // Currently being worked on
  "completed": []              // Historical records
}
```

### 2. Issue Workflow

**Flow:** Create Issue → Watcher posts plan → You approve → Pool Manager spawns agent → PR Reviewer merges

**Steps:**
1. Create an issue on GitHub
2. Issue Watcher detects it and posts a plan comment
3. Review the plan comment and add the `approved` label when ready
4. Pool Manager spawns an implementation agent in the next cycle
5. Implementation agent creates a PR
6. PR Reviewer reviews and approves the PR
7. PR gets merged automatically when ready

### 3. Approval Workflow

After the Issue Watcher posts a plan comment:

- **To approve:** Add the `approved` label to the issue
- **To revise:** Comment `replan` or add comments with feedback
- **To skip:** Close the issue or don't add the label

### 4. Running the Agents

#### Manual Trigger (Testing)

```bash
# Test Issue Watcher
cd /home/albin/code/tool
# Read `.claude/scripts/issue-watcher.md` and run the steps manually
# Or invoke a Claude agent with the prompt

# Test Pool Manager
# Similar process with `.claude/scripts/pool-manager.md`

# Test PR Reviewer
# Similar process with `.claude/scripts/pr-reviewer.md`

# Run Bug Detector
bash .claude/scripts/bug-detector.sh
```

#### Automated (Recommended)

Set up scheduled tasks using CronCreate:

```
# Issue Watcher (every 15 minutes)
/cron CronCreate \
  --schedule "*/15 * * * *" \
  --name "Issue Watcher" \
  --command "cd /home/albin/code/tool && claude-invoke-agent issue-watcher"

# Pool Manager (every 30 minutes)
/cron CronCreate \
  --schedule "*/30 * * * *" \
  --name "Pool Manager" \
  --command "cd /home/albin/code/tool && claude-invoke-agent pool-manager"

# Bug Detector (daily at 2am UTC)
/cron CronCreate \
  --schedule "0 2 * * *" \
  --name "Bug Detector" \
  --command "cd /home/albin/code/tool && bash .claude/scripts/bug-detector.sh"

# PR Reviewer (every 10 minutes)
/cron CronCreate \
  --schedule "*/10 * * * *" \
  --name "PR Reviewer" \
  --command "cd /home/albin/code/tool && claude-invoke-agent pr-reviewer"
```

> **Note:** Actual CronCreate setup requires Claude Code integration. See documentation on `/help` for details.

### 5. Implementation Agent

When an issue is approved, Pool Manager spawns an implementation agent. The agent:

1. Reads the issue and approved plan
2. Creates a feature branch: `git checkout -b fix/issue-<N>`
3. Implements the changes per the plan
4. Creates commits with clear messages
5. Pushes branch and creates a PR linking back to the issue
6. Reports completion

The agent does **NOT** merge the PR — PR Reviewer handles that.

### 6. Monitoring

Check state.json to see current progress:

```bash
cat .claude/workflow-state.json | jq '.'
```

Watch the git history for agent activity:

```bash
git log --oneline --all | head -20
```

## Customization

### Change Max Concurrent Agents

Edit `.claude/scripts/pool-manager.md`:
```markdown
- **Max concurrent agents:** 3
```
Change `3` to your desired limit.

### Adjust Bug Detector Sensitivity

Edit `.claude/scripts/bug-detector.sh`:
- Large file threshold: change `500` to different line count
- Add/remove scan types (TODO/FIXME, TypeScript errors, etc.)

### Modify PR Reviewer Rules

Edit `.claude/scripts/pr-reviewer.md`:
- `auto-merge policy` section controls when PRs are auto-merged
- Adjust checks that must pass before merge
- Change approval requirements

### Change Schedule

Edit the cron expressions:
- `*/15 * * * *` = every 15 minutes
- `*/30 * * * *` = every 30 minutes
- `0 2 * * *` = daily at 2am UTC

See [crontab.guru](https://crontab.guru) for expression help.

## Debugging

### Issue Watcher not detecting issues?

- Check that issues are created by your GitHub account
- Verify `.claude/workflow-state.json` is readable
- Check agent logs for errors (if running via CronCreate)

### Agents not spawning?

- Verify `approved_issues` queue has items in state.json
- Check that `in_progress` count is < 3
- Ensure git authentication is working (`git push` test)

### PR Reviewer not merging?

- Check PR CI status (must pass before merge)
- Look for merge conflicts in PR
- Verify PR is linked to an approved issue

## Workflow Examples

### Example 1: Simple Bug Fix

1. Create issue: "Fix typo in README"
2. Issue Watcher posts plan: "Change 'teh' to 'the' in line 42"
3. You add `approved` label
4. Pool Manager spawns agent
5. Agent edits file and creates PR
6. PR Reviewer auto-merges
7. Issue closed, PR merged ✅

### Example 2: Feature with Revisions

1. Create issue: "Add dark mode"
2. Issue Watcher posts plan (5 steps)
3. You review and comment: "Can you explain step 3?"
4. You comment `replan` to request revision
5. Watcher regenerates plan
6. You review revised plan and add `approved` label
7. Agent implements and creates PR
8. Reviewer requests code review changes
9. Agent pushes fixes
10. Reviewer approves and merges ✅

## Limitations & Future Improvements

- Max 3 concurrent agents (hardcoded, change in pool-manager.md)
- Bug detector runs daily (can make more frequent)
- No automatic issue assignment (manual approval required)
- PR conflicts require manual resolution in some cases
- No integration with external CI/CD (yet)

## Troubleshooting

**Q: Workflow state got corrupted**
A: Backup current state.json, edit manually to fix issues, or run `git checkout .claude/workflow-state.json` to reset.

**Q: An agent is stuck**
A: Move the issue back to `approved_issues`, restart the agent via manual trigger.

**Q: PR Reviewer merged something wrong**
A: Revert the merge with `git revert <commit>` and fix manually.

**Q: Too many false positives from Bug Detector**
A: Adjust the scan rules in `bug-detector.sh` or add a denylist for known false positives.
