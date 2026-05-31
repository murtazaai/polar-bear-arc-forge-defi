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
/// Creates a [`TokenValidator`] instance configured to use the Solana devnet API.
use pretty_assertions::assert_eq;

/// Returns a [`TokenValidator`] instance configured to use the Solana devnet API.
///
/// This function is used to create a [`TokenValidator`] instance that is configured to use the
/// Solana devnet API.  The validator is returned so that tests can use it to validate mint
/// information.
fn validator() -> TokenValidator {
    TokenValidator::new("https://api.devnet.solana.com")
}

/// Returns a [`MintInfo`] instance representing a safe mint account.
///
/// This function is used to create a [`MintInfo`] instance that represents a safe mint account.
/// The mint info is returned so that tests can use it to validate mint information.
///
/// The mint address is hardcoded to `SafeMint111111111111111111111111111111111`.
///
/// The mint supply is set to `1_000_000_000_000_000` tokens, with 9 decimal places.
///
/// The mint authority is set to `None`, indicating that no authority can mint additional tokens.
/// The freeze authority is also set to `None`, indicating that no authority can freeze any holder
/// account. The mint account is marked as initialized, but no authority is set for minting or
/// freezing.
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

/// All checks pass for a safe mint account.
///
/// The mint supply is set to `1_000_000_000_000_000` tokens, with 9 decimal places.
/// The mint authority is set to `None`, indicating that no authority can mint additional tokens.
/// The freeze authority is also set to `None`, indicating that no authority can freeze any holder
/// account. The mint account is marked as initialized, but no authority is set for minting or
/// freezing.
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

/// The freeze authority is set to a known dangerous key, so the mint is marked as dangerous.
/// The freeze authority is set to `FreezeKey1111111111111111111111111111111`, which is a well-known
/// dangerous key that can freeze any holder account.
/// The mint is marked as dangerous because the freeze authority is set to a dangerous key.
/// The freeze authority is set to `None`, indicating that no authority can freeze any holder
/// account.
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

/// The mint authority is set to a known dangerous key, so the mint is marked as dangerous.
/// The mint authority is set to `MintKey1111111111111111111111111111111`, which is a well-known
/// dangerous key that can mint additional tokens.
/// The mint authority is set to `None`, indicating that no authority can mint additional tokens.
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

/// The mint authority is set to a known dangerous key, so the mint is marked as dangerous.
/// The mint authority is set to `MintKey1111111111111111111111111111111`, which is a well-known
/// dangerous key that can mint additional tokens.
/// The mint authority is set to `None`, indicating that no authority can mint additional tokens.
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

/// The mint authority is set to a known dangerous key, so the mint is marked as dangerous.
/// The mint authority is set to `StealthKey11111111111111111111111111111`, which is a well-known
/// dangerous key that can mint additional tokens.
/// The mint authority is set to `None`, indicating that no authority can mint additional tokens.
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

/// The mint authority is not set, so the mint is marked as dangerous.
/// The mint authority is set to `None`, indicating that no authority can mint additional tokens.
/// The mint supply is set to `0`, indicating that no tokens can be minted.
/// The mint is marked as dangerous because the mint supply is `0` and the mint authority is `None`.
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

/// The decimals are set to `0`, indicating that the mint is using the smallest unit (pre-decimal
/// adjustment). The decimals are set to `0`, indicating that the mint is using the smallest unit
/// (pre-decimal adjustment). The mint is marked as dangerous because the decimals are `0`,
/// indicating that the mint is not using the standard Solana token decimals.
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

/// The decimals are set to a value between `6` and `9`, indicating that the mint is using the
/// standard Solana token decimals. The decimals are set to a value between `6` and `9`,
/// indicating that the mint is using the standard Solana token decimals. The mint is marked as
/// safe because the decimals are within the standard Solana token decimals range.
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

/// The risk score is capped at `100`, so any score above `100` is treated as `100`.
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

/// The overall status is marked as `Dangerous` when the risk score is above `20`.
/// The mint is marked as dangerous because the risk score exceeds `20`, indicating that the mint
/// is using a non-standard Solana token decimals value and has a high freeze/mint authority risk.
#[test]
fn overall_dangerous_when_score_above_20() {
    let mut mint = safe_mint();
    mint.freeze_authority = Some("FreezeKey".to_string());
    mint.mint_authority = Some("MintKey".to_string());
    let report = validator().validate_mint_info(&mint);
    assert_eq!(report.overall_status, ValidationStatus::Dangerous);
}

/// The report is serialised to JSON, ensuring that the `overall_status` and `risk_score` fields
/// are present in the JSON output.
///
/// The JSON output is deserialised back into a `ValidationReport` struct, ensuring that the
/// `risk_score` field is correctly preserved.
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
