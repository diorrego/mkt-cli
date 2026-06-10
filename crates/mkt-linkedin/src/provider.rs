//! [`MarketingProvider`] implementation for LinkedIn Marketing.

use mkt_core::error::{MktError, Result};
use mkt_core::models::{
    Campaign, CampaignFilters, CampaignId, CreateCampaignInput, InsightsQuery, InsightsReport,
    Paginated, ProviderHealth, UpdateCampaignInput,
};
use mkt_core::provider::{MarketingProvider, ProviderCapabilities};
use tracing::instrument;

use crate::client::LinkedInClient;
use crate::mapping;

/// LinkedIn Marketing provider.
///
/// Wraps a [`LinkedInClient`] and implements [`MarketingProvider`] using
/// the versioned REST API (`/rest/adAccounts/.../adCampaigns`,
/// `/rest/adAnalytics`).
#[derive(Debug)]
pub struct LinkedInProvider {
    client: LinkedInClient,
}

impl LinkedInProvider {
    /// Create a new LinkedIn provider.
    pub const fn new(client: LinkedInClient) -> Self {
        Self { client }
    }

    /// Path of the campaigns collection for this ad account.
    fn campaigns_path(&self) -> String {
        format!("adAccounts/{}/adCampaigns", self.client.ad_account_id())
    }

    /// Fetch a single campaign by numeric ID.
    async fn fetch_campaign(&self, id: &str) -> Result<Campaign> {
        let path = format!("{}/{id}", self.campaigns_path());
        let json = self.client.get_raw(&path, "").await?;
        mapping::linkedin_campaign_to_domain(&json)
    }
}

impl MarketingProvider for LinkedInProvider {
    fn name(&self) -> &'static str {
        "linkedin"
    }

    fn display_name(&self) -> &'static str {
        "LinkedIn Marketing"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            campaigns: true,
            adsets: false,
            ads: false,
            creatives: false,
            audiences: false,
            insights: true,
            organic_posts: false,
            dark_posts: false,
            video_upload: false,
            image_upload: false,
            workflow_templates: false,
        }
    }

    // ── Campaigns ──────────────────────────────────────

    #[instrument(skip(self))]
    fn list_campaigns(
        &self,
        filters: &CampaignFilters,
    ) -> impl std::future::Future<Output = Result<Paginated<Campaign>>> + Send {
        async move {
            // Rest.li 2.0 finder syntax: structural characters stay
            // literal; only URN colons are percent-encoded.
            let mut query = String::from("q=search");
            if let Some(status) = &filters.status {
                let li_status = mapping::domain_status_to_linkedin(status);
                query = format!("{query}&search=(status:(values:List({li_status})))");
            }
            if let Some(limit) = filters.limit {
                query = format!("{query}&pageSize={limit}");
            }
            if let Some(cursor) = &filters.cursor {
                query = format!("{query}&pageToken={cursor}");
            }

            let resp = self.client.get_raw(&self.campaigns_path(), &query).await?;

            let items = resp["elements"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .map(mapping::linkedin_campaign_to_domain)
                .collect::<Result<Vec<_>>>()?;

            let next_cursor = resp["metadata"]["nextPageToken"].as_str().map(String::from);

            Ok(Paginated {
                data: items,
                next_cursor,
                total: None,
            })
        }
    }

    #[instrument(skip(self))]
    fn get_campaign(
        &self,
        id: &CampaignId,
    ) -> impl std::future::Future<Output = Result<Campaign>> + Send {
        async move { self.fetch_campaign(&id.0).await }
    }

    #[instrument(skip(self, input))]
    fn create_campaign(
        &self,
        input: &CreateCampaignInput,
    ) -> impl std::future::Future<Output = Result<Campaign>> + Send {
        async move {
            let now_ms = chrono::Utc::now().timestamp_millis();
            let body = mapping::domain_to_linkedin_create_campaign(
                input,
                self.client.ad_account_id(),
                now_ms,
            )?;

            let resp = self
                .client
                .post(&self.campaigns_path(), &body, None)
                .await?;
            let new_id = resp.restli_id.ok_or_else(|| MktError::ApiError {
                provider: "linkedin".into(),
                status: 0,
                message: "create response missing 'x-restli-id' header".into(),
                retry_after: None,
            })?;

            self.fetch_campaign(&new_id).await
        }
    }

    #[instrument(skip(self, input))]
    fn update_campaign(
        &self,
        id: &CampaignId,
        input: &UpdateCampaignInput,
    ) -> impl std::future::Future<Output = Result<Campaign>> + Send {
        async move {
            let patch = mapping::domain_to_linkedin_update_patch(input);
            let path = format!("{}/{}", self.campaigns_path(), id.0);
            let _resp = self
                .client
                .post(&path, &patch, Some("PARTIAL_UPDATE"))
                .await?;
            self.fetch_campaign(&id.0).await
        }
    }

    /// Soft-delete: LinkedIn only allows hard DELETE for `DRAFT`
    /// campaigns, so deletion sets status to `PENDING_DELETION`.
    #[instrument(skip(self))]
    fn delete_campaign(
        &self,
        id: &CampaignId,
    ) -> impl std::future::Future<Output = Result<()>> + Send {
        async move {
            let patch = serde_json::json!({
                "patch": { "$set": { "status": "PENDING_DELETION" } }
            });
            let path = format!("{}/{}", self.campaigns_path(), id.0);
            let _resp = self
                .client
                .post(&path, &patch, Some("PARTIAL_UPDATE"))
                .await?;
            Ok(())
        }
    }

    // ── Insights ─────────────────────────────────────────

    #[instrument(skip(self, query))]
    fn get_insights(
        &self,
        query: &InsightsQuery,
    ) -> impl std::future::Future<Output = Result<InsightsReport>> + Send {
        async move {
            let fields = if query.metrics.is_empty() {
                "impressions,clicks,costInLocalCurrency,dateRange,pivotValues".to_string()
            } else {
                let mut metrics = query.metrics.join(",");
                metrics.push_str(",dateRange,pivotValues");
                metrics
            };

            // URN colons must be %3A inside List(); structural characters
            // stay literal.
            let account_urn = format!(
                "urn%3Ali%3AsponsoredAccount%3A{}",
                self.client.ad_account_id()
            );
            let mut raw_query = format!(
                "q=analytics&pivot=CAMPAIGN&timeGranularity=DAILY\
                 &accounts=List({account_urn})&fields={fields}"
            );

            if let Some(range) = &query.date_range {
                let date_range = format!(
                    "&dateRange=(start:(year:{},month:{},day:{}),\
                     end:(year:{},month:{},day:{}))",
                    range.start.format("%Y"),
                    range.start.format("%-m"),
                    range.start.format("%-d"),
                    range.end.format("%Y"),
                    range.end.format("%-m"),
                    range.end.format("%-d"),
                );
                raw_query.push_str(&date_range);
            }

            let resp = self.client.get_raw("adAnalytics", &raw_query).await?;
            mapping::linkedin_analytics_to_domain(&resp)
        }
    }

    // ── Health check ───────────────────────────────────

    async fn health_check(&self) -> Result<ProviderHealth> {
        let start = std::time::Instant::now();
        let result = self
            .client
            .get_raw(&self.campaigns_path(), "q=search&pageSize=1")
            .await;
        #[allow(clippy::cast_possible_truncation)] // health-check latency fits in u64
        let latency = start.elapsed().as_millis() as u64;

        match result {
            Ok(_) => Ok(ProviderHealth {
                provider: "linkedin".into(),
                healthy: true,
                latency_ms: latency,
                message: None,
            }),
            Err(e) => Ok(ProviderHealth {
                provider: "linkedin".into(),
                healthy: false,
                latency_ms: latency,
                message: Some(e.to_string()),
            }),
        }
    }
}
