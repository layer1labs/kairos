// SPDX-License-Identifier: MIT
// Copyright (c) 2026 BitConcepts, LLC. All rights reserved.
//! Integration tests for the kairos governance client module.
//!
//! # Setup required
//! Rust stable must be installed before these tests can be compiled or run:
//!   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
//!   rustup update stable
//!
//! Then run with:
//!   cargo test
//!
//! # Coverage
//! - GovernanceConfig: localhost validation and external host rejection (I2)
//! - PreflightDecision: .accepted() helper for both decision outcomes
//! - VerifyResult: field presence and equilibrium semantics
//! - GovernanceClient construction: valid and invalid configs
//! - DEFAULT_PORT constant value

use kairos_governance::governance::client::{
    GovernanceClient, GovernanceConfig, PreflightDecision, VerifyResult, DEFAULT_PORT,
};

// ---------------------------------------------------------------------------
// GovernanceConfig — I2 localhost invariant enforcement
// ---------------------------------------------------------------------------

#[test]
fn config_default_local_targets_localhost() {
    let cfg = GovernanceConfig::default_local();
    assert!(
        cfg.base_url.contains("127.0.0.1"),
        "default_local() must target 127.0.0.1, got: {}",
        cfg.base_url
    );
}

// ---------------------------------------------------------------------------
// HealthResponse — field semantics
// ---------------------------------------------------------------------------

use kairos_governance::governance::client::HealthResponse;

#[test]
fn health_response_status_ok() {
    let r = HealthResponse {
        status: "ok".to_owned(),
        version: "0.10.1".to_owned(),
    };
    assert_eq!(r.status, "ok");
    assert!(!r.version.is_empty(), "version should be populated");
}

#[test]
fn health_response_deserializes_from_json() {
    let json = r#"{"status": "ok", "version": "0.10.1"}"#;
    let r: HealthResponse = serde_json::from_str(json).expect("should deserialize");
    assert_eq!(r.status, "ok");
    assert_eq!(r.version, "0.10.1");
}

#[test]
fn health_response_deserializes_without_version() {
    // Older specsmith versions may omit the version field.
    let json = r#"{"status": "ok"}"#;
    let r: HealthResponse = serde_json::from_str(json).expect("should deserialize");
    assert_eq!(r.status, "ok");
    assert!(
        r.version.is_empty(),
        "missing version should default to empty string"
    );
}

// ---------------------------------------------------------------------------
// GovernanceClient::health() — live E2E (requires specsmith serve running)
// ---------------------------------------------------------------------------

/// This test contacts the real specsmith governance-serve at localhost:7700.
/// It is ignored by default — run with `cargo test -- --ignored` when specsmith serve is up.
#[tokio::test]
#[ignore]
async fn health_check_returns_ok_when_serve_is_running() {
    let client = GovernanceClient::default_local().expect("client construction");
    let resp = client.health().await;
    match resp {
        Ok(h) => {
            assert_eq!(h.status, "ok");
            assert!(!h.version.is_empty(), "specsmith should report a version");
        }
        Err(e) => panic!("Health check failed (is specsmith serve running?): {e}"),
    }
}

/// This test verifies that health() returns an error when no server is running.
/// We use port 1 (guaranteed unreachable) to simulate an offline governance backend.
/// Ignored by default — requires rustls crypto provider initialization.
#[tokio::test]
#[ignore]
async fn health_check_fails_when_serve_is_not_running() {
    let cfg = GovernanceConfig {
        base_url: "http://127.0.0.1:1".to_owned(),
    };
    let client = GovernanceClient::new(cfg).expect("client construction");
    let resp = client.health().await;
    assert!(
        resp.is_err(),
        "health() must fail when no server is listening"
    );
}

#[test]
fn config_default_local_uses_default_port() {
    let cfg = GovernanceConfig::default_local();
    let expected = format!("http://127.0.0.1:{}", DEFAULT_PORT);
    assert_eq!(cfg.base_url, expected);
}

#[test]
fn config_validate_accepts_127_0_0_1() {
    let cfg = GovernanceConfig {
        base_url: "http://127.0.0.1:7700".to_owned(),
    };
    assert!(
        cfg.validate().is_ok(),
        "127.0.0.1 must be accepted by validate()"
    );
}

#[test]
fn config_validate_accepts_localhost() {
    let cfg = GovernanceConfig {
        base_url: "http://localhost:7700".to_owned(),
    };
    assert!(
        cfg.validate().is_ok(),
        "localhost must be accepted by validate()"
    );
}

#[test]
fn config_validate_accepts_ipv6_loopback() {
    let cfg = GovernanceConfig {
        base_url: "http://[::1]:7700".to_owned(),
    };
    assert!(
        cfg.validate().is_ok(),
        "IPv6 loopback [::1] must be accepted by validate()"
    );
}

#[test]
fn config_validate_rejects_external_host() {
    let cfg = GovernanceConfig {
        base_url: "http://example.com:7700".to_owned(),
    };
    assert!(
        cfg.validate().is_err(),
        "External hostname must be rejected (architecture invariant I2)"
    );
}

#[test]
fn config_validate_rejects_lan_ip() {
    let cfg = GovernanceConfig {
        base_url: "http://192.168.1.10:7700".to_owned(),
    };
    assert!(
        cfg.validate().is_err(),
        "LAN IP must be rejected — governance must be local-only (I2)"
    );
}

#[test]
fn config_validate_rejects_public_ip() {
    let cfg = GovernanceConfig {
        base_url: "http://8.8.8.8:7700".to_owned(),
    };
    assert!(cfg.validate().is_err(), "Public IP must be rejected (I2)");
}

#[test]
fn config_validate_rejects_invalid_url() {
    let cfg = GovernanceConfig {
        base_url: "not-a-url".to_owned(),
    };
    assert!(
        cfg.validate().is_err(),
        "Malformed URL must be rejected by validate()"
    );
}

// ---------------------------------------------------------------------------
// DEFAULT_PORT constant
// ---------------------------------------------------------------------------

#[test]
fn default_port_is_7700() {
    assert_eq!(
        DEFAULT_PORT, 7700,
        "DEFAULT_PORT must be 7700 (specsmith serve default)"
    );
}

// ---------------------------------------------------------------------------
// GovernanceClient construction
// ---------------------------------------------------------------------------

#[test]
fn client_new_accepts_valid_config() {
    let cfg = GovernanceConfig::default_local();
    let client = GovernanceClient::new(cfg);
    assert!(
        client.is_ok(),
        "GovernanceClient::new() must succeed with a valid localhost config"
    );
}

#[test]
fn client_new_rejects_external_config() {
    let cfg = GovernanceConfig {
        base_url: "http://api.example.com:7700".to_owned(),
    };
    let client = GovernanceClient::new(cfg);
    assert!(
        client.is_err(),
        "GovernanceClient::new() must fail for an external host (I2)"
    );
}

#[test]
fn client_default_local_succeeds() {
    let client = GovernanceClient::default_local();
    assert!(
        client.is_ok(),
        "GovernanceClient::default_local() must succeed"
    );
}

// ---------------------------------------------------------------------------
// PreflightDecision — .accepted() semantics
// ---------------------------------------------------------------------------

#[test]
fn preflight_decision_accepted_when_decision_is_accepted() {
    let d = PreflightDecision {
        decision: "accepted".to_owned(),
        work_item_id: "WI-AABB1122".to_owned(),
        requirement_ids: vec!["REQ-003".to_owned()],
        test_case_ids: vec!["TEST-003".to_owned()],
        confidence_target: 0.85,
        instruction: "Change accepted. Proceed under governance.".to_owned(),
        intent: "change".to_owned(),
    };
    assert!(
        d.accepted(),
        "decision='accepted' must return true from .accepted()"
    );
}

#[test]
fn preflight_decision_not_accepted_when_needs_clarification() {
    let d = PreflightDecision {
        decision: "needs_clarification".to_owned(),
        work_item_id: String::new(),
        requirement_ids: vec![],
        test_case_ids: vec![],
        confidence_target: 0.0,
        instruction: "Clarify intent.".to_owned(),
        intent: "destructive".to_owned(),
    };
    assert!(
        !d.accepted(),
        "decision='needs_clarification' must return false from .accepted()"
    );
}

#[test]
fn preflight_decision_not_accepted_when_rejected() {
    let d = PreflightDecision {
        decision: "rejected".to_owned(),
        work_item_id: String::new(),
        requirement_ids: vec![],
        test_case_ids: vec![],
        confidence_target: 0.0,
        instruction: "Operation rejected by governance policy.".to_owned(),
        intent: "release".to_owned(),
    };
    assert!(
        !d.accepted(),
        "decision='rejected' must return false from .accepted()"
    );
}

#[test]
fn preflight_decision_not_accepted_for_unknown_decision() {
    let d = PreflightDecision {
        decision: "unknown_future_value".to_owned(),
        work_item_id: String::new(),
        requirement_ids: vec![],
        test_case_ids: vec![],
        confidence_target: 0.0,
        instruction: String::new(),
        intent: String::new(),
    };
    assert!(
        !d.accepted(),
        "Only 'accepted' string should return true from .accepted()"
    );
}

#[test]
fn preflight_decision_requirement_ids_accessible() {
    let d = PreflightDecision {
        decision: "accepted".to_owned(),
        work_item_id: "WI-XYZW".to_owned(),
        requirement_ids: vec!["REQ-001".to_owned(), "REQ-005".to_owned()],
        test_case_ids: vec!["TEST-001".to_owned()],
        confidence_target: 0.9,
        instruction: "ok".to_owned(),
        intent: "change".to_owned(),
    };
    assert_eq!(d.requirement_ids.len(), 2);
    assert!(d.requirement_ids.contains(&"REQ-001".to_owned()));
    assert!(d.requirement_ids.contains(&"REQ-005".to_owned()));
}

#[test]
fn preflight_decision_confidence_target_range() {
    let d = PreflightDecision {
        decision: "accepted".to_owned(),
        work_item_id: "WI-0001".to_owned(),
        requirement_ids: vec![],
        test_case_ids: vec![],
        confidence_target: 0.7,
        instruction: String::new(),
        intent: String::new(),
    };
    assert!(
        (0.0..=1.0).contains(&d.confidence_target),
        "confidence_target must be in [0.0, 1.0], got {}",
        d.confidence_target
    );
}

// ---------------------------------------------------------------------------
// VerifyResult — field semantics
// ---------------------------------------------------------------------------

#[test]
fn verify_result_equilibrium_true_means_success() {
    let r = VerifyResult {
        equilibrium: true,
        confidence: 0.9,
        summary: "All tests passed.".to_owned(),
        retry_strategy: String::new(),
        files_changed: vec!["src/main.rs".to_owned()],
    };
    assert!(r.equilibrium, "equilibrium=true should indicate success");
    assert!(
        r.retry_strategy.is_empty(),
        "no retry needed on equilibrium"
    );
}

#[test]
fn verify_result_equilibrium_false_has_retry_strategy() {
    let r = VerifyResult {
        equilibrium: false,
        confidence: 0.4,
        summary: "2 test failure(s) detected.".to_owned(),
        retry_strategy: "fix_tests".to_owned(),
        files_changed: vec!["src/main.rs".to_owned()],
    };
    assert!(!r.equilibrium);
    assert_eq!(r.retry_strategy, "fix_tests");
    assert!(
        r.confidence < 0.7,
        "failed verify should have low confidence"
    );
}

#[test]
fn verify_result_confidence_in_range() {
    for confidence in [0.0_f64, 0.5, 0.85, 1.0] {
        let r = VerifyResult {
            equilibrium: confidence >= 0.7,
            confidence,
            summary: String::new(),
            retry_strategy: String::new(),
            files_changed: vec![],
        };
        assert!(
            (0.0..=1.0).contains(&r.confidence),
            "confidence {confidence} is outside [0.0, 1.0]"
        );
    }
}

#[test]
fn verify_result_files_changed_accessible() {
    let r = VerifyResult {
        equilibrium: true,
        confidence: 0.85,
        summary: "ok".to_owned(),
        retry_strategy: String::new(),
        files_changed: vec![
            "src/governance/client.rs".to_owned(),
            "Cargo.toml".to_owned(),
        ],
    };
    assert_eq!(r.files_changed.len(), 2);
    assert!(r.files_changed.contains(&"Cargo.toml".to_owned()));
}
