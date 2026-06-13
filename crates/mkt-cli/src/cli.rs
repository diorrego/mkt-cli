//! Top-level CLI definition using `clap`.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

/// Exit code contract, shown in `--help` so scripts and coding agents can
/// branch on failures without parsing error text.
const EXIT_CODES_HELP: &str = "\
Exit codes:
  0  success
  1  unexpected error (I/O, transport, bug)
  2  invalid input or configuration
  3  authentication failed (check credentials, run 'mkt doctor')
  4  resource or provider not found
  5  rate limited (transient — retry after the suggested delay)
  6  feature not supported by the provider (see 'mkt providers')
  7  provider API rejected the request

With --output json, data goes to stdout and errors are emitted on stderr as
a single JSON object: {\"ok\":false,\"error\":{\"type\",\"message\",\"suggestion\"}}.
Use --dry-run on any mutating command to preview without executing.";

/// Multi-platform marketing CLI.
///
/// Manage ads, audiences, organic posts, and analytics across
/// Meta, Google Ads, TikTok, and LinkedIn from a single terminal.
#[derive(Parser, Debug)]
#[command(name = "mkt", version, about, long_about = None, after_help = EXIT_CODES_HELP)]
pub struct Cli {
    /// Profile to use.
    #[arg(long, env = "MKT_PROFILE", default_value = "default", global = true)]
    pub profile: String,

    /// Output format.
    #[arg(
        short = 'o',
        long,
        env = "MKT_OUTPUT",
        default_value = "table",
        value_enum,
        global = true
    )]
    pub output: OutputFormatArg,

    /// Enable debug logging.
    #[arg(short = 'v', long, global = true, conflicts_with = "quiet")]
    pub verbose: bool,

    /// Suppress all output except errors.
    #[arg(short = 'q', long, global = true)]
    pub quiet: bool,

    /// Show what would happen without executing.
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Path to config file.
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    /// Command to run.
    #[command(subcommand)]
    pub command: Commands,
}

/// Output format argument for clap.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormatArg {
    /// Tabular output for terminal display.
    Table,
    /// JSON output for machine consumption.
    Json,
    /// CSV output for spreadsheets.
    Csv,
}

impl From<OutputFormatArg> for mkt_core::output::OutputFormat {
    fn from(arg: OutputFormatArg) -> Self {
        match arg {
            OutputFormatArg::Table => Self::Table,
            OutputFormatArg::Json => Self::Json,
            OutputFormatArg::Csv => Self::Csv,
        }
    }
}

/// Top-level commands.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Meta (Facebook/Instagram) provider.
    #[cfg(feature = "meta")]
    Meta {
        /// Domain subcommand.
        #[command(subcommand)]
        domain: MetaDomain,
    },

    /// Google Ads provider.
    #[cfg(feature = "google")]
    Google {
        /// Domain subcommand.
        #[command(subcommand)]
        domain: GoogleDomain,
    },

    /// TikTok for Business provider.
    #[cfg(feature = "tiktok")]
    Tiktok {
        /// Domain subcommand.
        #[command(subcommand)]
        domain: TiktokDomain,
    },

    /// LinkedIn Marketing provider.
    #[cfg(feature = "linkedin")]
    Linkedin {
        /// Domain subcommand.
        #[command(subcommand)]
        domain: LinkedinDomain,
    },

    /// List available providers and their capabilities.
    Providers,

    /// Verify config, tokens, and API connectivity.
    Doctor,

    /// Manage profiles.
    Profile {
        /// Profile action.
        #[command(subcommand)]
        action: ProfileAction,
    },

    /// Generate shell completions (bash, zsh, fish, powershell, elvish).
    Completions {
        /// Target shell.
        shell: clap_complete::Shell,
    },

    /// MCP (Model Context Protocol) server for chat agents without a
    /// terminal (Claude Desktop, `ChatGPT`). Coding agents should use the
    /// CLI directly.
    #[cfg(feature = "mcp")]
    Mcp {
        /// MCP action.
        #[command(subcommand)]
        action: McpAction,
    },
}

/// MCP server actions.
#[cfg(feature = "mcp")]
#[derive(Subcommand, Debug)]
pub enum McpAction {
    /// Serve MCP over stdio (logs go to stderr).
    Serve,
}

/// Meta provider domain subcommands.
#[cfg(feature = "meta")]
#[derive(Subcommand, Debug)]
pub enum MetaDomain {
    /// Campaign management.
    Campaign {
        /// Campaign action.
        #[command(subcommand)]
        action: CampaignAction,
    },
    /// Ad set management.
    Adset {
        /// Ad set action.
        #[command(subcommand)]
        action: AdsetAction,
    },
    /// Audience management.
    Audience {
        /// Audience action.
        #[command(subcommand)]
        action: AudienceAction,
    },
    /// Insights / analytics.
    Insight {
        /// Insight action.
        #[command(subcommand)]
        action: InsightAction,
    },
    /// Organic post management.
    Post {
        /// Post action.
        #[command(subcommand)]
        action: PostAction,
    },
    /// Creative management.
    Creative {
        /// Creative action.
        #[command(subcommand)]
        action: CreativeAction,
    },
    /// Media upload.
    Media {
        /// Media action.
        #[command(subcommand)]
        action: MediaAction,
    },
    /// Raw API escape hatch.
    Raw {
        /// Raw action.
        #[command(subcommand)]
        action: RawAction,
    },
}

/// Google Ads provider domain subcommands.
#[cfg(feature = "google")]
#[derive(Subcommand, Debug)]
pub enum GoogleDomain {
    /// Campaign management.
    ///
    /// For create, --objective is the advertising channel type
    /// (SEARCH, DISPLAY, `PERFORMANCE_MAX`, VIDEO, ...).
    Campaign {
        /// Campaign action.
        #[command(subcommand)]
        action: CampaignAction,
    },
    /// Insights / analytics (GAQL metrics).
    Insight {
        /// Insight action.
        #[command(subcommand)]
        action: InsightAction,
    },
}

/// TikTok provider domain subcommands.
#[cfg(feature = "tiktok")]
#[derive(Subcommand, Debug)]
pub enum TiktokDomain {
    /// Campaign management.
    ///
    /// For create, --objective is the TikTok objective type
    /// (TRAFFIC, `LEAD_GENERATION`, `WEB_CONVERSIONS`, REACH, ...).
    Campaign {
        /// Campaign action.
        #[command(subcommand)]
        action: CampaignAction,
    },
    /// Audience management (DMP custom audiences; list only).
    Audience {
        /// Audience action.
        #[command(subcommand)]
        action: AudienceAction,
    },
    /// Insights / analytics (integrated report).
    Insight {
        /// Insight action.
        #[command(subcommand)]
        action: InsightAction,
    },
}

/// LinkedIn provider domain subcommands.
#[cfg(feature = "linkedin")]
#[derive(Subcommand, Debug)]
pub enum LinkedinDomain {
    /// Campaign management.
    ///
    /// For create, --objective is the LinkedIn objective type
    /// (`LEAD_GENERATION`, `WEBSITE_VISIT`, `BRAND_AWARENESS`, ...) and
    /// --extra must carry the campaign group URN.
    Campaign {
        /// Campaign action.
        #[command(subcommand)]
        action: CampaignAction,
    },
    /// Insights / analytics (adAnalytics).
    Insight {
        /// Insight action.
        #[command(subcommand)]
        action: InsightAction,
    },
}

/// Campaign actions.
#[derive(Subcommand, Debug)]
pub enum CampaignAction {
    /// List campaigns.
    List {
        /// Filter by status.
        #[arg(long)]
        status: Option<String>,
        /// Filter by name substring.
        #[arg(long)]
        name: Option<String>,
        /// Maximum results.
        #[arg(long)]
        limit: Option<u32>,
    },
    /// Get campaign details.
    Get {
        /// Campaign ID.
        id: String,
    },
    /// Create a campaign.
    Create {
        /// Campaign name.
        #[arg(long)]
        name: String,
        /// Campaign objective (Meta: OUTCOME_*, Google: channel type like SEARCH).
        #[arg(long)]
        objective: String,
        /// Initial status.
        #[arg(long)]
        status: Option<String>,
        /// Daily budget in currency units (required for Google Ads and LinkedIn).
        #[arg(long)]
        daily_budget: Option<f64>,
        /// Provider-specific extra fields as inline JSON
        /// (e.g. LinkedIn: '{"campaignGroup":"urn:li:sponsoredCampaignGroup:123"}').
        #[arg(long)]
        extra: Option<String>,
        /// Load from JSON file.
        #[arg(long)]
        file: Option<PathBuf>,
    },
    /// Update a campaign.
    Update {
        /// Campaign ID.
        id: String,
        /// New name.
        #[arg(long)]
        name: Option<String>,
        /// New status.
        #[arg(long)]
        status: Option<String>,
    },
    /// Delete a campaign.
    Delete {
        /// Campaign ID.
        id: String,
    },
}

/// Ad set actions.
#[derive(Subcommand, Debug)]
pub enum AdsetAction {
    /// List ad sets for a campaign.
    List {
        /// Parent campaign ID.
        #[arg(long)]
        campaign: String,
    },
    /// Create an ad set.
    Create {
        /// Parent campaign ID.
        #[arg(long)]
        campaign: String,
        /// Ad set name.
        #[arg(long)]
        name: String,
        /// Initial status (active, paused).
        #[arg(long)]
        status: Option<String>,
        /// Targeting spec as inline JSON.
        #[arg(long)]
        targeting: Option<String>,
        /// Daily budget in minor units (e.g. cents).
        #[arg(long)]
        daily_budget: Option<f64>,
        /// Optimization goal (e.g. `LINK_CLICKS`, `OFFSITE_CONVERSIONS`).
        #[arg(long)]
        optimization_goal: Option<String>,
        /// Billing event (e.g. IMPRESSIONS).
        #[arg(long)]
        billing_event: Option<String>,
    },
}

/// Audience actions.
#[derive(Subcommand, Debug)]
pub enum AudienceAction {
    /// List audiences.
    List,
    /// Create an audience.
    Create {
        /// Audience name.
        #[arg(long)]
        name: String,
        /// Audience description.
        #[arg(long)]
        description: Option<String>,
    },
    /// Add users to an audience (PII is normalized and SHA-256 hashed locally).
    AddUsers {
        /// Audience ID.
        id: String,
        /// Email address (repeatable). Raw or pre-hashed SHA-256.
        #[arg(long)]
        email: Vec<String>,
        /// Phone number (repeatable). Raw or pre-hashed SHA-256.
        #[arg(long)]
        phone: Vec<String>,
    },
}

/// Insight actions.
#[derive(Subcommand, Debug)]
pub enum InsightAction {
    /// Get insights.
    Get {
        /// Metrics to retrieve (comma-separated).
        #[arg(long, value_delimiter = ',')]
        metrics: Vec<String>,
        /// Breakdowns (comma-separated).
        #[arg(long, value_delimiter = ',')]
        breakdowns: Vec<String>,
        /// Date range (e.g. "7d", "30d", "2026-01-01:2026-03-01").
        #[arg(long)]
        range: Option<String>,
    },
}

/// Post actions.
#[derive(Subcommand, Debug)]
pub enum PostAction {
    /// Create an organic post.
    Create {
        /// Target platform (facebook or instagram).
        #[arg(long, default_value = "facebook")]
        platform: String,
        /// Post message.
        #[arg(long)]
        message: Option<String>,
        /// Image URL.
        #[arg(long)]
        image_url: Option<String>,
        /// Link URL.
        #[arg(long)]
        link: Option<String>,
    },
    /// Promote an existing post as an ad (created paused, inside an existing ad set).
    Promote {
        /// Post ID.
        id: String,
        /// Ad set to create the promoted-post ad in.
        #[arg(long)]
        adset: String,
        /// Name for the created ad.
        #[arg(long)]
        name: Option<String>,
    },
}

/// Creative actions.
#[derive(Subcommand, Debug)]
pub enum CreativeAction {
    /// Create an ad creative.
    Create {
        /// Creative name.
        #[arg(long)]
        name: String,
        /// Body text.
        #[arg(long)]
        body: Option<String>,
        /// Image URL.
        #[arg(long)]
        image_url: Option<String>,
        /// Link URL.
        #[arg(long)]
        link_url: Option<String>,
    },
    /// Create a dark (unpublished) post.
    DarkPost {
        /// Post message.
        #[arg(long)]
        message: String,
        /// Link URL.
        #[arg(long)]
        link: Option<String>,
        /// Image URL.
        #[arg(long)]
        image_url: Option<String>,
    },
}

/// Media actions.
#[derive(Subcommand, Debug)]
pub enum MediaAction {
    /// Upload an image asset.
    UploadImage {
        /// Path to local image file.
        #[arg(long)]
        file: Option<PathBuf>,
        /// Image URL.
        #[arg(long)]
        url: Option<String>,
    },
    /// Upload a video asset.
    UploadVideo {
        /// Path to local video file.
        #[arg(long)]
        file: Option<PathBuf>,
        /// Video URL.
        #[arg(long)]
        url: Option<String>,
        /// Video title.
        #[arg(long)]
        title: Option<String>,
    },
}

/// Raw API actions.
#[derive(Subcommand, Debug)]
pub enum RawAction {
    /// Raw GET request.
    Get {
        /// API path.
        path: String,
        /// Query parameters as key=value pairs.
        #[arg(long, value_delimiter = ',')]
        fields: Vec<String>,
    },
    /// Raw POST request.
    Post {
        /// API path.
        path: String,
        /// JSON body.
        #[arg(long)]
        body: Option<String>,
    },
}

/// Profile management actions.
#[derive(Subcommand, Debug)]
pub enum ProfileAction {
    /// Configure a profile.
    Set {
        /// Profile name.
        name: String,
        /// Provider name.
        #[arg(long)]
        provider: Option<String>,
        /// Access token.
        #[arg(long)]
        access_token: Option<String>,
        /// Ad account ID.
        #[arg(long)]
        ad_account: Option<String>,
        /// Page ID.
        #[arg(long)]
        page_id: Option<String>,
        /// Instagram user ID.
        #[arg(long)]
        ig_user_id: Option<String>,
    },
    /// List all profiles.
    List,
    /// Show profile details.
    Show {
        /// Profile name.
        name: String,
    },
}
