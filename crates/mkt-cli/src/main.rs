//! Entry point for the `mkt` multi-platform marketing CLI.

mod cli;
mod commands;

use clap::Parser;

use cli::{Cli, Commands};

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();

    // Initialize tracing
    let filter = if cli.verbose {
        "debug"
    } else if cli.quiet {
        "error"
    } else {
        "info"
    };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    // Build tokio runtime
    let rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("error: failed to start async runtime: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };

    let emit_json_errors = matches!(cli.output, cli::OutputFormatArg::Json);
    match rt.block_on(run(cli)) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => report_error(&e, emit_json_errors),
    }
}

/// Print an error to stderr (structured JSON when `--output json`) and
/// translate it into the documented exit code contract.
fn report_error(error: &anyhow::Error, as_json: bool) -> std::process::ExitCode {
    use mkt_core::error::MktError;

    let mkt_error = error.downcast_ref::<MktError>();
    let exit_code = mkt_error.map_or(1, MktError::exit_code);

    if as_json {
        let mut err_obj = serde_json::json!({
            "type": mkt_error.map_or("unexpected_error", |e| e.error_type()),
            "message": error.to_string(),
        });
        if let Some(e) = mkt_error {
            if let Some(suggestion) = e.suggestion() {
                err_obj["suggestion"] = serde_json::Value::String(suggestion);
            }
            if e.is_transient() {
                err_obj["transient"] = serde_json::Value::Bool(true);
            }
        }
        let envelope = serde_json::json!({ "ok": false, "error": err_obj });
        eprintln!("{envelope}");
    } else {
        eprintln!("error: {error}");
        if let Some(suggestion) = mkt_error.and_then(MktError::suggestion) {
            eprintln!("hint: {suggestion}");
        }
    }

    std::process::ExitCode::from(exit_code)
}

async fn run(cli: Cli) -> anyhow::Result<()> {
    let output_format = mkt_core::output::OutputFormat::from(cli.output);

    let result = match &cli.command {
        Commands::Providers => Ok(commands::providers::execute()),

        Commands::Doctor => {
            commands::doctor::execute(cli.config.as_deref()).map_err(anyhow::Error::from)
        }

        Commands::Profile { action } => {
            commands::profile::execute(action).map_err(anyhow::Error::from)
        }

        Commands::Completions { shell } => {
            use clap::CommandFactory;
            let mut cmd = Cli::command();
            let mut buf = Vec::new();
            clap_complete::generate(*shell, &mut cmd, "mkt", &mut buf);
            Ok(String::from_utf8_lossy(&buf).into_owned())
        }

        #[cfg(feature = "meta")]
        Commands::Meta { domain } => handle_meta(domain, &cli, output_format).await,

        #[cfg(feature = "google")]
        Commands::Google { domain } => handle_google(domain, &cli, output_format).await,

        #[cfg(feature = "tiktok")]
        Commands::Tiktok { .. } => Ok("TikTok provider is not yet implemented.".into()),

        #[cfg(feature = "linkedin")]
        Commands::Linkedin { domain } => handle_linkedin(domain, &cli, output_format).await,
    };

    match result {
        Ok(output) => {
            if !cli.quiet {
                println!("{output}");
            }
            Ok(())
        }
        Err(e) => Err(e),
    }
}

#[cfg(feature = "meta")]
async fn handle_meta(
    domain: &cli::MetaDomain,
    cli: &Cli,
    output_format: mkt_core::output::OutputFormat,
) -> anyhow::Result<String> {
    use mkt_core::auth;
    use mkt_core::config::MktConfig;
    use secrecy::ExposeSecret;

    // Load config and resolve token
    let config = if let Some(path) = &cli.config {
        MktConfig::load_from_file(path)?
    } else {
        MktConfig::load()?
    };

    let profile = config.profile(&cli.profile).ok();
    let meta_config = profile.and_then(|p| p.meta.as_ref());

    let token = auth::resolve_token(
        "meta",
        "MKT_META_ACCESS_TOKEN",
        meta_config.and_then(|c| c.access_token.as_deref()),
    )?;

    let ad_account_id = meta_config
        .and_then(|c| c.ad_account_id.clone())
        .or_else(|| std::env::var("MKT_META_AD_ACCOUNT_ID").ok())
        .unwrap_or_else(|| "act_unknown".to_string());

    let api_version = meta_config
        .and_then(|c| c.api_version.as_deref())
        .unwrap_or("v25.0");

    let client = mkt_meta::MetaClient::new(
        secrecy::SecretString::from(token.expose_secret().to_string()),
        ad_account_id,
        Some(api_version),
    )?;

    let provider = mkt_meta::MetaProvider::new(
        client,
        meta_config.and_then(|c| c.page_id.clone()),
        meta_config.and_then(|c| c.ig_user_id.clone()),
    );

    match domain {
        cli::MetaDomain::Campaign { action } => {
            commands::campaign::execute(action, &provider, output_format, cli.dry_run)
                .await
                .map_err(anyhow::Error::from)
        }
        cli::MetaDomain::Adset { action } => {
            commands::adset::execute(action, &provider, output_format, cli.dry_run)
                .await
                .map_err(anyhow::Error::from)
        }
        cli::MetaDomain::Raw { action } => handle_meta_raw(action, &provider).await,
        cli::MetaDomain::Audience { action } => {
            commands::audience::execute(action, &provider, output_format, cli.dry_run)
                .await
                .map_err(anyhow::Error::from)
        }
        cli::MetaDomain::Insight { action } => {
            commands::insight::execute(action, &provider, output_format)
                .await
                .map_err(anyhow::Error::from)
        }
        cli::MetaDomain::Post { action } => {
            commands::post::execute(action, &provider, output_format, cli.dry_run)
                .await
                .map_err(anyhow::Error::from)
        }
        cli::MetaDomain::Creative { action } => {
            commands::creative::execute(action, &provider, output_format, cli.dry_run)
                .await
                .map_err(anyhow::Error::from)
        }
        cli::MetaDomain::Media { action } => {
            commands::media::execute(action, &provider, output_format, cli.dry_run)
                .await
                .map_err(anyhow::Error::from)
        }
    }
}

#[cfg(feature = "google")]
async fn handle_google(
    domain: &cli::GoogleDomain,
    cli: &Cli,
    output_format: mkt_core::output::OutputFormat,
) -> anyhow::Result<String> {
    use mkt_core::config::MktConfig;
    use mkt_core::error::MktError;

    // Load config and resolve credentials.
    let config = if let Some(path) = &cli.config {
        MktConfig::load_from_file(path)?
    } else {
        MktConfig::load()?
    };

    let profile = config.profile(&cli.profile).ok();
    let google_config = profile.and_then(|p| p.google.as_ref());

    let developer_token = std::env::var("MKT_GOOGLE_DEVELOPER_TOKEN")
        .ok()
        .or_else(|| google_config.and_then(|c| c.developer_token.clone()))
        .ok_or_else(|| {
            MktError::auth_error(
                "google",
                "No developer token found. Set MKT_GOOGLE_DEVELOPER_TOKEN or configure it \
                 in your profile.",
            )
        })?;

    let customer_id = std::env::var("MKT_GOOGLE_CUSTOMER_ID")
        .ok()
        .or_else(|| google_config.and_then(|c| c.customer_id.clone()))
        .ok_or_else(|| {
            MktError::auth_error(
                "google",
                "No customer ID found. Set MKT_GOOGLE_CUSTOMER_ID or configure it in your \
                 profile.",
            )
        })?;

    // Access token: direct env var wins; otherwise exchange the refresh token.
    let access_token = if let Ok(token) = std::env::var("MKT_GOOGLE_ACCESS_TOKEN") {
        secrecy::SecretString::from(token)
    } else {
        let (client_id, client_secret, refresh_token) = google_config
            .and_then(|c| {
                Some((
                    c.client_id.clone()?,
                    c.client_secret.clone()?,
                    c.refresh_token.clone()?,
                ))
            })
            .ok_or_else(|| {
                MktError::auth_error(
                    "google",
                    "No access token found. Set MKT_GOOGLE_ACCESS_TOKEN, or configure \
                     client_id, client_secret, and refresh_token in your profile.",
                )
            })?;
        mkt_google::fetch_access_token(
            &client_id,
            &client_secret,
            &refresh_token,
            mkt_google::GOOGLE_TOKEN_URL,
        )
        .await?
    };

    let client = mkt_google::GoogleClient::new(access_token, developer_token, &customer_id, None)?;
    let provider = mkt_google::GoogleProvider::new(client);

    match domain {
        cli::GoogleDomain::Campaign { action } => {
            commands::campaign::execute(action, &provider, output_format, cli.dry_run)
                .await
                .map_err(anyhow::Error::from)
        }
        cli::GoogleDomain::Insight { action } => {
            commands::insight::execute(action, &provider, output_format)
                .await
                .map_err(anyhow::Error::from)
        }
    }
}

#[cfg(feature = "linkedin")]
async fn handle_linkedin(
    domain: &cli::LinkedinDomain,
    cli: &Cli,
    output_format: mkt_core::output::OutputFormat,
) -> anyhow::Result<String> {
    use mkt_core::auth;
    use mkt_core::config::MktConfig;
    use mkt_core::error::MktError;
    use secrecy::ExposeSecret;

    let config = if let Some(path) = &cli.config {
        MktConfig::load_from_file(path)?
    } else {
        MktConfig::load()?
    };

    let profile = config.profile(&cli.profile).ok();
    let li_config = profile.and_then(|p| p.linkedin.as_ref());

    let token = auth::resolve_token(
        "linkedin",
        "MKT_LINKEDIN_ACCESS_TOKEN",
        li_config.and_then(|c| c.access_token.as_deref()),
    )?;

    let ad_account_id = std::env::var("MKT_LINKEDIN_AD_ACCOUNT_ID")
        .ok()
        .or_else(|| li_config.and_then(|c| c.ad_account_id.clone()))
        .ok_or_else(|| {
            MktError::auth_error(
                "linkedin",
                "No ad account ID found. Set MKT_LINKEDIN_AD_ACCOUNT_ID or configure it \
                 in your profile.",
            )
        })?;

    let client = mkt_linkedin::LinkedInClient::new(
        secrecy::SecretString::from(token.expose_secret().to_string()),
        ad_account_id,
    )?;
    let provider = mkt_linkedin::LinkedInProvider::new(client);

    match domain {
        cli::LinkedinDomain::Campaign { action } => {
            commands::campaign::execute(action, &provider, output_format, cli.dry_run)
                .await
                .map_err(anyhow::Error::from)
        }
        cli::LinkedinDomain::Insight { action } => {
            commands::insight::execute(action, &provider, output_format)
                .await
                .map_err(anyhow::Error::from)
        }
    }
}

#[cfg(feature = "meta")]
async fn handle_meta_raw(
    action: &cli::RawAction,
    provider: &mkt_meta::MetaProvider,
) -> anyhow::Result<String> {
    use mkt_core::models::HttpMethod;
    use mkt_core::provider::MarketingProvider;

    match action {
        cli::RawAction::Get { path, fields: _ } => {
            let result = provider
                .raw_request(HttpMethod::Get, path, &serde_json::Value::Null)
                .await?;
            Ok(serde_json::to_string_pretty(&result)?)
        }
        cli::RawAction::Post { path, body } => {
            let json_body = match body {
                Some(b) => serde_json::from_str(b)?,
                None => serde_json::Value::Object(serde_json::Map::new()),
            };
            let result = provider
                .raw_request(HttpMethod::Post, path, &json_body)
                .await?;
            Ok(serde_json::to_string_pretty(&result)?)
        }
    }
}
