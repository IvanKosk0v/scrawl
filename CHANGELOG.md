# Changelog
All notable changes are documented here. Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
versioning: [Semantic Versioning](https://semver.org/).

## [Unreleased]
- `strokebind record [--device PATH] [--config FILE]` prints the recognized stroke after each trigger release (read-only, no grab).
- `strokebind list-devices` lists mice (name and path) and explains how to fix permission errors.
- Config file parsing (TOML) with line-numbered validation errors.
- CLI skeleton: `record`, `run`, `check-config`, `list-devices` subcommands (stubs).
