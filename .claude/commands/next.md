---
description: Do the next unchecked task of the current milestone, test-first, then stop
argument-hint: [optional note for this task]
---
1. Read STATE.md and PLAN.md. Identify the first unchecked task of the current milestone.
2. If not on the milestone branch (see PLAN.md header), create it from main: `git switch -c m<N>-<slug>`.
3. Read only the files this task touches (plus docs/ARCHITECTURE.md if design-relevant).
4. Write the failing test(s) first, then the minimum code to pass. Follow CLAUDE.md "Definition of done".
5. Run `./scripts/check.sh` until it prints OK. Do not weaken tests.
6. Tick the box in PLAN.md, update STATE.md (current task + 3-line handoff + decisions), update CHANGELOG if user-visible.
7. Commit (Conventional Commits). Do not push.
8. Reply in at most 5 lines: what was done, the evidence (test names + check.sh result), next task id. Then STOP.
If blocked: ask ONE question. If the task needs scope/dependency change: add a numbered proposal to STATE.md and stop.
Note from user: $ARGUMENTS
