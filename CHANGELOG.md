# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-06-12

Reliability release: every provider call now retries transient failures
safely, and all four integrations were audited against the official API
docs current as of June 2026.

### Added

- Retry with exponential backoff and jitter in `mkt-cli-core`: reads
  retry transient failures (429/5xx/transport) up to 4 attempts; writes
  retry only rate limits and connection failures so a timed-out create
  can never duplicate spend. Server `Retry-After` hints override the
  computed backoff (clamped to 120s)
- All provider clients parse `Retry-After` and surface HTTP 429 as a
  structured rate-limit error with the server's wait hint
- Shared HTTP client: 10s connect timeout, TCP keepalive, contactable
  User-Agent (`mkt/x.y.z (+https://mktcli.com)`)
- TikTok: `campaign/create` sends a UUID `request_id` so network-level
  retries deduplicate server-side instead of creating duplicates
- MCP server: integration tests covering the `tools/call` error contract
  (structured `[error_type]` tags, recovery suggestions, no token leakage)
- Release workflow publishes all crates to crates.io via Trusted
  Publishing (OIDC) in dependency order; `RELEASING.md` documents the flow

### Fixed

- Google Ads: campaign creation now sends the mandatory EU political
  advertising declaration (`containsEuPoliticalAdvertising`, default
  `DOES_NOT_CONTAIN…`, overridable via `--extra`) — required by the API
  since 2026-04-01; without it every create failed
- Google Ads: `insight get` without `--date-range` now defaults to
  `LAST_30_DAYS` instead of emitting GAQL that the API rejects
- Google Ads: a bidding strategy passed via `--extra` replaces the
  `manualCpc` default instead of producing two members of the oneof
- LinkedIn: `insight get` always sends the required `dateRange`
  (defaulting to the last 30 days) instead of a guaranteed 400
- LinkedIn: more than 20 metrics per analytics request is rejected
  locally with a validation error (documented API maximum)
- LinkedIn: deleting a `DRAFT` campaign uses hard DELETE as documented;
  other statuses still transition to `PENDING_DELETION`
- Meta: audience reads request `approximate_count_lower_bound`/`_upper_bound`
  (v25.0 removed `approximate_count`) and `time_created`
- Meta: `media upload-image --url` downloads the asset and uploads it
  Base64-encoded — the `adimages` edge has no `url` parameter
- Meta: creating a creative without a configured `page_id` fails fast
  with a validation error instead of sending the invalid `"me"`
- TikTok: lifetime insights (no date range) no longer combine
  `query_lifetime=true` with the unsupported `stat_time_day` dimension

## [0.1.1] - 2026-06-10

### Added

- Per-crate READMEs so every published crate has a proper crates.io page
  (the library crates went out without one in 0.1.0)

## [0.1.0] - 2026-06-10

First public release: `cargo install mkt-cli` (binary `mkt`). The core
library is published as `mkt-cli-core` (the `mkt-core` name was taken on
crates.io); its library target is still `mkt_core`.

### Added

- TikTok for Business provider (API v1.3): campaign CRUD, integrated
  reporting, DMP audience listing; envelope error contract (40100 ->
  transient rate limit, 401xx -> auth)
- LinkedIn Marketing provider (versioned REST 202605): campaign CRUD via
  Rest.li finders and PARTIAL_UPDATE patches, adAnalytics insights,
  soft delete via PENDING_DELETION
- MCP server (`mkt mcp serve`): stdio Model Context Protocol server with
  six consolidated tools for chat agents; campaigns always created paused
- `mkt completions <shell>` for bash/zsh/fish/powershell/elvish
- `mkt doctor` now reports per-provider credential presence (never values)
- `--extra` JSON flag on campaign create for provider-specific fields
- Google Ads provider (REST v24): campaign CRUD via GAQL + mutate endpoints,
  insights with `costMicros` conversion, OAuth2 refresh-token exchange,
  `mkt google campaign|insight` commands (enabled by default)
- Meta ad sets: `mkt meta adset list|create` with targeting, budget,
  optimization goal, and billing event
- Meta post promotion (`mkt meta post promote --adset`): boost flow via
  `object_story_id` creative + ad, always created PAUSED for spend safety
- Meta audience user upload (`mkt meta audience add-users`) with local
  SHA-256 PII hashing (`mkt_core::pii`, shared across future providers)
- Agent-first contract: stable exit codes (0-7) documented in `--help`,
  structured JSON errors on stderr (`{ok, error: {type, message,
  suggestion, transient}}`) with `--output json`
- `AGENTS.md` operating guide and `llms.txt` index for coding agents
- `--daily-budget` flag on `campaign create`

### Changed

- Default Meta Graph API version bumped from v24.0 to v25.0
- `PromotePostInput` redesigned: promotion targets an existing ad set
  (budget/targeting live on the ad set, matching all platforms' semantics)

### Initial scaffolding

- Core workspace structure with 7 crates
- `MarketingProvider` trait with full CRUD method signatures
- Domain models: Campaign, AdSet, Ad, Creative, Audience, Insight, Post, Media
- Meta provider with Graph API integration (campaigns, posts, insights, raw API)
- CLI with clap: campaign, audience, insight, post, creative, media, raw commands
- Configuration system with TOML profiles and XDG paths
- Output formatting: table, JSON, CSV
- Token-bucket rate limiting
- Authentication with env var and config file fallback
- `mkt doctor` for config verification
- `mkt providers` to list available providers
- Profile management commands
- Dry-run mode for all write operations
- CI pipeline (lint, test, coverage, audit)
- Cross-platform release pipeline
