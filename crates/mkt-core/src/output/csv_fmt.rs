//! CSV output formatter.

use crate::error::Result;

use super::Formattable;

/// Format items as CSV with headers.
pub fn format_csv<T: Formattable>(items: &[T]) -> Result<String> {
    let mut wtr = csv::Writer::from_writer(Vec::new());

    wtr.write_record(T::headers())?;
    for item in items {
        wtr.write_record(item.row())?;
    }

    let bytes = wtr
        .into_inner()
        .map_err(|e| crate::error::MktError::ConfigError(format!("CSV write error: {e}")))?;
    String::from_utf8(bytes)
        .map_err(|e| crate::error::MktError::ConfigError(format!("CSV encoding error: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestItem {
        name: String,
        value: String,
    }

    impl Formattable for TestItem {
        fn headers() -> Vec<String> {
            vec!["Name".into(), "Value".into()]
        }

        fn row(&self) -> Vec<String> {
            vec![self.name.clone(), self.value.clone()]
        }
    }

    #[test]
    fn csv_with_items() {
        let items = vec![
            TestItem {
                name: "Alice".into(),
                value: "42".into(),
            },
            TestItem {
                name: "Bob".into(),
                value: "7".into(),
            },
        ];
        let output = format_csv(&items).expect("format");
        assert!(output.contains("Name,Value"));
        assert!(output.contains("Alice,42"));
        assert!(output.contains("Bob,7"));
    }

    #[test]
    fn csv_escapes_commas() {
        let items = vec![TestItem {
            name: "Hello, World".into(),
            value: "test".into(),
        }];
        let output = format_csv(&items).expect("format");
        assert!(output.contains("\"Hello, World\""));
    }

    #[test]
    fn csv_empty() {
        let items: Vec<TestItem> = vec![];
        let output = format_csv(&items).expect("format");
        assert!(output.starts_with("Name,Value"));
    }
}
