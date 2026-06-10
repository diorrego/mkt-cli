# mkt

The ads CLI built for coding agents — and the humans who work with them. Free and open source (MIT / Apache-2.0), written in Rust. Manage ads, ad sets, audiences, organic posts, and analytics across **Meta, Google Ads, TikTok, and LinkedIn** from a single terminal.

Claude Code, Codex, Cursor, or any agent with a shell can drive your ad operations with `mkt` the same way they use the AWS or GitHub CLIs: stable JSON output on stdout, structured errors on stderr, a documented exit-code contract, `--dry-run` everywhere, and zero interactive prompts.

## Why mkt?

Marketing teams juggle multiple ad platforms daily, each with its own dashboard, API, and workflow. `mkt` brings all of them under one consistent command-line interface so you — or your coding agent — can script, automate, and manage campaigns without leaving the terminal. And because this tool spends real money, safety is the default: every campaign, ad set, and boosted post is **created paused**; starting delivery is always an explicit step.

## Features

- Unified command structure across all platforms: `mkt <provider> <domain> <action>`
- Campaign management on all four platforms (list, get, create, update, delete)
- Meta ad sets, ad creatives, media upload, and post boosting
- Audience management with local SHA-256 PII hashing before any upload
- Organic post publishing (Facebook and Instagram)
- Unified analytics (cost always in currency units) as table, JSON, or CSV
- Agent contract: stable exit codes (0-7), structured JSON errors with recovery hints
- Spend safety: `--dry-run` on every mutating command; everything is created paused
- MCP server (`mkt mcp serve`) for Claude Desktop / ChatGPT
- Shell completions, profile-based multi-account config, built-in rate limiting

## Supported Platforms

| Platform | Status | Domains |
|----------|--------|---------|
| Meta (Facebook/Instagram) | Available | campaign, adset, audience, insight, post, creative, media, raw |
| Google Ads | Available | campaign, insight |
| TikTok for Business | Available | campaign, audience, insight |
| LinkedIn Marketing | Available | campaign, insight |

## Installation

### From source

```bash
cargo install mkt-cli
```

### From releases

Download the latest binary from [GitHub Releases](https://github.com/diorrego/mkt-cli/releases) for your platform.

## Quick Start

### 1. Set your credentials

```bash
export MKT_META_ACCESS_TOKEN="your-token-here"
export MKT_META_AD_ACCOUNT_ID="act_123456789"
```

Or create a config file at `~/.config/mkt/config.toml`:

```toml
[defaults]
output = "table"
profile = "default"

[profiles.default]
provider = "meta"

[profiles.default.meta]
access_token = "your-token-here"
ad_account_id = "act_123456789"
```

### 2. Verify your setup

```bash
mkt doctor
```

### 3. Start managing campaigns

```bash
# List campaigns (works the same on meta, google, tiktok, linkedin)
mkt meta campaign list
mkt --output json google campaign list --status active

# Create campaigns — always paused until you activate them
mkt --dry-run meta campaign create --name "Test" --objective OUTCOME_TRAFFIC
mkt meta campaign create --name "Q1 Launch" --objective OUTCOME_LEADS
mkt google campaign create --name "Brand" --objective SEARCH --daily-budget 50
mkt tiktok campaign create --name "Spark" --objective TRAFFIC --daily-budget 50

# Meta ad sets and post boosting (boost ad is created PAUSED)
mkt meta adset create --campaign <id> --name "US 25-55" --status paused \
  --daily-budget 2500 --targeting '{"geo_locations":{"countries":["US"]}}'
mkt meta post promote <post_id> --adset <adset_id>

# Audiences — PII is SHA-256 hashed locally before upload
mkt meta audience add-users <audience_id> --email a@example.com

# Unified insights across platforms
mkt meta insight get --range 7d --metrics impressions,clicks,spend
mkt --output csv linkedin insight get

# Publish an Instagram post
mkt meta post create --platform instagram \
  --image-url "https://cdn.example.com/photo.jpg" --message "New product launch"

# Raw API access when you need it
mkt meta raw get "act_123/campaigns" --fields id,name,status

# MCP server for chat agents without a terminal
mkt mcp serve
```

## Command Reference

```
mkt [global flags] <provider> <domain> <action> [flags]

Global flags:
  --profile <name>     Profile to use (default: "default")
  --output <format>    Output format: table, json, csv
  --verbose            Enable debug logging
  --quiet              Suppress non-error output
  --dry-run            Preview actions without executing
  --config <path>      Custom config file path

Providers:
  meta                 Meta (Facebook / Instagram)
  google               Google Ads (REST v24, GAQL)
  tiktok               TikTok for Business (API v1.3)
  linkedin             LinkedIn Marketing (versioned REST)

Domains:
  campaign             Campaign management (list, get, create, update, delete)
  adset                Ad set management (list, create)
  audience             Audience management (list, create, add-users)
  insight              Analytics and reporting (get)
  post                 Organic posts (create, promote into an ad set)
  creative             Ad creatives (create, dark-post)
  media                Media assets (upload-image, upload-video)
  raw                  Direct API access (get, post)

Meta-commands:
  mkt providers        List available providers
  mkt doctor           Verify config, credentials, and connectivity
  mkt profile list     List configured profiles
  mkt profile show     Show profile details
  mkt completions      Generate shell completions (bash, zsh, fish, ...)
  mkt mcp serve        MCP server over stdio (Claude Desktop / ChatGPT)
```

## For coding agents

`mkt` is designed to be driven by AI coding agents (Claude Code, Codex, Cursor)
the same way they use the AWS or GitHub CLIs: machine-readable JSON on stdout,
structured JSON errors on stderr, a stable exit-code contract (0-7), `--dry-run`
on every mutating command, and zero interactive prompts. See [AGENTS.md](AGENTS.md)
for the full operating contract and recipes.

## Architecture

mkt is built as a Cargo workspace with clear separation of concerns:

- **mkt-core**: Traits, domain models, config, output formatting, rate limiting
  (published on crates.io as `mkt-cli-core`; the lib target is still `mkt_core`)
- **mkt-cli**: CLI definitions, command dispatch, the binary itself
- **mkt-meta**: Meta provider (Graph API, Marketing API, Pages API, Instagram API)
- **mkt-google**: Google Ads provider (REST v24: GAQL search + mutate, OAuth2 refresh)
- **mkt-tiktok**: TikTok provider (Business API v1.3: campaigns, reporting, DMP audiences)
- **mkt-linkedin**: LinkedIn provider (versioned REST: Rest.li finders, PARTIAL_UPDATE, adAnalytics)
- **mkt-testkit**: Shared test utilities, mocks, fixtures

Every platform implements the `MarketingProvider` trait from `mkt-core`, which means adding a new provider is just implementing that trait and registering it.

## Development

```bash
# Run all tests
cargo test --workspace --all-features

# Check formatting
cargo fmt --all -- --check

# Run linter
cargo clippy --workspace --all-targets --all-features

# Run the CLI in development
cargo run -p mkt-cli -- meta campaign list

# Generate coverage report
cargo llvm-cov --workspace --html
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed development guidelines.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
