---
description: Triage INBOX.md into proposals/state, then empty it
---
Read INBOX.md. For each item:
- BUG: if it breaks a done task's acceptance, add a numbered proposal "fix: ..." to STATE.md; else add as a proposal.
- IDEA / PLAN CHANGE / PRIORITY: add a numbered proposal (P<n>) to STATE.md with a one-line cost/benefit. Never edit PLAN.md.
- QUESTION: answer in the reply.
Then empty INBOX.md (keep its header comments).
Reply with a table: item, where recorded (P<n> or answered). Max 10 lines. End with "Use /approve P<n> or /reject P<n>." Stop.
