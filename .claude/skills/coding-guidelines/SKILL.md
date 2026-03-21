---
name: coding-guidelines
description: Rust coding guidelines and conventions. Activates for style questions, naming conventions, idiomatic Rust patterns, and general best practices. Use when writing new Rust code or reviewing existing code for idiom compliance.
---

# Rust Coding Guidelines

## Defaults
- **Edition**: 2024
- **MSRV**: 1.85
- **Formatter**: rustfmt (always run before commit)
- **Linter**: clippy with `#![warn(clippy::pedantic)]`

## Naming Conventions

### Functions
| Pattern | Usage | Example |
|---|---|---|
| `as_*` | Cheap reference-to-reference conversion | `as_str()`, `as_bytes()` |
| `to_*` | Expensive conversion, creates new value | `to_string()`, `to_vec()` |
| `into_*` | Consumes self, returns different type | `into_inner()`, `into_bytes()` |
| `from_*` | Associated function constructor | `from_str()`, `from_parts()` |
| `try_*` | Fallible version of another function | `try_from()`, `try_lock()` |
| `with_*` | Builder-pattern setter | `with_capacity()`, `with_timeout()` |
| `is_*` / `has_*` | Boolean query | `is_empty()`, `has_key()` |

**Do NOT use `get_` prefix** — use bare name: `fn name(&self) -> &str` not `fn get_name()`.

### Types
- Structs: `PascalCase` — `HttpClient`, `TokenBucket`
- Enums: `PascalCase` variants — `Color::DarkBlue`
- Traits: prefer adjective/verb — `Readable`, `Display`, `Iterator`
- Type aliases: `PascalCase` — `type Result<T> = std::result::Result<T, MyError>`

### Modules & Crates
- Modules: `snake_case` — `mod http_client;`
- Crates: `kebab-case` in Cargo.toml — `my-cool-crate`

## Data Types
- Prefer `&str` over `String` in function parameters
- Use `Cow<'_, str>` when ownership is conditional
- Use newtypes to enforce domain invariants: `struct UserId(u64);`
- Prefer `Vec<T>` over `&[T]` only when you need ownership
- Use `Box<[T]>` for fixed-size owned slices

## Error Handling
- Use `thiserror` for library error types
- Use `anyhow` for application error types
- Never `unwrap()` in library code — use `expect()` with a message or propagate with `?`
- Implement `std::error::Error` for all custom error types

## Modern Replacements
| Deprecated | Use Instead |
|---|---|
| `lazy_static!` | `std::sync::LazyLock` |
| `once_cell::Lazy` | `std::sync::LazyLock` |
| `once_cell::sync::OnceCell` | `std::sync::OnceLock` |
| `mem::uninitialized()` | `MaybeUninit<T>` |
| `mem::zeroed()` for non-zero types | `MaybeUninit<T>` |
| `trait_object` without `dyn` | `dyn Trait` explicitly |

## Patterns
- Prefer `impl Trait` over `Box<dyn Trait>` when possible
- Use `#[must_use]` on functions whose return value should not be ignored
- Prefer `Default::default()` over manual zero/empty initialization
- Use `#[non_exhaustive]` on public enums and structs
- Derive order: `Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize`

## Structure
- One type per file when the type has significant implementation
- Group related functionality in modules, not mega-files
- `pub(crate)` by default — only `pub` for the public API
- Re-export key types from `lib.rs` for ergonomic imports
