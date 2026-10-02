---
name: triage
description: Plan and label the Margin repo's open GitHub issues and answer new comments, via the triager agent. Use when the owner asks to plan issues, check the tracker, reply to comments, or invokes /triage. Pass "watch" to keep watching for new activity.
argument-hint: "[watch] [issue numbers…]"
disable-model-invocation: true
---

Run the `triager` agent (`.claude/agents/triager.md`) in the background, without worktree isolation (it doesn't edit code). Arguments: `$ARGUMENTS`.
- Prompt: "Do a pass" (limited to the given issue numbers, if any). If `watch` was passed, add "then watch mode".
- When it reports, relay to the owner briefly:
  - the issues it planned (links);
  - issues it closed as duplicates, already done or on the owner's instruction, and ones it flagged but left open for the owner to close;
  - the ready-to-build issues, with their sizes;
  - the decisions only the owner can make.
- Ready issues still need the owner's `approved` label before `/ship` builds them, unless the owner names them directly (`/ship 63 65`).
- In watch mode, resume the triager with SendMessage after each report until the owner says stop. Stop it with TaskStop when wrapping up.
