//! Custom assertion helpers for `mkt` domain types.
//!
//! These helpers produce clear, actionable failure messages that name the
//! specific field or invariant that was violated. They are thin wrappers
//! around the standard [`assert!`] and [`assert_eq!`] macros, enriched with
//! domain-aware context.
//!
//! All helpers follow the same convention: they take the subject by reference
//! and borrow any ancillary data they need. A descriptive panic message is
//! produced on failure so that test output points directly at the broken
//! invariant without requiring a debugger.

use mkt_core::error::MktError;
use mkt_core::models::{Campaign, CampaignStatus, Paginated};

// ── Campaign assertions ─────────────────────────────────────────────────────

/// Assert that a campaign has the given status.
///
/// # Panics
///
/// Panics with a descriptive message when `campaign.status != expected`.
#[allow(clippy::panic)]
pub fn assert_campaign_status(campaign: &Campaign, expected: &CampaignStatus) {
    assert_eq!(
        &campaign.status, expected,
        "campaign '{}' (id={}) has status {:?}, expected {:?}",
        campaign.name, campaign.id, campaign.status, expected,
    );
}

/// Assert that a campaign name matches exactly.
///
/// # Panics
///
/// Panics when the name does not match.
#[allow(clippy::panic)]
pub fn assert_campaign_name(campaign: &Campaign, expected: &str) {
    assert_eq!(
        campaign.name.as_str(),
        expected,
        "campaign id={} has name {:?}, expected {:?}",
        campaign.id,
        campaign.name,
        expected,
    );
}

/// Assert that a campaign belongs to the given provider.
///
/// # Panics
///
/// Panics when the provider field does not match.
#[allow(clippy::panic)]
pub fn assert_campaign_provider(campaign: &Campaign, expected_provider: &str) {
    assert_eq!(
        campaign.provider.as_str(),
        expected_provider,
        "campaign '{}' belongs to provider {:?}, expected {:?}",
        campaign.name,
        campaign.provider,
        expected_provider,
    );
}

/// Assert that a campaign is active.
///
/// Convenience wrapper around [`assert_campaign_status`].
///
/// # Panics
///
/// Panics when the status is not [`CampaignStatus::Active`].
pub fn assert_campaign_active(campaign: &Campaign) {
    assert_campaign_status(campaign, &CampaignStatus::Active);
}

/// Assert that a campaign is paused.
///
/// Convenience wrapper around [`assert_campaign_status`].
///
/// # Panics
///
/// Panics when the status is not [`CampaignStatus::Paused`].
pub fn assert_campaign_paused(campaign: &Campaign) {
    assert_campaign_status(campaign, &CampaignStatus::Paused);
}

// ── Paginated assertions ────────────────────────────────────────────────────

/// Assert that a paginated result contains exactly `n` items.
///
/// # Panics
///
/// Panics when `page.data.len() != expected`.
#[allow(clippy::panic)]
pub fn assert_page_len<T>(page: &Paginated<T>, expected: usize) {
    assert_eq!(
        page.data.len(),
        expected,
        "paginated result has {} items, expected {}",
        page.data.len(),
        expected,
    );
}

/// Assert that a paginated result has no next-page cursor.
///
/// # Panics
///
/// Panics when `page.next_cursor` is `Some`.
#[allow(clippy::panic)]
pub fn assert_page_is_last<T>(page: &Paginated<T>) {
    assert!(
        page.next_cursor.is_none(),
        "expected last page but next_cursor is Some({:?})",
        page.next_cursor,
    );
}

/// Assert that a paginated result has a next-page cursor.
///
/// # Panics
///
/// Panics when `page.next_cursor` is `None`.
#[allow(clippy::panic)]
pub fn assert_page_has_next<T>(page: &Paginated<T>) {
    assert!(
        page.next_cursor.is_some(),
        "expected more pages but next_cursor is None",
    );
}

// ── Error assertions ────────────────────────────────────────────────────────

/// Assert that a `Result` is an `Err` carrying [`MktError::ApiError`] with
/// the expected HTTP status code.
///
/// # Panics
///
/// Panics when:
/// - `result` is `Ok`.
/// - The error variant is not [`MktError::ApiError`].
/// - The status code does not match.
#[allow(clippy::panic)]
pub fn assert_api_error_status<T: std::fmt::Debug>(
    result: &Result<T, MktError>,
    expected_status: u16,
) {
    match result {
        Ok(v) => {
            panic!("expected Err(ApiError {{ status: {expected_status} }}) but got Ok({v:?})");
        }
        Err(MktError::ApiError { status, .. }) => {
            assert_eq!(
                *status, expected_status,
                "ApiError has status {status}, expected {expected_status}",
            );
        }
        Err(other) => {
            panic!("expected ApiError with status {expected_status} but got {other:?}",);
        }
    }
}

/// Assert that a `Result` is an `Err` carrying [`MktError::NotSupported`].
///
/// # Panics
///
/// Panics when the result is `Ok` or when the error variant is not
/// [`MktError::NotSupported`].
#[allow(clippy::panic)]
pub fn assert_not_supported<T: std::fmt::Debug>(result: &Result<T, MktError>) {
    match result {
        Ok(v) => panic!("expected Err(NotSupported) but got Ok({v:?})"),
        Err(MktError::NotSupported { .. }) => {}
        Err(other) => panic!("expected NotSupported but got {other:?}"),
    }
}

/// Assert that a `Result` is an `Err` carrying [`MktError::AuthError`].
///
/// # Panics
///
/// Panics when the result is `Ok` or when the error variant is not
/// [`MktError::AuthError`].
#[allow(clippy::panic)]
pub fn assert_auth_error<T: std::fmt::Debug>(result: &Result<T, MktError>) {
    match result {
        Ok(v) => panic!("expected Err(AuthError) but got Ok({v:?})"),
        Err(MktError::AuthError { .. }) => {}
        Err(other) => panic!("expected AuthError but got {other:?}"),
    }
}

/// Assert that an [`MktError`] message contains the given substring.
///
/// # Panics
///
/// Panics when `error.to_string()` does not contain `substr`.
#[allow(clippy::panic)]
pub fn assert_error_message_contains(error: &MktError, substr: &str) {
    let msg = error.to_string();
    assert!(
        msg.contains(substr),
        "error message {msg:?} does not contain {substr:?}",
    );
}

// ── JSON assertions ─────────────────────────────────────────────────────────

/// Assert that a JSON value is an object with the given key.
///
/// # Panics
///
/// Panics when the value is not an object or does not have the key.
#[allow(clippy::panic)]
pub fn assert_json_has_key(value: &serde_json::Value, key: &str) {
    let obj = value
        .as_object()
        .unwrap_or_else(|| panic!("expected JSON object but got {value:?}"));
    assert!(
        obj.contains_key(key),
        "JSON object does not have key {key:?}; available keys: {:?}",
        obj.keys().collect::<Vec<_>>(),
    );
}

/// Assert that a JSON array has exactly `expected` elements.
///
/// # Panics
///
/// Panics when the value is not an array or when the length does not match.
#[allow(clippy::panic)]
pub fn assert_json_array_len(value: &serde_json::Value, expected: usize) {
    let arr = value
        .as_array()
        .unwrap_or_else(|| panic!("expected JSON array but got {value:?}"));
    assert_eq!(
        arr.len(),
        expected,
        "JSON array has {} elements, expected {}",
        arr.len(),
        expected,
    );
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use chrono::Utc;
    use mkt_core::models::{CampaignId, CampaignStatus};

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

    // ── assert_campaign_status ────────────────────────────────────────

    #[test]
    fn test_assert_campaign_status_passes_on_match() {
        let c = make_campaign("1", "Alpha", CampaignStatus::Active);
        assert_campaign_status(&c, &CampaignStatus::Active);
    }

    #[test]
    #[should_panic(expected = "has status")]
    fn test_assert_campaign_status_panics_on_mismatch() {
        let c = make_campaign("1", "Alpha", CampaignStatus::Paused);
        assert_campaign_status(&c, &CampaignStatus::Active);
    }

    // ── assert_campaign_name ──────────────────────────────────────────

    #[test]
    fn test_assert_campaign_name_passes_on_match() {
        let c = make_campaign("1", "Summer Sale", CampaignStatus::Active);
        assert_campaign_name(&c, "Summer Sale");
    }

    #[test]
    #[should_panic(expected = "has name")]
    fn test_assert_campaign_name_panics_on_mismatch() {
        let c = make_campaign("1", "Summer Sale", CampaignStatus::Active);
        assert_campaign_name(&c, "Winter Sale");
    }

    // ── assert_campaign_provider ──────────────────────────────────────

    #[test]
    fn test_assert_campaign_provider_passes_on_match() {
        let c = make_campaign("1", "Alpha", CampaignStatus::Active);
        assert_campaign_provider(&c, "mock");
    }

    #[test]
    #[should_panic(expected = "belongs to provider")]
    fn test_assert_campaign_provider_panics_on_mismatch() {
        let c = make_campaign("1", "Alpha", CampaignStatus::Active);
        assert_campaign_provider(&c, "meta");
    }

    // ── assert_campaign_active / paused ───────────────────────────────

    #[test]
    fn test_assert_campaign_active_passes() {
        let c = make_campaign("1", "Alpha", CampaignStatus::Active);
        assert_campaign_active(&c);
    }

    #[test]
    #[should_panic(expected = "has status")]
    fn test_assert_campaign_active_panics_when_paused() {
        let c = make_campaign("1", "Alpha", CampaignStatus::Paused);
        assert_campaign_active(&c);
    }

    #[test]
    fn test_assert_campaign_paused_passes() {
        let c = make_campaign("1", "Beta", CampaignStatus::Paused);
        assert_campaign_paused(&c);
    }

    #[test]
    #[should_panic(expected = "has status")]
    fn test_assert_campaign_paused_panics_when_active() {
        let c = make_campaign("1", "Beta", CampaignStatus::Active);
        assert_campaign_paused(&c);
    }

    // ── assert_page_len ───────────────────────────────────────────────

    #[test]
    fn test_assert_page_len_passes_on_match() {
        let page: Paginated<Campaign> = Paginated {
            data: vec![make_campaign("1", "A", CampaignStatus::Active)],
            next_cursor: None,
            total: None,
        };
        assert_page_len(&page, 1);
    }

    #[test]
    #[should_panic(expected = "paginated result has")]
    fn test_assert_page_len_panics_on_mismatch() {
        let page: Paginated<Campaign> = Paginated {
            data: vec![make_campaign("1", "A", CampaignStatus::Active)],
            next_cursor: None,
            total: None,
        };
        assert_page_len(&page, 2);
    }

    // ── assert_page_is_last ───────────────────────────────────────────

    #[test]
    fn test_assert_page_is_last_passes_when_no_cursor() {
        let page: Paginated<Campaign> = Paginated {
            data: vec![],
            next_cursor: None,
            total: None,
        };
        assert_page_is_last(&page);
    }

    #[test]
    #[should_panic(expected = "expected last page")]
    fn test_assert_page_is_last_panics_when_cursor_present() {
        let page: Paginated<Campaign> = Paginated {
            data: vec![],
            next_cursor: Some("cursor123".to_string()),
            total: None,
        };
        assert_page_is_last(&page);
    }

    // ── assert_page_has_next ──────────────────────────────────────────

    #[test]
    fn test_assert_page_has_next_passes_when_cursor_present() {
        let page: Paginated<Campaign> = Paginated {
            data: vec![],
            next_cursor: Some("cursor456".to_string()),
            total: None,
        };
        assert_page_has_next(&page);
    }

    #[test]
    #[should_panic(expected = "expected more pages")]
    fn test_assert_page_has_next_panics_when_no_cursor() {
        let page: Paginated<Campaign> = Paginated {
            data: vec![],
            next_cursor: None,
            total: None,
        };
        assert_page_has_next(&page);
    }

    // ── assert_api_error_status ───────────────────────────────────────

    #[test]
    fn test_assert_api_error_status_passes_on_match() {
        let result: Result<Campaign, MktError> = Err(MktError::ApiError {
            provider: "mock".to_string(),
            status: 404,
            message: "not found".to_string(),
            retry_after: None,
        });
        assert_api_error_status(&result, 404);
    }

    #[test]
    #[should_panic(expected = "ApiError has status")]
    fn test_assert_api_error_status_panics_on_wrong_status() {
        let result: Result<Campaign, MktError> = Err(MktError::ApiError {
            provider: "mock".to_string(),
            status: 500,
            message: "server error".to_string(),
            retry_after: None,
        });
        assert_api_error_status(&result, 404);
    }

    #[test]
    #[should_panic(expected = "expected Err(ApiError")]
    fn test_assert_api_error_status_panics_on_ok() {
        let c = make_campaign("1", "Alpha", CampaignStatus::Active);
        let result: Result<Campaign, MktError> = Ok(c);
        assert_api_error_status(&result, 404);
    }

    // ── assert_not_supported ──────────────────────────────────────────

    #[test]
    fn test_assert_not_supported_passes_on_not_supported() {
        let result: Result<Campaign, MktError> = Err(MktError::not_supported("mock", "feature"));
        assert_not_supported(&result);
    }

    #[test]
    #[should_panic(expected = "expected NotSupported")]
    fn test_assert_not_supported_panics_on_other_error() {
        let result: Result<Campaign, MktError> = Err(MktError::ConfigError("bad".to_string()));
        assert_not_supported(&result);
    }

    // ── assert_auth_error ─────────────────────────────────────────────

    #[test]
    fn test_assert_auth_error_passes_on_auth_error() {
        let result: Result<Campaign, MktError> = Err(MktError::auth_error("meta", "token expired"));
        assert_auth_error(&result);
    }

    #[test]
    #[should_panic(expected = "expected AuthError")]
    fn test_assert_auth_error_panics_on_other_error() {
        let result: Result<Campaign, MktError> = Err(MktError::not_supported("mock", "feature"));
        assert_auth_error(&result);
    }

    // ── assert_error_message_contains ────────────────────────────────

    #[test]
    fn test_assert_error_message_contains_passes_on_match() {
        let err = MktError::ConfigError("missing profile section".to_string());
        assert_error_message_contains(&err, "missing profile");
    }

    #[test]
    #[should_panic(expected = "does not contain")]
    fn test_assert_error_message_contains_panics_on_mismatch() {
        let err = MktError::ConfigError("unrelated error".to_string());
        assert_error_message_contains(&err, "missing profile");
    }

    // ── assert_json_has_key ───────────────────────────────────────────

    #[test]
    fn test_assert_json_has_key_passes_when_key_present() {
        let v = serde_json::json!({ "id": "123", "name": "test" });
        assert_json_has_key(&v, "id");
        assert_json_has_key(&v, "name");
    }

    #[test]
    #[should_panic(expected = "does not have key")]
    fn test_assert_json_has_key_panics_when_key_absent() {
        let v = serde_json::json!({ "id": "123" });
        assert_json_has_key(&v, "missing_key");
    }

    #[test]
    #[should_panic(expected = "expected JSON object")]
    fn test_assert_json_has_key_panics_on_non_object() {
        let v = serde_json::json!([1, 2, 3]);
        assert_json_has_key(&v, "key");
    }

    // ── assert_json_array_len ─────────────────────────────────────────

    #[test]
    fn test_assert_json_array_len_passes_on_match() {
        let v = serde_json::json!([1, 2, 3]);
        assert_json_array_len(&v, 3);
    }

    #[test]
    #[should_panic(expected = "JSON array has")]
    fn test_assert_json_array_len_panics_on_wrong_count() {
        let v = serde_json::json!([1, 2, 3]);
        assert_json_array_len(&v, 5);
    }

    #[test]
    #[should_panic(expected = "expected JSON array")]
    fn test_assert_json_array_len_panics_on_non_array() {
        let v = serde_json::json!({ "key": "value" });
        assert_json_array_len(&v, 1);
    }
}
