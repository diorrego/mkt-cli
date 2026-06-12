//! [`MarketingProvider`] implementation for TikTok for Business.

use mkt_core::error::{MktError, Result};
use mkt_core::models::{
    Audience, Campaign, CampaignFilters, CampaignId, CreateCampaignInput, InsightsQuery,
    InsightsReport, Paginated, ProviderHealth, UpdateCampaignInput,
};
use mkt_core::provider::{MarketingProvider, ProviderCapabilities};
use tracing::instrument;

use crate::client::TikTokClient;
use crate::mapping;

/// TikTok for Business marketing provider.
///
/// Wraps a [`TikTokClient`] and implements [`MarketingProvider`] using the
/// Business API v1.3 (campaign endpoints, integrated reporting, DMP
/// audiences).
#[derive(Debug)]
pub struct TikTokProvider {
    client: TikTokClient,
}

impl TikTokProvider {
    /// Create a new TikTok provider.
    pub const fn new(client: TikTokClient) -> Self {
        Self { client }
    }

    /// Fetch one campaign by ID via the filtered list endpoint.
    async fn fetch_campaign(&self, id: &str) -> Result<Campaign> {
        let filtering = serde_json::json!({ "campaign_ids": [id] }).to_string();
        let params = [
            ("advertiser_id", self.client.advertiser_id()),
            ("filtering", filtering.as_str()),
        ];
        let data = self.client.get("campaign/get/", &params).await?;
        let row = data["list"]
            .as_array()
            .and_then(|rows| rows.first())
            .ok_or_else(|| MktError::ApiError {
                provider: "tiktok".into(),
                status: 404,
                message: format!("campaign {id} not found"),
                retry_after: None,
            })?;
        mapping::tiktok_campaign_to_domain(row)
    }
}

impl MarketingProvider for TikTokProvider {
    fn name(&self) -> &'static str {
        "tiktok"
    }

    fn display_name(&self) -> &'static str {
        "TikTok for Business"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            campaigns: true,
            adsets: false,
            ads: false,
            creatives: false,
            audiences: true,
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
            let mut params: Vec<(&str, String)> =
                vec![("advertiser_id", self.client.advertiser_id().to_string())];

            let mut filtering = serde_json::Map::new();
            if let Some(status) = &filters.status {
                filtering.insert(
                    "secondary_status".into(),
                    serde_json::Value::String(mapping::domain_status_to_filter(status).into()),
                );
            }
            if let Some(name) = &filters.name_contains {
                filtering.insert(
                    "campaign_name".into(),
                    serde_json::Value::String(name.clone()),
                );
            }
            if !filtering.is_empty() {
                params.push((
                    "filtering",
                    serde_json::Value::Object(filtering).to_string(),
                ));
            }
            if let Some(limit) = filters.limit {
                params.push(("page_size", limit.to_string()));
            }
            if let Some(cursor) = &filters.cursor {
                params.push(("page", cursor.clone()));
            }

            let param_refs: Vec<(&str, &str)> =
                params.iter().map(|(k, v)| (*k, v.as_str())).collect();
            let data = self.client.get("campaign/get/", &param_refs).await?;

            let items = data["list"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .map(mapping::tiktok_campaign_to_domain)
                .collect::<Result<Vec<_>>>()?;

            // Page-number pagination: expose the next page as a cursor.
            let page_info = &data["page_info"];
            let next_cursor = match (page_info["page"].as_u64(), page_info["total_page"].as_u64()) {
                (Some(page), Some(total)) if page < total => Some((page + 1).to_string()),
                _ => None,
            };

            Ok(Paginated {
                data: items,
                next_cursor,
                total: page_info["total_number"].as_u64(),
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
            // A fresh request_id lets TikTok deduplicate network-level
            // retries instead of creating a second campaign.
            let request_id = uuid::Uuid::new_v4().to_string();
            let body = mapping::domain_to_tiktok_create_campaign(
                input,
                self.client.advertiser_id(),
                &request_id,
            );
            let data = self.client.post("campaign/create/", &body).await?;

            let new_id = data["campaign_id"]
                .as_i64()
                .map(|v| v.to_string())
                .or_else(|| data["campaign_id"].as_str().map(String::from))
                .ok_or_else(|| MktError::ApiError {
                    provider: "tiktok".into(),
                    status: 0,
                    message: "create response missing 'campaign_id'".into(),
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
            // Name/budget changes go to /campaign/update/.
            if input.name.is_some() || input.budget.is_some() {
                let mut body = serde_json::json!({
                    "advertiser_id": self.client.advertiser_id(),
                    "campaign_id": id.0,
                });
                if let Some(name) = &input.name {
                    body["campaign_name"] = serde_json::Value::String(name.clone());
                }
                if let Some(budget) = &input.budget {
                    body["budget"] = serde_json::json!(budget.amount);
                }
                let _data = self.client.post("campaign/update/", &body).await?;
            }

            // Status changes go to /campaign/status/update/.
            if let Some(status) = &input.status {
                let body = serde_json::json!({
                    "advertiser_id": self.client.advertiser_id(),
                    "campaign_ids": [id.0],
                    "operation_status": mapping::domain_status_to_tiktok(status),
                });
                let _data = self.client.post("campaign/status/update/", &body).await?;
            }

            self.fetch_campaign(&id.0).await
        }
    }

    /// Deletion is a status transition (`operation_status: DELETE`) and is
    /// irreversible on TikTok.
    #[instrument(skip(self))]
    fn delete_campaign(
        &self,
        id: &CampaignId,
    ) -> impl std::future::Future<Output = Result<()>> + Send {
        async move {
            let body = serde_json::json!({
                "advertiser_id": self.client.advertiser_id(),
                "campaign_ids": [id.0],
                "operation_status": "DELETE",
            });
            let _data = self.client.post("campaign/status/update/", &body).await?;
            Ok(())
        }
    }

    // ── Audiences ─────────────────────────────────────────

    #[instrument(skip(self))]
    fn list_audiences(&self) -> impl std::future::Future<Output = Result<Vec<Audience>>> + Send {
        async move {
            let params = [
                ("advertiser_id", self.client.advertiser_id()),
                ("page_size", "100"),
            ];
            let data = self
                .client
                .get("dmp/custom_audience/list/", &params)
                .await?;
            mapping::tiktok_audiences_to_domain(&data)
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
                serde_json::json!(["spend", "impressions", "clicks", "ctr", "cpc"])
            } else {
                serde_json::json!(query.metrics)
            }
            .to_string();
            // Lifetime metrics reject time dimensions, so stat_time_day
            // only applies to date-bounded queries.
            let dimensions = if query.date_range.is_some() {
                serde_json::json!(["campaign_id", "stat_time_day"])
            } else {
                serde_json::json!(["campaign_id"])
            }
            .to_string();

            let mut params: Vec<(&str, String)> = vec![
                ("advertiser_id", self.client.advertiser_id().to_string()),
                ("report_type", "BASIC".to_string()),
                ("data_level", "AUCTION_CAMPAIGN".to_string()),
                ("dimensions", dimensions),
                ("metrics", metrics),
                ("page_size", "200".to_string()),
            ];

            if let Some(range) = &query.date_range {
                params.push(("start_date", range.start.format("%Y-%m-%d").to_string()));
                params.push(("end_date", range.end.format("%Y-%m-%d").to_string()));
            } else {
                params.push(("query_lifetime", "true".to_string()));
            }

            let param_refs: Vec<(&str, &str)> =
                params.iter().map(|(k, v)| (*k, v.as_str())).collect();
            let data = self
                .client
                .get("report/integrated/get/", &param_refs)
                .await?;
            mapping::tiktok_report_to_domain(&data)
        }
    }

    // ── Health check ───────────────────────────────────

    async fn health_check(&self) -> Result<ProviderHealth> {
        let start = std::time::Instant::now();
        let params = [
            ("advertiser_id", self.client.advertiser_id()),
            ("page_size", "1"),
        ];
        let result = self.client.get("campaign/get/", &params).await;
        #[allow(clippy::cast_possible_truncation)] // health-check latency fits in u64
        let latency = start.elapsed().as_millis() as u64;

        match result {
            Ok(_) => Ok(ProviderHealth {
                provider: "tiktok".into(),
                healthy: true,
                latency_ms: latency,
                message: None,
            }),
            Err(e) => Ok(ProviderHealth {
                provider: "tiktok".into(),
                healthy: false,
                latency_ms: latency,
                message: Some(e.to_string()),
            }),
        }
    }
}
