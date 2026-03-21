//! Insights and reporting domain model.

use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use super::common::DateRange;

/// Query parameters for fetching insights.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InsightsQuery {
    /// Optional date range to constrain the report.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_range: Option<DateRange>,
    /// List of metric names to retrieve (e.g. "impressions", "clicks").
    pub metrics: Vec<String>,
    /// Breakdown dimensions (e.g. "age", "gender").
    pub breakdowns: Vec<String>,
    /// The entity level at which to aggregate data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<InsightsLevel>,
    /// IDs of entities to include in the report.
    pub entity_ids: Vec<String>,
    /// Maximum number of rows to return.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// The aggregation level for an insights report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InsightsLevel {
    /// Aggregated at the account level.
    Account,
    /// Aggregated at the campaign level.
    Campaign,
    /// Aggregated at the ad set level.
    AdSet,
    /// Aggregated at the individual ad level.
    Ad,
}

impl fmt::Display for InsightsLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Account => write!(f, "account"),
            Self::Campaign => write!(f, "campaign"),
            Self::AdSet => write!(f, "ad_set"),
            Self::Ad => write!(f, "ad"),
        }
    }
}

/// A complete insights report returned by a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightsReport {
    /// Which provider generated this report.
    pub provider: String,
    /// The date range the report covers, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_range: Option<DateRange>,
    /// Individual data rows.
    pub rows: Vec<InsightsRow>,
    /// Original API response for debugging and raw access.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<serde_json::Value>,
}

/// A single row in an insights report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightsRow {
    /// Dimension values keyed by dimension name (e.g. `{"age": "25-34"}`).
    pub dimensions: HashMap<String, String>,
    /// Metric values keyed by metric name (e.g. `{"clicks": 42}`).
    pub metrics: HashMap<String, MetricValue>,
}

/// A single metric value, optionally with a pre-formatted string.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricValue {
    /// Numeric metric value.
    pub value: f64,
    /// Human-readable formatted string (e.g. "$1,234.56").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formatted: Option<String>,
}

impl crate::output::Formattable for InsightsRow {
    fn headers() -> Vec<String> {
        vec!["Dimensions".into(), "Metrics".into()]
    }

    fn row(&self) -> Vec<String> {
        let dims: Vec<String> = self
            .dimensions
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect();
        let mets: Vec<String> = self
            .metrics
            .iter()
            .map(|(k, v)| {
                v.formatted
                    .as_deref()
                    .map_or_else(|| format!("{k}={}", v.value), |f| format!("{k}={f}"))
            })
            .collect();
        vec![dims.join(", "), mets.join(", ")]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case(InsightsLevel::Account, "account" ; "account")]
    #[test_case(InsightsLevel::Campaign, "campaign" ; "campaign")]
    #[test_case(InsightsLevel::AdSet, "ad_set" ; "adset")]
    #[test_case(InsightsLevel::Ad, "ad" ; "ad")]
    #[allow(clippy::needless_pass_by_value)]
    fn insights_level_display(level: InsightsLevel, expected: &str) {
        assert_eq!(level.to_string(), expected);
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn insights_level_serde_roundtrip() {
        let json = serde_json::to_string(&InsightsLevel::Campaign).expect("serialize");
        assert_eq!(json, r#""campaign""#);
        let back: InsightsLevel = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, InsightsLevel::Campaign);
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn insights_level_adset_serde_roundtrip() {
        let json = serde_json::to_string(&InsightsLevel::AdSet).expect("serialize");
        assert_eq!(json, r#""ad_set""#);
        let back: InsightsLevel = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, InsightsLevel::AdSet);
    }

    #[test]
    fn insights_query_default() {
        let query = InsightsQuery::default();
        assert!(query.date_range.is_none());
        assert!(query.metrics.is_empty());
        assert!(query.breakdowns.is_empty());
        assert!(query.level.is_none());
        assert!(query.entity_ids.is_empty());
        assert!(query.limit.is_none());
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn insights_query_serde_roundtrip() {
        let query = InsightsQuery {
            date_range: None,
            metrics: vec!["impressions".into(), "clicks".into()],
            breakdowns: vec!["age".into()],
            level: Some(InsightsLevel::Campaign),
            entity_ids: vec!["camp_1".into()],
            limit: Some(100),
        };
        let json = serde_json::to_string(&query).expect("serialize InsightsQuery");
        let back: InsightsQuery = serde_json::from_str(&json).expect("deserialize InsightsQuery");
        assert_eq!(back.metrics, vec!["impressions", "clicks"]);
        assert_eq!(back.breakdowns, vec!["age"]);
        assert_eq!(back.level, Some(InsightsLevel::Campaign));
        assert_eq!(back.limit, Some(100));
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn metric_value_serde_roundtrip() {
        let mv = MetricValue {
            value: 1234.56,
            formatted: Some("$1,234.56".into()),
        };
        let json = serde_json::to_string(&mv).expect("serialize MetricValue");
        let back: MetricValue = serde_json::from_str(&json).expect("deserialize MetricValue");
        assert!((back.value - 1234.56).abs() < f64::EPSILON);
        assert_eq!(back.formatted.as_deref(), Some("$1,234.56"));
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn metric_value_skips_none_formatted() {
        let mv = MetricValue {
            value: 42.0,
            formatted: None,
        };
        let json = serde_json::to_string(&mv).expect("serialize MetricValue");
        assert!(!json.contains("formatted"));
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn insights_row_serde_roundtrip() {
        let mut dimensions = HashMap::new();
        dimensions.insert("age".into(), "25-34".into());
        let mut metrics = HashMap::new();
        metrics.insert(
            "clicks".into(),
            MetricValue {
                value: 42.0,
                formatted: None,
            },
        );
        let row = InsightsRow {
            dimensions,
            metrics,
        };
        let json = serde_json::to_string(&row).expect("serialize InsightsRow");
        let back: InsightsRow = serde_json::from_str(&json).expect("deserialize InsightsRow");
        assert_eq!(
            back.dimensions.get("age").map(String::as_str),
            Some("25-34")
        );
        assert!((back.metrics["clicks"].value - 42.0).abs() < f64::EPSILON);
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn insights_report_skips_none_fields() {
        let report = InsightsReport {
            provider: "meta".into(),
            date_range: None,
            rows: vec![],
            raw: None,
        };
        let json = serde_json::to_string(&report).expect("serialize InsightsReport");
        assert!(!json.contains("date_range"));
        assert!(!json.contains("raw"));
    }
}
