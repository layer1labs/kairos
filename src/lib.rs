// SPDX-License-Identifier: MIT
// Copyright (c) 2026 BitConcepts, LLC. All rights reserved.
//! Kairos library crate.
//!
//! Exposes the `governance` module so that integration tests under `tests/`
//! can import types without duplicating source. The binary (`src/main.rs`)
//! uses this crate as a dependency via the split `[lib]` + `[[bin]]` layout.

pub mod governance;
