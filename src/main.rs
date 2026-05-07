//! Kairos governance daemon stub.
//!
//! NOTE: Kairos is NOT a standalone terminal.
//! Kairos IS a fork of the open-source Warp terminal (with BYOE support)
//! with all Warp cloud/AI services removed and replaced by the specsmith
//! AEE governance layer.
//!
//! This binary is a temporary stub used during development to:
//!   - Validate the governance client↔server integration
//!   - Test the GovernanceServer spawn lifecycle
//!   - Demonstrate the session preflight gate
//!
//! In production Kairos, this code lives INSIDE the Warp fork:
//!   - GovernanceServer::spawn() is called at Warp startup
//!   - POST /preflight is called before any AI command execution
//!   - POST /verify is called after shell command output is captured
//!   - The governance dashboard is a Warp-native WebView panel (REQ-005)
//!   - BYOE routes the AI endpoint through specsmith governance-serve
//!
//! # What gets removed from the Warp fork
//! - Warp cloud sync / Warp Drive
//! - Warp AI (replaced by specsmith governance gate)
//! - Warp telemetry and analytics
//! - Warp account / login / licensing
//! - Warp AI model configuration (replaced by BYOE → specsmith)
//!
//! # What gets added
//! - specsmith governance lifecycle (this crate's governance module)
//! - Kairos brand: name, colors, theme (not Warp blue/purple)
//! - GovernanceServer auto-spawn at startup
//! - WebView governance dashboard panel
//!
//! # Architecture invariants
//! - I1: No LLM API calls are made directly from Kairos
//! - I2: Governance HTTP calls target 127.0.0.1 only
//! - I3: specsmith governance-serve is spawned as a managed child process

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;

use kairos::governance::{
    client::{GovernanceClient, GovernanceConfig, DEFAULT_PORT},
    server::GovernanceServer,
};
use kairos::session::SessionConfig;

/// Kairos governance daemon (development stub).
///
/// In production this is embedded in the Warp fork, not a standalone binary.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Project root directory (governance context for specsmith).
    #[arg(short, long, value_name = "DIR")]
    project_dir: Option<PathBuf>,

    /// Skip spawning specsmith governance-serve; connect to an existing instance.
    #[arg(long)]
    no_spawn: bool,

    /// Governance server port (default: 7700).
    #[arg(long, default_value_t = DEFAULT_PORT)]
    port: u16,

    /// Startup timeout in seconds (default: 15).
    #[arg(long, default_value_t = 15)]
    startup_timeout: u64,
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("[kairos] Fatal error: {e:#}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let args = Args::parse();

    let mut config = SessionConfig::new(args.project_dir);
    config.governance_port = args.port;
    config.startup_timeout = Duration::from_secs(args.startup_timeout);
    config.no_spawn = args.no_spawn;

    eprintln!("[kairos] Governance daemon starting");
    eprintln!("[kairos] Project:      {}", config.project_dir.display());
    eprintln!("[kairos] Specsmith:    {}", config.specsmith_cmd);
    eprintln!("[kairos] Port:         {}", config.governance_port);

    // ── REQ-002: Spawn specsmith governance-serve ─────────────────────────
    let _server: Option<GovernanceServer> = if config.no_spawn {
        eprintln!("[kairos] --no-spawn: skipping server spawn");
        None
    } else {
        eprintln!("[kairos] Spawning specsmith governance-serve…");
        match GovernanceServer::spawn(
            &config.specsmith_cmd,
            config.governance_port,
            config.startup_timeout,
        ) {
            Ok(s) => { eprintln!("[kairos] Governance server ready."); Some(s) }
            Err(e) => {
                eprintln!("[kairos] Could not spawn governance server: {e}");
                eprintln!("[kairos] Continuing without auto-spawn — start manually:");
                eprintln!("[kairos]   specsmith governance-serve --port {}", config.governance_port);
                None
            }
        }
    };

    // ── REQ-001 / I2: Build governance client ─────────────────────────────
    let client = GovernanceClient::new(GovernanceConfig {
        base_url: format!("http://127.0.0.1:{}", config.governance_port),
    }).context("Failed to build governance client")?;

    // ── Health check ──────────────────────────────────────────────────────
    match client.health().await {
        Ok(h) => eprintln!("[kairos] Backend healthy — specsmith {}", h.version),
        Err(e) => eprintln!("[kairos] Backend unreachable: {e}"),
    }

    // ── REG-003: Session preflight gate ───────────────────────────────────
    let project_dir = config.project_dir_str().to_owned();
    match client.preflight("start kairos terminal session", Some(&project_dir)).await {
        Ok(d) if d.accepted() => eprintln!(
            "[kairos] Preflight ACCEPTED (WI:{}, confidence ≥ {:.2})",
            d.work_item_id, d.confidence_target
        ),
        Ok(d) => eprintln!("[kairos] Preflight NOT ACCEPTED: {}", d.instruction),
        Err(e) => eprintln!("[kairos] Preflight skipped: {e}"),
    }

    eprintln!("[kairos] Governance daemon ready.");
    eprintln!("[kairos] (In production Kairos, Warp now takes over.)");
    eprintln!("[kairos] Press Ctrl-C to stop.");

    // Block until Ctrl-C in stub mode.
    tokio::signal::ctrl_c().await.ok();

    // _server drops here → governance-serve terminates (REQ-002).
    eprintln!("[kairos] Shutdown.");
    Ok(())
}
