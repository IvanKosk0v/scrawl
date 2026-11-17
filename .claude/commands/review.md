---
description: Independent review of uncommitted or recent changes via the reviewer subagent
argument-hint: [commit-range, default HEAD~1]
---
Use the reviewer subagent on `git diff ${ARGUMENTS:-HEAD~1}`. Return only its findings, highest severity first, max 10 lines.
