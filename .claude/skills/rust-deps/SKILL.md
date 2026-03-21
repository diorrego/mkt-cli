---
name: rust-deps
description: Rust dependency management and Cargo.toml configuration. Activates for dependency selection, feature flags, version management, workspace dependencies, publishing, and Cargo.toml questions.
---

# Rust Dependencies Skill

## Cargo.toml Best Practices

### Edition & MSRV
```toml
[package]
edition = "2024"
rust-version = "1.85"
```

### Dependency Specification
```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }   # With features
tokio = { version = "1", features = ["full"] }
log = "0.4"                                            # Minimal version

# Optional dependencies
serde_json = { version = "1.0", optional = true }

[features]
default = []
json = ["dep:serde_json"]
```

### Workspace Dependencies (monorepo)
```toml
# Root Cargo.toml
[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1", features = ["full"] }

# Crate Cargo.toml
[dependencies]
serde = { workspace = true }
tokio = { workspace = true }
```

## Recommended Crates by Category

### Essential
| Category | Crate | Purpose |
|---|---|---|
| Serialization | `serde` + `serde_json` | De/serialization framework |
| Error (lib) | `thiserror` | Derive Error for library types |
| Error (app) | `anyhow` | Flexible application errors |
| Logging | `tracing` | Structured, async-aware logging |
| CLI | `clap` (derive) | Command-line argument parsing |
| HTTP client | `reqwest` | Ergonomic HTTP client |
| HTTP server | `axum` | Modular web framework |
| Async runtime | `tokio` | Industry-standard async runtime |
| Database | `sqlx` | Compile-time checked SQL |
| Testing | `insta` | Snapshot testing |
| Date/time | `chrono` or `time` | Date and time handling |

### Performance
| Crate | Purpose |
|---|---|
| `rayon` | Parallel iterators |
| `dashmap` | Concurrent hashmap |
| `bytes` | Efficient byte buffer |
| `smallvec` | Stack-allocated small vectors |
| `criterion` | Benchmarking framework |
| `compact_str` | Optimized small strings |

### Development
| Crate | Purpose |
|---|---|
| `cargo-watch` | Auto-rebuild on changes |
| `cargo-expand` | Expand macros |
| `cargo-deny` | Audit dependencies |
| `cargo-udeps` | Find unused dependencies |
| `cargo-hakari` | Workspace-hack for build speed |

## Dependency Audit
```bash
# Security audit
cargo audit

# License check
cargo deny check licenses

# Find unused deps
cargo udeps

# Check for outdated deps
cargo outdated
```

## Version Policy
- Pin major version: `serde = "1"` (accepts 1.x.y)
- Pin minor for stability: `tokio = "1.40"` (accepts 1.40.x)
- Exact pin for reproducibility: `my-crate = "=1.2.3"`
- Use `cargo update` to update within semver range
