// SPDX-License-Identifier: MIT
// Copyright (c) 2026 BitConcepts, LLC. All rights reserved.
//! kairos-governance — specsmith AEE governance integration crate.
//!
//! Provides the GovernanceClient, GovernanceServer, and SessionConfig
//! used throughout the Kairos terminal to gate AI requests through
//! the specsmith governance backend (REQ-001..REQ-008).

pub mod governance;
pub mod session;

// Re-export the most commonly used types at the crate root for ergonomic use.
pub use governance::client::{GovernanceClient, GovernanceConfig};
pub use governance::server::GovernanceServer;
pub use session::{find_specsmith_cmd, SessionConfig};
