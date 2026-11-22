# scrawl

Rust daemon: mouse-stroke gestures on Linux, any Wayland compositor (and X11).
Reads mouse via evdev, exclusive grab, forwards via uinput, runs actions on recognized strokes.

## Source of truth (read only what the task needs)
1. `STATE.md` current milestone/task, handoff, decisions, numbered proposals (P1, P2...)
2. `PLAN.md` milestones + acceptance criteria. The plan is rigid.
3. `docs/ARCHITECTURE.md` design decisions, allowed dependencies
4. `docs/RESEARCH.md` market/feature context. Only for README/launch work.

## Rules
- Work ONLY on the current milestone in STATE.md. One task per `/next`.
- Never change PLAN.md scope, add tasks, or add dependencies on your own. Write a numbered proposal in STATE.md
  and stop. Only `/approve P<n>` from the user authorizes the change.
- Ask the user at most ONE question, only when blocked. Otherwise decide and log it under Decisions.
- Safety: never run the daemon with a grab on the user's real mouse unless the task says so, and then with `--timeout`.
  Grab code must release on drop/panic/SIGINT/SIGTERM. Never use sudo. Never push.
- Do not claim a task done without evidence: name the passing tests and quote `./scripts/check.sh` output.
  Never weaken, skip, or delete a test to get green. If a requirement is unclear, stop and ask.

## Git
- Branch per milestone: `m<N>-<slug>`, created from `main`. Never commit to `main` directly.
- Conventional Commits (`feat:`, `fix:`, `test:`, `docs:`, `refactor:`, `chore:`), one commit per task, subject <= 72 chars.
- Merging a finished milestone into `main` happens only in `/milestone`, after the user confirms.

## Definition of done (every task)
1. Failing test written first, then minimal code. Recognizer logic also gets property tests (proptest).
2. `./scripts/check.sh` prints OK (fmt, clippy -D warnings, tests, deny if installed).
3. No `unwrap`/`expect`/`todo!`/`dbg!` outside tests. Public items documented with `///`.
4. User-visible change: add a line under `[Unreleased]` in CHANGELOG.md.
5. PLAN.md box ticked, STATE.md updated (3-line handoff), commit made.

## Code style
Rust 2021, MSRV in Cargo.toml. `unsafe` only in `src/sys/` with a `// SAFETY:` comment on each use.
Pure logic separated from device I/O. Errors: `thiserror` in library code, `anyhow` only in `main`.
Small modules, small diffs. No speculative abstractions.

## Output discipline (token economy)
Use `./scripts/check.sh` (quiet), never raw cargo logs. Don't read `target/`. Read file ranges, not whole files.
Final reply of any command: the length its command file specifies, no recap of the diff.
Only need files which are actually needed.

## Compact instructions
When compacting keep: current task id, failing output, files changed, decisions, open proposals. Drop exploration noise.
