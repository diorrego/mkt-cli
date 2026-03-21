//! A configurable in-process mock implementation of [`MarketingProvider`].
//!
//! Use [`MockProvider`] in tests to simulate provider responses without
//! making real HTTP requests. Construct it with [`MockProvider::new`] for an
//! empty-but-healthy provider, [`MockProvider::with_campaigns`] to seed
//! campaign data, or [`MockProvider::failing`] to exercise error paths.
//!
//! # Examples
//!
//! ```rust,no_run
//! use mkt_testkit::mock_provider::MockProvider;
//! use mkt_core::provider::MarketingProvider;
//! use mkt_core::models::CampaignFilters;
//!
//! #[tokio::main]
//! async fn main() {
//!     let provider = MockProvider::new();
//!     let result = provider.list_campaigns(&CampaignFilters::default()).await;
//!     assert!(result.is_ok());
//! }
//! ```

use mkt_core::error::{MktError, Result};
use mkt_core::models::{
    Campaign, CampaignFilters, CampaignId, CreateCampaignInput, Paginated, UpdateCampaignInput,
};
use mkt_core::provider::{MarketingProvider, ProviderCapabilities};

/// A configurable mock implementation of [`MarketingProvider`] for use in tests.
///
/// All campaign operations are backed by an in-memory `Vec<Campaign>`. Every
/// other optional capability returns [`MktError::NotSupported`] by default,
/// or [`MktError::ApiError`] when the provider was constructed with
/// [`MockProvider::failing`].
#[derive(Debug, Clone, Default)]
pub struct MockProvider {
    /// The campaigns returned by list/get operations.
    pub campaigns: Vec<Campaign>,
    /// When `true`, every method returns an error instead of data.
    pub should_error: bool,
    /// The error message used when `should_error` is `true`.
    pub error_message: String,
}

impl MockProvider {
    /// Create a healthy mock with no campaigns and no errors.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a healthy mock pre-seeded with the given campaigns.
    pub fn with_campaigns(campaigns: Vec<Campaign>) -> Self {
        Self {
            campaigns,
            should_error: false,
            error_message: String::new(),
        }
    }

    /// Create a mock that returns [`MktError::ApiError`] for every call.
    ///
    /// The `message` string is stored and surfaced in every error response.
    pub fn failing(message: &str) -> Self {
        Self {
            campaigns: Vec::new(),
            should_error: true,
            error_message: message.to_string(),
        }
    }

    /// Build an [`MktError::ApiError`] using the stored error message.
    fn make_api_error(&self) -> MktError {
        MktError::ApiError {
            provider: "mock".to_string(),
            status: 500,
            message: self.error_message.clone(),
            retry_after: None,
        }
    }
}

impl MarketingProvider for MockProvider {
    fn name(&self) -> &'static str {
        "mock"
    }

    fn display_name(&self) -> &'static str {
        "Mock Provider"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            campaigns: true,
            adsets: false,
            ads: false,
            creatives: false,
            audiences: false,
            insights: false,
            organic_posts: false,
            dark_posts: false,
            video_upload: false,
            image_upload: false,
            workflow_templates: false,
        }
    }

    async fn list_campaigns(&self, filters: &CampaignFilters) -> Result<Paginated<Campaign>> {
        if self.should_error {
            return Err(self.make_api_error());
        }

        let mut data: Vec<Campaign> = self.campaigns.clone();

        if let Some(ref status_filter) = filters.status {
            data.retain(|c| &c.status == status_filter);
        }

        if let Some(ref name_substr) = filters.name_contains {
            data.retain(|c| c.name.contains(name_substr.as_str()));
        }

        if let Some(limit) = filters.limit {
            data.truncate(limit as usize);
        }

        Ok(Paginated {
            data,
            next_cursor: None,
            total: None,
        })
    }

    async fn get_campaign(&self, id: &CampaignId) -> Result<Campaign> {
        if self.should_error {
            return Err(self.make_api_error());
        }

        self.campaigns
            .iter()
            .find(|c| &c.id == id)
            .cloned()
            .ok_or_else(|| MktError::ApiError {
                provider: "mock".to_string(),
                status: 404,
                message: format!("campaign '{}' not found", id),
                retry_after: None,
            })
    }

    async fn create_campaign(&self, _input: &CreateCampaignInput) -> Result<Campaign> {
        if self.should_error {
            return Err(self.make_api_error());
        }

        Err(MktError::not_supported("mock", "create_campaign"))
    }

    async fn update_campaign(
        &self,
        id: &CampaignId,
        _input: &UpdateCampaignInput,
    ) -> Result<Campaign> {
        if self.should_error {
            return Err(self.make_api_error());
        }

        self.campaigns
            .iter()
            .find(|c| &c.id == id)
            .cloned()
            .ok_or_else(|| MktError::ApiError {
                provider: "mock".to_string(),
                status: 404,
                message: format!("campaign '{}' not found", id),
                retry_after: None,
            })
    }

    async fn delete_campaign(&self, id: &CampaignId) -> Result<()> {
        if self.should_error {
            return Err(self.make_api_error());
        }

        if self.campaigns.iter().any(|c| &c.id == id) {
            Ok(())
        } else {
            Err(MktError::ApiError {
                provider: "mock".to_string(),
                status: 404,
                message: format!("campaign '{}' not found", id),
                retry_after: None,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use mkt_core::models::CampaignStatus;

    fn make_campaign(id: &str, name: &str, status: CampaignStatus) -> Campaign {
        Campaign {
            id: CampaignId(id.to_string()),
            provider: "mock".to_string(),
            name: name.to_string(),
            status,
            objective: "OUTCOME_LEADS".to_string(),
            budget: None,
            created_at: Utc::now(),
            updated_at: None,
            raw: None,
        }
    }

    // ── name / display_name / capabilities ───────────────────────────

    #[test]
    fn test_mock_provider_name_returns_mock() {
        let p = MockProvider::new();
        assert_eq!(p.name(), "mock");
    }

    #[test]
    fn test_mock_provider_display_name_returns_mock_provider() {
        let p = MockProvider::new();
        assert_eq!(p.display_name(), "Mock Provider");
    }

    #[test]
    fn test_mock_provider_capabilities_campaigns_true() {
        let caps = MockProvider::new().capabilities();
        assert!(caps.campaigns);
    }

    #[test]
    fn test_mock_provider_capabilities_non_campaign_features_false() {
        let caps = MockProvider::new().capabilities();
        assert!(!caps.adsets);
        assert!(!caps.ads);
        assert!(!caps.creatives);
        assert!(!caps.audiences);
        assert!(!caps.insights);
        assert!(!caps.organic_posts);
        assert!(!caps.dark_posts);
        assert!(!caps.video_upload);
        assert!(!caps.image_upload);
        assert!(!caps.workflow_templates);
    }

    // ── constructors ─────────────────────────────────────────────────

    #[test]
    fn test_mock_provider_new_has_no_campaigns() {
        let p = MockProvider::new();
        assert!(p.campaigns.is_empty());
        assert!(!p.should_error);
    }

    #[test]
    fn test_mock_provider_with_campaigns_stores_them() {
        let campaigns = vec![make_campaign("1", "Alpha", CampaignStatus::Active)];
        let p = MockProvider::with_campaigns(campaigns.clone());
        assert_eq!(p.campaigns.len(), 1);
        assert_eq!(p.campaigns[0].id.0, "1");
    }

    #[test]
    fn test_mock_provider_failing_sets_error_flag() {
        let p = MockProvider::failing("boom");
        assert!(p.should_error);
        assert_eq!(p.error_message, "boom");
    }

    // ── list_campaigns ────────────────────────────────────────────────

    #[tokio::test]
    async fn test_list_campaigns_empty_provider_returns_empty_page() {
        let p = MockProvider::new();
        let result = p
            .list_campaigns(&CampaignFilters::default())
            .await
            .expect("list_campaigns failed");
        assert!(result.data.is_empty());
        assert!(result.next_cursor.is_none());
    }

    #[tokio::test]
    async fn test_list_campaigns_returns_all_campaigns_when_no_filters() {
        let campaigns = vec![
            make_campaign("1", "Alpha", CampaignStatus::Active),
            make_campaign("2", "Beta", CampaignStatus::Paused),
        ];
        let p = MockProvider::with_campaigns(campaigns);
        let result = p
            .list_campaigns(&CampaignFilters::default())
            .await
            .expect("list_campaigns failed");
        assert_eq!(result.data.len(), 2);
    }

    #[tokio::test]
    async fn test_list_campaigns_filters_by_status() {
        let campaigns = vec![
            make_campaign("1", "Alpha", CampaignStatus::Active),
            make_campaign("2", "Beta", CampaignStatus::Paused),
            make_campaign("3", "Gamma", CampaignStatus::Active),
        ];
        let p = MockProvider::with_campaigns(campaigns);
        let filters = CampaignFilters {
            status: Some(CampaignStatus::Active),
            ..Default::default()
        };
        let result = p
            .list_campaigns(&filters)
            .await
            .expect("list_campaigns failed");
        assert_eq!(result.data.len(), 2);
        assert!(
            result
                .data
                .iter()
                .all(|c| c.status == CampaignStatus::Active)
        );
    }

    #[tokio::test]
    async fn test_list_campaigns_filters_by_name_contains() {
        let campaigns = vec![
            make_campaign("1", "Summer Sale", CampaignStatus::Active),
            make_campaign("2", "Winter Promo", CampaignStatus::Active),
            make_campaign("3", "Summer Awareness", CampaignStatus::Active),
        ];
        let p = MockProvider::with_campaigns(campaigns);
        let filters = CampaignFilters {
            name_contains: Some("Summer".to_string()),
            ..Default::default()
        };
        let result = p
            .list_campaigns(&filters)
            .await
            .expect("list_campaigns failed");
        assert_eq!(result.data.len(), 2);
        assert!(result.data.iter().all(|c| c.name.contains("Summer")));
    }

    #[tokio::test]
    async fn test_list_campaigns_respects_limit() {
        let campaigns = (0..10)
            .map(|i| {
                make_campaign(
                    &i.to_string(),
                    &format!("Campaign {i}"),
                    CampaignStatus::Active,
                )
            })
            .collect();
        let p = MockProvider::with_campaigns(campaigns);
        let filters = CampaignFilters {
            limit: Some(3),
            ..Default::default()
        };
        let result = p
            .list_campaigns(&filters)
            .await
            .expect("list_campaigns failed");
        assert_eq!(result.data.len(), 3);
    }

    #[tokio::test]
    async fn test_list_campaigns_failing_provider_returns_api_error() {
        let p = MockProvider::failing("downstream unavailable");
        let err = p
            .list_campaigns(&CampaignFilters::default())
            .await
            .expect_err("expected error");
        assert!(matches!(err, MktError::ApiError { status: 500, .. }));
    }

    #[tokio::test]
    async fn test_list_campaigns_error_message_is_preserved() {
        let p = MockProvider::failing("custom error text");
        let err = p
            .list_campaigns(&CampaignFilters::default())
            .await
            .expect_err("expected error");
        if let MktError::ApiError { message, .. } = err {
            assert_eq!(message, "custom error text");
        } else {
            panic!("expected ApiError");
        }
    }

    // ── get_campaign ──────────────────────────────────────────────────

    #[tokio::test]
    async fn test_get_campaign_returns_matching_campaign() {
        let campaigns = vec![make_campaign(
            "camp_42",
            "My Campaign",
            CampaignStatus::Active,
        )];
        let p = MockProvider::with_campaigns(campaigns);
        let result = p
            .get_campaign(&CampaignId("camp_42".to_string()))
            .await
            .expect("get_campaign failed");
        assert_eq!(result.id.0, "camp_42");
        assert_eq!(result.name, "My Campaign");
    }

    #[tokio::test]
    async fn test_get_campaign_missing_id_returns_404_error() {
        let p = MockProvider::new();
        let err = p
            .get_campaign(&CampaignId("nonexistent".to_string()))
            .await
            .expect_err("expected error");
        assert!(matches!(err, MktError::ApiError { status: 404, .. }));
    }

    #[tokio::test]
    async fn test_get_campaign_failing_provider_returns_api_error() {
        let p = MockProvider::failing("service down");
        let err = p
            .get_campaign(&CampaignId("any".to_string()))
            .await
            .expect_err("expected error");
        assert!(matches!(err, MktError::ApiError { status: 500, .. }));
    }

    // ── create_campaign ───────────────────────────────────────────────

    #[tokio::test]
    async fn test_create_campaign_returns_not_supported() {
        let p = MockProvider::new();
        let input = CreateCampaignInput {
            name: "New Camp".to_string(),
            objective: "OUTCOME_LEADS".to_string(),
            ..Default::default()
        };
        let err = p.create_campaign(&input).await.expect_err("expected error");
        assert!(matches!(err, MktError::NotSupported { .. }));
    }

    #[tokio::test]
    async fn test_create_campaign_failing_provider_returns_api_error() {
        let p = MockProvider::failing("unavailable");
        let input = CreateCampaignInput::default();
        let err = p.create_campaign(&input).await.expect_err("expected error");
        assert!(matches!(err, MktError::ApiError { status: 500, .. }));
    }

    // ── update_campaign ───────────────────────────────────────────────

    #[tokio::test]
    async fn test_update_campaign_returns_existing_campaign() {
        let campaigns = vec![make_campaign("camp_1", "Old Name", CampaignStatus::Active)];
        let p = MockProvider::with_campaigns(campaigns);
        let result = p
            .update_campaign(
                &CampaignId("camp_1".to_string()),
                &UpdateCampaignInput::default(),
            )
            .await
            .expect("update_campaign failed");
        assert_eq!(result.id.0, "camp_1");
    }

    #[tokio::test]
    async fn test_update_campaign_missing_id_returns_404_error() {
        let p = MockProvider::new();
        let err = p
            .update_campaign(
                &CampaignId("ghost".to_string()),
                &UpdateCampaignInput::default(),
            )
            .await
            .expect_err("expected error");
        assert!(matches!(err, MktError::ApiError { status: 404, .. }));
    }

    #[tokio::test]
    async fn test_update_campaign_failing_provider_returns_api_error() {
        let p = MockProvider::failing("error");
        let err = p
            .update_campaign(
                &CampaignId("camp_1".to_string()),
                &UpdateCampaignInput::default(),
            )
            .await
            .expect_err("expected error");
        assert!(matches!(err, MktError::ApiError { status: 500, .. }));
    }

    // ── delete_campaign ───────────────────────────────────────────────

    #[tokio::test]
    async fn test_delete_campaign_existing_id_returns_ok() {
        let campaigns = vec![make_campaign("del_1", "To Delete", CampaignStatus::Active)];
        let p = MockProvider::with_campaigns(campaigns);
        let result = p.delete_campaign(&CampaignId("del_1".to_string())).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_campaign_missing_id_returns_404_error() {
        let p = MockProvider::new();
        let err = p
            .delete_campaign(&CampaignId("ghost".to_string()))
            .await
            .expect_err("expected error");
        assert!(matches!(err, MktError::ApiError { status: 404, .. }));
    }

    #[tokio::test]
    async fn test_delete_campaign_failing_provider_returns_api_error() {
        let p = MockProvider::failing("error");
        let err = p
            .delete_campaign(&CampaignId("camp_1".to_string()))
            .await
            .expect_err("expected error");
        assert!(matches!(err, MktError::ApiError { status: 500, .. }));
    }

    // ── default trait methods (optional capabilities) ─────────────────

    #[tokio::test]
    async fn test_list_adsets_returns_not_supported() {
        let p = MockProvider::new();
        let err = p
            .list_adsets(&CampaignId("c".to_string()))
            .await
            .expect_err("expected error");
        assert!(matches!(err, MktError::NotSupported { .. }));
    }

    #[tokio::test]
    async fn test_get_insights_returns_not_supported() {
        use mkt_core::models::InsightsQuery;
        let p = MockProvider::new();
        let err = p
            .get_insights(&InsightsQuery::default())
            .await
            .expect_err("expected error");
        assert!(matches!(err, MktError::NotSupported { .. }));
    }

    #[tokio::test]
    async fn test_health_check_returns_not_supported() {
        let p = MockProvider::new();
        let err = p.health_check().await.expect_err("expected error");
        assert!(matches!(err, MktError::NotSupported { .. }));
    }
}
