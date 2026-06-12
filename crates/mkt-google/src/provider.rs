//! [`MarketingProvider`] implementation for Google Ads.

use mkt_core::error::{MktError, Result};
use mkt_core::models::{
    Campaign, CampaignFilters, CampaignId, CreateCampaignInput, InsightsQuery, InsightsReport,
    Paginated, ProviderHealth, UpdateCampaignInput,
};
use mkt_core::provider::{MarketingProvider, ProviderCapabilities};
use tracing::instrument;

use crate::client::GoogleClient;
use crate::mapping;

/// Google Ads marketing provider.
///
/// Wraps a [`GoogleClient`] and implements [`MarketingProvider`] using the
/// Google Ads REST API: GAQL for reads, `:mutate` endpoints for writes.
#[derive(Debug)]
pub struct GoogleProvider {
    client: GoogleClient,
}

impl GoogleProvider {
    /// Create a new Google Ads provider.
    pub const fn new(client: GoogleClient) -> Self {
        Self { client }
    }

    /// Fetch one campaign by numeric ID via GAQL.
    async fn fetch_campaign(&self, id: &str) -> Result<Campaign> {
        let query = format!(
            "SELECT {} FROM campaign WHERE campaign.id = {id}",
            mapping::CAMPAIGN_GAQL_FIELDS
        );
        let resp = self.client.search(&query).await?;
        let row = resp["results"]
            .as_array()
            .and_then(|rows| rows.first())
            .ok_or_else(|| MktError::ApiError {
                provider: "google".into(),
                status: 404,
                message: format!("campaign {id} not found"),
                retry_after: None,
            })?;
        mapping::google_row_to_campaign(row)
    }
}

impl MarketingProvider for GoogleProvider {
    fn name(&self) -> &'static str {
        "google"
    }

    fn display_name(&self) -> &'static str {
        "Google Ads"
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
            let mut query = format!("SELECT {} FROM campaign", mapping::CAMPAIGN_GAQL_FIELDS);

            let mut conditions: Vec<String> = Vec::new();
            if let Some(status) = &filters.status {
                conditions.push(format!(
                    "campaign.status = '{}'",
                    mapping::domain_status_to_google(status)
                ));
            }
            if let Some(name) = &filters.name_contains {
                // GAQL LIKE with % wildcards; single quotes escaped.
                let escaped = name.replace('\'', "\\'");
                conditions.push(format!("campaign.name LIKE '%{escaped}%'"));
            }
            if !conditions.is_empty() {
                query.push_str(" WHERE ");
                query.push_str(&conditions.join(" AND "));
            }
            if let Some(limit) = filters.limit {
                query = format!("{query} LIMIT {limit}");
            }

            let resp = self.client.search(&query).await?;

            let items = resp["results"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .map(mapping::google_row_to_campaign)
                .collect::<Result<Vec<_>>>()?;

            let next_cursor = resp["nextPageToken"].as_str().map(String::from);

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
            // Google requires a budget resource before the campaign.
            let budget_ops = mapping::budget_create_operation(input).ok_or_else(|| {
                MktError::ValidationError {
                    field: "budget".into(),
                    message: "a budget is required to create a Google Ads campaign".into(),
                }
            })?;
            let budget_resp = self.client.mutate("campaignBudgets", &budget_ops).await?;
            let budget_resource = budget_resp["results"][0]["resourceName"]
                .as_str()
                .ok_or_else(|| MktError::ApiError {
                    provider: "google".into(),
                    status: 0,
                    message: "budget mutate response missing resourceName".into(),
                    retry_after: None,
                })?;

            let campaign_ops = mapping::campaign_create_operation(input, budget_resource);
            let campaign_resp = self.client.mutate("campaigns", &campaign_ops).await?;
            let new_id = mapping::campaign_id_from_mutate(&campaign_resp)?;

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
            let ops = mapping::campaign_update_operation(self.client.customer_id(), &id.0, input);
            let _resp = self.client.mutate("campaigns", &ops).await?;
            self.fetch_campaign(&id.0).await
        }
    }

    #[instrument(skip(self))]
    fn delete_campaign(
        &self,
        id: &CampaignId,
    ) -> impl std::future::Future<Output = Result<()>> + Send {
        async move {
            let resource = mapping::campaign_resource_name(self.client.customer_id(), &id.0);
            let ops = serde_json::json!([{ "remove": resource }]);
            let _resp = self.client.mutate("campaigns", &ops).await?;
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
            let metrics = if query.metrics.is_empty() {
                "metrics.impressions, metrics.clicks, metrics.cost_micros, metrics.ctr".to_string()
            } else {
                query
                    .metrics
                    .iter()
                    .map(|m| format!("metrics.{m}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            };

            let gaql = format!(
                "SELECT campaign.id, campaign.name, segments.date, {metrics} FROM campaign"
            );

            // GAQL requires a finite date range whenever a core date
            // segment is selected, so an unbounded query must not be sent.
            let gaql = query.date_range.as_ref().map_or_else(
                || format!("{gaql} WHERE segments.date DURING LAST_30_DAYS"),
                |range| {
                    format!(
                        "{gaql} WHERE segments.date BETWEEN '{}' AND '{}'",
                        range.start.format("%Y-%m-%d"),
                        range.end.format("%Y-%m-%d")
                    )
                },
            );

            let resp = self.client.search(&gaql).await?;
            mapping::google_insights_to_domain(&resp)
        }
    }

    // ── Health check ───────────────────────────────────

    async fn health_check(&self) -> Result<ProviderHealth> {
        let start = std::time::Instant::now();
        let result = self
            .client
            .search("SELECT customer.id FROM customer LIMIT 1")
            .await;
        #[allow(clippy::cast_possible_truncation)] // health-check latency fits in u64
        let latency = start.elapsed().as_millis() as u64;

        match result {
            Ok(_) => Ok(ProviderHealth {
                provider: "google".into(),
                healthy: true,
                latency_ms: latency,
                message: None,
            }),
            Err(e) => Ok(ProviderHealth {
                provider: "google".into(),
                healthy: false,
                latency_ms: latency,
                message: Some(e.to_string()),
            }),
        }
    }
}
