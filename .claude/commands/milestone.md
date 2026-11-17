---
description: Close the current milestone: verify every acceptance line, then (on user confirmation) merge to main
---
1. Confirm every task of the current milestone is ticked. For each acceptance line cite the test or evidence. Run `./scripts/check.sh`.
2. Use the reviewer subagent on `git diff main...HEAD`. List any HIGH/MED findings.
3. If any gap or HIGH/MED finding exists: list them (max 8 lines) and STOP.
4. If clean: show a 5-line summary and ask the user to confirm the merge. Only after "yes": `git switch main && git merge --no-ff <branch>`,
   set STATE.md to the next milestone and first task, commit `chore: close M<N>`. Do not push; remind the user to push and open CI.
