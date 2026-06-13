//! MCP (Model Context Protocol) server over stdio.
//!
//! `mkt mcp serve` exposes the same provider operations as the CLI as MCP
//! tools, for chat environments without a terminal (Claude Desktop,
//! `ChatGPT`). Coding agents with shell access should prefer the CLI
//! directly — see `AGENTS.md`.
//!
//! Safety mirrors the CLI: campaign creation is always paused, and
//! mutating tools state so in their descriptions.

use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
    transport::stdio,
};

use mkt_core::models::{
    CampaignFilters, CampaignId, CampaignStatus, CreateCampaignInput, InsightsQuery,
    UpdateCampaignInput,
};
use mkt_core::provider::MarketingProvider;

use crate::providers;

// ── Parameter types ───────────────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CampaignListParams {
    /// Ad platform: "meta", "google", "tiktok", or "linkedin".
    pub provider: String,
    /// Filter by status: active, paused, archived, draft, deleted.
    pub status: Option<String>,
    /// Maximum number of campaigns to return.
    pub limit: Option<u32>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CampaignGetParams {
    /// Ad platform: "meta", "google", "tiktok", or "linkedin".
    pub provider: String,
    /// Campaign ID on that platform.
    pub campaign_id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CampaignCreateParams {
    /// Ad platform: "meta", "google", "tiktok", or "linkedin".
    pub provider: String,
    /// Campaign name.
    pub name: String,
    /// Platform objective (Meta: OUTCOME_*; Google: channel type like
    /// SEARCH; TikTok: `TRAFFIC/LEAD_GENERATION`; LinkedIn: `LEAD_GENERATION`/
    /// `WEBSITE_VISIT`/...).
    pub objective: String,
    /// Daily budget in currency units (required by Google and LinkedIn).
    pub daily_budget: Option<f64>,
    /// Provider-specific extra fields as a JSON object (e.g. LinkedIn
    /// campaignGroup URN).
    pub extra: Option<serde_json::Value>,
    /// Preview only: when true, describe the would-be campaign without
    /// calling the platform (no credentials needed).
    pub dry_run: Option<bool>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CampaignSetStatusParams {
    /// Ad platform: "meta", "google", "tiktok", or "linkedin".
    pub provider: String,
    /// Campaign ID on that platform.
    pub campaign_id: String,
    /// New status: "active" (starts spend!) or "paused".
    pub status: String,
    /// Preview only: when true, describe the would-be transition without
    /// calling the platform (no credentials needed).
    pub dry_run: Option<bool>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct InsightsParams {
    /// Ad platform: "meta", "google", "tiktok", or "linkedin".
    pub provider: String,
    /// Metric names (platform-specific; sensible defaults if omitted).
    pub metrics: Option<Vec<String>>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct HealthParams {
    /// Ad platform: "meta", "google", "tiktok", or "linkedin".
    pub provider: String,
}

// ── Server ────────────────────────────────────────────────────────────────────

/// The `mkt` MCP server: stateless per call, credentials resolved from
/// env vars / config exactly like the CLI.
#[derive(Clone)]
pub struct MktMcpServer {
    profile: String,
}

/// Serve MCP over stdio until the client disconnects.
///
/// # Errors
///
/// Returns an error if the transport fails to start or terminates
/// abnormally.
pub async fn serve(profile: String) -> anyhow::Result<()> {
    let service = MktMcpServer { profile }.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}

/// Run `op` against the provider named in `provider`.
///
/// Providers use RPITIT (not object-safe), so dispatch is a match over
/// concrete types calling a generic helper.
macro_rules! dispatch_provider {
    ($self:expr, $provider:expr, |$p:ident| $body:expr) => {{
        let config = providers::load_config(None).map_err(to_mcp_error)?;
        match $provider {
            "meta" => {
                let $p = providers::build_meta(&config, &$self.profile).map_err(to_mcp_error)?;
                $body
            }
            "google" => {
                let $p = providers::build_google(&config, &$self.profile)
                    .await
                    .map_err(to_mcp_error)?;
                $body
            }
            "tiktok" => {
                let $p = providers::build_tiktok(&config, &$self.profile).map_err(to_mcp_error)?;
                $body
            }
            "linkedin" => {
                let $p =
                    providers::build_linkedin(&config, &$self.profile).map_err(to_mcp_error)?;
                $body
            }
            other => Err(McpError::invalid_params(
                format!("unknown provider '{other}'; use meta, google, tiktok, or linkedin"),
                None,
            )),
        }
    }};
}

#[tool_router]
impl MktMcpServer {
    #[tool(
        description = "List ad campaigns on a platform (meta, google, tiktok, linkedin). \
                       Read-only. Returns JSON with id, name, status, objective, budget."
    )]
    async fn campaign_list(
        &self,
        Parameters(params): Parameters<CampaignListParams>,
    ) -> Result<CallToolResult, McpError> {
        let filters = CampaignFilters {
            status: params.status.as_deref().map(parse_status),
            limit: params.limit,
            ..Default::default()
        };
        dispatch_provider!(self, params.provider.as_str(), |p| {
            let page = p.list_campaigns(&filters).await.map_err(to_mcp_error)?;
            json_result(&page.data)
        })
    }

    #[tool(
        description = "Get a single ad campaign by ID. Read-only. Returns the full \
                       campaign as JSON including the raw platform payload."
    )]
    async fn campaign_get(
        &self,
        Parameters(params): Parameters<CampaignGetParams>,
    ) -> Result<CallToolResult, McpError> {
        let id = CampaignId::from(params.campaign_id.as_str());
        dispatch_provider!(self, params.provider.as_str(), |p| {
            let campaign = p.get_campaign(&id).await.map_err(to_mcp_error)?;
            json_result(&campaign)
        })
    }

    #[tool(
        description = "Create an ad campaign. SPENDS MONEY when activated — campaigns \
                       are always created PAUSED; use campaign_set_status to start \
                       delivery explicitly. Google and LinkedIn require daily_budget; \
                       LinkedIn also requires extra.campaignGroup (URN). Pass \
                       dry_run=true to preview without executing."
    )]
    async fn campaign_create(
        &self,
        Parameters(params): Parameters<CampaignCreateParams>,
    ) -> Result<CallToolResult, McpError> {
        if params.dry_run.unwrap_or(false) {
            return json_result(&serde_json::json!({
                "dry_run": true,
                "would_create": {
                    "provider": params.provider,
                    "name": params.name,
                    "objective": params.objective,
                    "daily_budget": params.daily_budget,
                    "status": "PAUSED",
                },
                "note": "No API call was made. Campaigns are always created \
                         PAUSED; activating spend is a separate explicit step.",
            }));
        }
        let input = CreateCampaignInput {
            name: params.name.clone(),
            objective: params.objective.clone(),
            // The MCP surface never creates active campaigns.
            status: Some(CampaignStatus::Paused),
            budget: params.daily_budget.map(|amount| mkt_core::models::Budget {
                amount,
                currency: "USD".into(),
                kind: mkt_core::models::BudgetKind::Daily,
            }),
            extra: params.extra.clone(),
        };
        dispatch_provider!(self, params.provider.as_str(), |p| {
            let campaign = p.create_campaign(&input).await.map_err(to_mcp_error)?;
            json_result(&campaign)
        })
    }

    #[tool(
        description = "Set a campaign's status. 'active' STARTS AD SPEND — only do \
                       this when the user has explicitly confirmed. 'paused' stops \
                       delivery. Pass dry_run=true to preview without executing."
    )]
    async fn campaign_set_status(
        &self,
        Parameters(params): Parameters<CampaignSetStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        if params.dry_run.unwrap_or(false) {
            return json_result(&serde_json::json!({
                "dry_run": true,
                "would_set": {
                    "provider": params.provider,
                    "campaign_id": params.campaign_id,
                    "status": params.status,
                },
                "note": "No API call was made. Setting status to 'active' \
                         starts real ad spend — confirm with the user first.",
            }));
        }
        let id = CampaignId::from(params.campaign_id.as_str());
        let input = UpdateCampaignInput {
            status: Some(parse_status(&params.status)),
            ..Default::default()
        };
        dispatch_provider!(self, params.provider.as_str(), |p| {
            let campaign = p.update_campaign(&id, &input).await.map_err(to_mcp_error)?;
            json_result(&campaign)
        })
    }

    #[tool(
        description = "Get advertising performance metrics (impressions, clicks, \
                       spend/cost, ...). Read-only."
    )]
    async fn insights_get(
        &self,
        Parameters(params): Parameters<InsightsParams>,
    ) -> Result<CallToolResult, McpError> {
        let query = InsightsQuery {
            metrics: params.metrics.clone().unwrap_or_default(),
            ..Default::default()
        };
        dispatch_provider!(self, params.provider.as_str(), |p| {
            let report = p.get_insights(&query).await.map_err(to_mcp_error)?;
            json_result(&report)
        })
    }

    #[tool(
        description = "Check that credentials work and the platform API is reachable. \
                       Read-only; never prints token values."
    )]
    async fn provider_health(
        &self,
        Parameters(params): Parameters<HealthParams>,
    ) -> Result<CallToolResult, McpError> {
        dispatch_provider!(self, params.provider.as_str(), |p| {
            let health = p.health_check().await.map_err(to_mcp_error)?;
            json_result(&health)
        })
    }
}

#[tool_handler]
impl ServerHandler for MktMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("mkt", env!("CARGO_PKG_VERSION")))
            .with_instructions(
                "mkt manages ad campaigns across Meta, Google Ads, TikTok, and \
                 LinkedIn. Campaigns are always created PAUSED; activating one \
                 (campaign_set_status to 'active') starts real ad spend — confirm \
                 with the user first. Credentials come from MKT_* environment \
                 variables or ~/.config/mkt/config.toml."
                    .to_string(),
            )
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Serialize a value as pretty JSON tool output.
fn json_result<T: serde::Serialize>(value: &T) -> Result<CallToolResult, McpError> {
    let text = serde_json::to_string_pretty(value)
        .map_err(|e| McpError::internal_error(format!("serialization failed: {e}"), None))?;
    Ok(CallToolResult::success(vec![Content::text(text)]))
}

/// Convert any mkt error into an MCP error, preserving the structured
/// type/suggestion contract in the message where available.
fn to_mcp_error<E: Into<anyhow::Error>>(error: E) -> McpError {
    use mkt_core::error::MktError;
    let error: anyhow::Error = error.into();
    let detail = error.downcast_ref::<MktError>().map_or_else(
        || error.to_string(),
        |e| {
            let mut msg = format!("[{}] {e}", e.error_type());
            if let Some(suggestion) = e.suggestion() {
                use std::fmt::Write as _;
                let _ = write!(msg, " — {suggestion}");
            }
            msg
        },
    );
    McpError::internal_error(detail, None)
}

fn parse_status(s: &str) -> CampaignStatus {
    match s.to_lowercase().as_str() {
        "active" => CampaignStatus::Active,
        "paused" => CampaignStatus::Paused,
        "archived" => CampaignStatus::Archived,
        "draft" => CampaignStatus::Draft,
        "deleted" => CampaignStatus::Deleted,
        other => CampaignStatus::Other(other.to_string()),
    }
}
