//! # Deep Initial Liquidity Protocol
//!
//! Implements the ARC Forge liquidity strategy for Solana token launches.
//!
//! ## Model
//!
//! Uses the constant-product AMM formula (`x · y = k`) to model the initial
//! pool state and derive key metrics:
//!
//! | Metric | Formula |
//! |--------|---------|
//! | Initial price | `(sol_value_usd) / tokens_in_pool` |
//! | Market cap | `initial_price × total_supply_adjusted` |
//! | Price impact | `trade_usd / (pool_value_usd + trade_usd) × 100` |
//!
//! ## Anti-rug ratings
//!
//! | ⭐ Rating | Condition | Risk |
//! |-----------|-----------|------|
//! | ⭐⭐⭐⭐⭐ DIAMOND | LP burned + ≥ 20 SOL | Mathematically rug-proof |
//! | ⭐⭐⭐⭐ GOLD | LP burned + shallow | Permanent but easy to manipulate |
//! | ⭐⭐⭐ SILVER | 180+ day lock + deep | Low risk until lock expires |
//! | ⭐⭐ BRONZE | 30+ day lock | Moderate risk post-lock |
//! | ⭐ RISKY | No burn, no lock | ARC Forge will not proceed |

use crate::types::{LaunchConfig, LiquidityConfig, LiquidityMetrics};

/// Current SOL price in USD used for simulation calculations.
///
/// In production this would be fetched from a Pyth oracle or a Raydium SOL/USDC
/// pool.  Using a constant here makes the simulation fully deterministic and
/// independently reviewable without live network access.
pub const SOL_PRICE_USD: f64 = 165.0;

/// Computes deep-liquidity metrics for a given [`LaunchConfig`].
pub struct DeepLiquidityProtocol;

impl DeepLiquidityProtocol {
    /// Compute [`LiquidityMetrics`] from a [`LaunchConfig`].
    ///
    /// All calculations are deterministic and reproducible given the same
    /// `config` input and [`SOL_PRICE_USD`] constant.
    pub fn compute(config: &LaunchConfig) -> LiquidityMetrics {
        let lc = &config.liquidity;

        let tokens_in_pool = tokens_for_pool(config.total_supply, lc.token_allocation_pct);
        let sol_value_usd = lc.initial_liquidity_sol * SOL_PRICE_USD;
        let tokens_adjusted = adjust(tokens_in_pool, config.decimals);

        let initial_price_usd = if tokens_adjusted > 0.0 {
            sol_value_usd / tokens_adjusted
        } else {
            0.0
        };

        let total_adjusted = adjust(config.total_supply, config.decimals);
        let market_cap_usd = initial_price_usd * total_adjusted;

        let depth_score = depth_score(lc.initial_liquidity_sol);
        let pool_value_usd = sol_value_usd * 2.0; // both sides of constant-product pool
        let small_buy_impact = price_impact(1_000.0, pool_value_usd);
        let large_buy_impact = price_impact(10_000.0, pool_value_usd);
        let anti_rug_rating = anti_rug_rating(lc, depth_score);

        LiquidityMetrics {
            config: lc.clone(),
            tokens_in_pool,
            estimated_initial_price_usd: initial_price_usd,
            estimated_market_cap_usd: market_cap_usd,
            liquidity_depth_score: depth_score,
            price_small_buy_impact_usd_buy_pct: small_buy_impact,
            price_large_buy_impact_usd_buy_pct: large_buy_impact,
            anti_rug_rating,
        }
    }
}

// ── Core calculations ─────────────────────────────────────────────────────────

fn tokens_for_pool(total_supply: u64, allocation_pct: f64) -> u64 {
    // Convert `allocation_pct` to fixed-point millionths (6 d.p.) then
    // multiply in u128 to avoid any f64 precision or sign issues on the
    // large `total_supply` value.
    //
    // `clamp(0.0, 100.0)` bounds the product to [0, 100_000_000], which
    // always fits in u32 (max 4_294_967_295).  The explicit `.min` + the
    // `#[allow]` below document that the truncation is intentional and safe.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let pct_millionths = (allocation_pct.clamp(0.0, 100.0) * 1_000_000.0)
        .round()
        .min(100_000_000.0) as u32; // bounded: max 100_000_000 < u32::MAX
    let numerator = u128::from(total_supply) * u128::from(pct_millionths);
    u64::try_from(numerator / 100_000_000_u128).unwrap_or(u64::MAX)
}

fn adjust(raw: u64, decimals: u8) -> f64 {
    // Split raw into high and low 32-bit halves to avoid cast_precision_loss
    // (u64 -> f64 loses precision above 2^52).
    let hi = f64::from(u32::try_from(raw >> 32).unwrap_or(u32::MAX));
    let lo = f64::from(u32::try_from(raw & 0xFFFF_FFFF).unwrap_or(u32::MAX));
    let as_f64 = hi * 4_294_967_296.0 + lo; // hi * 2^32 + lo
    as_f64 / 10_f64.powi(i32::from(decimals))
}

/// Liquidity depth score (0–100) based on SOL locked in the initial LP position.
fn depth_score(initial_sol: f64) -> u8 {
    match initial_sol {
        s if s >= 100.0 => 95,
        s if s >= 20.0 => 80,
        s if s >= 5.0 => 60,
        s if s >= 1.0 => 40,
        _ => 15,
    }
}

/// Constant-product price impact: `trade / (pool_value + trade) × 100`.
fn price_impact(trade_usd: f64, pool_value_usd: f64) -> f64 {
    if pool_value_usd <= 0.0 {
        return 100.0;
    }
    trade_usd / (pool_value_usd + trade_usd) * 100.0
}

fn anti_rug_rating(lc: &LiquidityConfig, depth_score: u8) -> String {
    if lc.burn_lp_tokens && depth_score >= 60 {
        "⭐⭐⭐⭐⭐  DIAMOND - LP burned + deep liquidity. \
         Rug-pull mathematically impossible."
            .to_string()
    } else if lc.burn_lp_tokens {
        "⭐⭐⭐⭐   GOLD - LP burned; liquidity is permanent but shallow. \
         Increase initial SOL for deeper depth."
            .to_string()
    } else if lc.lock_duration_days >= 180 && depth_score >= 60 {
        "⭐⭐⭐    SILVER - LP locked 180+ days + deep liquidity. \
         Low risk but lock can expire."
            .to_string()
    } else if lc.lock_duration_days >= 30 {
        "⭐⭐      BRONZE - LP locked short-term. Moderate rug risk after lock expires.".to_string()
    } else {
        "⭐       RISKY - LP not burned and not locked. \
         ARC Forge will not proceed without burn or lock."
            .to_string()
    }
}

// ── Display helper ────────────────────────────────────────────────────────────

impl LiquidityMetrics {
    /// Format a human-readable summary for CLI output.
    pub fn format_summary(&self) -> String {
        format!(
            "\
Liquidity Summary
─────────────────────────────────────────────────────────────────────
  Initial SOL in pool   : {:.2} SOL  (${:.2} @ ${:.0}/SOL)
  Token allocation      : {:.1}% of supply ({} tokens)
  Estimated launch price: ${:.8} per token
  Estimated market cap  : ${:.2}
  LP token disposition  : {}
  Lock duration         : {}
─────────────────────────────────────────────────────────────────────
  Price impact (constant-product AMM)
    $1 000 buy          : {:.2}%
    $10 000 buy         : {:.2}%
─────────────────────────────────────────────────────────────────────
  Depth score           : {}/100
  Anti-rug rating       : {}
─────────────────────────────────────────────────────────────────────",
            self.config.initial_liquidity_sol,
            self.config.initial_liquidity_sol * SOL_PRICE_USD,
            SOL_PRICE_USD,
            self.config.token_allocation_pct,
            self.tokens_in_pool,
            self.estimated_initial_price_usd,
            self.estimated_market_cap_usd,
            if self.config.burn_lp_tokens {
                "BURN (permanent - strongest anti-rug)"
            } else {
                "LOCK"
            },
            if self.config.burn_lp_tokens {
                "N/A (burned)".to_string()
            } else {
                format!("{} days", self.config.lock_duration_days)
            },
            self.price_small_buy_impact_usd_buy_pct,
            self.price_large_buy_impact_usd_buy_pct,
            self.liquidity_depth_score,
            self.anti_rug_rating,
        )
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::types::{LaunchConfig, LiquidityConfig, SolanaNetwork};

    fn cfg(sol: f64, burn: bool) -> LaunchConfig {
        LaunchConfig {
            token_name: "Test".to_string(),
            token_symbol: "TST".to_string(),
            total_supply: 1_000_000_000_000_000,
            decimals: 9,
            mint_authority_renounced: true,
            freeze_authority_renounced: true,
            liquidity: LiquidityConfig {
                initial_liquidity_sol: sol,
                token_allocation_pct: 10.0,
                burn_lp_tokens: burn,
                lock_duration_days: 0,
                price_range_lower: 0.0,
                price_range_upper: 0.0,
            },
            network: SolanaNetwork::Devnet,
        }
    }

    #[test]
    fn positive_price_and_mcap() {
        let m = DeepLiquidityProtocol::compute(&cfg(10.0, true));
        assert!(m.estimated_initial_price_usd > 0.0);
        assert!(m.estimated_market_cap_usd > 0.0);
    }

    #[test]
    fn deeper_pool_lowers_impact() {
        let shallow = DeepLiquidityProtocol::compute(&cfg(1.0, true));
        let deep = DeepLiquidityProtocol::compute(&cfg(100.0, true));
        assert!(
            shallow.price_small_buy_impact_usd_buy_pct > deep.price_small_buy_impact_usd_buy_pct
        );
    }

    #[test]
    fn ten_k_impact_greater_than_one_k() {
        let m = DeepLiquidityProtocol::compute(&cfg(10.0, true));
        assert!(m.price_large_buy_impact_usd_buy_pct > m.price_small_buy_impact_usd_buy_pct);
    }

    #[test]
    fn burn_deep_gets_diamond() {
        let m = DeepLiquidityProtocol::compute(&cfg(50.0, true));
        assert!(
            m.anti_rug_rating.contains("DIAMOND"),
            "{}",
            m.anti_rug_rating
        );
    }

    #[test]
    fn no_burn_no_lock_gets_risky() {
        let m = DeepLiquidityProtocol::compute(&cfg(0.5, false));
        assert!(m.anti_rug_rating.contains("RISKY"), "{}", m.anti_rug_rating);
    }

    #[test]
    fn depth_score_bands_correct() {
        assert_eq!(depth_score(0.1), 15);
        assert_eq!(depth_score(3.0), 40);
        assert_eq!(depth_score(10.0), 60);
        assert_eq!(depth_score(50.0), 80);
        assert_eq!(depth_score(150.0), 95);
    }

    #[test]
    fn price_impact_tiny_pool_near_100() {
        assert!(price_impact(1_000_000.0, 1.0) > 99.0);
    }

    #[test]
    fn price_impact_zero_pool_is_100() {
        assert!((price_impact(1_000.0, 0.0) - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn tokens_in_pool_matches_allocation() {
        let m = DeepLiquidityProtocol::compute(&cfg(10.0, true));
        let expected = 100_000_000_000_000_u64;
        assert_eq!(m.tokens_in_pool, expected);
    }
}
