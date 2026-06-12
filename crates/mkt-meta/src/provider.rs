//! [`MarketingProvider`] implementation for Meta (Facebook / Instagram).

use mkt_core::error::{MktError, Result};
use mkt_core::models::{
    Ad, AdSet, Audience, AudienceId, AudienceUpdateResult, AudienceUser, Campaign, CampaignFilters,
    CampaignId, CreateAdSetInput, CreateAudienceInput, CreateCampaignInput, CreateCreativeInput,
    CreateDarkPostInput, Creative, CreativeId, HttpMethod, InsightsQuery, InsightsReport,
    MediaAsset, MediaAssetId, MediaType, Paginated, Post, PostId, PromotePostInput, ProviderHealth,
    PublishPostInput, UpdateCampaignInput, UploadImageInput, UploadVideoInput,
};
use mkt_core::provider::{MarketingProvider, ProviderCapabilities};
use tracing::instrument;

use crate::client::MetaClient;
use crate::mapping;

/// Meta (Facebook / Instagram) marketing provider.
///
/// Wraps a [`MetaClient`] and implements the
/// [`MarketingProvider`] trait using the Graph API.
#[derive(Debug)]
pub struct MetaProvider {
    client: MetaClient,
    page_id: Option<String>,
    ig_user_id: Option<String>,
}

impl MetaProvider {
    /// Create a new Meta provider.
    pub const fn new(
        client: MetaClient,
        page_id: Option<String>,
        ig_user_id: Option<String>,
    ) -> Self {
        Self {
            client,
            page_id,
            ig_user_id,
        }
    }

    /// Build the `act_{id}` path prefix.
    fn act_path(&self) -> String {
        format!("act_{}", self.client.ad_account_id())
    }

    /// Publish a post to a Facebook Page.
    async fn publish_facebook(&self, input: &PublishPostInput) -> Result<Post> {
        let page_id = self
            .page_id
            .as_deref()
            .ok_or_else(|| MktError::ValidationError {
                field: "page_id".into(),
                message: "page_id is required for Facebook posts".into(),
            })?;
        let path = format!("{page_id}/feed");
        let mut body = serde_json::json!({});
        if let Some(msg) = &input.message {
            body["message"] = serde_json::Value::String(msg.clone());
        }
        if let Some(link) = &input.link {
            body["link"] = serde_json::Value::String(link.clone());
        }
        let resp = self.client.post(&path, &body).await?;
        let post_id = resp["id"].as_str().ok_or_else(|| MktError::ApiError {
            provider: "meta".into(),
            status: 0,
            message: "create post response missing 'id'".into(),
            retry_after: None,
        })?;
        let get_resp = self
            .client
            .get(
                post_id,
                &[(
                    "fields",
                    "id,message,permalink_url,created_time,full_picture,link",
                )],
            )
            .await?;
        mapping::meta_post_to_domain(&get_resp, "facebook")
    }

    /// Publish a post to Instagram (two-step container flow).
    async fn publish_instagram(&self, input: &PublishPostInput) -> Result<Post> {
        let ig_user_id = self
            .ig_user_id
            .as_deref()
            .ok_or_else(|| MktError::ValidationError {
                field: "ig_user_id".into(),
                message: "ig_user_id is required for Instagram posts".into(),
            })?;

        // Step 1: Create a media container.
        let container_path = format!("{ig_user_id}/media");
        let mut container_body = serde_json::json!({});
        if let Some(msg) = &input.message {
            container_body["caption"] = serde_json::Value::String(msg.clone());
        }
        if let Some(url) = &input.image_url {
            container_body["image_url"] = serde_json::Value::String(url.clone());
        }
        let container_resp = self.client.post(&container_path, &container_body).await?;
        let container_id = container_resp["id"]
            .as_str()
            .ok_or_else(|| MktError::ApiError {
                provider: "meta".into(),
                status: 0,
                message: "Instagram container creation missing 'id'".into(),
                retry_after: None,
            })?;

        // Step 2: Publish the container.
        let publish_path = format!("{ig_user_id}/media_publish");
        let publish_body = serde_json::json!({ "creation_id": container_id });
        let publish_resp = self.client.post(&publish_path, &publish_body).await?;
        let media_id = publish_resp["id"]
            .as_str()
            .ok_or_else(|| MktError::ApiError {
                provider: "meta".into(),
                status: 0,
                message: "Instagram media_publish missing 'id'".into(),
                retry_after: None,
            })?;

        let get_resp = self
            .client
            .get(
                media_id,
                &[("fields", "id,caption,permalink,timestamp,media_url")],
            )
            .await?;
        mapping::meta_post_to_domain(&get_resp, "instagram")
    }
}

impl MarketingProvider for MetaProvider {
    fn name(&self) -> &'static str {
        "meta"
    }

    fn display_name(&self) -> &'static str {
        "Meta (Facebook/Instagram)"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            campaigns: true,
            adsets: true,
            ads: true,
            creatives: true,
            audiences: true,
            insights: true,
            organic_posts: true,
            dark_posts: true,
            video_upload: true,
            image_upload: true,
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
                vec![("fields", mapping::CAMPAIGN_FIELDS.to_string())];

            if let Some(limit) = filters.limit {
                params.push(("limit", limit.to_string()));
            }
            if let Some(ref cursor) = filters.cursor {
                params.push(("after", cursor.clone()));
            }
            if let Some(ref status) = filters.status {
                let meta_status = mapping::domain_status_to_meta(status);
                let filter = format!(
                    "[{{'field':'effective_status',\
                      'operator':'IN',\
                      'value':['{meta_status}']}}]"
                );
                params.push(("filtering", filter));
            }

            let param_refs: Vec<(&str, &str)> =
                params.iter().map(|(k, v)| (*k, v.as_str())).collect();

            let path = format!("{}/campaigns", self.act_path());
            let json = self.client.get(&path, &param_refs).await?;

            let items = json["data"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .map(mapping::meta_campaign_to_domain)
                .collect::<Result<Vec<_>>>()?;

            let next_cursor = json["paging"]["cursors"]["after"]
                .as_str()
                .map(String::from);

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
        async move {
            let params = [("fields", mapping::CAMPAIGN_FIELDS)];
            let json = self.client.get(&id.0, &params).await?;
            mapping::meta_campaign_to_domain(&json)
        }
    }

    #[instrument(skip(self))]
    fn create_campaign(
        &self,
        input: &CreateCampaignInput,
    ) -> impl std::future::Future<Output = Result<Campaign>> + Send {
        async move {
            let body = mapping::domain_to_meta_create_campaign(input);
            let path = format!("{}/campaigns", self.act_path());
            let result = self.client.post(&path, &body).await?;

            // The create response only contains {"id":"..."}.
            let new_id = result["id"].as_str().ok_or_else(|| MktError::ApiError {
                provider: "meta".into(),
                status: 0,
                message: "create response missing 'id'".into(),
                retry_after: None,
            })?;

            let get_params = [("fields", mapping::CAMPAIGN_FIELDS)];
            let json = self.client.get(new_id, &get_params).await?;
            mapping::meta_campaign_to_domain(&json)
        }
    }

    #[instrument(skip(self))]
    fn update_campaign(
        &self,
        id: &CampaignId,
        input: &UpdateCampaignInput,
    ) -> impl std::future::Future<Output = Result<Campaign>> + Send {
        async move {
            let body = mapping::domain_to_meta_update_campaign(input);
            let _result = self.client.post(&id.0, &body).await?;

            let params = [("fields", mapping::CAMPAIGN_FIELDS)];
            let json = self.client.get(&id.0, &params).await?;
            mapping::meta_campaign_to_domain(&json)
        }
    }

    #[instrument(skip(self))]
    fn delete_campaign(
        &self,
        id: &CampaignId,
    ) -> impl std::future::Future<Output = Result<()>> + Send {
        async move {
            let _result = self.client.delete(&id.0).await?;
            Ok(())
        }
    }

    // ── Ad Sets ───────────────────────────────────────────

    #[instrument(skip(self))]
    fn list_adsets(
        &self,
        campaign_id: &CampaignId,
    ) -> impl std::future::Future<Output = Result<Paginated<AdSet>>> + Send {
        async move {
            let path = format!("{}/adsets", campaign_id.0);
            let params = [("fields", mapping::ADSET_FIELDS)];
            let json = self.client.get(&path, &params).await?;

            let items = json["data"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .map(mapping::meta_adset_to_domain)
                .collect::<Result<Vec<_>>>()?;

            let next_cursor = json["paging"]["cursors"]["after"]
                .as_str()
                .map(String::from);

            Ok(Paginated {
                data: items,
                next_cursor,
                total: None,
            })
        }
    }

    #[instrument(skip(self, input))]
    fn create_adset(
        &self,
        input: &CreateAdSetInput,
    ) -> impl std::future::Future<Output = Result<AdSet>> + Send {
        async move {
            let body = mapping::domain_to_meta_create_adset(input);
            let path = format!("{}/adsets", self.act_path());
            let result = self.client.post(&path, &body).await?;

            // The create response only contains {"id":"..."}.
            let new_id = result["id"].as_str().ok_or_else(|| MktError::ApiError {
                provider: "meta".into(),
                status: 0,
                message: "create adset response missing 'id'".into(),
                retry_after: None,
            })?;

            let get_params = [("fields", mapping::ADSET_FIELDS)];
            let json = self.client.get(new_id, &get_params).await?;
            mapping::meta_adset_to_domain(&json)
        }
    }

    // ── Audiences ─────────────────────────────────────────

    #[instrument(skip(self))]
    fn list_audiences(&self) -> impl std::future::Future<Output = Result<Vec<Audience>>> + Send {
        async move {
            let path = format!("{}/customaudiences", self.act_path());
            let params = [(
                "fields",
                "id,name,description,approximate_count_lower_bound,approximate_count_upper_bound,time_created,subtype",
            )];
            let resp = self.client.get(&path, &params).await?;
            mapping::meta_audiences_to_domain(&resp)
        }
    }

    #[instrument(skip(self, input))]
    fn create_audience(
        &self,
        input: &CreateAudienceInput,
    ) -> impl std::future::Future<Output = Result<Audience>> + Send {
        async move {
            let path = format!("{}/customaudiences", self.act_path());
            let body = mapping::domain_to_meta_create_audience(input);
            let resp = self.client.post(&path, &body).await?;

            let id = resp["id"].as_str().ok_or_else(|| MktError::ApiError {
                provider: "meta".into(),
                status: 0,
                message: "create audience response missing 'id'".into(),
                retry_after: None,
            })?;

            let audience_resp = self
                .client
                .get(
                    id,
                    &[("fields", "id,name,description,approximate_count_lower_bound,approximate_count_upper_bound,time_created,subtype")],
                )
                .await?;
            mapping::meta_audience_to_domain(&audience_resp)
        }
    }

    #[instrument(skip(self, users))]
    fn add_users_to_audience(
        &self,
        id: &AudienceId,
        users: &[AudienceUser],
    ) -> impl std::future::Future<Output = Result<AudienceUpdateResult>> + Send {
        let body = mapping::domain_users_to_meta_payload(users);
        async move {
            if users.is_empty() {
                return Err(MktError::ValidationError {
                    field: "users".into(),
                    message: "at least one user is required".into(),
                });
            }
            let path = format!("{}/users", id.0);
            let resp = self.client.post(&path, &body).await?;

            Ok(AudienceUpdateResult {
                audience_id: AudienceId(
                    resp["audience_id"]
                        .as_str()
                        .unwrap_or(id.0.as_str())
                        .to_string(),
                ),
                num_received: resp["num_received"].as_u64().unwrap_or(0),
                num_invalid: resp["num_invalid_entries"].as_u64().unwrap_or(0),
            })
        }
    }

    // ── Insights ─────────────────────────────────────────

    #[instrument(skip(self, query))]
    fn get_insights(
        &self,
        query: &InsightsQuery,
    ) -> impl std::future::Future<Output = Result<InsightsReport>> + Send {
        async move {
            let path = format!("{}/insights", self.act_path());
            let metrics = if query.metrics.is_empty() {
                "impressions,clicks,spend,cpc,cpm,ctr".to_string()
            } else {
                query.metrics.join(",")
            };

            let mut params: Vec<(&str, String)> = vec![("fields", metrics)];

            if !query.breakdowns.is_empty() {
                params.push(("breakdowns", query.breakdowns.join(",")));
            }

            if let Some(range) = &query.date_range {
                let time_range = format!(
                    "{{\"since\":\"{}\",\"until\":\"{}\"}}",
                    range.start.format("%Y-%m-%d"),
                    range.end.format("%Y-%m-%d")
                );
                params.push(("time_range", time_range));
            }

            if let Some(level) = &query.level {
                params.push(("level", level.to_string()));
            }

            let param_refs: Vec<(&str, &str)> =
                params.iter().map(|(k, v)| (*k, v.as_ref())).collect();

            let resp = self.client.get(&path, &param_refs).await?;
            mapping::meta_insights_to_domain(&resp)
        }
    }

    // ── Organic Posts ────────────────────────────────────

    #[instrument(skip(self, input))]
    fn publish_post(
        &self,
        input: &PublishPostInput,
    ) -> impl std::future::Future<Output = Result<Post>> + Send {
        async move {
            match input.platform.as_str() {
                "instagram" => self.publish_instagram(input).await,
                _ => self.publish_facebook(input).await,
            }
        }
    }

    #[instrument(skip(self, input))]
    fn promote_post(
        &self,
        post_id: &PostId,
        input: &PromotePostInput,
    ) -> impl std::future::Future<Output = Result<Ad>> + Send {
        async move {
            // Step 1: create an ad creative that references the organic post.
            let creative_path = format!("{}/adcreatives", self.act_path());
            let ad_name = input
                .name
                .clone()
                .unwrap_or_else(|| format!("Boost — {}", post_id.0));
            let creative_body = serde_json::json!({
                "name": ad_name,
                "object_story_id": post_id.0,
            });
            let creative_resp = self.client.post(&creative_path, &creative_body).await?;
            let creative_id = creative_resp["id"]
                .as_str()
                .ok_or_else(|| MktError::ApiError {
                    provider: "meta".into(),
                    status: 0,
                    message: "promote creative response missing 'id'".into(),
                    retry_after: None,
                })?;

            // Step 2: create the ad in the target ad set. Always paused so a
            // human (or agent) activates spend explicitly.
            let ads_path = format!("{}/ads", self.act_path());
            let mut ad_body = serde_json::json!({
                "name": ad_name,
                "adset_id": input.adset_id,
                "creative": { "creative_id": creative_id },
                "status": "PAUSED",
            });
            if let Some(extra) = &input.extra {
                if let (Some(base), Some(overlay)) = (ad_body.as_object_mut(), extra.as_object()) {
                    for (k, v) in overlay {
                        base.insert(k.clone(), v.clone());
                    }
                }
            }
            let ad_resp = self.client.post(&ads_path, &ad_body).await?;
            let ad_id = ad_resp["id"].as_str().ok_or_else(|| MktError::ApiError {
                provider: "meta".into(),
                status: 0,
                message: "promote ad response missing 'id'".into(),
                retry_after: None,
            })?;

            // Step 3: fetch the full ad object.
            let params = [("fields", mapping::AD_FIELDS)];
            let json = self.client.get(ad_id, &params).await?;
            mapping::meta_ad_to_domain(&json)
        }
    }

    // ── Creatives ────────────────────────────────────────

    #[instrument(skip(self, input))]
    fn create_creative(
        &self,
        input: &CreateCreativeInput,
    ) -> impl std::future::Future<Output = Result<Creative>> + Send {
        async move {
            let path = format!("{}/adcreatives", self.act_path());
            let mut object_story_spec = serde_json::json!({});

            // object_story_spec requires a real Page ID; "me" resolves to
            // the user and the API rejects it.
            let page_id = self
                .page_id
                .as_deref()
                .ok_or_else(|| MktError::ValidationError {
                    field: "page_id".into(),
                    message: "page_id is required to create creatives (set meta.page_id in the profile or MKT_META_PAGE_ID)".into(),
                })?;
            object_story_spec["page_id"] = serde_json::Value::String(page_id.to_string());

            let mut link_data = serde_json::json!({});
            if let Some(body) = &input.body {
                link_data["message"] = serde_json::Value::String(body.clone());
            }
            if let Some(link_url) = &input.link_url {
                link_data["link"] = serde_json::Value::String(link_url.clone());
            }
            if let Some(image_url) = &input.image_url {
                link_data["picture"] = serde_json::Value::String(image_url.clone());
            }
            if let Some(cta) = &input.call_to_action {
                link_data["call_to_action"] = serde_json::json!({"type": cta});
            }
            object_story_spec["link_data"] = link_data;

            let body = serde_json::json!({
                "name": input.name,
                "object_story_spec": object_story_spec,
            });

            let resp = self.client.post(&path, &body).await?;
            let id = resp["id"].as_str().ok_or_else(|| MktError::ApiError {
                provider: "meta".into(),
                status: 0,
                message: "create creative response missing 'id'".into(),
                retry_after: None,
            })?;

            Ok(Creative {
                id: CreativeId(id.to_string()),
                provider: "meta".to_string(),
                name: input.name.clone(),
                body: input.body.clone(),
                image_url: input.image_url.clone(),
                video_url: input.video_url.clone(),
                link_url: input.link_url.clone(),
                call_to_action: input.call_to_action.clone(),
                created_at: chrono::Utc::now(),
                raw: Some(resp),
            })
        }
    }

    #[instrument(skip(self, input))]
    fn create_dark_post(
        &self,
        input: &CreateDarkPostInput,
    ) -> impl std::future::Future<Output = Result<Creative>> + Send {
        async move {
            let path = format!("{}/feed", input.page_id);
            let mut body = serde_json::json!({
                "message": input.message,
                "published": false,
            });
            if let Some(link) = &input.link {
                body["link"] = serde_json::Value::String(link.clone());
            }
            let resp = self.client.post(&path, &body).await?;
            let post_id = resp["id"].as_str().ok_or_else(|| MktError::ApiError {
                provider: "meta".into(),
                status: 0,
                message: "create dark post response missing 'id'".into(),
                retry_after: None,
            })?;

            Ok(Creative {
                id: CreativeId(post_id.to_string()),
                provider: "meta".to_string(),
                name: format!("dark_post_{post_id}"),
                body: Some(input.message.clone()),
                image_url: input.image_url.clone(),
                video_url: None,
                link_url: input.link.clone(),
                call_to_action: input.call_to_action.clone(),
                created_at: chrono::Utc::now(),
                raw: Some(resp),
            })
        }
    }

    // ── Media Upload ─────────────────────────────────────

    #[instrument(skip(self, input))]
    fn upload_image(
        &self,
        input: &UploadImageInput,
    ) -> impl std::future::Future<Output = Result<MediaAsset>> + Send {
        async move {
            let path = format!("{}/adimages", self.act_path());
            let url = input
                .url
                .as_deref()
                .ok_or_else(|| MktError::ValidationError {
                    field: "url".into(),
                    message: "image URL is required for Meta image upload".into(),
                })?;
            // The adimages edge only accepts bytes (Base64) or copy_from,
            // so a URL import downloads the asset first.
            let image_bytes = self.client.fetch_bytes(url).await?;
            use base64::Engine as _;
            let encoded = base64::engine::general_purpose::STANDARD.encode(&image_bytes);
            let mut body = serde_json::json!({ "bytes": encoded });
            if let Some(name) = &input.name {
                body["name"] = serde_json::Value::String(name.clone());
            }
            let resp = self.client.post(&path, &body).await?;

            // Meta returns images keyed by hash.
            let hash = resp["images"]
                .as_object()
                .and_then(|m| m.values().next())
                .and_then(|v| v["hash"].as_str())
                .ok_or_else(|| MktError::ApiError {
                    provider: "meta".into(),
                    status: 0,
                    message: "image upload response missing hash".into(),
                    retry_after: None,
                })?;

            Ok(MediaAsset {
                id: MediaAssetId(hash.to_string()),
                provider: "meta".to_string(),
                media_type: MediaType::Image,
                url: Some(url.to_string()),
                filename: input.name.clone(),
                size_bytes: None,
                created_at: chrono::Utc::now(),
                raw: Some(resp),
            })
        }
    }

    #[instrument(skip(self, input))]
    fn upload_video(
        &self,
        input: &UploadVideoInput,
    ) -> impl std::future::Future<Output = Result<MediaAsset>> + Send {
        async move {
            let path = format!("{}/advideos", self.act_path());
            let url = input
                .url
                .as_deref()
                .ok_or_else(|| MktError::ValidationError {
                    field: "url".into(),
                    message: "video URL is required for Meta video upload".into(),
                })?;
            let mut body = serde_json::json!({ "file_url": url });
            if let Some(title) = &input.title {
                body["title"] = serde_json::Value::String(title.clone());
            }
            if let Some(desc) = &input.description {
                body["description"] = serde_json::Value::String(desc.clone());
            }
            let resp = self.client.post(&path, &body).await?;
            let video_id = resp["id"].as_str().ok_or_else(|| MktError::ApiError {
                provider: "meta".into(),
                status: 0,
                message: "video upload response missing 'id'".into(),
                retry_after: None,
            })?;

            Ok(MediaAsset {
                id: MediaAssetId(video_id.to_string()),
                provider: "meta".to_string(),
                media_type: MediaType::Video,
                url: Some(url.to_string()),
                filename: input.name.clone(),
                size_bytes: None,
                created_at: chrono::Utc::now(),
                raw: Some(resp),
            })
        }
    }

    // ── Raw escape hatch ───────────────────────────────

    fn raw_request(
        &self,
        method: HttpMethod,
        path: &str,
        params: &serde_json::Value,
    ) -> impl std::future::Future<Output = Result<serde_json::Value>> + Send {
        // We need owned copies so the future is `Send + 'static`-compatible.
        let path = path.to_string();
        let params = params.clone();

        async move {
            match method {
                HttpMethod::Get => {
                    let query_pairs: Vec<(String, String)> = params
                        .as_object()
                        .map(|m| {
                            m.iter()
                                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                                .collect()
                        })
                        .unwrap_or_default();

                    let refs: Vec<(&str, &str)> = query_pairs
                        .iter()
                        .map(|(k, v)| (k.as_str(), v.as_str()))
                        .collect();

                    self.client.get(&path, &refs).await
                }
                HttpMethod::Post => self.client.post(&path, &params).await,
                HttpMethod::Delete => self.client.delete(&path).await,
            }
        }
    }

    // ── Health check ───────────────────────────────────

    async fn health_check(&self) -> Result<ProviderHealth> {
        let start = std::time::Instant::now();
        let result = self.client.get("me", &[]).await;
        #[allow(clippy::cast_possible_truncation)] // health-check latency fits in u64
        let latency = start.elapsed().as_millis() as u64;

        match result {
            Ok(_) => Ok(ProviderHealth {
                provider: "meta".into(),
                healthy: true,
                latency_ms: latency,
                message: None,
            }),
            Err(e) => Ok(ProviderHealth {
                provider: "meta".into(),
                healthy: false,
                latency_ms: latency,
                message: Some(e.to_string()),
            }),
        }
    }
}
