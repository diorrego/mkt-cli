---
name: rust-ci
description: Rust CI/CD setup and release automation. Activates for GitHub Actions configuration, clippy/rustfmt CI, release workflows, cross-compilation, and deployment questions. Invoke with /rust-ci.
disable-model-invocation: true
---

# Rust CI/CD Skill

## GitHub Actions — Standard CI

```yaml
name: CI
on:
  push:
    branches: [main]
  pull_request:

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: "-Dwarnings"

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2

      - name: Format
        run: cargo fmt --all -- --check

      - name: Clippy
        run: cargo clippy --all-targets --all-features

      - name: Test
        run: cargo test --all-features

      - name: Doc
        run: cargo doc --no-deps --all-features
        env:
          RUSTDOCFLAGS: "-Dwarnings"

  msrv:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: "1.85"
      - run: cargo check --all-features
```

## Release Workflow (cargo-dist)

```yaml
name: Release
on:
  push:
    tags: ["v*"]

jobs:
  release:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: macos-latest
            target: aarch64-apple-darwin
          - os: windows-latest
            target: x86_64-pc-windows-msvc
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - run: cargo build --release --target ${{ matrix.target }}
      - uses: softprops/action-gh-release@v2
        with:
          files: target/${{ matrix.target }}/release/my-binary*
```

## Local CI Checks (pre-push)
```bash
#!/bin/bash
# scripts/ci-local.sh
set -euo pipefail

echo "==> Format check"
cargo fmt --all -- --check

echo "==> Clippy"
cargo clippy --all-targets --all-features -- -D warnings

echo "==> Tests"
cargo test --all-features

echo "==> Doc build"
RUSTDOCFLAGS="-Dwarnings" cargo doc --no-deps --all-features

echo "All checks passed!"
```

## Clippy Configuration
```toml
# clippy.toml
too-many-arguments-threshold = 7
type-complexity-threshold = 300
```

```rust
// In lib.rs or main.rs
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
```

## Cross-Compilation Targets
| Target | Use Case |
|---|---|
| `x86_64-unknown-linux-gnu` | Standard Linux |
| `x86_64-unknown-linux-musl` | Static Linux binary |
| `aarch64-unknown-linux-gnu` | ARM64 Linux |
| `aarch64-apple-darwin` | macOS Apple Silicon |
| `x86_64-pc-windows-msvc` | Windows |
| `wasm32-unknown-unknown` | WebAssembly |
| `thumbv7em-none-eabihf` | ARM Cortex-M (embedded) |

Use `cross` for easy cross-compilation: `cross build --target aarch64-unknown-linux-gnu`
