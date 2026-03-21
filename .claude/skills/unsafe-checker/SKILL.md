---
name: unsafe-checker
description: Unsafe Rust code reviewer. Activates when encountering unsafe blocks, FFI code, raw pointers, transmute, MaybeUninit, or when reviewing code for memory safety. Use explicitly with /unsafe-checker to audit unsafe code.
---

# Unsafe Rust Checker

## SAFETY Comment Requirement

Every `unsafe` block MUST have a `// SAFETY:` comment explaining why the invariants are upheld:

```rust
// SAFETY: We verified that `ptr` is non-null and properly aligned
// in the check above. The lifetime is bounded by `self`.
unsafe { &*ptr }
```

## Unsafe Review Checklist

For each `unsafe` block, verify:

- [ ] **Justification**: Is `unsafe` actually necessary? Can this be done safely?
- [ ] **SAFETY comment**: Does it explain WHY the invariants hold, not just WHAT it does?
- [ ] **Pointer validity**: Are all pointers non-null, aligned, and pointing to valid memory?
- [ ] **Aliasing**: Are there no mutable aliases to the same memory?
- [ ] **Lifetime correctness**: Do references not outlive the data they point to?
- [ ] **Initialization**: Is all memory properly initialized before reads?
- [ ] **Thread safety**: Is access properly synchronized across threads?
- [ ] **Exception safety**: What happens if a panic occurs inside the unsafe block?

## Deprecated → Modern Replacements

| Deprecated | Use Instead | Why |
|---|---|---|
| `mem::uninitialized()` | `MaybeUninit<T>` | UB for any type with invalid values |
| `mem::zeroed()` (non-zero types) | `MaybeUninit<T>` | UB for `NonZero*`, references, etc. |
| `transmute` between integer sizes | `as` cast or `try_into()` | Safer, clearer intent |
| `transmute` for &T ↔ &U | `bytemuck::cast_ref` | Verified at compile time |
| `slice::from_raw_parts` unchecked | Validate length + alignment first | Prevents buffer overflows |

## FFI Guidelines

### Crate Recommendations
| Task | Crate |
|---|---|
| C header → Rust bindings | `bindgen` |
| Rust → C header | `cbindgen` |
| Python bindings | `PyO3` |
| Node.js bindings | `napi-rs` |
| WebAssembly | `wasm-bindgen` |

### FFI Function Checklist
- [ ] `extern "C"` ABI specified
- [ ] `#[no_mangle]` on exported functions
- [ ] Null pointer checks on all pointer parameters
- [ ] Error codes returned (not panics — panics across FFI boundary are UB)
- [ ] `catch_unwind` at FFI boundary if Rust code might panic
- [ ] String encoding documented (UTF-8 vs null-terminated C string)

### Common FFI Patterns
```rust
// Returning errors across FFI
#[no_mangle]
pub extern "C" fn my_func(ptr: *const u8, len: usize) -> i32 {
    let result = std::panic::catch_unwind(|| {
        // SAFETY: caller guarantees ptr is valid for len bytes
        let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
        process(slice)
    });
    match result {
        Ok(Ok(())) => 0,
        Ok(Err(_)) => -1,
        Err(_) => -2, // panic occurred
    }
}
```

## Red Flags in Unsafe Code
- `transmute` without size/alignment validation
- Raw pointer arithmetic without bounds checking
- `unsafe impl Send/Sync` without justification
- `unsafe` blocks with no SAFETY comment
- Casting function pointers
- `union` types without clear invariant documentation
