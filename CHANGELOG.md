# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
