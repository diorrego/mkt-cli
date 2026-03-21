//! Top-level CLI definition using `clap`.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

/// Multi-platform marketing CLI.
///
/// Manage ads, audiences, organic posts, and analytics across
/// Meta, Google Ads, `TikTok`, and `LinkedIn` from a single terminal.
#[derive(Parser, Debug)]
#[command(name = "mkt", version, about, long_about = None)]
pub struct Cli {
    /// Profile to use.
    #[arg(long, default_value = "default", global = true)]
    pub profile: String,

    /// Output format.
    #[arg(long, default_value = "table", value_enum, global = true)]
    pub output: OutputFormatArg,

    /// Enable debug logging.
    #[arg(long, global = true)]
    pub verbose: bool,

    /// Suppress all output except errors.
    #[arg(long, global = true)]
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
        domain: StubDomain,
    },

    /// `TikTok` for Business provider.
    #[cfg(feature = "tiktok")]
    Tiktok {
        /// Domain subcommand.
        #[command(subcommand)]
        domain: StubDomain,
    },

    /// `LinkedIn` Marketing provider.
    #[cfg(feature = "linkedin")]
    Linkedin {
        /// Domain subcommand.
        #[command(subcommand)]
        domain: StubDomain,
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
        /// Campaign objective.
        #[arg(long)]
        objective: String,
        /// Initial status.
        #[arg(long)]
        status: Option<String>,
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
    /// Promote an existing post as an ad.
    Promote {
        /// Post ID.
        id: String,
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

/// Stub domain for providers not yet implemented.
#[derive(Subcommand, Debug)]
pub enum StubDomain {
    /// This provider is not yet implemented.
    Status,
}
