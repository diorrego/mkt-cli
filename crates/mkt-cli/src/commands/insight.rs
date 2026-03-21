//! Insight command handlers.

use chrono::{DateTime, Utc};
use mkt_core::error::Result;
use mkt_core::models::{DateRange, InsightsQuery};
use mkt_core::output::{OutputFormat, format_output};
use mkt_core::provider::MarketingProvider;

use crate::cli::InsightAction;

/// Execute an insight action against the given provider.
pub async fn execute(
    action: &InsightAction,
    provider: &impl MarketingProvider,
    output_format: OutputFormat,
) -> Result<String> {
    match action {
        InsightAction::Get {
            metrics,
            breakdowns,
            range,
        } => {
            let date_range = range.as_deref().and_then(parse_date_range);
            let query = InsightsQuery {
                date_range,
                metrics: metrics.clone(),
                breakdowns: breakdowns.clone(),
                level: None,
                entity_ids: vec![],
                limit: None,
            };
            let report = provider.get_insights(&query).await?;
            format_output(&report.rows, output_format)
        }
    }
}

/// Parse a date range string.
///
/// Supported formats:
/// - `"7d"`, `"30d"` -- last N days
/// - `"2026-01-01:2026-03-01"` -- explicit start:end
fn parse_date_range(s: &str) -> Option<DateRange> {
    if let Some(days_str) = s.strip_suffix('d') {
        if let Ok(days) = days_str.parse::<i64>() {
            let end = Utc::now();
            let start = end - chrono::Duration::days(days);
            return Some(DateRange { start, end });
        }
    }

    if let Some((start_str, end_str)) = s.split_once(':') {
        if let (Ok(start), Ok(end)) = (
            start_str.parse::<DateTime<Utc>>(),
            end_str.parse::<DateTime<Utc>>(),
        ) {
            return Some(DateRange { start, end });
        }
        // Try date-only format.
        if let (Ok(start), Ok(end)) = (
            chrono::NaiveDate::parse_from_str(start_str, "%Y-%m-%d"),
            chrono::NaiveDate::parse_from_str(end_str, "%Y-%m-%d"),
        ) {
            let start_dt = start.and_hms_opt(0, 0, 0);
            let end_dt = end.and_hms_opt(23, 59, 59);
            if let (Some(s), Some(e)) = (start_dt, end_dt) {
                return Some(DateRange {
                    start: s.and_utc(),
                    end: e.and_utc(),
                });
            }
        }
    }

    None
}
