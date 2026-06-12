# mkt-meta

Meta (Facebook/Instagram) provider for [`mkt`](https://crates.io/crates/mkt-cli),
the multi-platform ads CLI built for coding agents — <https://mktcli.com>.

Implements the `MarketingProvider` trait from
[`mkt-cli-core`](https://crates.io/crates/mkt-cli-core) against the Meta Graph
API (default v25.0):

- Campaign and ad set CRUD (`mkt meta campaign …`, `mkt meta adset …`)
- Post promotion / boost flow (`mkt meta post promote`), always created PAUSED
  for spend safety
- Custom audience user upload with local SHA-256 PII hashing
- Insights and raw Graph API access

Most users want the CLI itself: `cargo install mkt-cli` (this provider is
enabled by default via the `meta` feature).

Part of the [mkt workspace](https://github.com/diorrego/mkt-cli).
Licensed under MIT OR Apache-2.0.
