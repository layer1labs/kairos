// SPDX-License-Identifier: MIT
// Copyright (c) 2026 BitConcepts, LLC. All rights reserved.
//! Kairos library crate.
//!
//! Exposes the public module tree so that integration tests under `tests/`
//! and the binary (`src/main.rs`) can share types without duplication.
//! The `[lib]` + `[[bin]]` layout in Cargo.toml enables this.

pub mod governance;
pub mod session;
pub mod tui;
