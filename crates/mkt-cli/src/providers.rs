//! Provider construction from config + environment.
//!
//! Shared by the CLI command handlers and the MCP server so both surfaces
//! resolve credentials identically (env vars take precedence over the
//! profile config; no interactive prompts anywhere).

use std::path::Path;

use mkt_core::config::MktConfig;
use mkt_core::error::MktError;

/// Load the config from an explicit path or the default location.
pub fn load_config(config_path: Option<&Path>) -> anyhow::Result<MktConfig> {
    Ok(if let Some(path) = config_path {
        MktConfig::load_from_file(path)?
    } else {
        MktConfig::load()?
    })
}

/// Build a Meta provider for the given profile.
#[cfg(feature = "meta")]
pub fn build_meta(
    config: &MktConfig,
    profile_name: &str,
) -> anyhow::Result<mkt_meta::MetaProvider> {
    use mkt_core::auth;
    use secrecy::ExposeSecret;

    let profile = config.profile(profile_name).ok();
    let meta_config = profile.and_then(|p| p.meta.as_ref());

    let token = auth::resolve_token(
        "meta",
        "MKT_META_ACCESS_TOKEN",
        meta_config.and_then(|c| c.access_token.as_deref()),
    )?;

    let ad_account_id = meta_config
        .and_then(|c| c.ad_account_id.clone())
        .or_else(|| std::env::var("MKT_META_AD_ACCOUNT_ID").ok())
        .unwrap_or_else(|| "act_unknown".to_string());

    let api_version = meta_config
        .and_then(|c| c.api_version.as_deref())
        .unwrap_or("v25.0");

    let client = mkt_meta::MetaClient::new(
        secrecy::SecretString::from(token.expose_secret().to_string()),
        ad_account_id,
        Some(api_version),
    )?;

    Ok(mkt_meta::MetaProvider::new(
        client,
        meta_config.and_then(|c| c.page_id.clone()),
        meta_config.and_then(|c| c.ig_user_id.clone()),
    ))
}

/// Build a Google Ads provider for the given profile.
///
/// Resolves the access token from `MKT_GOOGLE_ACCESS_TOKEN` or, failing
/// that, exchanges the configured `OAuth2` refresh token.
#[cfg(feature = "google")]
pub async fn build_google(
    config: &MktConfig,
    profile_name: &str,
) -> anyhow::Result<mkt_google::GoogleProvider> {
    let profile = config.profile(profile_name).ok();
    let google_config = profile.and_then(|p| p.google.as_ref());

    let developer_token = std::env::var("MKT_GOOGLE_DEVELOPER_TOKEN")
        .ok()
        .or_else(|| google_config.and_then(|c| c.developer_token.clone()))
        .ok_or_else(|| {
            MktError::auth_error(
                "google",
                "No developer token found. Set MKT_GOOGLE_DEVELOPER_TOKEN or configure it \
                 in your profile.",
            )
        })?;

    let customer_id = std::env::var("MKT_GOOGLE_CUSTOMER_ID")
        .ok()
        .or_else(|| google_config.and_then(|c| c.customer_id.clone()))
        .ok_or_else(|| {
            MktError::auth_error(
                "google",
                "No customer ID found. Set MKT_GOOGLE_CUSTOMER_ID or configure it in your \
                 profile.",
            )
        })?;

    let access_token = if let Ok(token) = std::env::var("MKT_GOOGLE_ACCESS_TOKEN") {
        secrecy::SecretString::from(token)
    } else {
        let (client_id, client_secret, refresh_token) = google_config
            .and_then(|c| {
                Some((
                    c.client_id.clone()?,
                    c.client_secret.clone()?,
                    c.refresh_token.clone()?,
                ))
            })
            .ok_or_else(|| {
                MktError::auth_error(
                    "google",
                    "No access token found. Set MKT_GOOGLE_ACCESS_TOKEN, or configure \
                     client_id, client_secret, and refresh_token in your profile.",
                )
            })?;
        mkt_google::fetch_access_token(
            &client_id,
            &client_secret,
            &refresh_token,
            mkt_google::GOOGLE_TOKEN_URL,
        )
        .await?
    };

    let client = mkt_google::GoogleClient::new(access_token, developer_token, &customer_id, None)?;
    Ok(mkt_google::GoogleProvider::new(client))
}

/// Build a TikTok provider for the given profile.
#[cfg(feature = "tiktok")]
pub fn build_tiktok(
    config: &MktConfig,
    profile_name: &str,
) -> anyhow::Result<mkt_tiktok::TikTokProvider> {
    use mkt_core::auth;
    use secrecy::ExposeSecret;

    let profile = config.profile(profile_name).ok();
    let tt_config = profile.and_then(|p| p.tiktok.as_ref());

    let token = auth::resolve_token(
        "tiktok",
        "MKT_TIKTOK_ACCESS_TOKEN",
        tt_config.and_then(|c| c.access_token.as_deref()),
    )?;

    let advertiser_id = std::env::var("MKT_TIKTOK_ADVERTISER_ID")
        .ok()
        .or_else(|| tt_config.and_then(|c| c.advertiser_id.clone()))
        .ok_or_else(|| {
            MktError::auth_error(
                "tiktok",
                "No advertiser ID found. Set MKT_TIKTOK_ADVERTISER_ID or configure it \
                 in your profile.",
            )
        })?;

    let client = mkt_tiktok::TikTokClient::new(
        secrecy::SecretString::from(token.expose_secret().to_string()),
        advertiser_id,
    )?;
    Ok(mkt_tiktok::TikTokProvider::new(client))
}

/// Build a LinkedIn provider for the given profile.
#[cfg(feature = "linkedin")]
pub fn build_linkedin(
    config: &MktConfig,
    profile_name: &str,
) -> anyhow::Result<mkt_linkedin::LinkedInProvider> {
    use mkt_core::auth;
    use secrecy::ExposeSecret;

    let profile = config.profile(profile_name).ok();
    let li_config = profile.and_then(|p| p.linkedin.as_ref());

    let token = auth::resolve_token(
        "linkedin",
        "MKT_LINKEDIN_ACCESS_TOKEN",
        li_config.and_then(|c| c.access_token.as_deref()),
    )?;

    let ad_account_id = std::env::var("MKT_LINKEDIN_AD_ACCOUNT_ID")
        .ok()
        .or_else(|| li_config.and_then(|c| c.ad_account_id.clone()))
        .ok_or_else(|| {
            MktError::auth_error(
                "linkedin",
                "No ad account ID found. Set MKT_LINKEDIN_AD_ACCOUNT_ID or configure it \
                 in your profile.",
            )
        })?;

    let client = mkt_linkedin::LinkedInClient::new(
        secrecy::SecretString::from(token.expose_secret().to_string()),
        ad_account_id,
    )?;
    Ok(mkt_linkedin::LinkedInProvider::new(client))
}
