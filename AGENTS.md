# mkt — Agent Guide

`mkt` is a multi-platform marketing CLI (Meta, Google Ads, TikTok, LinkedIn)
designed to be driven by coding agents and scripts. This file is the operating
contract: command shape, output formats, exit codes, and safety rules.

## Command shape

```
mkt [global flags] <provider> <domain> <action> [flags]
```

- Providers: `meta` (available), `google` / `tiktok` / `linkedin` (in development).
- Domains: `campaign`, `adset`, `audience`, `insight`, `post`, `creative`, `media`, `raw`.
- Meta-commands: `mkt providers`, `mkt doctor`, `mkt profile list|show|set`.
- Every command and subcommand has `--help` with examples. Discovery is a
  deterministic tree walk: `mkt --help` → `mkt meta --help` → `mkt meta campaign --help`.

## Machine-readable output

- `--output json` prints data as JSON to **stdout**. Logs and diagnostics go to **stderr**.
- On failure with `--output json`, stderr carries exactly one JSON object:

```json
{"ok": false, "error": {"type": "auth_error", "message": "...", "suggestion": "Run 'mkt doctor' ...", "transient": true}}
```

- `error.type` is a stable snake_case identifier: `validation_error`,
  `config_error`, `auth_error`, `provider_not_found`, `rate_limited`,
  `not_supported`, `api_error`, `http_error`, `io_error`, `serde_error`.
- `transient: true` means the same request may succeed if retried (rate
  limits, 5xx). Honor `suggestion` when present — it names the recovery command.
- `--output csv` and `--output table` (default) are for humans/spreadsheets.

## Exit codes (stable contract)

| Code | Meaning                                  | Agent action                       |
|------|------------------------------------------|------------------------------------|
| 0    | success                                  | continue                           |
| 1    | unexpected error (I/O, transport, bug)   | inspect stderr                     |
| 2    | invalid input or configuration           | fix flags/config, do not retry     |
| 3    | authentication failed                    | run `mkt doctor`, check env vars   |
| 4    | resource or provider not found           | verify the ID                      |
| 5    | rate limited (transient)                 | wait the suggested delay and retry |
| 6    | feature not supported by the provider    | run `mkt providers`                |
| 7    | provider API rejected the request        | inspect `error.message`            |

## Safety rules (this tool spends real money)

1. **Always `--dry-run` first** on any mutating command (`create`, `update`,
   `delete`, `promote`, `add-users`). It prints what would happen and exits 0
   without calling the API.
2. Promoted-post ads and new campaigns should start **paused**
   (`--status paused`; `post promote` always creates the ad PAUSED — a human
   or an explicit follow-up command activates spend).
3. Never echo access tokens. `mkt doctor` validates credentials without
   printing them.
4. Audience uploads hash PII (emails/phones) locally with SHA-256 before any
   network call; you may pass raw or pre-hashed values.

## Authentication (non-interactive)

Environment variables take precedence over the config file:

```bash
export MKT_META_ACCESS_TOKEN="..."     # Meta system-user token
export MKT_META_AD_ACCOUNT_ID="act_123456789"
```

Or `~/.config/mkt/config.toml` (override dir with `MKT_CONFIG_DIR`):

```toml
[profiles.default.meta]
access_token = "..."
ad_account_id = "act_123456789"
page_id = "..."        # required for Facebook posts / dark posts
ig_user_id = "..."     # required for Instagram posts
api_version = "v25.0"  # optional override
```

Verify with `mkt doctor` (exit 0 = ready). There are no interactive prompts
anywhere in the CLI.

## Recipes

```bash
# List active campaigns as JSON
mkt --output json meta campaign list --status active

# Create a paused campaign (safe), then inspect it
mkt --dry-run meta campaign create --name "Q3 Launch" --objective OUTCOME_SALES
mkt --output json meta campaign create --name "Q3 Launch" --objective OUTCOME_SALES --status paused

# Ad sets within a campaign
mkt --output json meta adset list --campaign 120330000000000001
mkt meta adset create --campaign 120330000000000001 --name "US 25-55" \
  --status paused --daily-budget 2500 \
  --targeting '{"geo_locations":{"countries":["US"]},"age_min":25,"age_max":55}' \
  --optimization-goal LINK_CLICKS --billing-event IMPRESSIONS

# Boost an organic post (ad is created PAUSED inside an existing ad set)
mkt meta post promote 123456789_987654321 --adset 23845600000000001

# Upload hashed customers to an audience
mkt meta audience add-users 23842000001 --email a@example.com --phone "+1 555 123 4567"

# Insights for the last 7 days
mkt --output json meta insight get --range 7d --metrics impressions,clicks,spend

# Raw escape hatch for unwrapped Graph API endpoints
mkt meta raw get "act_123/campaigns" --fields id,name,status
mkt meta raw post "act_123/campaigns" --body '{"name":"X","objective":"OUTCOME_TRAFFIC","special_ad_categories":[]}'
```

## Development

```bash
cargo test --workspace --all-features   # full suite
cargo fmt --all -- --check              # formatting gate
cargo clippy --workspace --all-targets --all-features  # zero-warning policy
cargo run -p mkt-cli -- meta campaign list
```

Architecture: Cargo workspace — `mkt-core` (trait `MarketingProvider`, models,
config, output, PII hashing), `mkt-meta` (Graph API v25.0), `mkt-cli` (clap
binary), `mkt-testkit` (wiremock fixtures). New providers implement
`MarketingProvider` and register in `mkt-cli`. See `CLAUDE.md` and
`dev/MKT_PROJECT_SPEC.md` for full conventions.
