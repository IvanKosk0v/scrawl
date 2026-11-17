# Changelog
All notable changes are documented here. Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
versioning: [Semantic Versioning](https://semver.org/).

## [Unreleased]
- `strokebind run [--device PATH] [--config FILE] [--timeout N]` grabs the mouse, forwards it through a virtual mouse (plain clicks replayed unchanged) and prints recognized strokes.
- `run` refuses to start when the escape chord uses a button the mouse does not have.
- Failsafes for `run`: escape chord (config `escape`, default `left+right+middle`), `--timeout N`, and release of the grab on Ctrl-C, SIGTERM or a crash.
- `list-devices` and `record` skip strokebind's own virtual mice (names starting with `strokebind: `).
- `strokebind record [--device PATH] [--config FILE]` prints the recognized stroke after each trigger release (read-only, no grab).
- `strokebind list-devices` lists mice (name and path) and explains how to fix permission errors.
- Config file parsing (TOML) with line-numbered validation errors.
- CLI skeleton: `record`, `run`, `check-config`, `list-devices` subcommands (stubs).
