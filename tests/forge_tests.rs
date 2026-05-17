//! Integration tests - ARC Forge PEV loop
//!
//! All tests are deterministic (no network access required).
//!
//! Run with:
//! ```text
//! cargo test --test forge_tests
//! ```

use polar_bear_arc_forge_defi::{
    forge::ArcForgeLauncher,
    types::{LaunchConfig, LiquidityConfig, SolanaNetwork, ValidationStatus},
};
use pretty_assertions::assert_eq;

fn launcher() -> ArcForgeLauncher {
    ArcForgeLauncher::new("https://api.devnet.solana.com")
}

fn safe_config() -> LaunchConfig {
    LaunchConfig {
        token_name: "Polar Bear Token".to_string(),
        token_symbol: "PBT".to_string(),
        total_supply: 1_000_000_000_000_000,
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

fn dangerous_config() -> LaunchConfig {
    LaunchConfig {
        token_name: "Danger Token".to_string(),
        token_symbol: "DNGR".to_string(),
        total_supply: 1_000_000_000_000_000,
        decimals: 9,
        mint_authority_renounced: false,   // ← inflation vector
        freeze_authority_renounced: false, // ← sniper-bot vector
        liquidity: LiquidityConfig {
            initial_liquidity_sol: 0.5,
            token_allocation_pct: 10.0,
            burn_lp_tokens: false,
            lock_duration_days: 0,
            price_range_lower: 0.0,
            price_range_upper: 0.0,
        },
        network: SolanaNetwork::Devnet,
    }
}

// ── Dry-run guarantee ─────────────────────────────────────────────────────────

#[test]
fn simulation_is_always_dry_run() {
    let sim = launcher().simulate_planned_launch(safe_config());
    assert!(sim.dry_run, "simulation must always set dry_run = true");
}

// ── Safe config ───────────────────────────────────────────────────────────────

#[test]
fn safe_config_scores_high_readiness() {
    let sim = launcher().simulate_planned_launch(safe_config());
    assert!(
        sim.launch_readiness_score >= 80,
        "expected ≥ 80, got {}",
        sim.launch_readiness_score
    );
}

#[test]
fn safe_config_activates_sniper_prevention() {
    let sim = launcher().simulate_planned_launch(safe_config());
    assert!(sim.sniper_bot_prevention_active);
}

#[test]
fn safe_config_zero_risk_score() {
    let sim = launcher().simulate_planned_launch(safe_config());
    assert_eq!(sim.validation_report.overall_status, ValidationStatus::Safe);
    assert_eq!(sim.validation_report.risk_score, 0);
}

// ── Dangerous config ──────────────────────────────────────────────────────────

#[test]
fn dangerous_config_deactivates_sniper_prevention() {
    let sim = launcher().simulate_planned_launch(dangerous_config());
    assert!(!sim.sniper_bot_prevention_active);
}

#[test]
fn dangerous_config_fails_validation() {
    let sim = launcher().simulate_planned_launch(dangerous_config());
    assert_eq!(
        sim.validation_report.overall_status,
        ValidationStatus::Dangerous
    );
    assert!(sim.validation_report.risk_score > 0);
}

#[test]
fn dangerous_config_low_readiness() {
    let sim = launcher().simulate_planned_launch(dangerous_config());
    assert!(
        sim.launch_readiness_score < 80,
        "dangerous config must score < 80, got {}",
        sim.launch_readiness_score
    );
}

#[test]
fn dangerous_config_pev_validate_says_blocked() {
    let sim = launcher().simulate_planned_launch(dangerous_config());
    assert!(
        sim.pev_loop_summary.validate.contains("BLOCKED"),
        "validate must say BLOCKED: {}",
        sim.pev_loop_summary.validate
    );
}

// ── PEV loop ──────────────────────────────────────────────────────────────────

#[test]
fn all_pev_phases_are_populated() {
    let sim = launcher().simulate_planned_launch(safe_config());
    assert!(
        !sim.pev_loop_summary.perceive.is_empty(),
        "perceive must be populated"
    );
    assert!(
        !sim.pev_loop_summary.evaluate.is_empty(),
        "evaluate must be populated"
    );
    assert!(
        !sim.pev_loop_summary.validate.is_empty(),
        "validate must be populated"
    );
}

#[test]
fn safe_config_pev_validate_says_validated() {
    let sim = launcher().simulate_planned_launch(safe_config());
    assert!(
        sim.pev_loop_summary.validate.contains("VALIDATED"),
        "validate must say VALIDATED: {}",
        sim.pev_loop_summary.validate
    );
}

// ── JSON serialisation ────────────────────────────────────────────────────────

#[test]
fn simulation_serialises_and_round_trips() {
    let sim = launcher().simulate_planned_launch(safe_config());
    let json = serde_json::to_string(&sim).expect("serialise");

    assert!(json.contains("\"dry_run\":true"));
    assert!(json.contains("\"token_symbol\":\"PBT\""));

    let back: polar_bear_arc_forge_defi::LaunchSimulation =
        serde_json::from_str(&json).expect("deserialise");
    assert_eq!(back.config.token_symbol, "PBT");
    assert!(back.dry_run);
    assert_eq!(back.launch_readiness_score, sim.launch_readiness_score);
}

// ── Liquidity metrics ─────────────────────────────────────────────────────────

#[test]
fn simulation_has_positive_price_and_mcap() {
    let sim = launcher().simulate_planned_launch(safe_config());
    assert!(sim.liquidity_metrics.estimated_initial_price_usd > 0.0);
    assert!(sim.liquidity_metrics.estimated_market_cap_usd > 0.0);
}

#[test]
fn simulation_no_agent_analysis_by_default() {
    let sim = launcher().simulate_planned_launch(safe_config());
    assert!(
        sim.agent_analysis.is_none(),
        "agent_analysis must be None before agent is called"
    );
}
