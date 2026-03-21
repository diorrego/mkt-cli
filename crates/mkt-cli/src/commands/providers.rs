//! `mkt providers` command handler.

/// List available providers and their capabilities.
pub fn execute() -> String {
    let mut lines = vec!["Available providers:".to_string()];

    #[cfg(feature = "meta")]
    lines.push("  meta       Meta (Facebook/Instagram)".into());

    #[cfg(feature = "google")]
    lines.push("  google     Google Ads".into());

    #[cfg(feature = "tiktok")]
    lines.push("  tiktok     TikTok for Business".into());

    #[cfg(feature = "linkedin")]
    lines.push("  linkedin   LinkedIn Marketing".into());

    if lines.len() == 1 {
        lines.push("  (none — rebuild with provider features enabled)".into());
    }

    lines.join("\n")
}
