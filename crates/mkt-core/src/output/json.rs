//! JSON output formatter.

use serde::Serialize;

use crate::error::Result;

/// Format items as pretty-printed JSON.
pub fn format_json<T: Serialize>(items: &[T]) -> Result<String> {
    let json = serde_json::to_string_pretty(items)?;
    Ok(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct TestItem {
        name: String,
    }

    #[test]
    fn json_output_is_valid() {
        let items = vec![TestItem {
            name: "test".into(),
        }];
        let output = format_json(&items).expect("format");
        let parsed: serde_json::Value = serde_json::from_str(&output).expect("re-parse");
        assert!(parsed.is_array());
    }

    #[test]
    fn json_output_empty_array() {
        let items: Vec<TestItem> = vec![];
        let output = format_json(&items).expect("format");
        assert_eq!(output, "[]");
    }
}
