# Contributing

1. Open an issue first for anything beyond a small fix.
2. Branch from `main`, keep changes focused.
3. Run `./scripts/check.sh` (fmt, clippy `-D warnings`, tests). CI runs the same checks.
4. Add tests for behavior changes. Recognizer logic must be covered by unit and property tests.
5. Commits follow [Conventional Commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`, `docs:`, `test:`, `refactor:`, `chore:`).
6. User-visible changes get a line under `## [Unreleased]` in CHANGELOG.md.
7. `unsafe` is allowed only in `src/sys/`, with a `// SAFETY:` comment on every use.

By contributing you agree your work is dual-licensed under MIT or Apache-2.0.
