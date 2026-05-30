//! Integration tests - deep liquidity protocol
//!
//! All tests are deterministic (no network, no randomness).
//!
//! Run with:
//! ```text
//! cargo test --test liquidity_tests
//! ```

use polar_bear_arc_forge_defi::{
    defi::DeepLiquidityProtocol,
    types::{LaunchConfig, LiquidityConfig, SolanaNetwork},
};
use pretty_assertions::assert_eq;

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

// ── Price and market cap ──────────────────────────────────────────────────────

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

// ── Token allocation ──────────────────────────────────────────────────────────

#[test]
fn tokens_in_pool_is_10_percent_of_supply() {
    let m = DeepLiquidityProtocol::compute(&cfg(10.0, true, 0));
    let expected = 100_000_000_000_000_u64;
    assert_eq!(m.tokens_in_pool, expected);
}

// ── Price impact ordering ─────────────────────────────────────────────────────

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

#[test]
fn ten_k_impact_exceeds_one_k_impact() {
    let m = DeepLiquidityProtocol::compute(&cfg(10.0, true, 0));
    assert!(m.price_large_buy_impact_usd_buy_pct > m.price_small_buy_impact_usd_buy_pct);
}

// ── Anti-rug ratings ──────────────────────────────────────────────────────────

#[test]
fn burn_deep_gets_diamond() {
    let m = DeepLiquidityProtocol::compute(&cfg(50.0, true, 0));
    assert!(
        m.anti_rug_rating.contains("DIAMOND"),
        "{}",
        m.anti_rug_rating
    );
}

#[test]
fn burn_shallow_gets_gold() {
    let m = DeepLiquidityProtocol::compute(&cfg(0.5, true, 0));
    assert!(m.anti_rug_rating.contains("GOLD"), "{}", m.anti_rug_rating);
}

#[test]
fn lock_180_days_deep_gets_silver() {
    let m = DeepLiquidityProtocol::compute(&cfg(50.0, false, 180));
    assert!(
        m.anti_rug_rating.contains("SILVER"),
        "{}",
        m.anti_rug_rating
    );
}

#[test]
fn lock_30_days_gets_bronze() {
    let m = DeepLiquidityProtocol::compute(&cfg(1.0, false, 30));
    assert!(
        m.anti_rug_rating.contains("BRONZE"),
        "{}",
        m.anti_rug_rating
    );
}

#[test]
fn no_burn_no_lock_gets_risky() {
    let m = DeepLiquidityProtocol::compute(&cfg(1.0, false, 0));
    assert!(m.anti_rug_rating.contains("RISKY"), "{}", m.anti_rug_rating);
}

// ── Depth score ───────────────────────────────────────────────────────────────

#[test]
fn depth_score_is_95_at_100_sol() {
    let m = DeepLiquidityProtocol::compute(&cfg(100.0, true, 0));
    assert_eq!(m.liquidity_depth_score, 95);
}

#[test]
fn depth_score_is_80_at_20_sol() {
    let m = DeepLiquidityProtocol::compute(&cfg(20.0, true, 0));
    assert_eq!(m.liquidity_depth_score, 80);
}

#[test]
fn depth_score_is_15_below_1_sol() {
    let m = DeepLiquidityProtocol::compute(&cfg(0.1, true, 0));
    assert_eq!(m.liquidity_depth_score, 15);
}
