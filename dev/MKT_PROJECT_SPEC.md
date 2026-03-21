# mkt — Multi-platform Marketing CLI

> Production-grade CLI in Rust for managing ads, audiences, organic posts and analytics across Meta, Google Ads, TikTok and LinkedIn from a single terminal.

---

## Table of contents

1. [Vision and goals](#1-vision-and-goals)
2. [Architecture overview](#2-architecture-overview)
3. [Project structure](#3-project-structure)
4. [Rust best practices and conventions](#4-rust-best-practices-and-conventions)
5. [Trait system and provider architecture](#5-trait-system-and-provider-architecture)
6. [Domain models](#6-domain-models)
7. [CLI design](#7-cli-design)
8. [Configuration and auth](#8-configuration-and-auth)
9. [Testing strategy (TDD)](#9-testing-strategy-tdd)
10. [CI/CD pipeline](#10-cicd-pipeline)
11. [Open source standards](#11-open-source-standards)
12. [AI-assisted development](#12-ai-assisted-development)
13. [Implementation roadmap](#13-implementation-roadmap)
14. [Release and distribution](#14-release-and-distribution)
15. [Security](#15-security)
16. [Appendix: CLAUDE.md](#appendix-claudemd)
17. [Appendix: .cursorrules](#appendix-cursorrules)

---

## 1. Vision and goals

### Problem

Marketing teams and developers managing ad campaigns across multiple platforms must switch between different web UIs, SDKs and scripts. There is no unified CLI that treats Meta, Google Ads, TikTok and LinkedIn as interchangeable providers behind a common interface.

### Solution

`mkt` is a single binary that abstracts away platform-specific APIs behind a consistent command structure:

```bash
mkt <provider> <domain> <action> [flags]
```

### Design principles

1. **Provider-agnostic core**: The CLI knows nothing about Graph API or Google Ads API. Providers implement traits.
2. **Offline-first config**: All credentials and profiles live in local TOML files. No cloud dependency.
3. **Test-driven from day one**: Every provider interaction is behind a trait boundary that can be mocked.
4. **AI-tool friendly**: The codebase is structured so that Claude Code, Cursor or Copilot can navigate, understand and extend it with minimal context.
5. **Ship as a single static binary**: Cross-compiled for Linux, macOS (Intel + Apple Silicon) and Windows.

### Non-goals (v1)

- GUI or TUI dashboard.
- Real-time bidding or programmatic optimization.
- OAuth browser flow embedded in the CLI (users provide tokens manually or via env vars).

---

## 2. Architecture overview

```
┌─────────────────────────────────────────────────────────┐
│                    CLI layer (clap)                      │
│           mkt [provider] [domain] [action]              │
└──────────────────────┬──────────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────────┐
│                   Core engine                           │
│  ┌──────────┐ ┌───────────┐ ┌──────────┐ ┌───────────┐ │
│  │ Config   │ │ Auth      │ │ Output   │ │ Rate      │ │
│  │ Manager  │ │ Manager   │ │ Formatter│ │ Limiter   │ │
│  └──────────┘ └───────────┘ └──────────┘ └───────────┘ │
│  ┌────────────────────────────────────────────────────┐ │
│  │        MarketingProvider trait                     │ │
│  └────────────────────────────────────────────────────┘ │
└──────────────────────┬──────────────────────────────────┘
                       │ implements
       ┌───────────────┼───────────────┐
       ▼               ▼               ▼
  ┌─────────┐   ┌───────────┐   ┌──────────┐
  │  Meta    │   │ Google    │   │ TikTok   │  ...
  │ Provider │   │ Provider  │   │ Provider │
  └─────────┘   └───────────┘   └──────────┘
       │               │               │
       ▼               ▼               ▼
  Graph API      Google Ads API   TikTok Marketing API
```

### Crate organization (Cargo workspace)

The project is a Cargo workspace with multiple crates to enforce separation of concerns and enable independent compilation:

```
mkt/
├── Cargo.toml              # workspace root
├── crates/
│   ├── mkt-core/           # traits, domain models, config, output, rate limiting
│   ├── mkt-cli/            # clap definitions, command dispatch, main binary
│   ├── mkt-meta/           # Meta provider (Graph API, Marketing API, Pages API, IG API)
│   ├── mkt-google/         # Google Ads provider (future)
│   ├── mkt-tiktok/         # TikTok provider (future)
│   ├── mkt-linkedin/       # LinkedIn provider (future)
│   └── mkt-testkit/        # shared test utilities, mocks, fixtures
├── tests/                  # integration tests
├── workflows/              # example YAML workflow files
├── templates/              # example creative templates
└── docs/                   # additional documentation
```

### Why a workspace?

- Providers compile in parallel.
- `mkt-core` can be published as a standalone crate for third-party providers.
- Feature flags on the CLI crate control which providers are included in the binary.
- `mkt-testkit` is shared across all provider crates without circular dependencies.

---

## 3. Project structure

```
mkt/
├── .github/
│   ├── workflows/
│   │   ├── ci.yml                # lint + test + coverage on every PR
│   │   ├── release.yml           # cross-compile + publish on tag
│   │   └── audit.yml             # weekly dependency audit
│   ├── ISSUE_TEMPLATE/
│   │   ├── bug_report.md
│   │   ├── feature_request.md
│   │   └── new_provider.md
│   ├── PULL_REQUEST_TEMPLATE.md
│   ├── dependabot.yml
│   └── CODEOWNERS
│
├── crates/
│   ├── mkt-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── provider.rs        # MarketingProvider trait
│   │       ├── models/
│   │       │   ├── mod.rs
│   │       │   ├── campaign.rs
│   │       │   ├── adset.rs
│   │       │   ├── ad.rs
│   │       │   ├── creative.rs
│   │       │   ├── audience.rs
│   │       │   ├── insight.rs
│   │       │   ├── post.rs
│   │       │   └── common.rs      # Budget, DateRange, Pagination, etc.
│   │       ├── config/
│   │       │   ├── mod.rs
│   │       │   ├── profile.rs
│   │       │   └── paths.rs
│   │       ├── auth/
│   │       │   ├── mod.rs
│   │       │   └── token.rs
│   │       ├── output/
│   │       │   ├── mod.rs
│   │       │   ├── table.rs
│   │       │   ├── json.rs
│   │       │   └── csv.rs
│   │       ├── http/
│   │       │   ├── mod.rs
│   │       │   ├── client.rs      # shared reqwest client builder
│   │       │   └── rate_limit.rs
│   │       ├── workflow/
│   │       │   ├── mod.rs
│   │       │   ├── parser.rs
│   │       │   └── runner.rs
│   │       ├── template/
│   │       │   ├── mod.rs
│   │       │   └── engine.rs
│   │       └── error.rs           # thiserror-based error types
│   │
│   ├── mkt-cli/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── cli.rs             # top-level clap Parser
│   │       ├── commands/
│   │       │   ├── mod.rs
│   │       │   ├── campaign.rs
│   │       │   ├── audience.rs
│   │       │   ├── creative.rs
│   │       │   ├── insight.rs
│   │       │   ├── post.rs
│   │       │   ├── workflow.rs
│   │       │   ├── profile.rs
│   │       │   └── raw.rs         # escape-hatch: raw API calls
│   │       └── registry.rs        # provider registry
│   │
│   ├── mkt-meta/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── provider.rs        # impl MarketingProvider for MetaProvider
│   │       ├── client.rs          # MetaClient (Graph API wrapper)
│   │       ├── auth.rs            # Meta-specific token handling
│   │       ├── ads/
│   │       │   ├── mod.rs
│   │       │   ├── campaign.rs
│   │       │   ├── adset.rs
│   │       │   ├── creative.rs
│   │       │   ├── ad.rs
│   │       │   └── audience.rs
│   │       ├── organic/
│   │       │   ├── mod.rs
│   │       │   ├── page.rs        # Facebook Pages API
│   │       │   └── instagram.rs   # IG Content Publishing API
│   │       ├── insights/
│   │       │   └── mod.rs
│   │       ├── mapping.rs         # Meta ↔ unified model conversions
│   │       └── error.rs
│   │
│   ├── mkt-google/                # same internal structure, stub for now
│   ├── mkt-tiktok/                # same internal structure, stub for now
│   ├── mkt-linkedin/              # same internal structure, stub for now
│   │
│   └── mkt-testkit/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── mock_provider.rs   # MockProvider implementing MarketingProvider
│           ├── fixtures/          # JSON fixtures for API responses
│           │   ├── meta/
│           │   │   ├── campaigns.json
│           │   │   ├── adsets.json
│           │   │   ├── insights.json
│           │   │   └── page_post.json
│           │   └── google/
│           ├── http.rs            # mock HTTP server helpers (wiremock)
│           └── assertions.rs      # custom assert macros
│
├── tests/
│   ├── integration/
│   │   ├── meta_campaign_flow.rs
│   │   ├── cross_provider_insights.rs
│   │   └── workflow_execution.rs
│   └── e2e/
│       └── cli_smoke.rs           # assert_cmd-based CLI tests
│
├── workflows/
│   └── examples/
│       ├── deploy-campaign.yml
│       └── weekly-report.yml
│
├── templates/
│   └── examples/
│       ├── lead-ad.toml
│       └── carousel-ad.toml
│
├── docs/
│   ├── ARCHITECTURE.md
│   ├── ADDING_A_PROVIDER.md
│   ├── WORKFLOWS.md
│   └── API_MAPPING.md            # which Meta/Google/etc endpoints map to which commands
│
├── Cargo.toml                     # [workspace]
├── Cargo.lock
├── rust-toolchain.toml
├── clippy.toml
├── rustfmt.toml
├── deny.toml                      # cargo-deny config
├── cliff.toml                     # git-cliff changelog config
├── CLAUDE.md                      # instructions for Claude Code
├── .cursorrules                   # instructions for Cursor
├── .env.example
├── .gitignore
├── LICENSE-MIT
├── LICENSE-APACHE
├── README.md
├── CONTRIBUTING.md
├── CHANGELOG.md
├── SECURITY.md
└── CODE_OF_CONDUCT.md
```

---

## 4. Rust best practices and conventions

### Toolchain

```toml
# rust-toolchain.toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy", "llvm-tools-preview"]
```

### Formatting

```toml
# rustfmt.toml
edition = "2024"
max_width = 100
use_field_init_shorthand = true
use_try_shorthand = true
imports_granularity = "Crate"
group_imports = "StdExternalCrate"
```

### Linting

```toml
# clippy.toml
too-many-arguments-threshold = 6
```

In the workspace `Cargo.toml`:

```toml
[workspace.lints.rust]
unsafe_code = "forbid"
unused_must_use = "deny"
missing_docs = "warn"

[workspace.lints.clippy]
all = { level = "deny", priority = -1 }
pedantic = { level = "warn", priority = -1 }
nursery = { level = "warn", priority = -1 }

# Specific overrides
module_name_repetitions = "allow"
must_use_candidate = "allow"

# Security-critical
unwrap_used = "deny"
expect_used = "warn"
panic = "deny"
```

### Error handling

Use `thiserror` for library errors and `anyhow` only at the CLI boundary:

```rust
// crates/mkt-core/src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MktError {
    #[error("Provider '{provider}' not found. Available: {available}")]
    ProviderNotFound {
        provider: String,
        available: String,
    },

    #[error("API error from {provider}: {status} — {message}")]
    ApiError {
        provider: String,
        status: u16,
        message: String,
        retry_after: Option<u64>,
    },

    #[error("Authentication failed for {provider}: {reason}")]
    AuthError {
        provider: String,
        reason: String,
    },

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Rate limit exceeded for {provider}. Retry after {retry_after_secs}s")]
    RateLimited {
        provider: String,
        retry_after_secs: u64,
    },

    #[error("Validation error: {field} — {message}")]
    ValidationError {
        field: String,
        message: String,
    },

    #[error("{provider} does not support '{feature}'")]
    NotSupported {
        provider: String,
        feature: String,
    },

    #[error(transparent)]
    Http(#[from] reqwest::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
}

impl MktError {
    pub fn not_supported(provider: &str, feature: &str) -> Self {
        Self::NotSupported {
            provider: provider.to_string(),
            feature: feature.to_string(),
        }
    }
}

pub type Result<T> = std::result::Result<T, MktError>;
```

### Coding conventions

1. **No `unwrap()` or `panic!()` in library code.** Use `?` and proper error variants.
2. **All public items have doc comments.** `#![warn(missing_docs)]` enforces this.
3. **Use `#[must_use]` on functions that return values that must not be ignored.**
4. **Prefer `impl Into<String>` over `String` in function parameters for ergonomics.**
5. **Use `builder` pattern for complex structs** (via `bon` or `typed-builder` crate).
6. **Use `newtype` pattern for IDs**: `pub struct CampaignId(String)` instead of raw strings.
7. **Keep functions under 40 lines.** If longer, extract helpers.
8. **Every module has a module-level doc comment** explaining its responsibility.
9. **Use `tracing` for structured logging**, not `println!` or `log`.
10. **All async functions use `tokio`**. No mixing runtimes.

### Dependency policy

```toml
# deny.toml (cargo-deny)
[advisories]
vulnerability = "deny"
unmaintained = "warn"

[licenses]
allow = ["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Unicode-3.0"]
confidence-threshold = 0.8

[bans]
multiple-versions = "warn"
deny = [
    { name = "openssl-sys" },
]

[sources]
unknown-registry = "deny"
unknown-git = "deny"
```

### Workspace dependencies

```toml
# Root Cargo.toml
[workspace]
resolver = "2"
members = [
    "crates/mkt-core",
    "crates/mkt-cli",
    "crates/mkt-meta",
    "crates/mkt-google",
    "crates/mkt-tiktok",
    "crates/mkt-linkedin",
    "crates/mkt-testkit",
]

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.85"
license = "MIT OR Apache-2.0"
repository = "https://github.com/your-org/mkt"
homepage = "https://mkt.dev"
keywords = ["marketing", "ads", "cli", "meta", "google-ads"]
categories = ["command-line-utilities"]

[workspace.dependencies]
# Async runtime
tokio = { version = "1", features = ["macros", "rt-multi-thread", "signal"] }

# HTTP
reqwest = { version = "0.12", default-features = false, features = [
    "json", "multipart", "rustls-tls", "gzip", "brotli"
] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"

# CLI
clap = { version = "4", features = ["derive", "env", "wrap_help"] }

# Error handling
thiserror = "2"
anyhow = "1"

# Logging / tracing
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["fmt", "env-filter", "json"] }

# Date / time
chrono = { version = "0.4", features = ["serde"] }

# Utilities
url = { version = "2", features = ["serde"] }
secrecy = { version = "0.10", features = ["serde"] }
dirs = "6"
uuid = { version = "1", features = ["v4", "serde"] }
async-trait = "0.1"

# Testing
wiremock = "0.6"
assert_cmd = "2"
predicates = "3"
insta = { version = "1", features = ["yaml", "json", "redactions"] }
mockall = "0.13"
tokio-test = "0.4"
test-case = "3"
fake = { version = "3", features = ["derive", "chrono"] }
proptest = "1"
```

---

## 5. Trait system and provider architecture

### Core trait

```rust
// crates/mkt-core/src/provider.rs

use async_trait::async_trait;
use crate::models::*;
use crate::error::{MktError, Result};

/// Describes the capabilities a provider supports.
/// Used by the CLI to show/hide commands dynamically.
#[derive(Debug, Clone)]
pub struct ProviderCapabilities {
    pub campaigns: bool,
    pub adsets: bool,
    pub ads: bool,
    pub creatives: bool,
    pub audiences: bool,
    pub insights: bool,
    pub organic_posts: bool,
    pub dark_posts: bool,
    pub video_upload: bool,
    pub image_upload: bool,
    pub workflow_templates: bool,
}

/// The core abstraction. Every platform implements this trait.
/// Methods return unified domain models, not platform-specific structs.
#[async_trait]
pub trait MarketingProvider: Send + Sync {
    /// Short lowercase name used in CLI commands: "meta", "google", "tiktok"
    fn name(&self) -> &'static str;

    /// Human-readable display name: "Meta (Facebook/Instagram)"
    fn display_name(&self) -> &'static str;

    /// What this provider can do
    fn capabilities(&self) -> ProviderCapabilities;

    // ── Campaigns ──────────────────────────────────────────

    async fn list_campaigns(&self, filters: &CampaignFilters) -> Result<Paginated<Campaign>>;
    async fn get_campaign(&self, id: &CampaignId) -> Result<Campaign>;
    async fn create_campaign(&self, input: &CreateCampaignInput) -> Result<Campaign>;
    async fn update_campaign(&self, id: &CampaignId, input: &UpdateCampaignInput) -> Result<Campaign>;
    async fn delete_campaign(&self, id: &CampaignId) -> Result<()>;

    // ── Ad sets / Ad groups ────────────────────────────────

    async fn list_adsets(&self, campaign_id: &CampaignId) -> Result<Paginated<AdSet>> {
        Err(MktError::not_supported(self.name(), "adsets"))
    }
    async fn create_adset(&self, input: &CreateAdSetInput) -> Result<AdSet> {
        Err(MktError::not_supported(self.name(), "adsets"))
    }

    // ── Creatives ──────────────────────────────────────────

    async fn create_creative(&self, input: &CreateCreativeInput) -> Result<Creative> {
        Err(MktError::not_supported(self.name(), "creatives"))
    }
    async fn create_dark_post(&self, input: &CreateDarkPostInput) -> Result<Creative> {
        Err(MktError::not_supported(self.name(), "dark_posts"))
    }

    // ── Audiences ──────────────────────────────────────────

    async fn list_audiences(&self) -> Result<Vec<Audience>>;
    async fn create_audience(&self, input: &CreateAudienceInput) -> Result<Audience>;
    async fn add_users_to_audience(
        &self,
        id: &AudienceId,
        users: &[AudienceUser],
    ) -> Result<AudienceUpdateResult> {
        Err(MktError::not_supported(self.name(), "audience_users"))
    }

    // ── Insights ───────────────────────────────────────────

    async fn get_insights(&self, query: &InsightsQuery) -> Result<InsightsReport>;

    // ── Organic posts ──────────────────────────────────────

    async fn publish_post(&self, input: &PublishPostInput) -> Result<Post> {
        Err(MktError::not_supported(self.name(), "organic_posts"))
    }
    async fn promote_post(&self, post_id: &PostId, input: &PromotePostInput) -> Result<Ad> {
        Err(MktError::not_supported(self.name(), "promote_post"))
    }

    // ── Media upload ───────────────────────────────────────

    async fn upload_image(&self, input: &UploadImageInput) -> Result<MediaAsset> {
        Err(MktError::not_supported(self.name(), "image_upload"))
    }
    async fn upload_video(&self, input: &UploadVideoInput) -> Result<MediaAsset> {
        Err(MktError::not_supported(self.name(), "video_upload"))
    }

    // ── Raw escape hatch ───────────────────────────────────

    /// Execute a raw API call. This is the "raw" escape hatch.
    async fn raw_request(
        &self,
        method: HttpMethod,
        path: &str,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value>;

    // ── Health check ───────────────────────────────────────

    /// Verify that credentials are valid and the API is reachable.
    async fn health_check(&self) -> Result<ProviderHealth>;
}
```

### Provider registry

```rust
// crates/mkt-cli/src/registry.rs

use std::collections::HashMap;
use mkt_core::provider::MarketingProvider;

pub struct ProviderRegistry {
    providers: HashMap<&'static str, Box<dyn MarketingProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self { providers: HashMap::new() }
    }

    pub fn register(&mut self, provider: Box<dyn MarketingProvider>) {
        self.providers.insert(provider.name(), provider);
    }

    pub fn get(&self, name: &str) -> Option<&dyn MarketingProvider> {
        self.providers.get(name).map(AsRef::as_ref)
    }

    pub fn list(&self) -> Vec<&'static str> {
        let mut names: Vec<_> = self.providers.keys().copied().collect();
        names.sort_unstable();
        names
    }
}
```

### Adding a new provider

A new provider only needs to:

1. Create a new crate `crates/mkt-<name>/`.
2. Implement `MarketingProvider` for its struct.
3. Add a feature flag in `crates/mkt-cli/Cargo.toml`.
4. Register it in `registry.rs`.
5. Write tests using `mkt-testkit` helpers.

The full guide lives in `docs/ADDING_A_PROVIDER.md`.

---

## 6. Domain models

All models use newtype IDs, `chrono::DateTime<Utc>` for timestamps, and carry a `raw` field with the original API response for debugging:

```rust
// crates/mkt-core/src/models/campaign.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Opaque campaign identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CampaignId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Campaign {
    pub id: CampaignId,
    pub provider: String,
    pub name: String,
    pub status: CampaignStatus,
    pub objective: String,
    pub budget: Option<Budget>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    /// Original API response for debugging and raw access.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CampaignStatus {
    Active,
    Paused,
    Archived,
    Draft,
    Deleted,
    /// Platform-specific status not mapped to a known variant.
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub amount: f64,
    pub currency: String,
    pub kind: BudgetKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetKind {
    Daily,
    Lifetime,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateCampaignInput {
    pub name: String,
    pub objective: String,
    pub status: Option<CampaignStatus>,
    pub budget: Option<Budget>,
    pub extra: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateCampaignInput {
    pub name: Option<String>,
    pub status: Option<CampaignStatus>,
    pub budget: Option<Budget>,
    pub extra: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CampaignFilters {
    pub status: Option<CampaignStatus>,
    pub name_contains: Option<String>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paginated<T> {
    pub data: Vec<T>,
    pub next_cursor: Option<String>,
    pub total: Option<u64>,
}
```

Apply the same pattern for `AdSet`, `Ad`, `Creative`, `Audience`, `Post`, `InsightsReport`, and `MediaAsset`. Each gets its own file in `crates/mkt-core/src/models/`.

---

## 7. CLI design

### Command structure

```
mkt [global flags] <provider> <domain> <action> [flags]

Global flags:
  --profile <name>     Profile to use (default: "default")
  --output <format>    Output format: table, json, csv (default: table)
  --verbose            Enable debug logging
  --quiet              Suppress all output except errors
  --dry-run            Show what would happen without executing
  --config <path>      Path to config file

Providers:
  meta                 Meta (Facebook / Instagram)
  google               Google Ads
  tiktok               TikTok for Business
  linkedin             LinkedIn Marketing

Domains and actions:
  campaign list        List campaigns
  campaign get <id>    Get campaign details
  campaign create      Create a campaign
  campaign update <id> Update a campaign
  campaign delete <id> Delete a campaign

  adset list           List ad sets for a campaign
  adset create         Create an ad set

  creative create      Create an ad creative
  creative dark-post   Create an unpublished post for ads

  audience list        List audiences
  audience create      Create a custom audience
  audience add-users   Add users to an audience

  insight get          Get insights/analytics
  insight compare      Compare insights across providers

  post create          Create an organic post
  post promote <id>    Promote an existing post as an ad

  media upload-image   Upload an image asset
  media upload-video   Upload a video asset

  raw get <path>       Raw API GET request
  raw post <path>      Raw API POST request

Meta-commands (no provider needed):
  mkt profile set <name>     Configure a profile
  mkt profile list           List profiles
  mkt profile show <name>    Show profile details

  mkt workflow run <file>    Execute a YAML workflow
  mkt workflow validate      Validate a workflow file

  mkt providers              List available providers and their capabilities
  mkt doctor                 Verify config, tokens, and API connectivity
```

### Usage examples

```bash
# Configure Meta profile
mkt profile set default \
  --provider meta \
  --access-token "$META_TOKEN" \
  --ad-account "act_123456789" \
  --page-id "987654321" \
  --ig-user-id "111222333"

# List active Meta campaigns as JSON
mkt --output json meta campaign list --status active

# Create a Google Ads campaign from a JSON file
mkt google campaign create --file campaign.json

# Get cross-platform spend report for last 7 days
mkt insight compare --providers meta,google --metrics spend,cpa --range 7d

# Publish an Instagram image
mkt meta post create \
  --platform instagram \
  --image-url "https://cdn.example.com/photo.jpg" \
  --caption "New product launch"

# Run a deployment workflow
mkt workflow run workflows/deploy-q1.yml --dry-run

# Verify everything is working
mkt doctor

# Raw escape hatch
mkt meta raw get "act_123/campaigns" --fields "id,name,status"
```

---

## 8. Configuration and auth

### Config file location

Following XDG Base Directory spec:

```
$XDG_CONFIG_HOME/mkt/config.toml   (Linux/macOS: ~/.config/mkt/config.toml)
%APPDATA%\mkt\config.toml          (Windows)
```

### Config structure

```toml
# config.toml
[defaults]
output = "table"
profile = "default"

[profiles.default]
provider = "meta"

[profiles.default.meta]
access_token = "EAAB..."      # or use env: MKT_META_ACCESS_TOKEN
ad_account_id = "act_123"
page_id = "456"
ig_user_id = "789"
api_version = "v25.0"

[profiles.default.google]
developer_token = "..."
client_id = "..."
client_secret = "..."
refresh_token = "..."
customer_id = "123-456-7890"

[profiles.client-acme]
provider = "meta"

[profiles.client-acme.meta]
access_token = "EAAC..."
ad_account_id = "act_789"
page_id = "012"
```

### Token security

- Tokens stored in config are encrypted at rest via OS keyring when available (`keyring` crate).
- Fallback: plaintext in config with `0600` file permissions.
- Env vars always take precedence: `MKT_META_ACCESS_TOKEN`, `MKT_GOOGLE_REFRESH_TOKEN`, etc.
- The `secrecy` crate wraps tokens so they never appear in debug output or logs.
- `mkt doctor` checks token validity without printing them.

---

## 9. Testing strategy (TDD)

### Testing pyramid

```
        ╱╲
       ╱  ╲        E2E tests (assert_cmd)
      ╱    ╲       CLI binary tests: invoke mkt and check stdout/stderr/exit code
     ╱──────╲
    ╱        ╲     Integration tests (wiremock)
   ╱          ╲    Provider ↔ mock HTTP server: verify real API payloads
  ╱────────────╲
 ╱              ╲  Unit tests (mockall + standard)
╱                ╲ Pure logic: mapping, config parsing, output formatting, validation
```

### Layer 1: Unit tests

Every module has a `#[cfg(test)] mod tests` block. Use `#[test_case]` for parameterized tests and `insta` for snapshot testing of output formatting.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test]
    fn campaign_status_serializes_to_snake_case() {
        let json = serde_json::to_string(&CampaignStatus::Active).unwrap();
        assert_eq!(json, r#""active""#);
    }

    #[test_case(CampaignStatus::Active, "ACTIVE" ; "active maps to ACTIVE")]
    #[test_case(CampaignStatus::Paused, "PAUSED" ; "paused maps to PAUSED")]
    fn status_maps_to_meta_format(status: CampaignStatus, expected: &str) {
        assert_eq!(status.to_meta_api_string(), expected);
    }

    #[test]
    fn unknown_status_deserializes_as_other() {
        let status: CampaignStatus =
            serde_json::from_str(r#""something_new""#).unwrap();
        assert_eq!(status, CampaignStatus::Other("something_new".into()));
    }
}
```

### Layer 2: Integration tests with wiremock

Test providers against mock HTTP servers to validate request construction and response parsing:

```rust
// crates/mkt-meta/tests/campaign_integration.rs

use mkt_core::provider::MarketingProvider;
use mkt_meta::MetaProvider;
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path, query_param};

#[tokio::test]
async fn list_campaigns_sends_correct_request() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v25.0/act_123/campaigns"))
        .and(query_param("access_token", "test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            mkt_testkit::fixtures::meta::campaigns_response()
        ))
        .expect(1)
        .mount(&server)
        .await;

    let provider = MetaProvider::new_with_base_url(
        "test-token".into(),
        "act_123".into(),
        server.uri(),
    );

    let result = provider.list_campaigns(&Default::default()).await.unwrap();

    assert_eq!(result.data.len(), 2);
    assert_eq!(result.data[0].provider, "meta");
    assert_eq!(result.data[0].status, CampaignStatus::Active);
}

#[tokio::test]
async fn create_campaign_handles_api_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v25.0/act_123/campaigns"))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "error": {
                "message": "Invalid objective",
                "type": "OAuthException",
                "code": 100,
            }
        })))
        .mount(&server)
        .await;

    let provider = MetaProvider::new_with_base_url(
        "test-token".into(),
        "act_123".into(),
        server.uri(),
    );

    let result = provider.create_campaign(&CreateCampaignInput {
        name: "Test".into(),
        objective: "INVALID".into(),
        ..Default::default()
    }).await;

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        MktError::ApiError { status: 400, .. }
    ));
}
```

### Layer 3: E2E tests with assert_cmd

Test the actual compiled binary:

```rust
// tests/e2e/cli_smoke.rs

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn shows_help_with_no_args() {
    Command::cargo_bin("mkt")
        .unwrap()
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage: mkt"));
}

#[test]
fn providers_command_lists_available_providers() {
    Command::cargo_bin("mkt")
        .unwrap()
        .args(["providers"])
        .assert()
        .success()
        .stdout(predicate::str::contains("meta"));
}

#[test]
fn doctor_reports_missing_config() {
    Command::cargo_bin("mkt")
        .unwrap()
        .args(["doctor"])
        .env("MKT_CONFIG_DIR", "/tmp/mkt-test-nonexistent")
        .assert()
        .failure()
        .stderr(predicate::str::contains("No configuration found"));
}

#[test]
fn dry_run_does_not_execute() {
    Command::cargo_bin("mkt")
        .unwrap()
        .args([
            "--dry-run",
            "meta", "campaign", "create",
            "--name", "Test Campaign",
            "--objective", "OUTCOME_LEADS",
        ])
        .env("MKT_META_ACCESS_TOKEN", "fake-token")
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run]"));
}
```

### Snapshot testing with insta

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    #[test]
    fn campaign_table_output_matches_snapshot() {
        let campaigns = vec![test_campaign()];
        let output = format_campaigns_table(&campaigns);
        assert_snapshot!(output);
    }
}
```

### Property-based testing with proptest

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn campaign_status_roundtrips(status in any::<CampaignStatus>()) {
        let json = serde_json::to_string(&status).unwrap();
        let back: CampaignStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(status, back);
    }
}
```

### Test fixtures

All API response fixtures live in `crates/mkt-testkit/src/fixtures/` as JSON files:

```rust
// crates/mkt-testkit/src/fixtures/meta.rs

pub fn campaigns_response() -> serde_json::Value {
    serde_json::from_str(include_str!("fixtures/meta/campaigns.json"))
        .expect("Invalid campaigns fixture")
}
```

### Coverage target

80%+ line coverage for `mkt-core` and provider crates. Measured with `cargo-llvm-cov`:

```bash
cargo llvm-cov --workspace --all-features --lcov --output-path coverage.lcov
```

---

## 10. CI/CD pipeline

### ci.yml — runs on every PR and push to main

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: '-Dwarnings'

jobs:
  check:
    name: Check & Lint
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - name: Check formatting
        run: cargo fmt --all -- --check
      - name: Clippy
        run: cargo clippy --workspace --all-targets --all-features
      - name: Check docs compile
        run: cargo doc --workspace --no-deps --document-private-items
        env:
          RUSTDOCFLAGS: '-Dwarnings'

  test:
    name: Test (${{ matrix.os }})
    needs: check
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Run tests
        run: cargo test --workspace --all-features
      - name: Run doc tests
        run: cargo test --workspace --doc

  coverage:
    name: Coverage
    needs: check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: llvm-tools-preview
      - uses: Swatinem/rust-cache@v2
      - uses: taiki-e/install-action@cargo-llvm-cov
      - name: Generate coverage
        run: cargo llvm-cov --workspace --all-features --lcov --output-path coverage.lcov
      - uses: codecov/codecov-action@v4
        with:
          files: coverage.lcov
          fail_ci_if_error: true

  deny:
    name: Dependency audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: EmbarkStudios/cargo-deny-action@v2

  msrv:
    name: Minimum supported Rust version
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: '1.85'
      - uses: Swatinem/rust-cache@v2
      - run: cargo check --workspace
```

### release.yml — triggered by version tags

```yaml
name: Release

on:
  push:
    tags: ['v*']

permissions:
  contents: write

jobs:
  build:
    name: Build ${{ matrix.target }}
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-musl
            os: ubuntu-latest
          - target: aarch64-unknown-linux-musl
            os: ubuntu-latest
          - target: x86_64-apple-darwin
            os: macos-latest
          - target: aarch64-apple-darwin
            os: macos-latest
          - target: x86_64-pc-windows-msvc
            os: windows-latest
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - uses: Swatinem/rust-cache@v2
      - name: Install cross (Linux ARM)
        if: matrix.target == 'aarch64-unknown-linux-musl'
        run: cargo install cross
      - name: Build
        shell: bash
        run: |
          if [[ "${{ matrix.target }}" == "aarch64-unknown-linux-musl" ]]; then
            cross build --release --target ${{ matrix.target }} -p mkt-cli
          else
            cargo build --release --target ${{ matrix.target }} -p mkt-cli
          fi
      - name: Package
        shell: bash
        run: |
          mkdir -p dist
          if [[ "${{ matrix.os }}" == "windows-latest" ]]; then
            cp target/${{ matrix.target }}/release/mkt.exe dist/
            cd dist && 7z a mkt-${{ matrix.target }}.zip mkt.exe
          else
            cp target/${{ matrix.target }}/release/mkt dist/
            cd dist && tar czf mkt-${{ matrix.target }}.tar.gz mkt
          fi
      - uses: actions/upload-artifact@v4
        with:
          name: mkt-${{ matrix.target }}
          path: dist/mkt-*

  publish:
    name: Create GitHub Release
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - uses: actions/download-artifact@v4
        with:
          path: artifacts
      - name: Generate changelog
        run: |
          cargo install git-cliff
          git cliff --latest --strip header > RELEASE_NOTES.md
      - uses: softprops/action-gh-release@v2
        with:
          body_path: RELEASE_NOTES.md
          files: artifacts/**/*

  crates-io:
    name: Publish to crates.io
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo publish -p mkt-core
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
      - run: sleep 30
      - run: cargo publish -p mkt-meta
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
      - run: sleep 30
      - run: cargo publish -p mkt-cli
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
```

### audit.yml — weekly security scan

```yaml
name: Security Audit

on:
  schedule:
    - cron: '0 8 * * 1'
  workflow_dispatch:

jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: rustsec/audit-check@v2
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
```

---

## 11. Open source standards

### Required repository files

| File                               | Purpose                                                     |
| ---------------------------------- | ----------------------------------------------------------- |
| `README.md`                        | Project overview, install instructions, quick start, badges |
| `LICENSE-MIT`                      | MIT license text                                            |
| `LICENSE-APACHE`                   | Apache 2.0 license text                                     |
| `CONTRIBUTING.md`                  | How to contribute, dev setup, commit conventions            |
| `CODE_OF_CONDUCT.md`               | Contributor Covenant v2.1                                   |
| `SECURITY.md`                      | Vulnerability reporting process                             |
| `CHANGELOG.md`                     | Auto-generated by git-cliff with conventional commits       |
| `.github/CODEOWNERS`               | Auto-assign reviewers by path                               |
| `.github/dependabot.yml`           | Automated dependency update PRs                             |
| `.github/ISSUE_TEMPLATE/`          | Bug, feature request, new provider templates                |
| `.github/PULL_REQUEST_TEMPLATE.md` | PR checklist                                                |

### Commit convention

Conventional Commits for automatic changelog generation:

```
feat(meta): add campaign creation endpoint
fix(core): handle empty pagination cursor
docs: add ADDING_A_PROVIDER guide
test(meta): add wiremock tests for insights
refactor(core): extract rate limiter into its own module
ci: add aarch64-linux cross-compilation
chore: update dependencies
```

### Versioning

SemVer. Use `cargo-release` for version bumps:

```bash
cargo release patch  # 0.1.0 → 0.1.1
cargo release minor  # 0.1.1 → 0.2.0
cargo release major  # 0.2.0 → 1.0.0
```

### Pull request checklist

```markdown
## What does this PR do?

## Checklist

- [ ] Tests pass locally (`cargo test --workspace`)
- [ ] New code has unit tests
- [ ] Integration tests added for new API interactions
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo fmt` applied
- [ ] Documentation updated (doc comments + .md files)
- [ ] Snapshot tests updated (`cargo insta review`)
- [ ] No new `unwrap()` or `panic!()` in library code
- [ ] CHANGELOG entry or conventional commit message

## Breaking changes

<!-- List any breaking changes and migration steps -->
```

---

## 12. AI-assisted development

### Design principles for AI readability

1. **One responsibility per file.** AI tools work best with focused files under 300 lines.
2. **Explicit types everywhere.** Avoid `impl Trait` in return positions for complex functions.
3. **Module-level doc comments.** Every `mod.rs` starts with `//!` explaining the module.
4. **Naming over comments.** Prefer `fn parse_meta_campaign_response` over `fn parse_response`.
5. **Test files mirror source files.** Makes navigation predictable.
6. **Fixtures over inline JSON.** AI tools can read fixture files and understand data shapes.
7. **No magic strings.** Constants and enums over string literals.

The two key files (`CLAUDE.md` and `.cursorrules`) are in the appendices below.

---

## 13. Implementation roadmap

### Phase 0 — Scaffolding (week 1)

- [ ] Initialize Cargo workspace with all crate skeletons
- [ ] Set up `rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`, `deny.toml`
- [ ] Create `CLAUDE.md`, `.cursorrules`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`
- [ ] Set up CI pipeline (`ci.yml` with check, test, coverage, deny)
- [ ] Implement `MktError` in `mkt-core`
- [ ] Implement config loading and profile management
- [ ] Implement `OutputFormatter` (table, json, csv)
- [ ] Write unit tests for config and output

### Phase 1 — Core + Meta provider (weeks 2–4)

- [ ] Define `MarketingProvider` trait with all method signatures
- [ ] Define all domain models (Campaign, AdSet, Creative, Audience, Post, Insights)
- [ ] Implement `MetaClient` (Graph API HTTP wrapper with rate limiting)
- [ ] Implement `MetaProvider` — campaign CRUD
- [ ] Implement campaign mapping: Meta API response → domain `Campaign`
- [ ] Write wiremock tests for all campaign endpoints
- [ ] Implement CLI commands: `meta campaign list/create/get/update/delete`
- [ ] Write E2E tests with assert_cmd
- [ ] Implement `meta audience list/create/add-users`
- [ ] Implement `meta post create` (Facebook Pages API)
- [ ] Implement `meta post create --platform instagram` (IG Content Publishing)
- [ ] Implement `meta insight get`
- [ ] Implement `meta creative create` and `meta creative dark-post`
- [ ] Implement `meta raw get/post` escape hatch
- [ ] Implement `mkt doctor` and `mkt providers` commands
- [ ] Snapshot tests for all output formats

### Phase 2 — Google Ads provider (weeks 5–7)

- [ ] Research Google Ads API: auth flow, endpoints, response shapes
- [ ] Implement `GoogleProvider` — campaign CRUD
- [ ] Implement Google ↔ domain model mapping
- [ ] Add Google-specific config fields
- [ ] Wiremock tests for Google endpoints
- [ ] E2E tests for `google campaign list/create`
- [ ] Implement `google insight get`
- [ ] Stabilize `MarketingProvider` trait based on second provider lessons

### Phase 3 — Cross-platform features (weeks 8–10)

- [ ] Implement `mkt insight compare --providers meta,google`
- [ ] Implement YAML workflow parser and runner
- [ ] Implement template engine for creative payloads
- [ ] Implement `--dry-run` for all write operations
- [ ] Implement retry with exponential backoff in HTTP client
- [ ] Add `--file` flag for all create commands (JSON input)
- [ ] Write integration tests for workflows
- [ ] Documentation: `WORKFLOWS.md`, `API_MAPPING.md`

### Phase 4 — More providers + polish (weeks 11–14)

- [ ] Implement `TikTokProvider` (TikTok Marketing API)
- [ ] Implement `LinkedInProvider` (LinkedIn Marketing API)
- [ ] Shell completions (bash, zsh, fish, PowerShell) via clap
- [ ] Man page generation
- [ ] `mkt upgrade` self-update command
- [ ] Performance: connection pooling, parallel provider queries
- [ ] Set up release pipeline (`release.yml`)
- [ ] Publish `mkt-core` to crates.io
- [ ] Binary releases on GitHub Releases
- [ ] Homebrew formula, Scoop manifest, AUR package
- [ ] README with badges, demo GIFs, docs links

---

## 14. Release and distribution

### Binary distribution channels

| Platform       | Method                                                      |
| -------------- | ----------------------------------------------------------- |
| macOS          | Homebrew tap: `brew install your-org/tap/mkt`               |
| Linux          | GitHub Releases tarball, AUR, Nix flake                     |
| Windows        | Scoop: `scoop install mkt`, GitHub Releases `.zip`          |
| Cross-platform | `cargo install mkt-cli` from crates.io                      |
| Docker         | `docker run ghcr.io/your-org/mkt:latest meta campaign list` |

### Dockerfile

```dockerfile
FROM rust:1.85-alpine AS builder
WORKDIR /build
COPY . .
RUN apk add --no-cache musl-dev && \
    cargo build --release -p mkt-cli --target x86_64-unknown-linux-musl

FROM alpine:3.21
RUN apk add --no-cache ca-certificates
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/mkt /usr/local/bin/
ENTRYPOINT ["mkt"]
```

---

## 15. Security

### Token handling

- Tokens wrapped in `secrecy::SecretString` — `Debug` prints `[REDACTED]`.
- Environment variables take precedence over config file values.
- Config files created with `0600` permissions on Unix.
- `mkt doctor` validates tokens without printing them.
- No tokens ever appear in logs, even at `TRACE` level.

### Dependency policy

- `cargo-deny` runs on every PR to block known vulnerabilities.
- Weekly `cargo audit` via GitHub Actions.
- `dependabot` opens PRs for dependency updates.
- No OpenSSL — `rustls` only.

### API security

- All HTTP requests use TLS (reqwest with `rustls-tls`).
- No HTTP fallback.
- Rate limiting with exponential backoff to avoid token suspension.
- Configurable request timeout (default: 30s).

### Reporting vulnerabilities

Private disclosure via GitHub Security Advisories. Details in `SECURITY.md`.

---

## Appendix: CLAUDE.md

This file goes at the repo root. Claude Code reads it automatically when working in the project.

```markdown
# CLAUDE.md — Instructions for Claude Code

## Project overview

mkt is a multi-platform marketing CLI written in Rust. It manages ads,
audiences, organic posts and analytics across Meta, Google Ads, TikTok and
LinkedIn through a unified command interface.

## Architecture

- Cargo workspace with crates in `crates/`.
- `mkt-core`: traits, domain models, config, output formatting, rate limiting.
- `mkt-cli`: clap-based CLI binary, command dispatch, provider registry.
- `mkt-meta`: Meta provider (Graph API / Marketing API / Pages API / IG API).
- `mkt-google`, `mkt-tiktok`, `mkt-linkedin`: future provider crates.
- `mkt-testkit`: shared mocks, fixtures, assertion helpers.

## Key patterns

- Every provider implements `MarketingProvider` trait from `mkt-core::provider`.
- Domain models are in `mkt-core::models`. They are provider-agnostic.
- Each provider has a `mapping.rs` that converts between API responses and
  domain models.
- Errors use `thiserror` in library crates. `anyhow` only in `mkt-cli/src/main.rs`.
- HTTP clients are built with `reqwest` and always injected, never global.
- Tests use `wiremock` for HTTP mocking and `insta` for snapshot testing.

## Commands

cargo test --workspace # Run all tests
cargo test -p mkt-meta # Test specific crate
cargo test -p mkt-meta campaign # Run specific test
cargo fmt --all -- --check # Check formatting
cargo clippy --workspace --all-targets # Lint
cargo llvm-cov --workspace --html # Coverage report
cargo insta review # Update snapshots
cargo run -p mkt-cli -- meta campaign list # Run CLI in dev

## Conventions

- No unwrap() or panic!() in library code. Use ? operator.
- All public items must have doc comments.
- Newtype IDs: CampaignId(String), not raw String.
- Functions under 40 lines. Files under 300 lines.
- Use tracing macros for logging, never println!.
- All HTTP goes through the provider's client struct.
- Use #[test_case] for parameterized tests.
- Use insta::assert_snapshot! for output formatting tests.

## File organization

- mod.rs files only re-export. Logic in named files.
- One struct/enum per file when it has significant implementation.
- Imports grouped: std → external → internal → local.

## Testing requirements

- Every new function needs at least one unit test.
- Every new API interaction needs a wiremock integration test.
- Every new CLI command needs an assert_cmd E2E test.
- Fixtures go in crates/mkt-testkit/src/fixtures/<provider>/.
- No tests that require network access.

## Adding a new provider

1. Create crates/mkt-<name>/ following mkt-meta/ structure.
2. Implement MarketingProvider for your struct.
3. Add mapping.rs for API ↔ domain model conversion.
4. Add fixtures in mkt-testkit/src/fixtures/<name>/.
5. Add wiremock integration tests.
6. Register in mkt-cli/src/registry.rs.
7. Add feature flag in mkt-cli/Cargo.toml.

## Adding a new command

1. Add clap subcommand in mkt-cli/src/commands/<domain>.rs.
2. Wire into mkt-cli/src/cli.rs.
3. Add trait method in mkt-core/src/provider.rs if needed.
4. Implement in relevant providers.
5. Add E2E test in tests/e2e/.

## Error handling

- Library errors: MktError variants from mkt-core/src/error.rs.
- API errors must include: provider name, HTTP status, message.
- Always propagate with ?. Never silently ignore.

## Do NOT

- Add unsafe code.
- Use openssl (rustls only).
- Use println! for output (use OutputFormatter).
- Write tests that need network access or real API tokens.
- Store secrets in source code or fixtures.
- Add unwrap() or panic!() in library code.
```

---

## Appendix: .cursorrules

This file goes at the repo root. Cursor reads it automatically.

```markdown
# Cursor rules for mkt project

## Language and framework

- Rust, edition 2024, stable toolchain
- Async runtime: tokio
- HTTP client: reqwest with rustls-tls
- CLI framework: clap with derive
- Error handling: thiserror (libraries), anyhow (CLI boundary only)
- Testing: wiremock, assert_cmd, insta, test-case, mockall

## Architecture

- Cargo workspace with crates in crates/
- Core trait: MarketingProvider in crates/mkt-core/src/provider.rs
- Every provider is a separate crate implementing that trait
- Domain models are provider-agnostic in crates/mkt-core/src/models/
- Each provider has mapping.rs for API ↔ domain model conversion

## Code style

- No unwrap() or panic!() in library code
- No unsafe
- All public items have doc comments
- Functions under 40 lines
- Files under 300 lines
- Newtype IDs: CampaignId(String) not raw strings
- Use tracing macros for logging, never println!
- Group imports: std → external → internal → local

## Testing rules

- TDD: write the test first, then the implementation
- Unit tests inline in #[cfg(test)] mod tests
- Integration tests use wiremock for HTTP mocking
- E2E tests use assert_cmd against the compiled binary
- Snapshot tests with insta for output formatting
- Parameterized tests with test-case
- Fixtures in crates/mkt-testkit/src/fixtures/
- No tests that require network access

## When generating code

- Always implement Display for enums used in CLI output
- Always derive Debug, Clone, Serialize, Deserialize for models
- Always add #[serde(rename_all = "snake_case")] on enums
- Always use async fn for anything that touches HTTP
- Always return Result<T> with the crate's error type
- Always add the corresponding test alongside the implementation
- Prefer impl Into<String> over String for function parameters
- Use builder pattern for structs with 4+ fields

## File naming

- Commands: crates/mkt-cli/src/commands/{domain}.rs
- Provider impl: crates/mkt-{provider}/src/provider.rs
- API mapping: crates/mkt-{provider}/src/mapping.rs
- Domain models: crates/mkt-core/src/models/{model}.rs
- Error types: crates/{crate}/src/error.rs
```

---

## How to use this document

This spec is designed to be consumed by both humans and AI tools:

1. **For AI coding tools**: Feed this file alongside `CLAUDE.md` and `.cursorrules` to Claude Code or Cursor. The AI will understand the full architecture, conventions, and testing requirements before writing any code.

2. **For human developers**: Use the roadmap as a task list. Each checkbox is a discrete, testable unit of work.

3. **For contributors**: Read this alongside `CONTRIBUTING.md` and `docs/ADDING_A_PROVIDER.md` to understand where your code goes.

The goal is that anyone — human or AI — can pick up any phase and produce code that integrates cleanly with the rest of the project.
