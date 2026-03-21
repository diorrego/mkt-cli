---
name: rust-refactor
description: Rust refactoring patterns and techniques. Activates for code simplification, extract function/module, rename, dead code removal, and API redesign. Invoke with /rust-refactor.
---

# Rust Refactoring Skill

## Refactoring Workflow

1. **Ensure tests pass** before starting
2. **Make one change at a time** — commit between steps
3. **Run tests after each change**
4. **Use compiler as guide** — let errors lead the refactor

## Common Refactorings

### Extract Function
Identify a block of code doing one thing and extract it:
```rust
// Before
fn process(data: &[u8]) -> Result<Output> {
    // 20 lines of validation
    // 30 lines of transformation
    // 10 lines of formatting
}

// After
fn process(data: &[u8]) -> Result<Output> {
    let validated = validate(data)?;
    let transformed = transform(&validated)?;
    format_output(&transformed)
}
```

### Replace Boolean Parameters with Enum
```rust
// Before
fn connect(host: &str, use_tls: bool) { ... }

// After
enum Transport { Plaintext, Tls }
fn connect(host: &str, transport: Transport) { ... }
```

### Replace Stringly-Typed Code with Newtypes
```rust
// Before
fn create_user(name: String, email: String, id: String) { ... }

// After
struct UserName(String);
struct Email(String);
struct UserId(String);
fn create_user(name: UserName, email: Email, id: UserId) { ... }
```

### Builder Pattern for Complex Construction
When a struct has many optional fields or complex initialization:
```rust
pub struct ServerConfig { ... }

pub struct ServerConfigBuilder {
    port: u16,
    host: String,
    max_connections: Option<usize>,
}

impl ServerConfigBuilder {
    pub fn new(port: u16) -> Self { ... }
    pub fn host(mut self, host: impl Into<String>) -> Self { ... }
    pub fn max_connections(mut self, n: usize) -> Self { ... }
    pub fn build(self) -> ServerConfig { ... }
}
```

### Convert `match` Chains to Method Dispatch
```rust
// Before
match shape {
    Shape::Circle(r) => area_circle(r),
    Shape::Rect(w, h) => area_rect(w, h),
}

// After — impl on the enum
impl Shape {
    fn area(&self) -> f64 {
        match self {
            Self::Circle(r) => std::f64::consts::PI * r * r,
            Self::Rect(w, h) => w * h,
        }
    }
}
```

## Dead Code Detection
```bash
# Compiler warnings
cargo check 2>&1 | grep "warning.*dead_code"

# Unused dependencies
cargo udeps --all-targets

# Unused features
cargo unused-features analyze
```

## API Simplification Checklist
- [ ] Can any parameters be derived from other parameters?
- [ ] Can related parameters be grouped into a struct?
- [ ] Is there a sensible `Default` implementation?
- [ ] Can `impl Into<T>` make the API more ergonomic?
- [ ] Are there builder methods that could replace complex constructors?
- [ ] Can lifetime parameters be elided?
