//! `OAuth2` refresh-token exchange for the LinkedIn Marketing API.
//!
//! Programmatic refresh tokens are only issued to applications approved
//! for the LinkedIn Marketing Developer Platform (MDP); other apps must
//! re-authorize through the OAuth consent flow when the access token
//! expires.

use mkt_core::error::{MktError, Result};
use secrecy::{ExposeSecret, SecretString};

/// LinkedIn's `OAuth2` token endpoint.
pub const LINKEDIN_TOKEN_URL: &str = "https://www.linkedin.com/oauth/v2/accessToken";

/// Exchange an `OAuth2` refresh token for a fresh access token.
///
/// `token_url` is parameterized for testing; production callers pass
/// [`LINKEDIN_TOKEN_URL`].
///
/// # Errors
///
/// Returns [`MktError::AuthError`] if the token endpoint rejects the
/// exchange or the response lacks an `access_token` field, and
/// [`MktError::Http`] on transport failures.
pub async fn refresh_access_token(
    client_id: &str,
    client_secret: &SecretString,
    refresh_token: &SecretString,
    token_url: &str,
) -> Result<SecretString> {
    let http = mkt_core::http::build_http_client(None)?;
    let params = [
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token.expose_secret()),
        ("client_id", client_id),
        ("client_secret", client_secret.expose_secret()),
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
            "linkedin",
            &format!(
                "OAuth refresh failed ({status}): {detail}. Programmatic refresh \
                 requires LinkedIn Marketing Developer Platform (MDP) approval; \
                 otherwise re-authorize via the OAuth consent flow to obtain a \
                 new access token."
            ),
        ));
    }

    body["access_token"].as_str().map_or_else(
        || {
            Err(MktError::auth_error(
                "linkedin",
                "token endpoint response missing 'access_token'",
            ))
        },
        |token| Ok(SecretString::new(token.to_string().into())),
    )
}
