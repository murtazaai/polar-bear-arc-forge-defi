//! tests/integration.rs
//! ─────────────────────────────────────────────────────────────────────────────
//! Integration tests for the ARC Forge DeFi platform.
//!
//! These tests run without any network calls (no Solana RPC, no Raydium API).
//! They validate the core business logic: PEV loop simulation, sniper-bot
//! prevention scoring, liquidity metrics, and JSON serialisation.
//!
//! Run with:   cargo test
//! Run single: cargo test test_full_launch_simulation -- --nocapture
//! ─────────────────────────────────────────────────────────────────────────────

use polar_bear_arc_forge_defi::{
    defi::liquidity::DeepLiquidityProtocol,
    forge::ArcForgeLauncher,
    types::{LaunchConfig, LiquidityConfig, MintInfo, SolanaNetwork, ValidationStatus},
    validator::TokenValidator,
};

// ─── Fixtures ─────────────────────────────────────────────────────────────────

/// A fully safe launch configuration - should score >= 80 readiness.
fn safe_launch_config() -> LaunchConfig {
    LaunchConfig {
        token_name: "Polar Bear Token".to_string(),
        token_symbol: "PBT".to_string(),
        total_supply: 1_000_000_000_000_000, // 1M PBT at 9 decimals
        decimals: 9,
        mint_authority_renounced: true,
        freeze_authority_renounced: true,
        liquidity: LiquidityConfig {
            initial_liquidity_sol: 50.0,
            token_allocation_pct: 10.0,
            burn_lp_tokens: true,
            lock_duration_days: 0,
            price_range_lower: 0.0,
            price_range_upper: 0.0,
        },
        network: SolanaNetwork::Devnet,
    }
}

/// A dangerous launch config - both authorities retained, no LP safety.
fn dangerous_launch_config() -> LaunchConfig {
    LaunchConfig {
        token_name: "Rug Pull Token".to_string(),
        token_symbol: "RUG".to_string(),
        total_supply: 1_000_000_000_000_000,
        decimals: 9,
        mint_authority_renounced: false, // ← DANGEROUS: can inflate supply
        freeze_authority_renounced: false, // ← DANGEROUS: can freeze accounts
        liquidity: LiquidityConfig {
            initial_liquidity_sol: 0.5, // ← Too shallow
            token_allocation_pct: 10.0,
            burn_lp_tokens: false, // ← LP not burned
            lock_duration_days: 0, // ← LP not locked
            price_range_lower: 0.0,
            price_range_upper: 0.0,
        },
        network: SolanaNetwork::Devnet,
    }
}

// ─── Simulation tests ─────────────────────────────────────────────────────────

#[test]
fn test_full_launch_simulation_safe_config() {
    let launcher = ArcForgeLauncher::new("https://api.devnet.solana.com");
    let sim = launcher.simulate_planned_launch(safe_launch_config());

    // Core safety assertions
    assert!(sim.dry_run, "Simulation must always be dry-run");
    assert!(
        sim.sniper_bot_prevention_active,
        "Sniper prevention should be active"
    );
    assert!(
        sim.launch_readiness_score >= 80,
        "Safe config should score >= 80, got {}",
        sim.launch_readiness_score
    );

    // Validation checks
    assert_eq!(sim.validation_report.overall_status, ValidationStatus::Safe);
    assert_eq!(sim.validation_report.risk_score, 0);

    // Liquidity metrics
    assert!(sim.liquidity_metrics.estimated_initial_price_usd > 0.0);
    assert!(sim.liquidity_metrics.estimated_market_cap_usd > 0.0);
    assert!(sim.liquidity_metrics.liquidity_depth_score >= 60); // 50 SOL → 80
    assert!(sim.liquidity_metrics.anti_rug_rating.contains("DIAMOND"));

    // PEV loop populated
    assert!(!sim.pev_loop_summary.perceive.is_empty());
    assert!(!sim.pev_loop_summary.evaluate.is_empty());
    assert!(!sim.pev_loop_summary.validate.is_empty());
}

#[test]
fn test_full_launch_simulation_dangerous_config() {
    let launcher = ArcForgeLauncher::new("https://api.devnet.solana.com");
    let sim = launcher.simulate_planned_launch(dangerous_launch_config());

    assert!(sim.dry_run);
    assert!(
        !sim.sniper_bot_prevention_active,
        "Sniper prevention must be INACTIVE for dangerous config"
    );
    assert!(
        sim.launch_readiness_score < 80,
        "Dangerous config should score < 80, got {}",
        sim.launch_readiness_score
    );
    assert_eq!(
        sim.validation_report.overall_status,
        ValidationStatus::Dangerous
    );
    assert!(sim.validation_report.risk_score > 0);
    assert!(sim.pev_loop_summary.validate.contains("BLOCKED"));
}

// ─── Validator tests ──────────────────────────────────────────────────────────

#[test]
fn test_validator_safe_mint_all_checks_pass() {
    let mint = MintInfo {
        address: "SafeMint111111111111111111111111111111111".to_string(),
        supply: 1_000_000_000_000_000,
        decimals: 9,
        is_initialized: true,
        mint_authority: None,
        freeze_authority: None,
    };
    let validator = TokenValidator::new("https://api.devnet.solana.com");
    let report = validator.validate_mint_info(&mint);

    assert_eq!(report.overall_status, ValidationStatus::Safe);
    assert_eq!(report.risk_score, 0);
    assert!(report.checks.iter().all(|c| c.passed));
    assert!(report.recommendation.contains("safe to launch"));
}

#[test]
fn test_validator_freeze_authority_is_critical() {
    let mint = MintInfo {
        address: "DangerMint11111111111111111111111111111111".to_string(),
        supply: 1_000_000_000_000_000,
        decimals: 9,
        is_initialized: true,
        mint_authority: None,
        freeze_authority: Some("FreezeKey1111111111111111111111111111111".to_string()),
    };
    let validator = TokenValidator::new("https://api.devnet.solana.com");
    let report = validator.validate_mint_info(&mint);

    assert_eq!(report.overall_status, ValidationStatus::Dangerous);
    let freeze_check = report
        .checks
        .iter()
        .find(|c| c.name == "Freeze Authority")
        .unwrap();
    assert!(!freeze_check.passed);
    assert_eq!(freeze_check.status, ValidationStatus::Dangerous);
}

// ─── Liquidity tests ──────────────────────────────────────────────────────────

#[test]
fn test_liquidity_metrics_price_consistency() {
    let config = safe_launch_config();
    let m = DeepLiquidityProtocol::compute(&config);

    // Price × total supply ≈ market cap (within floating-point tolerance)
    let total_adjusted = config.total_supply as f64 / 10f64.powi(config.decimals as i32);
    let expected_mcap = m.estimated_initial_price_usd * total_adjusted;
    let diff = (m.estimated_market_cap_usd - expected_mcap).abs();
    assert!(
        diff < 1.0,
        "Market cap mismatch: got {}, expected {}",
        m.estimated_market_cap_usd,
        expected_mcap
    );
}

#[test]
fn test_liquidity_tokens_in_pool() {
    let config = safe_launch_config();
    let m = DeepLiquidityProtocol::compute(&config);

    let expected = (config.total_supply as f64 * 0.10) as u64; // 10% alloc
    assert_eq!(m.tokens_in_pool, expected);
}

#[test]
fn test_price_impact_ordering() {
    let config = safe_launch_config();
    let m = DeepLiquidityProtocol::compute(&config);

    // A $10K buy must have higher price impact than a $1K buy
    assert!(
        m.price_impact_10k_usd_buy_pct > m.price_impact_1k_usd_buy_pct,
        "10K impact ({}) should exceed 1K impact ({})",
        m.price_impact_10k_usd_buy_pct,
        m.price_impact_1k_usd_buy_pct
    );
}

// ─── Serialisation tests ──────────────────────────────────────────────────────

#[test]
fn test_simulation_json_round_trip() {
    let launcher = ArcForgeLauncher::new("https://api.devnet.solana.com");
    let sim = launcher.simulate_planned_launch(safe_launch_config());

    let json = serde_json::to_string(&sim).expect("Serialisation failed");
    assert!(!json.is_empty());
    assert!(json.contains("\"dry_run\":true"));
    assert!(json.contains("\"token_symbol\":\"PBT\""));

    // Deserialise back
    let sim2: polar_bear_arc_forge_defi::types::LaunchSimulation =
        serde_json::from_str(&json).expect("Deserialisation failed");
    assert_eq!(sim2.config.token_symbol, "PBT");
    assert!(sim2.dry_run);
}

#[test]
fn test_validation_report_json_serialisable() {
    let mint = MintInfo {
        address: "TestMint1111111111111111111111111111111111".to_string(),
        supply: 1_000_000,
        decimals: 6,
        is_initialized: true,
        mint_authority: None,
        freeze_authority: None,
    };
    let validator = TokenValidator::new("https://api.devnet.solana.com");
    let report = validator.validate_mint_info(&mint);

    let json = serde_json::to_string_pretty(&report).expect("Failed to serialise report");
    assert!(json.contains("\"overall_status\""));
    assert!(json.contains("\"risk_score\""));
    assert!(json.contains("\"checks\""));
}
