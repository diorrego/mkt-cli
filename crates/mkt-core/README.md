# mkt-cli-core

Core library for [`mkt`](https://crates.io/crates/mkt-cli), the multi-platform
ads CLI built for coding agents — <https://mktcli.com>.

This crate provides the building blocks shared by every provider and the CLI:

- The `MarketingProvider` trait (campaign/ad set/ad CRUD, insights, audiences)
- Domain models: `Campaign`, `AdSet`, `Ad`, `Creative`, `Audience`, `Insight`,
  `Post`, `Media`
- `MktError` with the stable exit-code contract (0–7) and structured JSON
  error output for agents
- Configuration (TOML profiles, XDG paths, env-var overrides), output
  formatting (table/JSON/CSV), token-bucket rate limiting, and local SHA-256
  PII hashing (`pii` module) for audience uploads

> Note: the package is named `mkt-cli-core` (the `mkt-core` name was already
> taken on crates.io), but the library target is `mkt_core`, so code imports
> it as `use mkt_core::…`.

Most users want the CLI itself: `cargo install mkt-cli`. Use this crate
directly only to implement a new provider or embed mkt's models in another
tool.

Part of the [mkt workspace](https://github.com/diorrego/mkt-cli).
Licensed under MIT OR Apache-2.0.
