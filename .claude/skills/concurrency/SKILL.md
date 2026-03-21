---
name: concurrency
description: Rust concurrency and async programming. Activates for async/await, tokio, futures, Send/Sync traits, Arc/Mutex usage, channels, thread pools, parallelism with rayon, and concurrent data structure questions.
---

# Concurrency & Async Skill

## Decision Matrix

### CPU-bound vs I/O-bound
| Workload | Solution | Crate |
|---|---|---|
| CPU-bound parallelism | `rayon` or `std::thread` | `rayon` |
| I/O-bound concurrency | `async`/`await` | `tokio` |
| Mixed | Tokio + `spawn_blocking` | `tokio` |
| Simple background task | `std::thread::spawn` | stdlib |

### Runtime Selection
| Need | Runtime | Why |
|---|---|---|
| General async I/O | `tokio` (multi-thread) | Industry standard, full ecosystem |
| Single-threaded async | `tokio` (current-thread) | Lower overhead, no Send bounds |
| Embedded/no_std | `embassy` | Async for embedded systems |
| CPU parallelism only | `rayon` | Work-stealing thread pool |

## Send & Sync Quick Reference

| Type | `Send` | `Sync` | Notes |
|---|---|---|---|
| `Arc<T>` | if T: Send + Sync | if T: Send + Sync | Thread-safe shared ownership |
| `Mutex<T>` | if T: Send | Yes | Provides interior mutability across threads |
| `RwLock<T>` | if T: Send + Sync | if T: Send + Sync | Multiple readers or one writer |
| `Rc<T>` | No | No | Single-thread only |
| `RefCell<T>` | if T: Send | No | Single-thread interior mutability |
| `Cell<T>` | if T: Send | No | Single-thread, Copy types |
| `*mut T` / `*const T` | No | No | Raw pointers need manual impl |

## Async Patterns

### Structured Concurrency with `JoinSet`
```rust
use tokio::task::JoinSet;

let mut set = JoinSet::new();
for item in items {
    set.spawn(async move { process(item).await });
}
while let Some(result) = set.join_next().await {
    handle(result??);
}
```

### Graceful Shutdown
```rust
use tokio::signal;
use tokio_util::sync::CancellationToken;

let token = CancellationToken::new();
let cloned = token.clone();

tokio::select! {
    _ = signal::ctrl_c() => { token.cancel(); }
    _ = run_server(cloned) => {}
}
```

### Channel Selection
| Channel | Use Case |
|---|---|
| `tokio::sync::mpsc` | Multiple producers, single consumer |
| `tokio::sync::oneshot` | Single response (request-reply) |
| `tokio::sync::broadcast` | Multiple consumers, all get every message |
| `tokio::sync::watch` | Single producer, latest-value semantics |
| `crossbeam::channel` | Sync threads (not async) |
| `flume` | Both sync and async ends |

## Anti-Patterns

1. **`MutexGuard` held across `.await`**
   ```rust
   // BAD: guard lives across await point
   let guard = mutex.lock().await;
   do_something().await; // guard still held!

   // GOOD: drop guard before await
   let value = { let guard = mutex.lock().await; guard.clone() };
   do_something().await;
   ```

2. **Spawning without JoinHandle management**: Always track or abort spawned tasks

3. **Blocking in async context**: Use `tokio::task::spawn_blocking` for CPU-heavy or blocking I/O work

4. **Unbounded channels in production**: Use bounded channels with backpressure

5. **`Arc<Mutex<Vec<T>>>` when a concurrent collection would suffice**: Consider `dashmap` or `crossbeam`

## Domain Detection
| Domain | Recommended Stack |
|---|---|
| Web server | `tokio` + `axum`/`actix-web` |
| CLI tool | `tokio` (current-thread) or sync threads |
| Data pipeline | `rayon` for CPU, `tokio` for I/O stages |
| Game/simulation | `rayon` + custom scheduling |
| Embedded | `embassy` or bare threads |
