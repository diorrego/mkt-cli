//! Output formatting for CLI results.
//!
//! Supports table (via `comfy-table`), JSON, and CSV output formats.
//! All formatters operate on types implementing [`Formattable`].

mod csv_fmt;
mod json;
mod table;

use serde::Serialize;

use crate::error::Result;

/// Output format selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    /// Tabular output for terminal display.
    #[default]
    Table,
    /// JSON output for machine consumption.
    Json,
    /// CSV output for spreadsheets.
    Csv,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Table => write!(f, "table"),
            Self::Json => write!(f, "json"),
            Self::Csv => write!(f, "csv"),
        }
    }
}

impl std::str::FromStr for OutputFormat {
    type Err = crate::error::MktError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "table" => Ok(Self::Table),
            "json" => Ok(Self::Json),
            "csv" => Ok(Self::Csv),
            other => Err(crate::error::MktError::ValidationError {
                field: "output".into(),
                message: format!("Unknown output format: '{other}'. Use table, json, or csv."),
            }),
        }
    }
}

/// Trait for types that can be rendered in table or CSV format.
pub trait Formattable {
    /// Column headers for table/CSV output.
    fn headers() -> Vec<String>;
    /// Row values corresponding to the headers.
    fn row(&self) -> Vec<String>;
}

/// Format a list of items in the specified output format.
pub fn format_output<T: Formattable + Serialize>(
    items: &[T],
    format: OutputFormat,
) -> Result<String> {
    match format {
        OutputFormat::Table => Ok(table::format_table::<T>(items)),
        OutputFormat::Json => json::format_json(items),
        OutputFormat::Csv => csv_fmt::format_csv::<T>(items),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_format_display() {
        assert_eq!(OutputFormat::Table.to_string(), "table");
        assert_eq!(OutputFormat::Json.to_string(), "json");
        assert_eq!(OutputFormat::Csv.to_string(), "csv");
    }

    #[test]
    fn output_format_from_str() {
        assert_eq!(
            "table".parse::<OutputFormat>().expect("ok"),
            OutputFormat::Table
        );
        assert_eq!(
            "JSON".parse::<OutputFormat>().expect("ok"),
            OutputFormat::Json
        );
        assert_eq!(
            "csv".parse::<OutputFormat>().expect("ok"),
            OutputFormat::Csv
        );
    }

    #[test]
    fn output_format_from_str_invalid() {
        assert!("xml".parse::<OutputFormat>().is_err());
    }
}
