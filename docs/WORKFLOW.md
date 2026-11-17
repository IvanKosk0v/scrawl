# How to work with Claude Code on this project (for you; not loaded into Claude's context)

## One-time setup (Ubuntu)
1. Install Claude Code (npm package `@anthropic-ai/claude-code`; the official overview docs also describe a native installer).
2. Install Rust via https://rustup.rs (the repo's `rust-toolchain.toml` pulls rustfmt and clippy automatically).
   Optional but recommended: `cargo install cargo-deny cargo-msrv` (check.sh runs deny automatically when installed).
3. `cd strokebind && chmod +x scripts/check.sh .claude/hooks/rustfmt.sh`
4. `git init -b main && git add -A && git commit -m "chore: initial plan and Claude Code setup"`
5. Edit `LICENSE-MIT` and put your real name in the copyright line.
6. Run `claude` in the folder and sign in. Run `/context` once to see the baseline, `/hooks` and `/permissions` to confirm they loaded.
7. For device testing from M2 on: `sudo usermod -aG input $USER`, then log out and in.
8. When ready to publish: create the GitHub repo, `git remote add origin ...`, push `main` yourself (Claude never pushes).

## The whole interface: 8 commands
| You type | What happens | Cost |
|---|---|---|
| `/status` | 8-line progress report | tiny |
| `/next` | one plan task, test-first, on a milestone branch, committed, then stops | the real work |
| `/note <text>` | appends to INBOX.md, nothing else | tiny |
| `/inbox` | turns INBOX.md into numbered proposals (P1, P2...) | small |
| `/approve P3` or `/approve P3 reject` | applies or rejects a proposal; only this edits the plan | tiny |
| `/ask <question>` | answers from the docs, edits nothing | small |
| `/review` | independent reviewer subagent on the last change | small |
| `/milestone` | verifies acceptance, reviews, asks you, then merges to main | medium |
| `/handoff` then `/clear` | saves state, resets context | tiny |

Typical day: `/status` -> `/next` -> `/next` -> `/review` -> `/handoff` -> `/clear`.
Having an idea mid-task: `/note IDEA: support scroll-wheel gestures`. Later `/inbox`, then `/approve P1` or reject it.

## Why Claude stays on plan
- One task per `/next`, with an acceptance line it must prove with named tests and the `check.sh` result.
- It cannot change scope, add tasks or dependencies; it can only write numbered proposals. You decide with `/approve`.
- Branch per milestone, Conventional Commits, no direct commits to main, no pushing, `sudo` and force operations denied.
- `check.sh` is the gate: fmt, clippy `-D warnings`, tests, cargo-deny. The same checks run in GitHub CI.
- Definition of done (CLAUDE.md): failing test first, no unwrap/todo/dbg outside tests, docs on public items, CHANGELOG for user-visible changes.
- The `reviewer` subagent checks plan compliance, grab-release safety, test quality and dependencies in a separate context.

## Token and cost habits (highest impact first)
1. Context is re-sent every turn: `/handoff` then `/clear` between tasks. A fresh session reads only STATE.md + PLAN.md.
2. CLAUDE.md loads every turn: keep it small. Detail lives in docs/ and is read on demand.
3. Default model is Sonnet (settings.json). Use `/model` for a stronger model only on M3 (grab/passthrough) and M5, then switch back.
4. Plan mode (Shift+Tab) before M3 tasks.
5. Quiet tooling: `check.sh` prints "OK" or only the failure.
6. Subagents for noisy work (`/review`, `/milestone`).
7. Inspect with `/context` and `/cost`. Use `/compact` only mid-task when you can't `/clear`.
8. Don't add MCP servers or extra skills to this project; each adds baseline tokens.

## Safety for hardware tests
M3 grabs your real mouse. First grab test: run with `--timeout 10`. Know how to kill it from the keyboard
(Ctrl+C in the terminal, or switch to a TTY with Ctrl+Alt+F3 and `pkill strokebind`). Keep a second pointing device handy.

## Known limits of this setup
- It was assembled without running Claude Code itself. If a command or hook misbehaves, run `/hooks`, `/permissions` and `/help`
  and compare with `.claude/`. Permission rule syntax follows the common `Bash(cmd:*)` form.
- `deny.toml` and the CI workflow were written without running cargo-deny or GitHub Actions. Task M0.4 makes them pass for real.
- Compliance with instructions is strong but not guaranteed; `/review` and `/milestone` exist to catch drift.
