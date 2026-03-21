---
name: rust-domain-embedded
description: Rust embedded and no_std development. Activates for bare-metal programming, HAL usage, interrupt handling, no_std constraints, embedded-hal traits, and microcontroller development.
---

# Rust Embedded Domain Skill

## Stack Selection
| Component | Recommended |
|---|---|
| HAL abstraction | `embedded-hal` v1.0 |
| Async runtime | `embassy` |
| Logging | `defmt` |
| Testing | `defmt-test` |
| Probe/flash | `probe-rs` |
| Allocator | `embedded-alloc` or alloc-free |
| USB | `embassy-usb` |
| Networking | `embassy-net`, `smoltcp` |

## Project Setup
```toml
# Cargo.toml
[package]
edition = "2024"

[dependencies]
cortex-m = "0.7"
cortex-m-rt = "0.7"
embassy-executor = { version = "0.7", features = ["arch-cortex-m"] }
embassy-stm32 = { version = "0.2", features = ["stm32f411ce", "time-driver-any"] }
defmt = "0.3"
defmt-rtt = "0.4"
panic-probe = { version = "0.3", features = ["print-defmt"] }

[profile.release]
opt-level = "s"     # Optimize for size
lto = true
debug = true        # Keep debug info for defmt
```

## no_std Constraints
```rust
#![no_std]
#![no_main]

// Must provide:
// - Panic handler
// - Allocator (if using alloc)
// - Entry point

use panic_probe as _;
use defmt_rtt as _;
```

## Embassy Async Pattern
```rust
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    let led = Output::new(p.PA5, Level::Low, Speed::Low);

    spawner.spawn(blink_task(led)).unwrap();
}

#[embassy_executor::task]
async fn blink_task(mut led: Output<'static>) {
    loop {
        led.toggle();
        Timer::after_millis(500).await;
    }
}
```

## Memory-Constrained Patterns
- Use `heapless` collections (`Vec`, `String`, `HashMap` with fixed capacity)
- Prefer stack allocation over heap
- Use `#[repr(C)]` for hardware register structs
- Minimize `String` usage — prefer `&str` and fixed buffers
- Use `defmt` instead of `core::fmt` (much smaller binary size)

## Interrupt Safety
- Use `critical-section` crate for safe interrupt disabling
- Never hold locks across `await` points
- Use `embassy::mutex::Mutex` for shared state between tasks
- Prefer message passing (channels) over shared memory

## Hardware Abstraction
```rust
// Generic over HAL — portable across chips
fn read_sensor<I2C: embedded_hal::i2c::I2c>(i2c: &mut I2C) -> Result<f32, I2C::Error> {
    let mut buf = [0u8; 2];
    i2c.write_read(SENSOR_ADDR, &[TEMP_REG], &mut buf)?;
    Ok(i16::from_be_bytes(buf) as f32 * 0.0625)
}
```
