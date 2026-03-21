//! Table output using `comfy-table`.

use comfy_table::{ContentArrangement, Table};

use super::Formattable;

/// Format items as a terminal-friendly table.
pub fn format_table<T: Formattable>(items: &[T]) -> String {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(T::headers());

    for item in items {
        table.add_row(item.row());
    }

    table.to_string()
}

#[cfg(test)]
mod tests {
    use serde::Serialize;

    use super::*;

    #[derive(Serialize)]
    struct TestItem {
        name: String,
        value: i32,
    }

    impl Formattable for TestItem {
        fn headers() -> Vec<String> {
            vec!["Name".into(), "Value".into()]
        }

        fn row(&self) -> Vec<String> {
            vec![self.name.clone(), self.value.to_string()]
        }
    }

    #[test]
    fn table_with_items() {
        let items = vec![
            TestItem {
                name: "Alice".into(),
                value: 42,
            },
            TestItem {
                name: "Bob".into(),
                value: 7,
            },
        ];
        let output = format_table(&items);
        assert!(output.contains("Alice"));
        assert!(output.contains("Bob"));
        assert!(output.contains("Name"));
        assert!(output.contains("Value"));
    }

    #[test]
    fn table_empty() {
        let items: Vec<TestItem> = vec![];
        let output = format_table(&items);
        assert!(output.contains("Name"));
    }
}
