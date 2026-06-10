//! `OAuth2` refresh-token exchange for the Google Ads API.

use mkt_core::error::{MktError, Result};
use secrecy::SecretString;

/// Google's `OAuth2` token endpoint.
pub const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

/// Exchange an `OAuth2` refresh token for a short-lived access token.
///
/// `token_url` is parameterized for testing; production callers pass
/// [`GOOGLE_TOKEN_URL`].
///
/// # Errors
///
/// Returns [`MktError::AuthError`] if the token endpoint rejects the
/// exchange or the response lacks an `access_token` field, and
/// [`MktError::Http`] on transport failures.
pub async fn fetch_access_token(
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
    token_url: &str,
) -> Result<SecretString> {
    let http = mkt_core::http::build_http_client(None)?;
    let params = [
        ("grant_type", "refresh_token"),
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("refresh_token", refresh_token),
    ];

    let response = http.post(token_url).form(&params).send().await?;
    let status = response.status().as_u16();
    let body: serde_json::Value = response.json().await?;

    if !(200..300).contains(&status) {
        let detail = body["error_description"]
            .as_str()
            .or_else(|| body["error"].as_str())
            .unwrap_or("token endpoint returned an error");
        return Err(MktError::auth_error(
            "google",
            &format!("OAuth refresh failed ({status}): {detail}"),
        ));
    }

    body["access_token"].as_str().map_or_else(
        || {
            Err(MktError::auth_error(
                "google",
                "token endpoint response missing 'access_token'",
            ))
        },
        |token| Ok(SecretString::new(token.to_string().into())),
    )
}
