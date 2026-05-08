// On Windows, we don't want to display a console window when the application is running in release
// builds. See https://doc.rust-lang.org/reference/runtime.html#the-windows_subsystem-attribute.
#![cfg_attr(feature = "release_bundle", windows_subsystem = "windows")]

use anyhow::Result;
use warp_core::{
    channel::{Channel, ChannelConfig, ChannelState, OzConfig, WarpServerConfig},
    AppId,
};

// Kairos: spawn specsmith governance-serve at startup so the local AI gateway
// is available before the terminal UI renders. The child process is kept alive
// for the duration of the session and terminates when the app exits.
#[cfg(not(target_family = "wasm"))]
fn maybe_spawn_governance_server() {
    use std::time::Duration;
    let cmd = kairos_governance::find_specsmith_cmd();
    match kairos_governance::GovernanceServer::spawn(&cmd, 7700, Duration::from_secs(15)) {
        Ok(server) => {
            // Leak intentionally: the governance server must outlive the process.
            // It will be killed by the OS when the app exits.
            std::mem::forget(server);
            log::info!("Kairos: specsmith governance-serve started on port 7700");
        }
        Err(e) => {
            // Not fatal — the user may have already started governance-serve manually,
            // or SPECSMITH_CMD is unset. BYOE will fall back to whatever is on port 7700.
            log::warn!("Kairos: could not start specsmith governance-serve: {e}");
        }
    }
}

// Kairos terminal binary (replaces warp-oss / OpenWarp).
fn main() -> Result<()> {
    let mut state = ChannelState::new(
        Channel::Oss,
        ChannelConfig {
            app_id: AppId::new("io", "bitconcepts", "Kairos"),
            logfile_name: "kairos.log".into(),
            server_config: WarpServerConfig::disabled(),
            oz_config: OzConfig::disabled(),
            telemetry_config: None,
            crash_reporting_config: None,
            autoupdate_config: None,
            mcp_static_config: None,
        },
    );
    if cfg!(debug_assertions) {
        state = state.with_additional_features(warp_core::features::DEBUG_FLAGS);
    }
    ChannelState::set(state);

    // Start specsmith governance-serve so the local BYOE gateway is ready.
    #[cfg(not(target_family = "wasm"))]
    maybe_spawn_governance_server();

    warp::run()
}

// If we're not using an external plist, embed the following as the Info.plist.
#[cfg(all(not(feature = "extern_plist"), target_os = "macos"))]
embed_plist::embed_info_plist_bytes!(r#"
    <?xml version="1.0" encoding="UTF-8"?>
    <!DOCTYPE plist PUBLIC "-//Apple Computer//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
    <plist version="1.0">
    <dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>English</string>
    <key>CFBundleDisplayName</key>
    <string>Kairos</string>
    <key>CFBundleExecutable</key>
    <string>kairos</string>
    <key>CFBundleIdentifier</key>
    <string>io.bitconcepts.Kairos</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>Kairos</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.developer-tools</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>UIDesignRequiresCompatibility</key>
    <true/>
    <key>CFBundleURLTypes</key>
<array><dict><key>CFBundleURLName</key><string>Kairos</string><key>CFBundleURLSchemes</key><array><string>kairos</string></array></dict></array>
    <key>NSHumanReadableCopyright</key>
    <string>© 2026, BitConcepts</string>
    </dict>
    </plist>
"#.as_bytes());
