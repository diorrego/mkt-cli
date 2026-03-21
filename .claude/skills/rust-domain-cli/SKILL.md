---
name: rust-domain-cli
description: Rust CLI application development. Activates for command-line tool design, argument parsing with clap, TUI frameworks, terminal I/O, progress bars, and CLI UX patterns.
---

# Rust CLI Domain Skill

## Stack Selection
| Component | Recommended | Alternative |
|---|---|---|
| Arg parsing | `clap` (derive) | `argh` (minimal) |
| Config files | `config` + `serde` | `figment` |
| TUI | `ratatui` | `cursive` |
| Colors | `owo-colors` | `colored` |
| Progress bars | `indicatif` | — |
| Tables | `comfy-table` | `tabled` |
| Dialogs/prompts | `dialoguer` | `inquire` |
| Logging | `tracing` + `tracing-subscriber` | `env_logger` |

## Clap Derive Pattern
```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "my-tool", version, about = "Tool description")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new project
    Init {
        /// Project name
        #[arg(value_name = "NAME")]
        name: String,
    },
    /// Run the tool
    Run {
        /// Config file path
        #[arg(short, long, default_value = "config.toml")]
        config: PathBuf,
    },
}
```

## CLI UX Patterns
- Use `stderr` for progress/status, `stdout` for data output (pipe-friendly)
- Support `--json` flag for machine-readable output
- Use exit codes: 0 = success, 1 = runtime error, 2 = usage error
- Respect `NO_COLOR` env var and `--no-color` flag
- Show help when invoked with no arguments (if no default action)
- Use `anyhow` for errors displayed to users (good formatting)

## Error Display for CLI
```rust
fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}
```

## Config File Loading Order
1. Built-in defaults
2. System config (`/etc/my-tool/config.toml`)
3. User config (`~/.config/my-tool/config.toml`)
4. Project config (`./.my-tool.toml`)
5. Environment variables (`MY_TOOL_*`)
6. Command-line arguments (highest priority)
