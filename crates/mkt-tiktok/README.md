# mkt-tiktok

TikTok for Business provider for [`mkt`](https://crates.io/crates/mkt-cli),
the multi-platform ads CLI built for coding agents — <https://mktcli.com>.

Implements the `MarketingProvider` trait from
[`mkt-cli-core`](https://crates.io/crates/mkt-cli-core) against the TikTok
Business API (v1.3), handling its gotchas:

- `Access-Token` header (not `Authorization: Bearer`), JSON-encoded query
  params, and the HTTP-200-with-error-envelope contract (`40100` → transient
  rate limit, `401xx` → auth, `40002` → not found)
- Campaign CRUD (campaigns default to PAUSED for spend safety)
- Integrated reporting (BASIC / AUCTION_CAMPAIGN) and DMP audience listing

Most users want the CLI itself: `cargo install mkt-cli` (this provider is
enabled by default via the `tiktok` feature).

Part of the [mkt workspace](https://github.com/diorrego/mkt-cli).
Licensed under MIT OR Apache-2.0.
