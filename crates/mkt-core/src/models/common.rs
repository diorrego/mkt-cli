//! Common types shared across all domain models.

use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A paginated response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paginated<T> {
    /// The items in this page.
    pub data: Vec<T>,
    /// Cursor for the next page, if any.
    pub next_cursor: Option<String>,
    /// Total number of items across all pages, if known.
    pub total: Option<u64>,
}

impl<T> Default for Paginated<T> {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            next_cursor: None,
            total: None,
        }
    }
}

/// A monetary budget.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    /// Amount in the smallest currency unit (e.g. cents).
    pub amount: f64,
    /// ISO 4217 currency code (e.g. "USD").
    pub currency: String,
    /// Whether this is a daily or lifetime budget.
    pub kind: BudgetKind,
}

/// The type of budget allocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetKind {
    /// Budget resets daily.
    Daily,
    /// Budget is for the entire campaign lifetime.
    Lifetime,
}

impl fmt::Display for BudgetKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Daily => write!(f, "daily"),
            Self::Lifetime => write!(f, "lifetime"),
        }
    }
}

/// A date range for filtering or reporting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    /// Start of the range (inclusive).
    pub start: DateTime<Utc>,
    /// End of the range (inclusive).
    pub end: DateTime<Utc>,
}

/// HTTP methods for the raw escape hatch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    /// HTTP GET.
    Get,
    /// HTTP POST.
    Post,
    /// HTTP DELETE.
    Delete,
}

impl fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Get => write!(f, "GET"),
            Self::Post => write!(f, "POST"),
            Self::Delete => write!(f, "DELETE"),
        }
    }
}

/// Health check result from a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderHealth {
    /// Provider name.
    pub provider: String,
    /// Whether the provider is reachable and authenticated.
    pub healthy: bool,
    /// Round-trip latency in milliseconds.
    pub latency_ms: u64,
    /// Optional status message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paginated_default_is_empty() {
        let page: Paginated<String> = Paginated::default();
        assert!(page.data.is_empty());
        assert!(page.next_cursor.is_none());
        assert!(page.total.is_none());
    }

    #[test]
    fn budget_kind_display() {
        assert_eq!(BudgetKind::Daily.to_string(), "daily");
        assert_eq!(BudgetKind::Lifetime.to_string(), "lifetime");
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn budget_kind_serde_roundtrip() {
        let json = serde_json::to_string(&BudgetKind::Daily).expect("serialize");
        assert_eq!(json, r#""daily""#);
        let back: BudgetKind = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, BudgetKind::Daily);
    }

    #[test]
    fn http_method_display() {
        assert_eq!(HttpMethod::Get.to_string(), "GET");
        assert_eq!(HttpMethod::Post.to_string(), "POST");
        assert_eq!(HttpMethod::Delete.to_string(), "DELETE");
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn http_method_serde_roundtrip() {
        let json = serde_json::to_string(&HttpMethod::Post).expect("serialize");
        assert_eq!(json, r#""POST""#);
        let back: HttpMethod = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, HttpMethod::Post);
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn provider_health_serializes_without_none_message() {
        let health = ProviderHealth {
            provider: "meta".into(),
            healthy: true,
            latency_ms: 42,
            message: None,
        };
        let json = serde_json::to_string(&health).expect("serialize");
        assert!(!json.contains("message"));
    }
}
