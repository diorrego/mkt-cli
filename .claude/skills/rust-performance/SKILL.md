---
name: rust-performance
description: Rust performance optimization. Activates for benchmarking, profiling, allocation reduction, cache optimization, SIMD, zero-copy patterns, and performance-related questions. Invoke with /rust-performance.
---

# Rust Performance Skill

## Profiling Workflow

1. **Measure first** — never optimize blindly
2. **Profile** → identify hotspots
3. **Benchmark** → establish baseline
4. **Optimize** → apply targeted fix
5. **Benchmark again** → verify improvement

### Profiling Tools
| Tool | Use For | Command |
|---|---|---|
| `cargo flamegraph` | CPU profiling | `cargo flamegraph --bin my-app` |
| `perf` | Linux CPU profiling | `perf record --call-graph dwarf ./target/release/my-app` |
| `DHAT` (valgrind) | Heap allocation profiling | `valgrind --tool=dhat ./target/release/my-app` |
| `cargo-show-asm` | Inspect generated assembly | `cargo asm my_crate::my_func` |
| `criterion` | Microbenchmarks | In `benches/` directory |
| `hyperfine` | CLI benchmark | `hyperfine './target/release/my-app'` |

## Common Optimizations

### Allocation Reduction
```rust
// BAD: allocates on every iteration
for item in items {
    let s = format!("prefix_{}", item);
    process(&s);
}

// GOOD: reuse buffer
let mut buf = String::new();
for item in items {
    buf.clear();
    write!(&mut buf, "prefix_{}", item).unwrap();
    process(&buf);
}
```

### Pre-allocation
```rust
// BAD
let mut v = Vec::new();
for i in 0..1000 { v.push(i); }

// GOOD
let mut v = Vec::with_capacity(1000);
for i in 0..1000 { v.push(i); }
```

### String Optimization
- `&str` over `String` where possible
- `SmallString` / `compact_str` for short strings
- `String::with_capacity` when size is known
- `Cow<'_, str>` for conditionally owned strings

### Collection Selection
| Need | Collection | Why |
|---|---|---|
| Sequential access | `Vec<T>` | Cache-friendly, contiguous memory |
| Key-value lookup | `HashMap` / `BTreeMap` | O(1) / O(log n) lookup |
| Unique set | `HashSet` / `BTreeSet` | Deduplication |
| FIFO queue | `VecDeque<T>` | Efficient push/pop at both ends |
| Sorted data | `BTreeMap`/`BTreeSet` | Ordered iteration |
| Small fixed set | `[T; N]` or `ArrayVec` | Stack-allocated, no heap |

### Zero-Copy Patterns
- `bytes::Bytes` for shared byte buffers
- `memmap2` for memory-mapped files
- `zerocopy` for safe transmutation
- `nom`/`winnow` for zero-copy parsing

## Compile-Time Optimization
```toml
# Cargo.toml
[profile.release]
lto = "thin"        # Link-time optimization
codegen-units = 1   # Better optimization, slower build
strip = true        # Strip debug symbols
panic = "abort"     # Smaller binary

[profile.release.build-override]
opt-level = 3
```

## Red Flags
- `clone()` in hot loops
- `String` allocation where `&str` suffices
- `Box<dyn Trait>` in performance-critical paths (vtable indirection)
- `HashMap` with small N (linear scan on `Vec` is faster for N < ~20)
- Collecting into Vec just to iterate again
- `format!()` for string building in loops
