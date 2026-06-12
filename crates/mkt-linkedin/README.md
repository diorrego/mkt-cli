# mkt-linkedin

LinkedIn Marketing provider for [`mkt`](https://crates.io/crates/mkt-cli),
the multi-platform ads CLI built for coding agents — <https://mktcli.com>.

Implements the `MarketingProvider` trait from
[`mkt-cli-core`](https://crates.io/crates/mkt-cli-core) against the versioned
LinkedIn Marketing REST API (202605):

- Rest.li finders with raw syntax and `metadata.nextPageToken` pagination
- Campaign create (ID from the `x-restli-id` header, default PAUSED for
  spend safety), `PARTIAL_UPDATE` patches, soft delete via `PENDING_DELETION`
- adAnalytics insights with `costInLocalCurrency` → numeric `cost` metric

Most users want the CLI itself: `cargo install mkt-cli` (this provider is
enabled by default via the `linkedin` feature).

Part of the [mkt workspace](https://github.com/diorrego/mkt-cli).
Licensed under MIT OR Apache-2.0.
