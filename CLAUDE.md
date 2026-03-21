# Rust Development Environment

This directory contains a comprehensive Rust development toolkit for Claude Code with specialized skills and agents.

## Rust Defaults

- **Edition**: 2024
- **MSRV**: 1.85
- **Formatter**: `rustfmt` (always run before commit)
- **Linter**: `clippy` with `#![warn(clippy::pedantic)]`

## Skills Available

### Core Skills (auto-activate based on context)

- **rust-router** — Master routing skill, automatically directs to the right specialist
- **coding-guidelines** — Rust naming, patterns, modern idioms
- **ownership** — Ownership, borrowing, lifetimes (E0382, E0505, E0597, etc.)
- **error-handling** — Result/Option, thiserror/anyhow, error design
- **concurrency** — async/await, tokio, channels, Send/Sync
- **unsafe-checker** — Unsafe code audit, FFI, raw pointers

### Project Skills (invoke with /skill-name)

- `/rust-architecture` — Project structure, workspace design, crate splitting
- `/rust-testing` — Test strategies, mocking, property-based testing, benchmarks
- `/rust-performance` — Profiling, benchmarking, allocation optimization
- `/rust-deps` — Cargo.toml, dependency selection, feature flags
- `/rust-ci` — GitHub Actions, clippy CI, release workflows
- `/rust-refactor` — Extract function, newtypes, builder pattern, dead code

### Domain Skills (auto-activate with domain context)

- **rust-domain-cli** — CLI tools with clap, TUI, terminal UX
- **rust-domain-web** — Web backends with axum, sqlx, tower
- **rust-domain-embedded** — no_std, embedded-hal, embassy

## Agents Available

| Agent              | Role                                                   | Model  |
| ------------------ | ------------------------------------------------------ | ------ |
| `rust-architect`   | System design, workspace organization, crate splitting | Opus   |
| `rust-reviewer`    | Code review, idiomatic Rust checks, API design review  | Opus   |
| `rust-debugger`    | Diagnose and fix compilation/runtime errors            | Opus   |
| `rust-test-writer` | Create comprehensive test suites                       | Sonnet |
| `rust-optimizer`   | Performance profiling and optimization                 | Opus   |
| `rust-security`    | Security audit, unsafe review, dependency scanning     | Opus   |
| `rust-doc-writer`  | Rustdoc documentation, examples, API docs              | Sonnet |
| `rust-migrator`    | Edition upgrades, dependency migrations                | Sonnet |

## Workflow Conventions

- Run `cargo fmt` before every commit
- Run `cargo clippy --all-targets --all-features` before pushing
- Use `cargo test --all-features` for full test coverage
- Use `cargo audit` before releases
