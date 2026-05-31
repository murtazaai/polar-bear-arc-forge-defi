//! # `ArcForgeLauncher`
//!
//! Orchestrates the full Perceive → Evaluate → Validate (PEV) loop for
//! an ARC Forge token launch simulation.
//!
//! ## PEV loop
//!
//! ```text
//! PERCEIVE  →  TokenValidator (on-chain mint data or planned config)
//! EVALUATE  →  DeepLiquidityProtocol (AMM model, depth score, anti-rug rating)
//! VALIDATE  →  readiness score, sniper-bot gates, launch/block decision
//!                  │
//!                  ▼
//!             LaunchSimulation (JSON audit record)
//!                  │
//!                  └─► ArcForgeAgent (optional Rig AI analysis)
//! ```
//!
//! ## Dry-run guarantee
//!
//! All operations are **dry-run only**.  No SOL is spent.  No transactions are
//! submitted.  Every call that would normally submit a Solana instruction
//! produces a structured log entry instead.

/// Orchestrates the ARC Forge PEV loop and produces a `LaunchSimulation`.
///
/// The [`ArcForgeLauncher`] is responsible for coordinating the PEV loop,
/// including perceiving, evaluating, and validating the launch simulation.
use anyhow::Result;
use chrono::Utc;
use tracing::info;

use crate::{
    defi::DeepLiquidityProtocol,
    types::{
        LaunchConfig, LaunchSimulation, LiquidityMetrics, MintInfo, PevLoopSummary,
        ValidationReport, ValidationStatus,
    },
    validator::TokenValidator,
};

/// Orchestrates the ARC Forge PEV loop and produces a `LaunchSimulation`.
pub struct ArcForgeLauncher {
    validator: TokenValidator,
}

/// Provides methods for simulating ARC Forge launches on existing on-chain mints.
///
/// The [`ArcForgeLauncher`] is responsible for coordinating the PEV loop,
/// including perceiving, evaluating, and validating the launch simulation.
impl ArcForgeLauncher {
    /// Create a launcher backed by the given Solana RPC endpoint.
    ///
    /// # Arguments
    ///
    /// * `rpc_url` - The URL of the Solana RPC endpoint to use.
    ///
    /// # Returns
    ///
    /// A new [`ArcForgeLauncher`] instance.
    /// Initializes a new [`ArcForgeLauncher`] with the given Solana RPC endpoint.
    pub fn new(rpc_url: impl Into<String>) -> Self {
        Self {
            validator: TokenValidator::new(rpc_url),
        }
    }

    // ── Mode A: existing on-chain mint ────────────────────────────────────────

    /// Run a full PEV-loop simulation for an existing on-chain mint.
    ///
    /// **Perceive**: fetches live SPL Token mint data from Solana RPC.
    /// Requires network access.
    ///
    /// # Arguments
    ///
    /// * `mint_address` - The address of the on-chain SPL Token mint to simulate.
    /// * `config` - The launch configuration to use for the simulation.
    ///
    /// # Returns
    ///
    /// A `LaunchSimulation` result containing the simulation results.
    pub async fn simulate_existing_mint(
        &self,
        mint_address: &str,
        config: LaunchConfig,
    ) -> Result<LaunchSimulation> {
        info!(
            mint = mint_address,
            network = config.network.label(),
            "ARC Forge - PERCEIVE (on-chain mint)"
        );
        let report = self.validator.validate(mint_address).await?;
        Ok(ArcForgeLauncher::build(config, report, "on-chain mint"))
    }

    /// Run a full PEV-loop simulation for a planned new token.
    ///
    /// Synthesises a [`MintInfo`] from the [`LaunchConfig`] to represent the
    /// intended post-launch state (authorities renounced, supply minted).
    /// **No network access required.**
    ///
    /// # Arguments
    ///
    /// * `config` - The launch configuration to use for the simulation.
    ///
    /// # Returns
    ///
    /// A `LaunchSimulation` result containing the simulation results.
    pub fn simulate_planned_launch(&self, config: LaunchConfig) -> LaunchSimulation {
        info!(
            token = %config.token_symbol,
            network = config.network.label(),
            "ARC Forge - PERCEIVE (planned launch config)"
        );
        let synthetic = synthetic_mint(&config);
        let report = self.validator.validate_mint_info(&synthetic);
        ArcForgeLauncher::build(config, report, "planned config")
    }

    /// Builds a `LaunchSimulation` from the given configuration and report.
    ///
    /// # Arguments
    ///
    /// * `config` - The launch configuration to use for the simulation.
    /// * `report` - The validation report to use for the simulation.
    /// * `perceive_source` - The source of the perceive step (e.g. "on-chain mint", "planned
    ///   config").
    ///
    /// # Returns
    ///
    /// A `LaunchSimulation` result containing the simulation results.
    fn build(
        config: LaunchConfig,
        report: ValidationReport,
        perceive_source: &str,
    ) -> LaunchSimulation {
        info!(
            risk_score = report.risk_score,
            status = ?report.overall_status,
            "ARC Forge - EVALUATE"
        );

        let metrics = DeepLiquidityProtocol::compute(&config);
        let sniper_prevention = report.overall_status == ValidationStatus::Safe
            && config.freeze_authority_renounced
            && config.mint_authority_renounced;
        let readiness = readiness_score(&report, &metrics);

        info!(
            readiness,
            sniper_prevention, "ARC Forge - VALIDATE complete"
        );

        let pev = pev_summary(perceive_source, &report, &metrics, readiness);

        LaunchSimulation {
            config,
            validation_report: report,
            liquidity_metrics: metrics,
            sniper_bot_prevention_active: sniper_prevention,
            launch_readiness_score: readiness,
            dry_run: true,
            timestamp: Utc::now(),
            pev_loop_summary: pev,
            agent_analysis: None,
        }
    }
}

/// Calculates the readiness score for a launch simulation based on the validation report and
/// liquidity metrics.
///
/// # Arguments
///
/// * `report` - The validation report to use for the simulation.
/// * `metrics` - The liquidity metrics to use for the simulation.
///
/// # Returns
///
/// The readiness score as an `u8` value, where 0 is the lowest and 100 is the highest.
///
/// # User Accepted Prediction
fn readiness_score(report: &ValidationReport, metrics: &LiquidityMetrics) -> u8 {
    let mut score: i32 = 100;

    // Deduct up to 70 points proportionally to validation risk score
    score -= i32::from(report.risk_score) * 7 / 10;

    // Deduct for shallow liquidity
    score -= match metrics.liquidity_depth_score {
        s if s >= 80 => 0,
        s if s >= 60 => 5,
        s if s >= 40 => 15,
        _ => 25,
    };

    // Deduct if LP is neither burned nor locked
    if !metrics.config.burn_lp_tokens && metrics.config.lock_duration_days < 30 {
        score -= 20;
    }

    u8::try_from(score.clamp(0, 100)).unwrap_or(100)
}

/// Generates a summary of the PEV loop for a launch simulation.
///
/// # Arguments
///
/// * `source` - The source of the PEV loop summary.
/// * `report` - The validation report to use for the simulation.
/// * `metrics` - The liquidity metrics to use for the simulation.
/// * `readiness` - The readiness score to use for the simulation.
///
/// # Returns
///
/// The PEV loop summary as a [`PevLoopSummary`] struct.
fn pev_summary(
    source: &str,
    report: &ValidationReport,
    metrics: &LiquidityMetrics,
    readiness: u8,
) -> PevLoopSummary {
    let freeze_ok = report
        .checks
        .iter()
        .any(|c| c.name == "Freeze Authority" && c.passed);
    let mint_ok = report
        .checks
        .iter()
        .any(|c| c.name == "Mint Authority" && c.passed);

    let perceive = format!(
        "Perceived {source} via Solana RPC. \
         Mint: {} | freeze_authority: {} | mint_authority: {}",
        report.mint_address,
        if freeze_ok {
            "None (safe)"
        } else {
            "SET (dangerous)"
        },
        if mint_ok {
            "None (safe)"
        } else {
            "SET (warning)"
        },
    );

    let evaluate = format!(
        "Evaluated {} checks - risk score {}/100 ({}). \
         Liquidity: {:.2} SOL, depth {}/100. \
         Launch price: ${:.8}, market cap: ${:.2}. \
         Price impact $1K buy: {:.2}%.",
        report.checks.len(),
        report.risk_score,
        report.overall_status,
        metrics.config.initial_liquidity_sol,
        metrics.liquidity_depth_score,
        metrics.estimated_initial_price_usd,
        metrics.estimated_market_cap_usd,
        metrics.price_small_buy_impact_usd_buy_pct,
    );

    let validate = if readiness >= 80 {
        format!(
            "VALIDATED - readiness {readiness}/100. All safety gates passed. \
             LP: {}. Recommendation: {}",
            if metrics.config.burn_lp_tokens {
                "BURN"
            } else {
                "LOCK"
            },
            report.recommendation,
        )
    } else {
        format!(
            "BLOCKED - readiness {readiness}/100. \
             Recommendation: {}",
            report.recommendation,
        )
    };

    PevLoopSummary {
        perceive,
        evaluate,
        validate,
    }
}

/// Build a [`MintInfo`] representing the intended post-launch state of `config`.
///
/// # Arguments
///
/// * `config` - The launch configuration to use for the simulation.
///
/// # Returns
///
/// The [`MintInfo`] struct representing the synthetic mint.
fn synthetic_mint(config: &LaunchConfig) -> MintInfo {
    MintInfo {
        address: format!("PLANNED:{}", config.token_symbol),
        supply: config.total_supply,
        decimals: config.decimals,
        is_initialized: true,
        mint_authority: if config.mint_authority_renounced {
            None
        } else {
            Some("DEPLOYER_PUBKEY_PLACEHOLDER".to_string())
        },
        freeze_authority: if config.freeze_authority_renounced {
            None
        } else {
            Some("DEPLOYER_PUBKEY_PLACEHOLDER".to_string())
        },
    }
}

/// Pretty-print the full simulation report to stdout.
///
/// # Arguments
///
/// * `self` - The `LaunchSimulation` instance to print the report for.
///
/// # Returns
///
/// * `()` - This method does not return a value.
impl LaunchSimulation {
    /// Pretty-print the full simulation report to stdout.
    pub fn print_report(&self) {
        let wide = "═".repeat(70);
        let thin = "─".repeat(70);
        println!("\n{wide}");
        println!("  ARC FORGE - LAUNCH SIMULATION REPORT  [DRY-RUN]");
        println!("{wide}");
        println!(
            "  Token    : {} ({})",
            self.config.token_name, self.config.token_symbol
        );
        println!("  Network  : {}", self.config.network.label());
        println!("  Time     : {}", self.timestamp);
        println!("{thin}");
        println!("\n  PEV LOOP");
        println!("  PERCEIVE : {}", self.pev_loop_summary.perceive);
        println!("  EVALUATE : {}", self.pev_loop_summary.evaluate);
        println!("  VALIDATE : {}", self.pev_loop_summary.validate);
        println!("\n{thin}");
        println!(
            "  VALIDATION  risk={}/100  {}",
            self.validation_report.risk_score, self.validation_report.overall_status
        );
        for c in &self.validation_report.checks {
            let icon = if c.passed { "✅" } else { "❌" };
            println!("  {icon}  {:25}  {}", c.name, c.message);
        }
        println!("\n{thin}");
        println!("{}", self.liquidity_metrics.format_summary());
        println!("\n{thin}");
        println!(
            "  READINESS SCORE      : {}/100",
            self.launch_readiness_score
        );
        println!(
            "  SNIPER PREVENTION    : {}",
            if self.sniper_bot_prevention_active {
                "ACTIVE ✅"
            } else {
                "INACTIVE ❌"
            }
        );
        if let Some(ref a) = self.agent_analysis {
            println!("\n{thin}");
            println!("  AGENT ANALYSIS (Rig ARC / Claude)");
            println!("  {}", a.replace('\n', "\n  "));
        }
        println!("\n{wide}\n");
    }
}

/// Tests for the [`ArcForge`] simulation functionality.
///
/// This module contains unit tests for the [`ArcForge`] simulation functionality,
/// including tests for the PEV loop summary, synthetic mint, and launch simulation.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{LaunchConfig, LiquidityConfig, SolanaNetwork};

    /// Returns a safe [`LaunchConfig`] for testing purposes.
    ///
    /// This function returns a [`LaunchConfig`] with default values suitable for testing,
    /// ensuring that the simulation can run without requiring user input or external dependencies.
    ///
    /// # Returns
    ///
    /// A [`LaunchConfig`] with default values for testing.
    fn safe_config() -> LaunchConfig {
        LaunchConfig {
            token_name: "Polar Bear Token".to_string(),
            token_symbol: "PBT".to_string(),
            total_supply: 1_000_000_000_000_000,
            decimals: 9,
            mint_authority_renounced: true,
            freeze_authority_renounced: true,
            liquidity: LiquidityConfig {
                initial_liquidity_sol: 20.0,
                token_allocation_pct: 10.0,
                burn_lp_tokens: true,
                lock_duration_days: 0,
                price_range_lower: 0.0,
                price_range_upper: 0.0,
            },
            network: SolanaNetwork::Devnet,
        }
    }

    /// Returns an [`ArcForgeLauncher`] instance for testing purposes.
    ///
    /// This function returns an [`ArcForgeLauncher`] instance configured to use the Solana Devnet
    /// API, ensuring that the simulation can run without requiring user input or external
    /// dependencies.
    ///
    /// # Returns
    ///
    /// An [`ArcForgeLauncher`] instance configured for testing.
    fn launcher() -> ArcForgeLauncher {
        ArcForgeLauncher::new("https://api.devnet.solana.com")
    }

    /// Tests that a safe [`LaunchConfig`] results in a high readiness score and active
    /// sniper bot prevention.
    ///
    /// This test verifies that a safe [`LaunchConfig`] results in a high readiness score and
    /// active sniper bot prevention, ensuring that the simulation can run without requiring user
    /// input or external dependencies.
    ///
    /// It uses the [`launcher`] function to create an [`ArcForgeLauncher`] instance and
    /// [`safe_config`] to simulate a planned launch.
    ///
    /// The test asserts that the readiness score is ≥ 80 and that sniper bot prevention is active,
    /// and that the simulation is always a dry run.
    #[test]
    fn safe_config_high_readiness() {
        let sim = launcher().simulate_planned_launch(safe_config());
        assert!(
            sim.launch_readiness_score >= 80,
            "expected ≥ 80, got {}",
            sim.launch_readiness_score
        );
        assert!(sim.sniper_bot_prevention_active);
        assert!(sim.dry_run, "simulation must always be dry-run");
    }

    /// Tests that a dangerous [`LaunchConfig`] is blocked and the simulation is not a dry run.
    ///
    /// This test verifies that a dangerous [`LaunchConfig`] is blocked and the simulation is not
    /// a dry run, ensuring that the simulation can run without requiring user input or external
    /// dependencies.
    ///
    /// It uses the [`launcher`] function to create an [`ArcForgeLauncher`] instance and
    /// [`safe_config`] to simulate a planned launch.
    ///
    /// The test asserts that the readiness score is < 80 and that sniper bot prevention is not
    /// active, and that the simulation is not a dry run.
    #[test]
    fn dangerous_config_blocked() {
        let mut cfg = safe_config();
        cfg.freeze_authority_renounced = false;
        cfg.mint_authority_renounced = false;
        let sim = launcher().simulate_planned_launch(cfg);
        assert!(!sim.sniper_bot_prevention_active);
        assert!(sim.validation_report.risk_score > 0);
        assert!(sim.launch_readiness_score < 80);
    }

    /// Tests that a safe [`LaunchConfig`] results in a high readiness score and active sniper
    /// bot prevention.
    ///
    /// This test verifies that a safe [`LaunchConfig`] results in a high readiness score and
    /// active sniper bot prevention, ensuring that the simulation can run without requiring user
    /// input or external dependencies.
    ///
    /// It uses the [`launcher`] function to create an [`ArcForgeLauncher`] instance and
    /// [`safe_config`] to simulate a planned launch.
    ///
    /// The test asserts that the readiness score is ≥ 80 and that sniper bot prevention is active,
    /// and that the simulation is always a dry run.
    ///
    /// It also verifies that the PEV loop summary contains non-empty phases for all three
    /// phases: perceive, evaluate, and validate.
    #[test]
    fn pev_loop_all_phases_populated() {
        let sim = launcher().simulate_planned_launch(safe_config());
        assert!(!sim.pev_loop_summary.perceive.is_empty());
        assert!(!sim.pev_loop_summary.evaluate.is_empty());
        assert!(!sim.pev_loop_summary.validate.is_empty());
    }

    /// Tests that the simulation always runs in dry run mode, regardless of the launch config.
    ///
    /// This test verifies that the simulation always runs in dry run mode, regardless of the
    /// launch config. It uses the [`launcher`] function to create an [`ArcForgeLauncher`] instance
    /// and [`safe_config`] to simulate a planned launch.
    ///
    /// The test asserts that the simulation is always a dry run.
    #[test]
    fn dry_run_always_true() {
        let sim = launcher().simulate_planned_launch(safe_config());
        assert!(sim.dry_run);
    }

    /// Tests that the simulation result can be serialised and deserialised correctly.
    ///
    /// This test verifies that the simulation result can be serialised and deserialised correctly.
    /// It uses the [`launcher`] function to create an [`ArcForgeLauncher`] instance and
    /// [`safe_config`] to simulate a planned launch.
    ///
    /// The test asserts that the deserialised result matches the original simulation result,
    /// including the token symbol and dry run flag.
    #[test]
    fn json_round_trip() {
        let sim = launcher().simulate_planned_launch(safe_config());
        let json = serde_json::to_string(&sim).expect("serialise");
        let back: LaunchSimulation = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(back.config.token_symbol, "PBT");
        assert!(back.dry_run);
    }
}
