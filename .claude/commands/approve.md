---
description: Approve (or reject) a numbered proposal from STATE.md and apply it to the plan
argument-hint: P<n> [reject]
---
Argument: $ARGUMENTS
Find that proposal in STATE.md.
- If "reject" is present: remove it and log "rejected P<n>: <reason>" under Decisions. Done.
- Otherwise apply exactly what it describes to PLAN.md (and ARCHITECTURE.md if it adds a dependency), remove it from Proposals,
  log "approved P<n>" under Decisions, and commit as `docs: apply P<n>` on the current branch.
Reply in at most 3 lines: what changed in the plan. Do not start implementing. Stop.
