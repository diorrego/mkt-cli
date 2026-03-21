# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

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
