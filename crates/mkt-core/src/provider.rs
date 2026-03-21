//! Core provider trait for the `mkt` marketing CLI.
//!
//! Every platform (Meta, Google, TikTok, LinkedIn) implements
//! [`MarketingProvider`]. Methods return unified domain models from
//! [`crate::models`], not platform-specific structs.
//!
//! This trait uses native `async fn` in traits (Rust 2024 edition RPITIT).
//! Because RPITIT is not object-safe, the CLI uses enum dispatch
//! (`AnyProvider`) instead of `dyn MarketingProvider`.

use crate::error::{MktError, Result};
use crate::models::{
    Ad, AdSet, Audience, AudienceId, AudienceUpdateResult, AudienceUser, Campaign, CampaignFilters,
    CampaignId, CreateAdSetInput, CreateAudienceInput, CreateCampaignInput, CreateCreativeInput,
    CreateDarkPostInput, Creative, HttpMethod, InsightsQuery, InsightsReport, MediaAsset,
    Paginated, Post, PostId, PromotePostInput, ProviderHealth, PublishPostInput,
    UpdateCampaignInput, UploadImageInput, UploadVideoInput,
};

/// Describes the capabilities a provider supports.
///
/// Used by the CLI to show/hide commands dynamically and to provide
/// helpful error messages when a user tries an unsupported feature.
#[derive(Debug, Clone)]
pub struct ProviderCapabilities {
    /// Supports campaign CRUD.
    pub campaigns: bool,
    /// Supports ad set / ad group management.
    pub adsets: bool,
    /// Supports individual ad management.
    pub ads: bool,
    /// Supports creative asset management.
    pub creatives: bool,
    /// Supports audience management.
    pub audiences: bool,
    /// Supports insights / analytics queries.
    pub insights: bool,
    /// Supports organic post publishing.
    pub organic_posts: bool,
    /// Supports dark (unpublished) posts.
    pub dark_posts: bool,
    /// Supports video upload.
    pub video_upload: bool,
    /// Supports image upload.
    pub image_upload: bool,
    /// Supports workflow templates.
    pub workflow_templates: bool,
}

/// The core abstraction. Every platform implements this trait.
///
/// Methods return unified domain models, not platform-specific structs.
/// Optional capabilities have default implementations that return
/// [`MktError::NotSupported`].
///
/// # Object Safety
///
/// This trait is **not** object-safe due to the use of native `async fn`
/// (RPITIT). Use the `AnyProvider` enum in `mkt-cli` for dynamic dispatch.
pub trait MarketingProvider: Send + Sync {
    /// Short lowercase name used in CLI commands: `"meta"`, `"google"`, `"tiktok"`.
    fn name(&self) -> &'static str;

    /// Human-readable display name: `"Meta (Facebook/Instagram)"`.
    fn display_name(&self) -> &'static str;

    /// What this provider can do.
    fn capabilities(&self) -> ProviderCapabilities;

    // ── Campaigns ──────────────────────────────────────────

    /// List campaigns matching the given filters.
    fn list_campaigns(
        &self,
        filters: &CampaignFilters,
    ) -> impl std::future::Future<Output = Result<Paginated<Campaign>>> + Send;

    /// Get a single campaign by ID.
    fn get_campaign(
        &self,
        id: &CampaignId,
    ) -> impl std::future::Future<Output = Result<Campaign>> + Send;

    /// Create a new campaign.
    fn create_campaign(
        &self,
        input: &CreateCampaignInput,
    ) -> impl std::future::Future<Output = Result<Campaign>> + Send;

    /// Update an existing campaign.
    fn update_campaign(
        &self,
        id: &CampaignId,
        input: &UpdateCampaignInput,
    ) -> impl std::future::Future<Output = Result<Campaign>> + Send;

    /// Delete a campaign by ID.
    fn delete_campaign(
        &self,
        id: &CampaignId,
    ) -> impl std::future::Future<Output = Result<()>> + Send;

    // ── Ad Sets / Ad Groups ────────────────────────────────

    /// List ad sets for a campaign.
    fn list_adsets(
        &self,
        _campaign_id: &CampaignId,
    ) -> impl std::future::Future<Output = Result<Paginated<AdSet>>> + Send {
        async { Err(MktError::not_supported(self.name(), "adsets")) }
    }

    /// Create a new ad set.
    fn create_adset(
        &self,
        _input: &CreateAdSetInput,
    ) -> impl std::future::Future<Output = Result<AdSet>> + Send {
        async { Err(MktError::not_supported(self.name(), "adsets")) }
    }

    // ── Creatives ──────────────────────────────────────────

    /// Create an ad creative.
    fn create_creative(
        &self,
        _input: &CreateCreativeInput,
    ) -> impl std::future::Future<Output = Result<Creative>> + Send {
        async { Err(MktError::not_supported(self.name(), "creatives")) }
    }

    /// Create an unpublished (dark) post for use in ads.
    fn create_dark_post(
        &self,
        _input: &CreateDarkPostInput,
    ) -> impl std::future::Future<Output = Result<Creative>> + Send {
        async { Err(MktError::not_supported(self.name(), "dark_posts")) }
    }

    // ── Audiences ──────────────────────────────────────────

    /// List all audiences.
    fn list_audiences(&self) -> impl std::future::Future<Output = Result<Vec<Audience>>> + Send {
        async { Err(MktError::not_supported(self.name(), "audiences")) }
    }

    /// Create a new audience.
    fn create_audience(
        &self,
        _input: &CreateAudienceInput,
    ) -> impl std::future::Future<Output = Result<Audience>> + Send {
        async { Err(MktError::not_supported(self.name(), "audiences")) }
    }

    /// Add users to an existing audience.
    fn add_users_to_audience(
        &self,
        _id: &AudienceId,
        _users: &[AudienceUser],
    ) -> impl std::future::Future<Output = Result<AudienceUpdateResult>> + Send {
        async { Err(MktError::not_supported(self.name(), "audience_users")) }
    }

    // ── Insights ───────────────────────────────────────────

    /// Query analytics / insights.
    fn get_insights(
        &self,
        _query: &InsightsQuery,
    ) -> impl std::future::Future<Output = Result<InsightsReport>> + Send {
        async { Err(MktError::not_supported(self.name(), "insights")) }
    }

    // ── Organic Posts ──────────────────────────────────────

    /// Publish an organic post (Facebook Page, Instagram, etc.).
    fn publish_post(
        &self,
        _input: &PublishPostInput,
    ) -> impl std::future::Future<Output = Result<Post>> + Send {
        async { Err(MktError::not_supported(self.name(), "organic_posts")) }
    }

    /// Promote an existing organic post as an ad.
    fn promote_post(
        &self,
        _post_id: &PostId,
        _input: &PromotePostInput,
    ) -> impl std::future::Future<Output = Result<Ad>> + Send {
        async { Err(MktError::not_supported(self.name(), "promote_post")) }
    }

    // ── Media Upload ───────────────────────────────────────

    /// Upload an image asset.
    fn upload_image(
        &self,
        _input: &UploadImageInput,
    ) -> impl std::future::Future<Output = Result<MediaAsset>> + Send {
        async { Err(MktError::not_supported(self.name(), "image_upload")) }
    }

    /// Upload a video asset.
    fn upload_video(
        &self,
        _input: &UploadVideoInput,
    ) -> impl std::future::Future<Output = Result<MediaAsset>> + Send {
        async { Err(MktError::not_supported(self.name(), "video_upload")) }
    }

    // ── Raw Escape Hatch ───────────────────────────────────

    /// Execute a raw API call, bypassing model mapping.
    fn raw_request(
        &self,
        _method: HttpMethod,
        _path: &str,
        _params: &serde_json::Value,
    ) -> impl std::future::Future<Output = Result<serde_json::Value>> + Send {
        async { Err(MktError::not_supported(self.name(), "raw_request")) }
    }

    // ── Health Check ───────────────────────────────────────

    /// Verify that credentials are valid and the API is reachable.
    fn health_check(&self) -> impl std::future::Future<Output = Result<ProviderHealth>> + Send {
        async { Err(MktError::not_supported(self.name(), "health_check")) }
    }
}
