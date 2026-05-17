//! Kairos self-update manager.
//!
//! Provides:
//! - [`KairosUpdateChannel`] — user's preferred update channel (Stable or Latest).
//! - [`KairosUpdateStatus`] — result of the most recent update check.
//! - [`KairosUpdaterState`] — singleton model that persists the channel, polls GitHub,
//!   and notifies the About settings page.
//!
//! The channel preference is stored as plain text in `{data_dir}/kairos_update_channel`
//! so it survives app restarts without touching the settings.toml or user preferences.
//!
//! Actual binary download is **not** performed here (v0.x policy: notify + open browser link).
//! The About page opens `html_url` in the user's default browser on their request.

use std::sync::Arc;

use ::channel_versions::ParsedVersion;
use warpui::{AppContext, Entity, ModelContext, SingletonEntity};

use crate::autoupdate::github;
use crate::server::server_api::ServerApi;

// ── Channel ───────────────────────────────────────────────────────────────────

/// Which GitHub release stream to track.
///
/// - `Stable` → `/releases/latest` (most recent non-pre-release, non-draft).
/// - `Latest` → `/releases?per_page=1` (most recently published, may be a pre-release).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KairosUpdateChannel {
    /// Recommended for most users. Only stable tagged releases.
    Stable,
    /// Rolling channel. Tracks the most recently published release including
    /// pre-releases built from the `develop` branch.
    Latest,
}

impl Default for KairosUpdateChannel {
    fn default() -> Self {
        Self::Stable
    }
}

impl KairosUpdateChannel {
    pub fn label(self) -> &'static str {
        match self {
            Self::Stable => "Stable",
            Self::Latest => "Latest",
        }
    }

    fn as_file_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Latest => "latest",
        }
    }

    fn from_str(s: &str) -> Self {
        match s.trim() {
            "latest" => Self::Latest,
            _ => Self::Stable,
        }
    }
}

// ── Status ────────────────────────────────────────────────────────────────────

/// Result of the most recent update check.
#[derive(Debug, Clone, PartialEq)]
pub enum KairosUpdateStatus {
    /// No check has been run yet since startup.
    Idle,
    /// A check is currently in-flight.
    Checking,
    /// The installed version is at least as new as the remote version.
    UpToDate,
    /// A newer version was found on GitHub.
    Available {
        /// Version string from the release tag (leading `v` stripped).
        version: String,
        /// URL of the GitHub Release page to open in a browser.
        html_url: String,
    },
    /// The check failed (network error, rate-limit, etc.).
    Error(String),
}

impl Default for KairosUpdateStatus {
    fn default() -> Self {
        Self::Idle
    }
}

// ── Singleton model ───────────────────────────────────────────────────────────

pub struct KairosUpdaterState {
    /// User's preferred update channel (loaded from disk on init).
    pub channel: KairosUpdateChannel,
    /// Result of the most recent update check.
    pub status: KairosUpdateStatus,
    /// HTTP client borrowed from the Warp server API infrastructure.
    server_api: Arc<ServerApi>,
}

pub enum KairosUpdaterEvent {
    ChannelChanged,
    StatusChanged,
}

impl Entity for KairosUpdaterState {
    type Event = KairosUpdaterEvent;
}

impl SingletonEntity for KairosUpdaterState {}

impl KairosUpdaterState {
    // ── Registration ─────────────────────────────────────────────────────────

    /// Register this singleton model during `initialize_app`.
    ///
    /// Must be called after `server_api` is available (i.e. after
    /// `AutoupdateState::register`).
    pub fn register(ctx: &mut AppContext, server_api: Arc<ServerApi>) {
        ctx.add_singleton_model(move |ctx| {
            let mut me = Self {
                channel: KairosUpdateChannel::default(),
                status: KairosUpdateStatus::default(),
                server_api,
            };
            me.load_channel(ctx);
            me
        });
    }

    // ── Channel management ────────────────────────────────────────────────────

    /// Returns the path used to persist the update channel preference.
    fn channel_file_path() -> std::path::PathBuf {
        warp_core::paths::data_dir().join("kairos_update_channel")
    }

    /// Set the active channel and persist it to disk immediately.
    pub fn set_channel(&mut self, channel: KairosUpdateChannel, ctx: &mut ModelContext<Self>) {
        self.channel = channel;
        // Persist synchronously — the file write is tiny and immediate.
        let path = Self::channel_file_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&path, channel.as_file_str());

        ctx.emit(KairosUpdaterEvent::ChannelChanged);
        ctx.notify();
    }

    /// Load the persisted channel preference from disk (async, best-effort).
    fn load_channel(&mut self, ctx: &mut ModelContext<Self>) {
        let path = Self::channel_file_path();
        ctx.spawn(
            async move { std::fs::read_to_string(path).ok() },
            |me, content, ctx| {
                let channel = content
                    .as_deref()
                    .map(KairosUpdateChannel::from_str)
                    .unwrap_or_default();
                if me.channel != channel {
                    me.channel = channel;
                    ctx.notify();
                }
            },
        );
    }

    // ── Update check ──────────────────────────────────────────────────────────

    /// Kick off an async update check against GitHub Releases.
    ///
    /// No-ops if a check is already in-flight.  Updates `self.status` and emits
    /// [`KairosUpdaterEvent::StatusChanged`] when complete.
    pub fn check_for_update(&mut self, ctx: &mut ModelContext<Self>) {
        if matches!(self.status, KairosUpdateStatus::Checking) {
            return;
        }
        self.status = KairosUpdateStatus::Checking;
        ctx.notify();

        let channel = self.channel;
        let server_api = self.server_api.clone();

        ctx.spawn(
            async move {
                let client = server_api.http_client();
                match channel {
                    KairosUpdateChannel::Stable => github::fetch_latest_release(client)
                        .await
                        .map(|r| (r.version().to_owned(), r.html_url.clone())),
                    KairosUpdateChannel::Latest => github::fetch_latest_release_any(client)
                        .await
                        .map(|r| (r.version().to_owned(), r.html_url.clone())),
                }
            },
            |me, result, ctx| {
                me.status = Self::resolve_status(result);
                ctx.emit(KairosUpdaterEvent::StatusChanged);
                ctx.notify();
            },
        );
    }

    /// Compare the fetched release version against the installed version.
    fn resolve_status(
        result: Result<(String, String), anyhow::Error>,
    ) -> KairosUpdateStatus {
        let (remote_ver, html_url) = match result {
            Err(e) => return KairosUpdateStatus::Error(e.to_string()),
            Ok(pair) => pair,
        };

        let current_str = crate::channel::ChannelState::app_version().unwrap_or("0.0.0");
        let current = ParsedVersion::try_from(current_str);
        let remote = ParsedVersion::try_from(remote_ver.as_str());

        match (current, remote) {
            (Ok(c), Ok(r)) if r > c => KairosUpdateStatus::Available {
                version: remote_ver,
                html_url,
            },
            (Ok(_), Ok(_)) => KairosUpdateStatus::UpToDate,
            // If we can't parse either version, assume an update is available so
            // the user can make their own judgement via the link.
            _ => KairosUpdateStatus::Available {
                version: remote_ver,
                html_url,
            },
        }
    }
}
