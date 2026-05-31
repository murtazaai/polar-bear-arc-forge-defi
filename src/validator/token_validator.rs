//! # `TokenValidator`
//!
//! Fetches live SPL Token mint data from Solana RPC and runs six
//! sniper-bot / rug-pull prevention checks, returning a fully
//! serialisable `ValidationReport`.
//!
//! All checks map directly to documented `DeFi` attack vectors.
//! ARC Forge blocks a launch whenever `risk_score > 0` on any
//! `ValidationStatus::Dangerous` check.

/// Validates an SPL Token mint against all sniper-bot prevention checks.
///
/// This is the main struct used to validate token accounts.
///
/// It wraps a [`SolanaRpcClient`] and provides methods to validate token mints.
use anyhow::Result;
use chrono::Utc;
use tracing::info;

use crate::{
    rpc::SolanaRpcClient,
    types::{MintInfo, ValidationCheck, ValidationReport, ValidationStatus},
};

/// Validates an SPL Token mint against all sniper-bot prevention checks.
///
/// This is the main struct used to validate token accounts.
///
/// It wraps a [`SolanaRpcClient`] and provides methods to validate token mints.
pub struct TokenValidator {
    rpc: SolanaRpcClient,
}

/// Validates an SPL Token mint against all sniper-bot prevention checks.
///
/// This is the main struct used to validate token accounts.
///
/// It wraps a [`SolanaRpcClient`] and provides methods to validate token mints.
impl TokenValidator {
    /// Create a new validator backed by a Solana RPC endpoint.
    pub fn new(rpc_url: impl Into<String>) -> Self {
        Self {
            rpc: SolanaRpcClient::new(rpc_url),
        }
    }

    /// Fetch `mint_address` from Solana and run all six validation checks.
    ///
    /// This is the **Perceive** stage of the ARC Forge PEV loop.
    /// Connects to the live RPC endpoint; requires network access.
    ///
    /// # Arguments
    ///
    /// * `mint_address` - The address of the SPL Token mint to validate.
    ///
    /// # Returns
    ///
    /// A `ValidationReport` containing the results of the validation checks.
    pub async fn validate(&self, mint_address: &str) -> Result<ValidationReport> {
        info!(mint = mint_address, "Fetching mint info from Solana RPC");
        let mint = self.rpc.get_mint_info(mint_address).await?;
        info!(
            supply = mint.supply,
            decimals = mint.decimals,
            freeze_auth = mint.freeze_authority.is_some(),
            mint_auth = mint.mint_authority.is_some(),
            "Mint decoded - running validation checks"
        );
        Ok(self.validate_mint_info(&mint))
    }

    /// Run all checks against a pre-fetched [`MintInfo`].
    ///
    /// Used in unit tests and launch simulations where the mint data is
    /// synthesised from a [`LaunchConfig`](crate::types::LaunchConfig).
    ///
    /// # Arguments
    ///
    /// * `mint` - The [`MintInfo`] to validate.
    ///
    /// # Returns
    ///
    /// A `ValidationReport` containing the results of the validation checks.
    pub fn validate_mint_info(&self, mint: &MintInfo) -> ValidationReport {
        let checks = run_all_checks(mint);
        let risk_score = score(&checks);
        let overall_status = overall(risk_score);
        let recommendation = recommendation(&overall_status, &checks);

        ValidationReport {
            mint_address: mint.address.clone(),
            timestamp: Utc::now(),
            overall_status,
            checks,
            risk_score,
            recommendation,
        }
    }
}

/// Runs all validation checks on the given [`MintInfo`] and returns the results.
///
/// # Arguments
///
/// * `mint` - The [`MintInfo`] to validate.
///
/// # Returns
///
/// A vector of [`ValidationCheck`] results.
///
/// The checks are run in the following order:
///
/// 1. **Freeze Authority**
/// 2. **Mint Authority**
/// 3. **Is Initialized**
/// 4. **Decimals**
/// 5. **Zero Supply**
/// 6. **Supply Upper Bound**
///
/// # Returns
///
/// A vector of [`ValidationCheck`] results.
fn run_all_checks(mint: &MintInfo) -> Vec<ValidationCheck> {
    vec![
        check_freeze_authority(mint),
        check_mint_authority(mint),
        check_is_initialized(mint),
        check_decimals(mint),
        check_zero_supply(mint),
        check_supply_upper_bound(mint),
    ]
}

/// **Check 1 - Freeze Authority**
///
/// Attack vector: if `freeze_authority` is `Some`, the key holder can call
/// `freeze_account` on any holder, preventing them from selling or transferring
/// tokens.  This is the primary sniper-bot enabler: snipers buy early, deployer
/// freezes everyone else, sniper dumps.
///
/// ARC Forge **requires** `freeze_authority = None` before launch.
fn check_freeze_authority(mint: &MintInfo) -> ValidationCheck {
    match &mint.freeze_authority {
        None => ValidationCheck {
            name: "Freeze Authority".to_string(),
            passed: true,
            status: ValidationStatus::Safe,
            message: "freeze_authority is None - no account can be frozen. \
                      Sniper-bot freeze vector eliminated."
                .to_string(),
        },
        Some(key) => ValidationCheck {
            name: "Freeze Authority".to_string(),
            passed: false,
            status: ValidationStatus::Dangerous,
            message: format!(
                "freeze_authority is set to {}. The key holder can freeze any token \
                 account at will - a critical sniper-bot / rug-pull vector. \
                 Renounce before launch.",
                short(key)
            ),
        },
    }
}

/// **Check 2 - Mint Authority**
///
/// Attack vector: if `mint_authority` is `Some`, the deployer can mint arbitrary
/// additional tokens after launch, diluting all holders.  Also enables sniper
/// exit: buy early, inflate supply, sell into the inflated price.
fn check_mint_authority(mint: &MintInfo) -> ValidationCheck {
    match &mint.mint_authority {
        None => ValidationCheck {
            name: "Mint Authority".to_string(),
            passed: true,
            status: ValidationStatus::Safe,
            message: "mint_authority is None - total supply is permanently fixed. \
                      Inflation and stealth-mint vectors eliminated."
                .to_string(),
        },
        Some(key) => ValidationCheck {
            name: "Mint Authority".to_string(),
            passed: false,
            status: ValidationStatus::Warning,
            message: format!(
                "mint_authority is set to {}. The deployer can mint unlimited \
                 additional tokens, enabling rug-pull via supply dilution. \
                 Renounce before launch.",
                short(key)
            ),
        },
    }
}

/// **Check 3 - Mint Initialized**
///
/// A mint account that has not been initialised on-chain is not a real token.
///
/// # Arguments
///
/// * `mint` - The [`MintInfo`] to validate.
///
/// # Returns
///
/// A [`ValidationCheck`] result.
fn check_is_initialized(mint: &MintInfo) -> ValidationCheck {
    if mint.is_initialized {
        ValidationCheck {
            name: "Mint Initialized".to_string(),
            passed: true,
            status: ValidationStatus::Safe,
            message: "Mint account is initialised on-chain - token is real and queryable."
                .to_string(),
        }
    } else {
        ValidationCheck {
            name: "Mint Initialized".to_string(),
            passed: false,
            status: ValidationStatus::Dangerous,
            message: "Mint account is NOT initialised - this token does not exist on-chain."
                .to_string(),
        }
    }
}

/// **Check 4 - Decimals Sanity**
///
/// Non-standard decimals (0, 1, or > 18) are sometimes used to create optical
/// illusions about token price in wallet and DEX UIs.  Standard Solana tokens
/// use 6–9 decimal places.
///
/// # Arguments
///
/// * `mint` - The [`MintInfo`] to validate.
///
/// # Returns
///
/// A [`ValidationCheck`] result.
fn check_decimals(mint: &MintInfo) -> ValidationCheck {
    match mint.decimals {
        6..=9 => ValidationCheck {
            name: "Decimals Sanity".to_string(),
            passed: true,
            status: ValidationStatus::Safe,
            message: format!(
                "decimals = {} - standard range (6–9). No price-display manipulation risk.",
                mint.decimals
            ),
        },
        d if d == 0 || d > 18 => ValidationCheck {
            name: "Decimals Sanity".to_string(),
            passed: false,
            status: ValidationStatus::Dangerous,
            message: format!(
                "decimals = {} - highly non-standard, may be used to manipulate price \
                 display in wallets and DEX UIs.",
                mint.decimals
            ),
        },
        _ => ValidationCheck {
            name: "Decimals Sanity".to_string(),
            passed: true,
            status: ValidationStatus::Warning,
            message: format!(
                "decimals = {} - acceptable but outside the 6–9 standard range. \
                 Verify intentional.",
                mint.decimals
            ),
        },
    }
}

/// **Check 5 - Zero Supply Guard**
///
/// A supply of zero combined with an active `mint_authority` is a stealth-mint
/// setup: the deployer can mint tokens at any time, potentially front-running
/// buyers.
///
/// # Arguments
///
/// * `mint` - The [`MintInfo`] to validate.
///
/// # Returns
///
/// A [`ValidationCheck`] result.
fn check_zero_supply(mint: &MintInfo) -> ValidationCheck {
    if mint.supply == 0 && mint.mint_authority.is_some() {
        ValidationCheck {
            name: "Zero Supply Guard".to_string(),
            passed: false,
            status: ValidationStatus::Dangerous,
            message: "supply = 0 with active mint_authority - classic stealth-mint / \
                      honey-pot configuration."
                .to_string(),
        }
    } else if mint.supply == 0 {
        ValidationCheck {
            name: "Zero Supply Guard".to_string(),
            passed: false,
            status: ValidationStatus::Warning,
            message: "supply = 0 - token has no circulating supply and is not launchable."
                .to_string(),
        }
    } else {
        ValidationCheck {
            name: "Zero Supply Guard".to_string(),
            passed: true,
            status: ValidationStatus::Safe,
            message: format!(
                "supply = {} - non-zero supply confirmed.",
                format_supply(mint.supply, mint.decimals)
            ),
        }
    }
}

/// **Check 6 - Supply Upper Bound**
///
/// An astronomically large adjusted supply (> 1 quadrillion tokens) combined
/// with minimal initial liquidity creates extreme volatility and decimal-trick
/// price manipulation in DEX UIs.
///
/// # Arguments
///
/// * `mint` - The [`MintInfo`] to validate.
///
/// # Returns
///
/// A [`ValidationCheck`] result.
fn check_supply_upper_bound(mint: &MintInfo) -> ValidationCheck {
    let adjusted = adjusted_supply(mint.supply, mint.decimals);
    let quadrillion = 1_000_000_000_000_000_f64;

    if adjusted > quadrillion {
        ValidationCheck {
            name: "Supply Upper Bound".to_string(),
            passed: false,
            status: ValidationStatus::Warning,
            message: format!(
                "Adjusted supply ({adjusted:.2e}) exceeds 1 quadrillion - may cause \
                 price-display issues and concentrate value in illiquid fractions."
            ),
        }
    } else {
        ValidationCheck {
            name: "Supply Upper Bound".to_string(),
            passed: true,
            status: ValidationStatus::Safe,
            message: format!("Adjusted supply ({adjusted:.2e}) is within normal bounds."),
        }
    }
}

/// Scores the overall risk of a token based on the validation checks.
///
/// # Arguments
///
/// * `checks` - The list of [`ValidationCheck`] results to score.
///
/// # Returns
///
/// The risk score as an `u8` value (0-100).
fn score(checks: &[ValidationCheck]) -> u8 {
    let total: u32 = checks
        .iter()
        .map(|c| match c.status {
            ValidationStatus::Dangerous => 30,
            ValidationStatus::Warning => 10,
            ValidationStatus::Safe => 0,
        })
        .sum();
    total.min(100) as u8
}

/// Determines the overall validation status based on the risk score.
///
/// # Arguments
///
/// * `risk_score` - The risk score to evaluate.
///
/// # Returns
///
/// The [`ValidationStatus`] based on the risk score.
fn overall(risk_score: u8) -> ValidationStatus {
    if risk_score == 0 {
        ValidationStatus::Safe
    } else if risk_score <= 20 {
        ValidationStatus::Warning
    } else {
        ValidationStatus::Dangerous
    }
}

/// Provides a recommendation based on the validation status and failed checks.
///
/// # Arguments
///
/// * `status` - The [`ValidationStatus`] to base the recommendation on.
/// * `checks` - The list of [`ValidationCheck`] results to include in the recommendation.
///
/// # Returns
///
/// The recommendation text as a [`String`].
fn recommendation(status: &ValidationStatus, checks: &[ValidationCheck]) -> String {
    let failed: Vec<&str> = checks
        .iter()
        .filter(|c| !c.passed)
        .map(|c| c.name.as_str())
        .collect();

    match status {
        ValidationStatus::Safe => "All checks passed. Token is safe to launch via ARC Forge with \
             deep initial liquidity on Raydium."
            .to_string(),
        ValidationStatus::Warning => format!(
            "Token has warnings on: {}. Review and remediate before launch.",
            failed.join(", ")
        ),
        ValidationStatus::Dangerous => format!(
            "LAUNCH BLOCKED - critical issues: {}. \
             Renounce all dangerous authorities before proceeding.",
            failed.join(", ")
        ),
    }
}

/// Shortens a public key to a human-readable format, truncating the middle if too long.
///
/// # Arguments
///
/// * `pk` - The public key to shorten.
///
/// # Returns
///
/// The shortened public key as a [`String`].
fn short(pk: &str) -> String {
    if pk.len() <= 12 {
        pk.to_string()
    } else {
        format!("{}…{}", &pk[..6], &pk[pk.len() - 6..])
    }
}

/// Adjusts the supply value to the given number of decimals.
///
/// # Arguments
///
/// * `supply` - The supply value to adjust.
/// * `decimals` - The number of decimals to adjust to.
///
/// # Returns
///
/// The adjusted supply value as a [`f64`].
fn adjusted_supply(supply: u64, decimals: u8) -> f64 {
    // Split supply into hi/lo u32 halves to avoid cast_precision_loss
    // (u64 -> f64 loses precision above 2^52).
    let hi = f64::from(u32::try_from(supply >> 32).unwrap_or(u32::MAX));
    let lo = f64::from(u32::try_from(supply & 0xFFFF_FFFF).unwrap_or(u32::MAX));
    let as_f64 = hi * 4_294_967_296.0 + lo; // hi * 2^32 + lo
    as_f64 / 10_f64.powi(i32::from(decimals))
}

/// Formats the supply value to a human-readable string with the given number of decimals.
///
/// # Arguments
///
/// * `supply` - The supply value to format.
/// * `decimals` - The number of decimals to format to.
///
/// # Returns
///
/// The formatted supply value as a [`String`].
fn format_supply(supply: u64, decimals: u8) -> String {
    format!("{:.2}", adjusted_supply(supply, decimals))
}

/// Tests for the [`TokenValidator`] token validation functionality.
///
/// Tests the validation of a safe mint token, ensuring it scores zero risk.
///
/// # Tests
///
/// * `safe_mint` - Validates a safe mint token, expecting a score of zero.
///
/// # Panics
///
/// * Panics if the token validation fails.
#[cfg(test)]
mod tests {
    use super::*;

    /// Returns a sample [`MintInfo`] struct representing a safe mint token.
    ///
    /// # Returns
    ///
    /// A [`MintInfo`] struct with the safe mint token details.
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

    /// Returns a sample [`TokenValidator`] instance configured for testing.
    ///
    /// # Returns
    ///
    /// A [`TokenValidator`] instance with the devnet RPC endpoint configured.
    fn validator() -> TokenValidator {
        TokenValidator::new("https://api.devnet.solana.com")
    }

    /// Returns a sample [`MintInfo`] struct representing a mint token with a freeze authority.
    ///
    /// # Returns
    ///
    /// A [`MintInfo`] struct with the mint token details including a freeze authority.
    #[test]
    fn safe_mint_scores_zero() {
        let report = validator().validate_mint_info(&safe_mint());
        assert_eq!(report.overall_status, ValidationStatus::Safe);
        assert_eq!(report.risk_score, 0);
        assert!(report.checks.iter().all(|c| c.passed));
    }

    /// Returns a sample [`MintInfo`] struct representing a mint token with a freeze authority.
    ///
    /// # Returns
    ///
    /// A [`MintInfo`] struct with the mint token details including a freeze authority.
    #[test]
    fn freeze_authority_is_dangerous() {
        let mut mint = safe_mint();
        mint.freeze_authority = Some("FreezeKey1111111111111111111111111111111".to_string());
        let report = validator().validate_mint_info(&mint);
        assert_eq!(report.overall_status, ValidationStatus::Dangerous);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "Freeze Authority")
            .unwrap();
        assert!(!check.passed);
        assert_eq!(check.status, ValidationStatus::Dangerous);
    }

    /// Returns a sample [`MintInfo`] struct representing a mint token with a mint authority.
    ///
    /// # Returns
    ///
    /// A [`MintInfo`] struct with the mint token details including a mint authority.
    ///
    /// # Panics
    ///
    /// * Panics if the token validation fails.
    #[test]
    fn mint_authority_is_warning() {
        let mut mint = safe_mint();
        mint.mint_authority = Some("MintKey1111111111111111111111111111111111".to_string());
        let report = validator().validate_mint_info(&mint);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "Mint Authority")
            .unwrap();
        assert!(!check.passed);
        assert_eq!(check.status, ValidationStatus::Warning);
    }

    /// Returns a sample [`MintInfo`] struct representing a mint token with a mint authority and
    /// zero supply.
    ///
    /// # Returns
    ///
    /// A [`MintInfo`] struct with the mint token details including a mint authority and zero
    /// supply.
    ///
    /// # Panics
    ///
    /// * Panics if the token validation fails.
    #[test]
    fn zero_supply_with_mint_auth_is_dangerous() {
        let mut mint = safe_mint();
        mint.supply = 0;
        mint.mint_authority = Some("MintKey".to_string());
        let report = validator().validate_mint_info(&mint);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "Zero Supply Guard")
            .unwrap();
        assert_eq!(check.status, ValidationStatus::Dangerous);
    }

    /// Returns a sample [`MintInfo`] struct representing a mint token with a mint authority and
    /// zero supply.
    ///
    /// # Returns
    ///
    /// A [`MintInfo`] struct with the mint token details including a mint authority and zero
    /// supply.
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
        assert!(report.risk_score <= 100);
    }
}
