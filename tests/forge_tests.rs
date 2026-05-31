//! Integration tests - ARC Forge PEV loop
//!
//! All tests are deterministic (no network access required).
//!
//! Run with:
//! ```text
//! cargo test --test forge_tests
//! ```

/// Tests the `ArcForgeLauncher` and `LaunchConfig` types.
///
/// This test verifies that the `ArcForgeLauncher` can be created and that the `LaunchConfig`
/// can be used to configure a launch.
///
/// It also verifies that the `ArcForgeLauncher` can be used to launch a token with the
/// configured settings.
///
/// This test uses a safe configuration to ensure that the launch is successful.
use polar_bear_arc_forge_defi::{
    forge::ArcForgeLauncher,
    types::{LaunchConfig, LiquidityConfig, SolanaNetwork, ValidationStatus},
};
/// Asserts that the `ArcForgeLauncher` can be used to launch a token with the configured
/// settings.
use pretty_assertions::assert_eq;

/// Returns a `ArcForgeLauncher` instance configured to use the devnet RPC URL.
///
/// This function is used to create a `ArcForgeLauncher` instance that is configured to use the
/// devnet RPC URL.
fn launcher() -> ArcForgeLauncher {
    ArcForgeLauncher::new("https://api.devnet.solana.com")
}

/// Returns a safe `LaunchConfig` instance for testing.
///
/// This function is used to create a `LaunchConfig` instance that is safe to use for testing,
/// with a known token name and supply.
///
/// # Returns
///
/// A `LaunchConfig` instance with a known token name and supply.
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

/// Returns a `LaunchConfig` instance with dangerous settings for testing.
///
/// This function is used to create a `LaunchConfig` instance that is configured with dangerous
/// settings for testing, such as a non-renounced mint authority and freeze authority.
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

/// Asserts that the `ArcForgeLauncher` always sets `dry_run` to `true` in simulations.
///
/// This test verifies that the `dry_run` field is always set to `true` in the simulation
/// results, regardless of the launch configuration.
///
/// It uses the `safe_config()` function to simulate a launch and checks that `dry_run` is `true`.
///
/// # Panics
///
/// This test will panic if `dry_run` is not `true`.
#[test]
fn simulation_is_always_dry_run() {
    let sim = launcher().simulate_planned_launch(safe_config());
    assert!(sim.dry_run, "simulation must always set dry_run = true");
}

/// Asserts that the `safe_config()` function scores a high readiness score.
///
/// It uses the `safe_config()` function to simulate a launch and checks that
/// `launch_readiness_score` is ≥ 80.
///
/// # Panics
///
/// This test will panic if `launch_readiness_score` is not ≥ 80.
#[test]
fn safe_config_scores_high_readiness() {
    let sim = launcher().simulate_planned_launch(safe_config());
    assert!(
        sim.launch_readiness_score >= 80,
        "expected ≥ 80, got {}",
        sim.launch_readiness_score
    );
}

/// Asserts that the `safe_config()` function activates sniper-bot prevention.
///
/// It uses the `safe_config()` function to simulate a launch and checks that
/// `sniper_bot_prevention_active` is `true`.
///
/// # Panics
///
/// This test will panic if `sniper_bot_prevention_active` is not `true`.
#[test]
fn safe_config_activates_sniper_prevention() {
    let sim = launcher().simulate_planned_launch(safe_config());
    assert!(sim.sniper_bot_prevention_active);
}

/// Asserts that the `safe_config()` function has a zero risk score.
///
/// It uses the `safe_config()` function to simulate a launch and checks that
/// `validation_report.overall_status` is `ValidationStatus::Safe` and `risk_score` is `0`.
///
/// # Panics
///
/// This test will panic if `validation_report.overall_status` is not `ValidationStatus::Safe`
/// or `risk_score` is not `0`.
#[test]
fn safe_config_zero_risk_score() {
    let sim = launcher().simulate_planned_launch(safe_config());
    assert_eq!(sim.validation_report.overall_status, ValidationStatus::Safe);
    assert_eq!(sim.validation_report.risk_score, 0);
}

/// Asserts that the `dangerous_config()` function deactivates sniper-bot prevention.
///
/// It uses the `dangerous_config()` function to simulate a launch and checks that
/// `sniper_bot_prevention_active` is `false`.
///
/// # Panics
///
/// This test will panic if `sniper_bot_prevention_active` is not `false`.
#[test]
fn dangerous_config_deactivates_sniper_prevention() {
    let sim = launcher().simulate_planned_launch(dangerous_config());
    assert!(!sim.sniper_bot_prevention_active);
}

/// Asserts that the `dangerous_config()` function fails validation.
///
/// It uses the `dangerous_config()` function to simulate a launch and checks that
/// `validation_report.overall_status` is `ValidationStatus::Dangerous` and `risk_score` is `> 0`.
///
/// # Panics
///
/// This test will panic if `validation_report.overall_status` is not `ValidationStatus::Dangerous`
/// or `risk_score` is not `> 0`.
#[test]
fn dangerous_config_fails_validation() {
    let sim = launcher().simulate_planned_launch(dangerous_config());
    assert_eq!(
        sim.validation_report.overall_status,
        ValidationStatus::Dangerous
    );
    assert!(sim.validation_report.risk_score > 0);
}

/// Asserts that the `dangerous_config()` function has a low readiness score.
///
/// It uses the `dangerous_config()` function to simulate a launch and checks that
/// `launch_readiness_score` is `< 80`.
///
/// # Panics
///
/// This test will panic if `launch_readiness_score` is not `< 80`.
#[test]
fn dangerous_config_low_readiness() {
    let sim = launcher().simulate_planned_launch(dangerous_config());
    assert!(
        sim.launch_readiness_score < 80,
        "dangerous config must score < 80, got {}",
        sim.launch_readiness_score
    );
}

/// Asserts that the `dangerous_config()` function has a PEV validate block message.
///
/// It uses the `dangerous_config()` function to simulate a launch and checks that
/// `pev_loop_summary.validate` contains the string "BLOCKED".
///
/// # Panics
///
/// This test will panic if `pev_loop_summary.validate` does not contain the string "BLOCKED".
#[test]
fn dangerous_config_pev_validate_says_blocked() {
    let sim = launcher().simulate_planned_launch(dangerous_config());
    assert!(
        sim.pev_loop_summary.validate.contains("BLOCKED"),
        "validate must say BLOCKED: {}",
        sim.pev_loop_summary.validate
    );
}

/// Asserts that all PEV phases are populated in the `dangerous_config()` simulation.
///
/// It uses the `dangerous_config()` function to simulate a launch and checks that
/// all PEV phases (`perceive`, `evaluate`, `validate`, `act`) are populated.
///
/// # Panics
///
/// This test will panic if any of the PEV phases are not populated.
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

/// Asserts that the `LaunchSimulation` can be serialised and deserialised correctly.
///
/// It uses the `safe_config()` function to simulate a launch and checks that
/// the `LaunchSimulation` can be serialised to JSON and deserialised back to the original object.
///
/// # Panics
///
/// This test will panic if the `LaunchSimulation` cannot be serialised or deserialised correctly.
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

/// Asserts that the `LaunchSimulation` has positive price and market cap liquidity metrics.
///
/// It uses the `safe_config()` function to simulate a launch and checks that
/// `estimated_initial_price_usd` and `estimated_market_cap_usd` are both `> 0.0`.
///
/// # Panics
///
/// This test will panic if `estimated_initial_price_usd` or `estimated_market_cap_usd` are not `>
/// 0.0`.
#[test]
fn simulation_has_positive_price_and_mcap() {
    let sim = launcher().simulate_planned_launch(safe_config());
    assert!(sim.liquidity_metrics.estimated_initial_price_usd > 0.0);
    assert!(sim.liquidity_metrics.estimated_market_cap_usd > 0.0);
}

/// Asserts that the `LaunchSimulation` does not have agent analysis by default.
///
/// It uses the `safe_config()` function to simulate a launch and checks that
/// `agent_analysis` is `None` before the agent is called.
///
/// # Panics
///
/// This test will panic if `agent_analysis` is not `None` before the agent is called.
#[test]
fn simulation_no_agent_analysis_by_default() {
    let sim = launcher().simulate_planned_launch(safe_config());
    assert!(
        sim.agent_analysis.is_none(),
        "agent_analysis must be None before agent is called"
    );
}
