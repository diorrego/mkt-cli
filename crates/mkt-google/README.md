# mkt-google

Google Ads provider for [`mkt`](https://crates.io/crates/mkt-cli), the
multi-platform ads CLI built for coding agents — <https://mktcli.com>.

Implements the `MarketingProvider` trait from
[`mkt-cli-core`](https://crates.io/crates/mkt-cli-core) against the Google Ads
REST API (v24):

- Campaign CRUD via `campaignBudgets:mutate` + `campaigns:mutate`
  (campaigns default to PAUSED for spend safety)
- Insights via GAQL (`googleAds:search`) with `costMicros` → currency-unit
  conversion
- OAuth2: direct access token or refresh-token exchange

Most users want the CLI itself: `cargo install mkt-cli` (this provider is
enabled by default via the `google` feature).

Part of the [mkt workspace](https://github.com/diorrego/mkt-cli).
Licensed under MIT OR Apache-2.0.
