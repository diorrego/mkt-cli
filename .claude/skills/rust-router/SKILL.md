---
name: rust-router
description: Master routing skill for Rust development. Automatically activates when working with Rust code to route queries to the most appropriate specialized skill based on error codes, keywords, and context.
user-invocable: false
---

# Rust Skill Router v1.0

You are the central routing intelligence for Rust development skills. When a Rust-related task arrives, analyze it and load the appropriate skill(s).

## Routing Table

### By Error Code
| Error Code | Route To | Context |
|---|---|---|
| E0382, E0505, E0507, E0515 | ownership | Move/borrow errors |
| E0597, E0716, E0106 | ownership | Lifetime errors |
| E0277, E0308 | error-handling | Trait/type mismatch |
| E0433, E0432 | rust-deps | Import/module errors |
| E0599 | coding-guidelines | Method not found |
| E0133 | unsafe-checker | Unsafe block required |
| E0521, E0117 | coding-guidelines | Orphan rule violations |

### By Keyword
| Keywords | Route To |
|---|---|
| `async`, `tokio`, `futures`, `Send`, `Sync`, `spawn`, `Arc`, `Mutex`, `channel` | concurrency |
| `ownership`, `borrow`, `lifetime`, `move`, `&mut`, `'a`, `Drop` | ownership |
| `Result`, `Option`, `?`, `anyhow`, `thiserror`, `Error`, `unwrap`, `panic` | error-handling |
| `unsafe`, `FFI`, `*mut`, `*const`, `transmute`, `MaybeUninit` | unsafe-checker |
| `bench`, `flamegraph`, `criterion`, `perf`, `allocat`, `cache`, `SIMD` | rust-performance |
| `test`, `#[test]`, `assert`, `mock`, `proptest`, `quickcheck`, `coverage` | rust-testing |
| `architecture`, `design`, `module`, `workspace`, `crate structure`, `monorepo` | rust-architecture |
| `Cargo.toml`, `dependency`, `feature`, `version`, `semver`, `publish` | rust-deps |
| `CI`, `CD`, `GitHub Actions`, `clippy`, `rustfmt`, `release`, `deploy` | rust-ci |
| `refactor`, `simplify`, `extract`, `rename`, `dead code`, `cleanup` | rust-refactor |
| `clap`, `CLI`, `argument`, `subcommand`, `terminal`, `TUI` | rust-domain-cli |
| `axum`, `actix`, `warp`, `tower`, `HTTP`, `REST`, `API`, `web`, `server` | rust-domain-web |
| `no_std`, `embedded`, `HAL`, `register`, `interrupt`, `cortex`, `RTOS` | rust-domain-embedded |

### Dual-Skill Loading
When domain context is detected, load BOTH the domain skill AND the relevant technical skill:
- FinTech + error → `error-handling` + domain context
- Embedded + concurrency → `concurrency` + `rust-domain-embedded`
- Web API + testing → `rust-testing` + `rust-domain-web`

## Priority Hierarchy
1. Error code match (highest precision)
2. Explicit keyword match
3. Domain context detection
4. Default to `coding-guidelines`

## Agent Delegation
For complex tasks that benefit from focused execution, delegate to specialized agents:
- Architecture design → `rust-architect` agent
- Code review → `rust-reviewer` agent
- Debugging → `rust-debugger` agent
- Test creation → `rust-test-writer` agent
- Performance → `rust-optimizer` agent
- Security audit → `rust-security` agent
