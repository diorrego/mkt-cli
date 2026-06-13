//! Entry point for the `mkt` multi-platform marketing CLI.

mod cli;
mod commands;
#[cfg(feature = "mcp")]
mod mcp;
mod providers;

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
    // Diagnostics always go to stderr: stdout is reserved for data
    // (and for the MCP protocol when serving).
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(std::io::stderr)
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
        Commands::Providers => Ok(commands::providers::execute(cli.output.into())),

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

        #[cfg(feature = "mcp")]
        Commands::Mcp { action } => match action {
            cli::McpAction::Serve => {
                mcp::serve(cli.profile.clone()).await?;
                Ok(String::new())
            }
        },

        #[cfg(feature = "meta")]
        Commands::Meta { domain } => handle_meta(domain, &cli, output_format).await,

        #[cfg(feature = "google")]
        Commands::Google { domain } => handle_google(domain, &cli, output_format).await,

        #[cfg(feature = "tiktok")]
        Commands::Tiktok { domain } => handle_tiktok(domain, &cli, output_format).await,

        #[cfg(feature = "linkedin")]
        Commands::Linkedin { domain } => handle_linkedin(domain, &cli, output_format).await,
    };

    match result {
        Ok(output) => {
            if !cli.quiet && !output.is_empty() {
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
    let config = providers::load_config(cli.config.as_deref())?;
    let provider = providers::build_meta(&config, &cli.profile)?;

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
    let config = providers::load_config(cli.config.as_deref())?;
    let provider = providers::build_google(&config, &cli.profile).await?;

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

#[cfg(feature = "tiktok")]
async fn handle_tiktok(
    domain: &cli::TiktokDomain,
    cli: &Cli,
    output_format: mkt_core::output::OutputFormat,
) -> anyhow::Result<String> {
    let config = providers::load_config(cli.config.as_deref())?;
    let provider = providers::build_tiktok(&config, &cli.profile)?;

    match domain {
        cli::TiktokDomain::Campaign { action } => {
            commands::campaign::execute(action, &provider, output_format, cli.dry_run)
                .await
                .map_err(anyhow::Error::from)
        }
        cli::TiktokDomain::Audience { action } => {
            commands::audience::execute(action, &provider, output_format, cli.dry_run)
                .await
                .map_err(anyhow::Error::from)
        }
        cli::TiktokDomain::Insight { action } => {
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
    let config = providers::load_config(cli.config.as_deref())?;
    let provider = providers::build_linkedin(&config, &cli.profile)?;

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
