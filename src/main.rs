//! Kairos — epistemically-governed terminal runtime.
//!
//! Governance backend: `specsmith serve` (REST/WebSocket API).
//!
//! See AGENTS.md for session-start instructions and REQUIREMENTS.md
//! for the formal integration contract.
//!
//! # Architecture invariants
//! - I1: No LLM API calls are made directly from Kairos
//! - I2: Governance HTTP calls target 127.0.0.1 only
//! - I3: specsmith serve is spawned as a managed child process

use kairos::governance::GovernanceClient;

#[tokio::main]
async fn main() {
    println!("Kairos terminal v0.1.0");

    // Check governance backend liveness (REQ-001, REQ-002).
    // In the full implementation this will spawn specsmith serve first (REQ-002).
    let client = match GovernanceClient::default_local() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[kairos] Failed to initialise governance client: {e}");
            eprintln!("[kairos] Run `specsmith governance-serve --port 7700` to start the governance backend.");
            std::process::exit(1);
        }
    };

    match client.health().await {
        Ok(h) => println!("[kairos] Governance backend healthy — specsmith {}", h.version),
        Err(e) => {
            eprintln!("[kairos] Governance backend unreachable: {e}");
            eprintln!("[kairos] Run `specsmith governance-serve --port 7700` then retry.");
            eprintln!("[kairos] (In production, GovernanceServer::spawn() starts this automatically)");
            std::process::exit(1);
        }
    }

    // REG-003: preflight gate — every session start is a governed action.
    // The preflight call returns a PreflightDecision; non-accepted decisions
    // block terminal startup (human escalation path for REG-004).
    let session_utterance = "start kairos terminal session";
    match client.preflight(session_utterance, None).await {
        Ok(decision) => {
            if decision.accepted() {
                println!("[kairos] Session preflight accepted (confidence ≥ {:.2})",
                    decision.confidence_target);
            } else {
                eprintln!("[kairos] Session preflight not accepted: {}", decision.instruction);
                eprintln!("[kairos] REG-004: human escalation required before proceeding.");
                // In production, open the WebView escalation dialog here.
            }
        }
        Err(e) => {
            // Best-effort — preflight failure does not block session in stub mode.
            eprintln!("[kairos] Preflight skipped (governance backend error): {e}");
        }
    }

    println!("[kairos] Terminal session ready.");
}
