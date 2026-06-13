//! `mkt profile` command handlers.

use mkt_core::config;
use mkt_core::error::{MktError, Result};

use crate::cli::ProfileAction;

/// Execute a profile management action.
pub fn execute(action: &ProfileAction) -> Result<String> {
    match action {
        ProfileAction::List => list_profiles(),
        ProfileAction::Show { name } => show_profile(name),
        ProfileAction::Set {
            name,
            provider,
            access_token,
            ad_account,
            page_id,
            ig_user_id,
        } => set_profile(
            name,
            provider.as_ref(),
            access_token.as_ref(),
            ad_account.as_ref(),
            page_id.as_ref(),
            ig_user_id.as_ref(),
        ),
    }
}

fn set_profile(
    name: &str,
    provider: Option<&String>,
    access_token: Option<&String>,
    ad_account: Option<&String>,
    page_id: Option<&String>,
    ig_user_id: Option<&String>,
) -> Result<String> {
    let mut cfg = config::MktConfig::load()?;

    // Get or create the profile entry.
    let profile = cfg
        .profiles
        .entry(name.to_string())
        .or_insert_with(|| config::Profile {
            provider: provider.cloned().unwrap_or_default(),
            meta: None,
            google: None,
            tiktok: None,
            linkedin: None,
        });

    // Update provider if specified (and profile already existed).
    if let Some(p) = provider {
        profile.provider.clone_from(p);
    }

    // Update meta-specific fields if any are provided.
    if access_token.is_some() || ad_account.is_some() || page_id.is_some() || ig_user_id.is_some() {
        let meta = profile.meta.get_or_insert(config::MetaConfig {
            access_token: None,
            ad_account_id: None,
            page_id: None,
            ig_user_id: None,
            api_version: None,
        });

        if let Some(tok) = access_token {
            meta.access_token = Some(tok.clone());
        }
        if let Some(acc) = ad_account {
            meta.ad_account_id = Some(acc.clone());
        }
        if let Some(pid) = page_id {
            meta.page_id = Some(pid.clone());
        }
        if let Some(ig) = ig_user_id {
            meta.ig_user_id = Some(ig.clone());
        }
    }

    // Serialize the config to TOML.
    let toml_content = toml::to_string_pretty(&cfg)
        .map_err(|e| MktError::ConfigError(format!("Failed to serialize config: {e}")))?;

    // Ensure the config directory exists.
    let config_dir = config::config_dir()?;
    std::fs::create_dir_all(&config_dir)?;

    // Owner-only from the first byte: chmod-after-write would leave a
    // window where the umask decides who can read the tokens.
    let config_path = config::config_file()?;
    config::write_private(&config_path, &toml_content)?;

    Ok(format!(
        "Profile '{name}' saved to {}.",
        config_path.display()
    ))
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
