//! Entry point for the `mkt` multi-platform marketing CLI.

mod cli;
mod commands;

use clap::Parser;

use cli::{Cli, Commands};

fn main() -> anyhow::Result<()> {
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
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(run(cli))
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

        #[cfg(feature = "meta")]
        Commands::Meta { domain } => handle_meta(domain, &cli, output_format).await,

        #[cfg(feature = "google")]
        Commands::Google { .. } => Ok("Google Ads provider is not yet implemented.".into()),

        #[cfg(feature = "tiktok")]
        Commands::Tiktok { .. } => Ok("TikTok provider is not yet implemented.".into()),

        #[cfg(feature = "linkedin")]
        Commands::Linkedin { .. } => Ok("LinkedIn provider is not yet implemented.".into()),
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
