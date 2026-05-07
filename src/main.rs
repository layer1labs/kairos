//! Kairos — epistemically-governed terminal runtime.
//!
//! Governance backend: `specsmith governance-serve` (REST API).
//!
//! # Full session lifecycle
//! 1. Parse CLI args (`--project-dir`, `--no-spawn`, `--port`)
//! 2. Resolve specsmith command via [`kairos::session::find_specsmith_cmd`]
//! 3. Spawn `specsmith governance-serve --port <port>` unless `--no-spawn`
//! 4. Wait for backend health (`GET /health`) within startup timeout
//! 5. Run session preflight (`POST /preflight`) — REG-003
//! 6. Launch TUI event loop — REQ-005
//! 7. On exit, `GovernanceServer` is dropped (process terminated) — REQ-002
//!
//! # Architecture invariants
//! - I1: No LLM API calls are made directly from Kairos
//! - I2: Governance HTTP calls target 127.0.0.1 only (enforced in GovernanceConfig)
//! - I3: specsmith serve is spawned as a managed child process

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;

use kairos::governance::{
    client::{GovernanceClient, GovernanceConfig, DEFAULT_PORT},
    server::GovernanceServer,
};
use kairos::session::SessionConfig;

// ---------------------------------------------------------------------------
// CLI arguments
// ---------------------------------------------------------------------------

/// Kairos — epistemically-governed terminal runtime.
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Project root directory for governance context.
    /// Specsmith resolves docs/REQUIREMENTS.md relative to this path.
    /// Defaults to the current working directory.
    #[arg(short, long, value_name = "DIR")]
    project_dir: Option<PathBuf>,

    /// Do not spawn `specsmith governance-serve`; connect to an already-running
    /// instance on `--port`. Useful for development / debugging.
    #[arg(long)]
    no_spawn: bool,

    /// Port for `specsmith governance-serve` (default: 7700).
    #[arg(long, default_value_t = DEFAULT_PORT)]
    port: u16,

    /// Timeout in seconds for governance server startup (default: 15).
    #[arg(long, default_value_t = 15)]
    startup_timeout: u64,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("[kairos] Fatal error: {e:#}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let args = Args::parse();

    // ── 1. Build session config ────────────────────────────────────────────
    let mut config = SessionConfig::new(args.project_dir);
    config.governance_port = args.port;
    config.startup_timeout = Duration::from_secs(args.startup_timeout);
    config.no_spawn = args.no_spawn;

    eprintln!("[kairos] Starting — project: {}", config.project_dir.display());
    eprintln!("[kairos] specsmith cmd: {}", config.specsmith_cmd);

    // ── 2. Optionally spawn governance server (REQ-002) ───────────────────
    let _server: Option<GovernanceServer> = if config.no_spawn {
        eprintln!(
            "[kairos] --no-spawn: connecting to existing governance server on port {}",
            config.governance_port
        );
        None
    } else {
        eprintln!(
            "[kairos] Spawning specsmith governance-serve on port {}…",
            config.governance_port
        );
        match GovernanceServer::spawn(
            &config.specsmith_cmd,
            config.governance_port,
            config.startup_timeout,
        ) {
            Ok(server) => {
                eprintln!("[kairos] Governance server ready.");
                Some(server)
            }
            Err(e) => {
                // Non-fatal: TUI will show disconnected state and user can
                // start the server manually.
                eprintln!("[kairos] Could not spawn governance server: {e}");
                eprintln!(
                    "[kairos] Start manually: specsmith governance-serve --port {}",
                    config.governance_port
                );
                None
            }
        }
    };

    // ── 3. Build governance client (REQ-001 / I2) ─────────────────────────
    let gov_cfg = GovernanceConfig {
        base_url: format!("http://127.0.0.1:{}", config.governance_port),
    };
    let client = GovernanceClient::new(gov_cfg)
        .context("Failed to build governance client")?;

    // ── 4. Session preflight (REG-003) ────────────────────────────────────
    // Best-effort: if the backend isn't up yet, the TUI will show the error
    // and the user can still start the session once they connect.
    let project_dir = config.project_dir_str().to_owned();
    match client
        .preflight("start kairos terminal session", Some(&project_dir))
        .await
    {
        Ok(d) if d.accepted() => eprintln!(
            "[kairos] Session preflight accepted (confidence ≥ {:.2})",
            d.confidence_target
        ),
        Ok(d) => eprintln!(
            "[kairos] Session preflight not accepted: {} (REG-004 escalation)",
            d.instruction
        ),
        Err(e) => eprintln!("[kairos] Preflight skipped (backend not ready): {e}"),
    }

    // ── 5. Launch TUI (REQ-005) ───────────────────────────────────────────
    kairos::tui::run(config, client).await?;

    // ── 6. _server is dropped here, terminating governance-serve (REQ-002) ─
    eprintln!("[kairos] Session ended. Governance server shutting down.");
    Ok(())
}
