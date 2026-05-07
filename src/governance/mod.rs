//! Governance client module — connects Kairos to `specsmith serve`.
//!
//! All governance interactions (preflight, verify, audit) are routed through
//! the specsmith serve REST/WebSocket API running at `http://127.0.0.1:7700`.
//!
//! # Architecture invariants
//! - I1: No LLM API calls are made directly from Kairos. All LLM interaction
//!   goes through `specsmith serve`.
//! - I2: All governance HTTP calls target 127.0.0.1 only.
//! - I3: `specsmith serve` is spawned as a managed child process; its lifecycle
//!   is owned by this module's `GovernanceServer` handle.

pub mod client;
pub mod server;

pub use client::{GovernanceClient, GovernanceConfig, PreflightDecision, VerifyResult};
pub use server::GovernanceServer;
