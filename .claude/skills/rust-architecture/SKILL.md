---
name: rust-architecture
description: Rust project architecture and design. Use for project structure decisions, workspace organization, module hierarchy, crate splitting, dependency injection, and trait-based design. Invoke with /rust-architecture for architecture reviews.
---

# Rust Architecture Skill

## Project Structure Patterns

### Single Crate (small-medium project)
```
my-project/
├── Cargo.toml
├── src/
│   ├── lib.rs          # Public API re-exports
│   ├── main.rs         # Binary entry (if applicable)
│   ├── config.rs       # Configuration
│   ├── error.rs        # Error types
│   ├── domain/         # Core business logic (no I/O)
│   │   ├── mod.rs
│   │   ├── models.rs
│   │   └── services.rs
│   ├── infra/          # External integrations (DB, HTTP, FS)
│   │   ├── mod.rs
│   │   ├── database.rs
│   │   └── http.rs
│   └── api/            # Public interface (CLI, REST, gRPC)
│       ├── mod.rs
│       └── handlers.rs
├── tests/              # Integration tests
└── benches/            # Benchmarks
```

### Workspace (large project)
```
my-workspace/
├── Cargo.toml          # [workspace] members
├── crates/
│   ├── core/           # Domain logic, zero external deps
│   ├── storage/        # Database, file system
│   ├── api/            # HTTP/gRPC handlers
│   ├── cli/            # CLI binary
│   └── common/         # Shared types and utilities
```

## Crate Splitting Heuristics

Split into a separate crate when:
- A module has a distinct set of dependencies
- You need different feature flags
- Build times would benefit from parallel compilation
- The module is reusable across projects
- Different release cadence is needed

Do NOT split when:
- It's premature — start with modules, split later
- It would create circular dependencies
- The overhead of managing crates outweighs benefits

## Trait-Based Design

### Dependency Inversion
```rust
// Define trait in the core crate (no I/O dependencies)
pub trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: UserId) -> Result<Option<User>>;
    async fn save(&self, user: &User) -> Result<()>;
}

// Implement in the infrastructure crate
pub struct PostgresUserRepo { pool: PgPool }

impl UserRepository for PostgresUserRepo {
    async fn find_by_id(&self, id: UserId) -> Result<Option<User>> { ... }
    async fn save(&self, user: &User) -> Result<()> { ... }
}

// Service depends on trait, not impl
pub struct UserService<R: UserRepository> {
    repo: R,
}
```

### When to Use Traits vs Enums
| Use Trait When | Use Enum When |
|---|---|
| Open set of implementations | Closed set of variants |
| External implementors expected | All variants known at compile time |
| Different behavior per impl | Pattern matching on variant |
| Dynamic dispatch needed | Exhaustive handling required |

## Module Visibility Strategy
- `pub(crate)` — default for internal types
- `pub` — only for the public API surface
- `pub(super)` — share with parent module only
- Private — implementation details

## Feature Flag Conventions
```toml
[features]
default = []
serde = ["dep:serde", "dep:serde_json"]
tokio = ["dep:tokio"]
full = ["serde", "tokio"]
```
- Never enable heavy deps by default
- Use `dep:` syntax to avoid implicit features
- Document each feature in the crate-level docs
