//! Integration test for Settings → Governance page (REQ-005).
//!
//! Verifies that the Settings → Governance page opens and renders the
//! governance engine status section without panicking or layout errors.
//! The page performs a real `GET /health` call via `GovernanceClient` on
//! initialization; this test verifies the UI renders correctly regardless of
//! whether specsmith serve is running (health status is `Unknown` when the
//! async check hasn't resolved yet, which is fine).
//!
//! ## Runtime prerequisites
//! - Real display required (the test exercises the full Kairos UI pipeline).
//! - Optional: `specsmith serve --port 7700` running for the health check to
//!   resolve to `Healthy`. Without it, the dot shows grey (Unreachable).
//!
//! ## Running manually
//! ```bash
//! cargo run -p integration --bin integration -- test_governance_page_renders
//! # or with a real display on Linux:
//! WARPUI_USE_REAL_DISPLAY_IN_INTEGRATION_TESTS=1 \
//!   cargo nextest run --no-fail-fast --workspace test_governance_page_renders
//! ```
//!
//! ## CI status
//! Marked `#[ignore]` in `ui_tests.rs` — requires a real display. Run
//! manually or in a display-enabled environment.

use std::time::Duration;

use warp::{
    integration_testing::{
        step::new_step_with_default_assertions,
        terminal::wait_until_bootstrapped_single_pane_for_tab,
    },
    settings_view::{SettingsSection, SettingsView},
};
use warpui::{async_assert_eq, ViewHandle};

use super::{assert_tab_count, assert_tab_title, new_builder, Builder};

/// Verifies that the Settings → Governance page opens and renders without
/// panicking (REQ-005).
///
/// Steps:
/// 1. Bootstrap the terminal.
/// 2. Open the Settings tab with `Cmd/Ctrl+,`.
/// 3. Navigate to the Governance section via the sidebar nav item.
/// 4. Assert `SettingsSection::Governance` becomes the active section.
///
/// What this covers: REQ-005 requires a governance dashboard in Settings that
/// is reachable without panicking. This test verifies that the sidebar click
/// activates `SettingsSection::Governance` and the settings view renders
/// cleanly. The live `GET /health` call (also in REQ-005) is covered by the
/// unit tests in `crates/kairos-governance/tests/governance_tests.rs`.
pub fn test_governance_page_renders() -> Builder {
    new_builder()
        // Step 0: wait for the shell to bootstrap before touching settings.
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        // Step 1: open the Settings tab.
        .with_step(
            new_step_with_default_assertions("Open Settings tab via ⌘/Ctrl+,")
                .with_keystrokes(&["cmdorctrl-,"])
                .add_named_assertion("Settings tab opened (tab count == 2)", assert_tab_count(2))
                .add_named_assertion(
                    "New tab is titled 'Settings'",
                    assert_tab_title(1, "Settings"),
                ),
        )
        // Step 2: click the Governance sidebar item.
        // The position key "settings_nav_item:Governance" is saved by the
        // settings sidebar render for SettingsSection::Governance.
        .with_step(
            new_step_with_default_assertions("Navigate to Governance section via sidebar click")
                .set_timeout(Duration::from_secs(10))
                .with_click_on_saved_position("settings_nav_item:Governance")
                .add_named_assertion(
                    "Active section becomes SettingsSection::Governance",
                    |app, window_id| {
                        let views: Vec<ViewHandle<SettingsView>> = app
                            .views_of_type(window_id)
                            .expect("SettingsView must exist when Settings tab is open");
                        let sv = views.first().expect("SettingsView must exist");
                        sv.read(app, |view, _| {
                            async_assert_eq!(
                                view.current_settings_section(),
                                SettingsSection::Governance,
                                "Governance sidebar click must set current_settings_section \
                                 to SettingsSection::Governance (REQ-005)"
                            )
                        })
                    },
                ),
        )
}
