//! Integration tests - deep liquidity protocol
//!
//! All tests are deterministic (no network, no randomness).
//!
//! Run with:
//! ```text
//! cargo test --test liquidity_tests
//! ```

/// Asserts that the `DeepLiquidityProtocol` correctly computes the number of tokens in the
/// pool.
///
/// This test uses the `safe_launch_config()` function to compute liquidity metrics and
/// verifies that the number of tokens in the pool matches the expected 10% allocation.
use polar_bear_arc_forge_defi::{
    defi::DeepLiquidityProtocol,
    types::{LaunchConfig, LiquidityConfig, SolanaNetwork},
};
/// Asserts that the `LaunchConfig` is valid for the `DeepLiquidityProtocol`.
///
/// This test verifies that the `LaunchConfig` is valid by creating a `DeepLiquidityProtocol`
/// instance and checking that it does not panic during initialization.
use pretty_assertions::assert_eq;

/// Helper function to create a `LaunchConfig` for testing.
///
/// This function creates a `LaunchConfig` with the specified liquidity parameters and default
/// values for the other fields.
///
/// # Arguments
///
/// * `sol` - The initial liquidity in SOL.
/// * `burn` - Whether to burn LP tokens.
/// * `lock_days` - The lock duration in days.
///
/// # Returns
///
/// A `LaunchConfig` with the specified liquidity parameters and default values for the other
/// fields.
fn cfg(sol: f64, burn: bool, lock_days: u32) -> LaunchConfig {
    LaunchConfig {
        token_name: "Test Token".to_string(),
        token_symbol: "TST".to_string(),
        total_supply: 1_000_000_000_000_000,
        decimals: 9,
        mint_authority_renounced: true,
        freeze_authority_renounced: true,
        liquidity: LiquidityConfig {
            initial_liquidity_sol: sol,
            token_allocation_pct: 10.0,
            burn_lp_tokens: burn,
            lock_duration_days: lock_days,
            price_range_lower: 0.0,
            price_range_upper: 0.0,
        },
        network: SolanaNetwork::Devnet,
    }
}

/// Test that the estimated initial price and market cap are positive.
///
/// This test verifies that the estimated initial price and market cap are positive after
/// computing liquidity metrics for a given configuration.
#[test]
fn price_and_mcap_are_positive() {
    let m = DeepLiquidityProtocol::compute(&cfg(10.0, true, 0));
    assert!(m.estimated_initial_price_usd > 0.0);
    assert!(m.estimated_market_cap_usd > 0.0);
}

#[test]
fn mcap_equals_price_times_total_supply() {
    let m = DeepLiquidityProtocol::compute(&cfg(10.0, true, 0));
    let expected = m.estimated_initial_price_usd * (1_000_000_000_000_000_f64 / 1_000_000_000_f64); // supply / 10^9 decimals
    let diff = (m.estimated_market_cap_usd - expected).abs();
    assert!(diff < 1.0, "market cap mismatch: {diff:.4}");
}

/// Test that the number of tokens in the pool is 10% of the total supply.
///
/// This test verifies that the number of tokens in the pool is 10% of the total supply after
/// computing liquidity metrics for a given configuration.
#[test]
fn tokens_in_pool_is_10_percent_of_supply() {
    let m = DeepLiquidityProtocol::compute(&cfg(10.0, true, 0));
    let expected = 100_000_000_000_000_u64;
    assert_eq!(m.tokens_in_pool, expected);
}

/// Test that deeper liquidity lowers price impact.
///
/// This test verifies that deeper liquidity (more tokens in the pool) lowers the price impact
/// for a $1 000 and $10 000 buy orders.
#[test]
fn deeper_liquidity_lowers_price_impact() {
    let shallow = DeepLiquidityProtocol::compute(&cfg(1.0, true, 0));
    let deep = DeepLiquidityProtocol::compute(&cfg(100.0, true, 0));
    assert!(
        shallow.price_small_buy_impact_usd_buy_pct > deep.price_small_buy_impact_usd_buy_pct,
        "shallow: {:.4}  deep: {:.4}",
        shallow.price_small_buy_impact_usd_buy_pct,
        deep.price_small_buy_impact_usd_buy_pct
    );
}

/// Test that the impact of a $10 000 buy order exceeds the impact of a $1 000 buy order.
///
/// This test verifies that the impact of a $10 000 buy order exceeds the impact of a $1 000
/// buy order after computing liquidity metrics for a given configuration.
#[test]
fn ten_k_impact_exceeds_one_k_impact() {
    let m = DeepLiquidityProtocol::compute(&cfg(10.0, true, 0));
    assert!(m.price_large_buy_impact_usd_buy_pct > m.price_small_buy_impact_usd_buy_pct);
}

/// Test that burn deep gets diamond anti-rug rating.
///
/// This test verifies that a deep liquidity configuration with token burning gets a diamond
/// anti-rug rating after computing liquidity metrics.
#[test]
fn burn_deep_gets_diamond() {
    let m = DeepLiquidityProtocol::compute(&cfg(50.0, true, 0));
    assert!(
        m.anti_rug_rating.contains("DIAMOND"),
        "{}",
        m.anti_rug_rating
    );
}

/// Test that burn shallow gets gold anti-rug rating.
///
/// This test verifies that a shallow liquidity configuration with token burning gets a gold
/// anti-rug rating after computing liquidity metrics.
#[test]
fn burn_shallow_gets_gold() {
    let m = DeepLiquidityProtocol::compute(&cfg(0.5, true, 0));
    assert!(m.anti_rug_rating.contains("GOLD"), "{}", m.anti_rug_rating);
}

/// Test that lock 180 days deep gets silver anti-rug rating.
///
/// This test verifies that a deep liquidity configuration with token locking gets a silver
/// anti-rug rating after computing liquidity metrics.
#[test]
fn lock_180_days_deep_gets_silver() {
    let m = DeepLiquidityProtocol::compute(&cfg(50.0, false, 180));
    assert!(
        m.anti_rug_rating.contains("SILVER"),
        "{}",
        m.anti_rug_rating
    );
}

/// Test that lock 30 days gets bronze anti-rug rating.
///
/// This test verifies that a shallow liquidity configuration with token locking gets a bronze
/// anti-rug rating after computing liquidity metrics.
#[test]
fn lock_30_days_gets_bronze() {
    let m = DeepLiquidityProtocol::compute(&cfg(1.0, false, 30));
    assert!(
        m.anti_rug_rating.contains("BRONZE"),
        "{}",
        m.anti_rug_rating
    );
}

/// Test that no burn, no lock gets risky anti-rug rating.
///
/// This test verifies that a liquidity configuration with no token burning and no locking gets
/// a risky anti-rug rating after computing liquidity metrics.
#[test]
fn no_burn_no_lock_gets_risky() {
    let m = DeepLiquidityProtocol::compute(&cfg(1.0, false, 0));
    assert!(m.anti_rug_rating.contains("RISKY"), "{}", m.anti_rug_rating);
}

/// Test that depth score is 95 at 100 SOL.
///
/// This test verifies that the liquidity depth score is 95 at 100 SOL after computing liquidity
/// metrics.
#[test]
fn depth_score_is_95_at_100_sol() {
    let m = DeepLiquidityProtocol::compute(&cfg(100.0, true, 0));
    assert_eq!(m.liquidity_depth_score, 95);
}

/// Test that depth score is 80 at 20 SOL.
///
/// This test verifies that the liquidity depth score is 80 at 20 SOL after computing liquidity
/// metrics.
#[test]
fn depth_score_is_80_at_20_sol() {
    let m = DeepLiquidityProtocol::compute(&cfg(20.0, true, 0));
    assert_eq!(m.liquidity_depth_score, 80);
}

/// Test that depth score is 15 below 1 SOL.
///
/// This test verifies that the liquidity depth score is 15 below 1 SOL after computing liquidity
/// metrics.
#[test]
fn depth_score_is_15_below_1_sol() {
    let m = DeepLiquidityProtocol::compute(&cfg(0.1, true, 0));
    assert_eq!(m.liquidity_depth_score, 15);
}
