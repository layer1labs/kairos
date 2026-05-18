// Kairos autoupdate uses the GitHub Releases API (not the original Warp channel_versions / GCS).
// This module is responsible only for fetching the latest release metadata and picking
// the right asset by name; the actual download + directory opening is done by windows.rs / mac.rs.

use std::sync::Mutex;
use std::time::Duration;

use anyhow::{Context as _, Result};
use lazy_static::lazy_static;
use serde::Deserialize;

const REPO_OWNER: &str = "BitConcepts";
const REPO_NAME: &str = "kairos";

// GitHub requires a User-Agent; we also pin the API version to avoid future default drift.
const USER_AGENT: &str = "Kairos-Autoupdate";
const ACCEPT: &str = "application/vnd.github+json";
const API_VERSION: &str = "2022-11-28";

const FETCH_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Deserialize)]
pub struct GithubRelease {
    pub tag_name: String,
    pub html_url: String,
    pub assets: Vec<GithubAsset>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GithubAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

impl GithubRelease {
    pub fn version(&self) -> &str {
        self.tag_name.trim_start_matches('v')
    }

    pub fn find_asset(&self, expected_name: &str) -> Option<&GithubAsset> {
        self.assets.iter().find(|a| a.name == expected_name)
    }
}

lazy_static! {
    /// The most recently fetched release. Written by fetch_latest_release, read by download_update.
    /// This avoids a second API call during the download phase and prevents races
    /// (where a new release could appear between the two requests).
    static ref LATEST_RELEASE: Mutex<Option<GithubRelease>> = Mutex::new(None);
}

pub fn cached_release() -> Option<GithubRelease> {
    LATEST_RELEASE.lock().ok().and_then(|g| g.clone())
}

fn store_cached(release: GithubRelease) {
    if let Ok(mut guard) = LATEST_RELEASE.lock() {
        *guard = Some(release);
    }
}

pub async fn fetch_latest_release(client: &http_client::Client) -> Result<GithubRelease> {
    let url = format!("https://api.github.com/repos/{REPO_OWNER}/{REPO_NAME}/releases/latest");
    log::info!("Fetching latest release from {url}");
    let release: GithubRelease = client
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", ACCEPT)
        .header("X-GitHub-Api-Version", API_VERSION)
        .timeout(FETCH_TIMEOUT)
        .send()
        .await
        .context("GitHub Releases API request failed")?
        .error_for_status()
        .context("GitHub Releases API returned a non-2xx status code")?
        .json()
        .await
        .context("Failed to parse GitHub Releases JSON")?;
    log::info!(
        "GitHub latest release: tag={} assets={}",
        release.tag_name,
        release.assets.len()
    );
    store_cached(release.clone());
    Ok(release)
}

/// Fetch the most recent release **including pre-releases** (latest channel).
///
/// Uses `/releases?per_page=1` which returns releases sorted by creation date,
/// so the first element is always the most recently published release regardless
/// of whether it is a pre-release or not.
pub async fn fetch_latest_release_any(client: &http_client::Client) -> Result<GithubRelease> {
    let url = format!("https://api.github.com/repos/{REPO_OWNER}/{REPO_NAME}/releases?per_page=1");
    log::info!("Fetching latest release (any channel) from {url}");
    let releases: Vec<GithubRelease> = client
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", ACCEPT)
        .header("X-GitHub-Api-Version", API_VERSION)
        .timeout(FETCH_TIMEOUT)
        .send()
        .await
        .context("GitHub Releases API request failed")?
        .error_for_status()
        .context("GitHub Releases API returned a non-2xx status code")?
        .json()
        .await
        .context("Failed to parse GitHub Releases JSON")?;
    let release = releases
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No releases found on GitHub"))?;
    log::info!(
        "GitHub latest-any release: tag={} assets={}",
        release.tag_name,
        release.assets.len()
    );
    store_cached(release.clone());
    Ok(release)
}

/// Returns the current GitHub release API URL (for logging / diagnostics).
pub fn releases_api_url() -> String {
    format!("https://api.github.com/repos/{REPO_OWNER}/{REPO_NAME}/releases/latest")
}
