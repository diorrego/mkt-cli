---
name: ownership
description: Rust ownership, borrowing, and lifetimes specialist. Activates on move errors (E0382, E0505, E0507, E0515), lifetime errors (E0597, E0716, E0106), borrow checker issues, and questions about ownership semantics, Clone vs Copy, and reference lifetimes.
---

# Ownership & Lifetimes Skill

## Error-to-Design-Question Mapping

When encountering an ownership error, don't just fix the immediate issue — trace it to the underlying design question:

| Error | Surface Problem | Real Question |
|---|---|---|
| E0382 | Use of moved value | Who should own this data? Should it be shared (`Arc`/`Rc`) or cloned? |
| E0505 | Move out of borrowed | Is the borrow scope too wide? Can we restructure to release the borrow earlier? |
| E0507 | Move out of borrowed content | Should this field be `Option<T>` so we can `.take()`? Or should we clone? |
| E0515 | Return reference to local | Should the function return owned data? Or take a reference parameter? |
| E0597 | Value doesn't live long enough | Is the lifetime annotation wrong, or is the data structure design wrong? |
| E0716 | Temporary value freed | Should this be stored in a named binding? |
| E0106 | Missing lifetime annotation | What is the actual relationship between input and output lifetimes? |

## Cognitive Tracing

### Trace UP (from error to design)
```
Compiler error → Immediate fix → Why this pattern exists → Better design
```

### Trace DOWN (from design to implementation)
```
Data ownership model → Who creates → Who reads → Who mutates → Who drops
```

## Ownership Pattern Quick Reference

### When to Clone
- Data is small and `Copy`/`Clone` is cheap
- Shared ownership adds more complexity than the clone cost
- Crossing thread boundaries without `Arc`

### When to Use References
- Data is large and cloning is expensive
- The borrower doesn't need to outlive the owner
- Function just needs to read, not own

### When to Use `Arc<T>` / `Rc<T>`
- Multiple owners with unclear lifetimes
- Data shared across threads (`Arc`) or within one thread (`Rc`)
- Consider `Arc<T>` + `Clone` over complex lifetime annotations

### When to Use `Cow<'a, T>`
- Sometimes borrowed, sometimes owned (e.g., conditional string processing)
- API that accepts both `&str` and `String`

## Anti-Patterns

1. **Clone carpet-bombing**: Adding `.clone()` everywhere to silence the borrow checker → Redesign ownership instead
2. **Lifetime annotation sprawl**: `'a` on everything → Simplify by returning owned data or using `Arc`
3. **`Rc<RefCell<T>>` everywhere**: Usually means the data model needs rethinking
4. **Fighting the borrow checker in iterators**: Use `.iter().cloned()` or collect into a Vec before mutating

## Decision Flowchart
```
Need to share data?
├── No → Pass by reference (&T / &mut T)
├── Single thread?
│   ├── Single owner → &T / &mut T
│   └── Multiple owners → Rc<T> (+ RefCell for mutability)
└── Multiple threads?
    ├── Read-only → Arc<T>
    └── Read-write → Arc<Mutex<T>> or Arc<RwLock<T>>
```
