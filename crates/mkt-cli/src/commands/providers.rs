//! `mkt providers` command handler.

use mkt_core::output::OutputFormat;

/// The providers compiled into this binary.
#[allow(clippy::vec_init_then_push)] // pushes are feature-gated
fn available() -> Vec<(&'static str, &'static str)> {
    let mut providers = Vec::new();
    #[cfg(feature = "meta")]
    providers.push(("meta", "Meta (Facebook/Instagram)"));
    #[cfg(feature = "google")]
    providers.push(("google", "Google Ads"));
    #[cfg(feature = "tiktok")]
    providers.push(("tiktok", "TikTok for Business"));
    #[cfg(feature = "linkedin")]
    providers.push(("linkedin", "LinkedIn Marketing"));
    providers
}

/// List available providers, honoring the requested output format so
/// agents can parse the result.
pub fn execute(format: OutputFormat) -> String {
    let providers = available();

    if matches!(format, OutputFormat::Json) {
        let entries: Vec<serde_json::Value> = providers
            .iter()
            .map(|(name, display)| serde_json::json!({ "name": name, "display_name": display }))
            .collect();
        return serde_json::json!({ "ok": true, "providers": entries }).to_string();
    }

    let mut lines = vec!["Available providers:".to_string()];
    for (name, display) in &providers {
        lines.push(format!("  {name:<10} {display}"));
    }
    if lines.len() == 1 {
        lines.push("  (none — rebuild with provider features enabled)".into());
    }
    lines.join("\n")
}
