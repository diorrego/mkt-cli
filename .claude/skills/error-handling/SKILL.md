---
name: error-handling
description: Rust error handling patterns and strategies. Activates for Result/Option usage, custom error types, error propagation with ?, anyhow/thiserror usage, panic handling, and error type design questions.
---

# Error Handling Skill

## Strategy Selection

| Context | Crate | Pattern |
|---|---|---|
| Library crate | `thiserror` | Custom enum error types with `#[error(...)]` |
| Application binary | `anyhow` | `anyhow::Result<T>` with `.context()` |
| Mixed (lib + bin) | Both | `thiserror` in lib, `anyhow` in bin |
| `no_std` | Neither | Manual `Error` impl or custom approach |

## Error Type Design

### Library Error Enum Pattern
```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MyError {
    #[error("failed to parse config: {0}")]
    ConfigParse(#[from] toml::de::Error),

    #[error("IO error: {path}")]
    Io {
        #[source]
        source: std::io::Error,
        path: PathBuf,
    },

    #[error("invalid value: {value} (expected {expected})")]
    InvalidValue { value: String, expected: String },
}
```

### Rules
1. Every variant should have a meaningful `#[error("...")]` message
2. Use `#[from]` for direct 1:1 error conversions
3. Use `#[source]` when adding context to an underlying error
4. Make error types `#[non_exhaustive]` in public APIs
5. Include enough context for debugging (file paths, values, indices)

## Propagation Patterns

### The `?` Operator
```rust
// Good: propagate with context
let file = File::open(&path)
    .with_context(|| format!("failed to open {}", path.display()))?;

// Bad: bare ? with no context
let file = File::open(&path)?;
```

### Converting Between Error Types
```rust
// Option → Result
let value = map.get("key").ok_or(MyError::KeyNotFound)?;
let value = map.get("key").ok_or_else(|| MyError::KeyNotFound { key: "key".into() })?;

// Result → Option (when you want to discard the error)
let value = parse_config().ok();
```

## Anti-Patterns

| Anti-Pattern | Fix |
|---|---|
| `unwrap()` in library code | Use `?` or `expect("reason")` |
| `Box<dyn Error>` as public API | Define a proper error enum |
| String errors (`Err("failed".into())`) | Use typed errors |
| Catching panics for control flow | Use `Result` instead |
| Ignoring errors with `let _ =` | At minimum log them, or use `if let Err(e) =` |
| `panic!` in production code paths | Reserve for truly unrecoverable states |

## Option Combinators

Prefer combinators over match when intent is clear:
```rust
// Instead of match on Option
option.unwrap_or_default()
option.unwrap_or_else(|| compute_default())
option.map(|v| transform(v))
option.and_then(|v| might_fail(v))
option.filter(|v| v.is_valid())
```

## Error Context Checklist
When adding errors, ensure they answer:
- **What** failed? (the operation)
- **Why** it failed? (the cause)
- **Where** it failed? (file, line, key, index — whatever is relevant)
