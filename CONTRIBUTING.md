# Contributing to mkt

Thanks for your interest in contributing to mkt! This document covers everything you need to know to get started.

## Getting Started

### Prerequisites

- Rust 1.85 or later (we use the 2024 edition)
- Git

### Setup

```bash
git clone https://github.com/diorrego/mkt-cli.git
cd mkt-cli
cargo build --workspace
cargo test --workspace --all-features
```

## Development Workflow

1. Fork the repo and create a feature branch from `main`
2. Make your changes
3. Run the checks before pushing:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features
cargo test --workspace --all-features
```

4. Open a pull request against `main`

## Code Style

We follow standard Rust conventions with a few project-specific rules:

- **No `unwrap()` or `panic!()` in library code.** Use the `?` operator and proper error types.
- **All public items need doc comments.** The `missing_docs` lint will catch anything you miss.
- **Use newtype IDs** like `CampaignId(String)` instead of bare strings.
- **Keep functions under 40 lines.** If it's getting long, extract a helper.
- **Use `tracing` for logging.** Never `println!` in library code.

### Formatting

We use `rustfmt` with the 2024 edition settings. Run `cargo fmt --all` before committing.

### Linting

Clippy runs with pedantic warnings enabled. Run `cargo clippy --workspace --all-targets --all-features` and fix any warnings.

## Testing

Every change should include tests:

- **Unit tests** for pure logic (config parsing, model conversion, output formatting)
- **Integration tests** with `wiremock` for provider API interactions
- **E2E tests** with `assert_cmd` for CLI commands

Test fixtures go in `crates/mkt-testkit/src/fixtures/`.

```bash
# Run all tests
cargo test --workspace --all-features

# Run tests for a specific crate
cargo test -p mkt-meta

# Run a specific test
cargo test -p mkt-core config
```

## Adding a New Provider

1. Create a new crate at `crates/mkt-<name>/`
2. Implement the `MarketingProvider` trait from `mkt-core`
3. Add a feature flag in `crates/mkt-cli/Cargo.toml`
4. Register it in the CLI command dispatch
5. Add tests using `mkt-testkit` helpers

Look at `mkt-meta` as the reference implementation.

## Pull Requests

- Keep PRs focused on a single change
- Write a clear description of what changed and why
- Make sure CI passes before requesting review
- Use the PR template

## Reporting Issues

Use the GitHub issue templates for bug reports and feature requests. Include as much detail as you can, especially reproduction steps for bugs.

## License

By contributing, you agree that your contributions will be licensed under the same terms as the project (MIT or Apache-2.0, at the user's choice).
