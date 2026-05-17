//! Integration tests - token validator
//!
//! All tests run without network access.  [`MintInfo`] fixtures are
//! constructed in-process and passed directly to [`TokenValidator::validate_mint_info`].
//!
//! Run with:
//! ```text
//! cargo test --test validator_tests
//! ```

use polar_bear_arc_forge_defi::{
    types::{MintInfo, ValidationStatus},
    validator::TokenValidator,
};
use pretty_assertions::assert_eq;

fn validator() -> TokenValidator {
    TokenValidator::new("https://api.devnet.solana.com")
}

fn safe_mint() -> MintInfo {
    MintInfo {
        address: "SafeMint111111111111111111111111111111111".to_string(),
        supply: 1_000_000_000_000_000,
        decimals: 9,
        is_initialized: true,
        mint_authority: None,
        freeze_authority: None,
    }
}

// ── Happy path ────────────────────────────────────────────────────────────────

#[test]
fn safe_mint_all_checks_pass() {
    let report = validator().validate_mint_info(&safe_mint());
    assert_eq!(report.overall_status, ValidationStatus::Safe);
    assert_eq!(report.risk_score, 0);
    assert!(
        report.checks.iter().all(|c| c.passed),
        "all checks must pass"
    );
    assert!(report.recommendation.contains("safe to launch"));
}

// ── Freeze authority ──────────────────────────────────────────────────────────

#[test]
fn freeze_authority_set_is_dangerous() {
    let mut mint = safe_mint();
    mint.freeze_authority = Some("FreezeKey1111111111111111111111111111111".to_string());
    let report = validator().validate_mint_info(&mint);

    assert_eq!(report.overall_status, ValidationStatus::Dangerous);
    assert!(report.risk_score >= 30);

    let check = report
        .checks
        .iter()
        .find(|c| c.name == "Freeze Authority")
        .unwrap();
    assert!(!check.passed);
    assert_eq!(check.status, ValidationStatus::Dangerous);
}

#[test]
fn freeze_authority_none_passes() {
    let report = validator().validate_mint_info(&safe_mint());
    let check = report
        .checks
        .iter()
        .find(|c| c.name == "Freeze Authority")
        .unwrap();
    assert!(check.passed);
    assert_eq!(check.status, ValidationStatus::Safe);
}

// ── Mint authority ────────────────────────────────────────────────────────────

#[test]
fn mint_authority_set_is_warning() {
    let mut mint = safe_mint();
    mint.mint_authority = Some("MintKey11111111111111111111111111111111".to_string());
    let report = validator().validate_mint_info(&mint);

    let check = report
        .checks
        .iter()
        .find(|c| c.name == "Mint Authority")
        .unwrap();
    assert!(!check.passed);
    assert_eq!(check.status, ValidationStatus::Warning);
}

// ── Zero supply guard ─────────────────────────────────────────────────────────

#[test]
fn zero_supply_with_mint_auth_is_stealth_mint() {
    let mut mint = safe_mint();
    mint.supply = 0;
    mint.mint_authority = Some("StealthKey11111111111111111111111111111".to_string());
    let report = validator().validate_mint_info(&mint);

    let check = report
        .checks
        .iter()
        .find(|c| c.name == "Zero Supply Guard")
        .unwrap();
    assert!(!check.passed);
    assert_eq!(check.status, ValidationStatus::Dangerous);
}

#[test]
fn zero_supply_without_mint_auth_is_warning() {
    let mut mint = safe_mint();
    mint.supply = 0;
    let report = validator().validate_mint_info(&mint);

    let check = report
        .checks
        .iter()
        .find(|c| c.name == "Zero Supply Guard")
        .unwrap();
    assert!(!check.passed);
    assert_eq!(check.status, ValidationStatus::Warning);
}

// ── Decimals sanity ───────────────────────────────────────────────────────────

#[test]
fn zero_decimals_is_dangerous() {
    let mut mint = safe_mint();
    mint.decimals = 0;
    let report = validator().validate_mint_info(&mint);

    let check = report
        .checks
        .iter()
        .find(|c| c.name == "Decimals Sanity")
        .unwrap();
    assert!(!check.passed);
    assert_eq!(check.status, ValidationStatus::Dangerous);
}

#[test]
fn decimals_6_to_9_are_safe() {
    for d in 6..=9 {
        let mut mint = safe_mint();
        mint.decimals = d;
        let report = validator().validate_mint_info(&mint);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "Decimals Sanity")
            .unwrap();
        assert_eq!(
            check.status,
            ValidationStatus::Safe,
            "decimals={d} must be Safe"
        );
    }
}

// ── Risk score ────────────────────────────────────────────────────────────────

#[test]
fn risk_score_caps_at_100() {
    let mint = MintInfo {
        address: "x".to_string(),
        supply: 0,
        decimals: 0,
        is_initialized: false,
        mint_authority: Some("x".to_string()),
        freeze_authority: Some("x".to_string()),
    };
    let report = validator().validate_mint_info(&mint);
    assert!(report.risk_score <= 100, "risk score must never exceed 100");
}

#[test]
fn overall_dangerous_when_score_above_20() {
    let mut mint = safe_mint();
    mint.freeze_authority = Some("FreezeKey".to_string());
    mint.mint_authority = Some("MintKey".to_string());
    let report = validator().validate_mint_info(&mint);
    assert_eq!(report.overall_status, ValidationStatus::Dangerous);
}

// ── JSON serialisation ────────────────────────────────────────────────────────

#[test]
fn report_serialises_to_json() {
    let report = validator().validate_mint_info(&safe_mint());
    let json = serde_json::to_string_pretty(&report).expect("serialise");
    assert!(json.contains("\"overall_status\""));
    assert!(json.contains("\"risk_score\""));
    assert!(json.contains("\"checks\""));
    // Round-trip
    let back: polar_bear_arc_forge_defi::ValidationReport =
        serde_json::from_str(&json).expect("deserialise");
    assert_eq!(back.risk_score, report.risk_score);
}
