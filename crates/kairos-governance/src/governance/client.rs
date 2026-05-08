//! HTTP client for the `specsmith serve` governance API.
//!
//! All endpoint calls target `http://127.0.0.1:{port}` only (architecture invariant I2).
//!
//! # REQ-001 — specsmith serve as Sole Governance Interface
//! # REQ-003 — Preflight via REST API
//! # REQ-004 — Verify via REST API
//! # REQ-008 — Local-Only Governance Communication

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Default port for `specsmith serve`.
pub const DEFAULT_PORT: u16 = 7700;

/// Connection timeout for all governance API calls (H11 — all blocking waits must have a timeout).
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for the governance API client.
#[derive(Debug, Clone)]
pub struct GovernanceConfig {
    /// Base URL for `specsmith serve`. Must be a localhost address (I2).
    pub base_url: String,
}

impl GovernanceConfig {
    /// Create a config targeting the default local port.
    pub fn default_local() -> Self {
        Self {
            base_url: format!("http://127.0.0.1:{DEFAULT_PORT}"),
        }
    }

    /// Validate that the base URL is a localhost address (enforces invariant I2).
    pub fn validate(&self) -> Result<()> {
        let url = url::Url::parse(&self.base_url).context("Invalid governance base URL")?;
        let host = url.host_str().unwrap_or("");
        if host != "127.0.0.1" && host != "localhost" && host != "::1" {
            return Err(anyhow!(
                "Governance base URL must target localhost (127.0.0.1), got: {host}"
            ));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

/// Request body for `POST /preflight`.
#[derive(Debug, Serialize)]
pub struct PreflightRequest {
    /// Natural-language description of the action to be gated.
    pub utterance: String,
    /// Optional project directory for specsmith to resolve requirements against.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_dir: Option<String>,
}

/// Response from `POST /preflight`.
#[derive(Debug, Deserialize)]
pub struct PreflightDecision {
    /// `"accepted"` | `"needs_clarification"` | `"rejected"`.
    pub decision: String,
    /// Assigned work item ID (e.g. `"WI-XXXX-001"`).
    pub work_item_id: String,
    /// Matched requirement IDs (e.g. `["REQ-003"]`).
    pub requirement_ids: Vec<String>,
    /// Matched test case IDs (e.g. `["TEST-003"]`).
    pub test_case_ids: Vec<String>,
    /// Minimum confidence target for this work item.
    pub confidence_target: f64,
    /// Human-readable guidance for the agent.
    pub instruction: String,
    /// Intent classification: `"change"` | `"read_only_ask"` | etc.
    #[serde(default)]
    pub intent: String,
}

impl PreflightDecision {
    /// Returns `true` if the action was accepted by the governance gate.
    pub fn accepted(&self) -> bool {
        self.decision == "accepted"
    }
}

/// Request body for `POST /verify`.
#[derive(Debug, Serialize)]
pub struct VerifyRequest {
    /// Unified diff of changes made.
    pub diff: String,
    /// List of files that were changed.
    pub files_changed: Vec<String>,
    /// Test results summary (e.g. `{"passed": 5, "failed": 0}`).
    pub test_results: serde_json::Value,
}

/// Response from `POST /verify`.
#[derive(Debug, Deserialize)]
pub struct VerifyResult {
    /// Whether the change reached epistemic equilibrium.
    pub equilibrium: bool,
    /// Confidence score (0.0–1.0).
    pub confidence: f64,
    /// Human-readable verification summary.
    pub summary: String,
    /// Retry strategy if equilibrium was not reached (empty string if equilibrium = true).
    #[serde(default)]
    pub retry_strategy: String,
    /// Files affected by the change.
    #[serde(default)]
    pub files_changed: Vec<String>,
}

/// Response from `GET /health`.
#[derive(Debug, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    #[serde(default)]
    pub version: String,
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Async HTTP client for the `specsmith serve` governance API.
///
/// All methods are `async` and include timeouts (H11).
pub struct GovernanceClient {
    config: GovernanceConfig,
    http: reqwest::Client,
}

impl GovernanceClient {
    /// Create a new client with the given configuration.
    pub fn new(config: GovernanceConfig) -> Result<Self> {
        config.validate()?;
        let http = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .context("Failed to build HTTP client")?;
        Ok(Self { config, http })
    }

    /// Create a client targeting the default local `specsmith serve` port.
    pub fn default_local() -> Result<Self> {
        Self::new(GovernanceConfig::default_local())
    }

    // -----------------------------------------------------------------------
    // Governance API calls (REQ-001, REQ-003, REQ-004, REQ-008)
    // -----------------------------------------------------------------------

    /// Check that `specsmith serve` is reachable and healthy.
    ///
    /// Returns an error if the backend is not running or unreachable.
    /// H11: connect_timeout (5s) and request_timeout (30s) are set on the client.
    pub async fn health(&self) -> Result<HealthResponse> {
        let url = format!("{}/health", self.config.base_url);
        let resp = self.http.get(&url).send().await.with_context(|| {
            format!(
                "Health check failed — is specsmith serve running at {}?",
                url
            )
        })?;
        if !resp.status().is_success() {
            return Err(anyhow!("Health check returned HTTP {}", resp.status()));
        }
        resp.json::<HealthResponse>()
            .await
            .context("Failed to parse health response")
    }

    /// Gate an action through the governance preflight check (REQ-003).
    ///
    /// Returns the preflight decision. Call `.accepted()` on the result to determine
    /// whether execution should proceed.
    pub async fn preflight(
        &self,
        utterance: &str,
        project_dir: Option<&str>,
    ) -> Result<PreflightDecision> {
        let url = format!("{}/preflight", self.config.base_url);
        let body = PreflightRequest {
            utterance: utterance.to_owned(),
            project_dir: project_dir.map(ToOwned::to_owned),
        };
        let resp = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Preflight request failed")?;
        let status = resp.status();
        if !status.is_success() && status.as_u16() != 200 {
            let text = resp.text().await.unwrap_or_default();
            // specsmith preflight can return 200 with decision=needs_clarification
            // or non-2xx on hard errors only.
            if status.as_u16() >= 500 {
                return Err(anyhow!("Preflight server error {status}: {text}"));
            }
            // For non-fatal non-success (4xx), fall through and try to parse the body.
            // Re-fetch the response isn't possible after consuming text, so just return an error.
            return Err(anyhow!("Preflight client error {status}: {text}"));
        }
        resp.json::<PreflightDecision>()
            .await
            .context("Failed to parse preflight response")
    }

    /// Run a post-change verification check (REQ-004).
    ///
    /// Returns the verification result including confidence score and equilibrium status.
    pub async fn verify(
        &self,
        diff: &str,
        files_changed: Vec<String>,
        test_results: serde_json::Value,
    ) -> Result<VerifyResult> {
        let url = format!("{}/verify", self.config.base_url);
        let body = VerifyRequest {
            diff: diff.to_owned(),
            files_changed,
            test_results,
        };
        let resp = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Verify request failed")?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            // 200 = equilibrium; 202 = no equilibrium (retry recommended).
            // In all non-success cases we consume resp via text() so we must return here.
            return Err(anyhow!("Verify error {status}: {text}"));
        }
        resp.json::<VerifyResult>()
            .await
            .context("Failed to parse verify response")
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_validates_localhost() {
        let cfg = GovernanceConfig::default_local();
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn config_rejects_external_host() {
        let cfg = GovernanceConfig {
            base_url: "http://example.com:7700".to_owned(),
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn preflight_decision_accepted() {
        let d = PreflightDecision {
            decision: "accepted".to_owned(),
            work_item_id: "WI-0001".to_owned(),
            requirement_ids: vec!["REQ-003".to_owned()],
            test_case_ids: vec!["TEST-003".to_owned()],
            confidence_target: 0.85,
            instruction: "Proceed with change.".to_owned(),
            intent: "change".to_owned(),
        };
        assert!(d.accepted());
    }

    #[test]
    fn preflight_decision_needs_clarification() {
        let d = PreflightDecision {
            decision: "needs_clarification".to_owned(),
            work_item_id: String::new(),
            requirement_ids: vec![],
            test_case_ids: vec![],
            confidence_target: 0.0,
            instruction: "Clarify intent.".to_owned(),
            intent: "destructive".to_owned(),
        };
        assert!(!d.accepted());
    }
}
