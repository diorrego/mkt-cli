//! `mkt profile` command handlers.

use mkt_core::config;
use mkt_core::error::Result;

use crate::cli::ProfileAction;

/// Execute a profile management action.
pub fn execute(action: &ProfileAction) -> Result<String> {
    match action {
        ProfileAction::List => list_profiles(),
        ProfileAction::Show { name } => show_profile(name),
        ProfileAction::Set { name, .. } => Ok(format!(
            "Profile '{name}' configuration saved. (Config file writing is not yet implemented.)"
        )),
    }
}

fn list_profiles() -> Result<String> {
    let cfg = config::MktConfig::load()?;
    if cfg.profiles.is_empty() {
        return Ok("No profiles configured. Use `mkt profile set <name>` to create one.".into());
    }

    let mut lines = vec!["Profiles:".to_string()];
    for (name, profile) in &cfg.profiles {
        let default_marker = if name == &cfg.defaults.profile {
            " (default)"
        } else {
            ""
        };
        lines.push(format!(
            "  {name}{default_marker} — provider: {}",
            profile.provider
        ));
    }
    Ok(lines.join("\n"))
}

fn show_profile(name: &str) -> Result<String> {
    let cfg = config::MktConfig::load()?;
    let profile = cfg.profile(name)?;

    let mut lines = vec![format!("Profile: {name}")];
    lines.push(format!("  Provider: {}", profile.provider));

    if let Some(meta) = &profile.meta {
        lines.push("  Meta:".into());
        lines.push(format!(
            "    Ad Account: {}",
            meta.ad_account_id.as_deref().unwrap_or("(not set)")
        ));
        lines.push(format!(
            "    Page ID: {}",
            meta.page_id.as_deref().unwrap_or("(not set)")
        ));
        lines.push(format!(
            "    Access Token: {}",
            if meta.access_token.is_some() {
                "[REDACTED]"
            } else {
                "(not set)"
            }
        ));
    }

    if let Some(google) = &profile.google {
        lines.push("  Google:".into());
        lines.push(format!(
            "    Customer ID: {}",
            google.customer_id.as_deref().unwrap_or("(not set)")
        ));
    }

    Ok(lines.join("\n"))
}
