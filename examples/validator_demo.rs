//! # Token Validator Demo
//!
//! Demonstrates the ARC Forge sniper-bot prevention checks against a live
//! Solana SPL Token mint (USDC on mainnet).
//!
//! ## Usage
//!
//! ```text
//! cargo run --example validator_demo
//! ```
//!
//! No API key required.  Connects to the Solana mainnet RPC.

/// Re-exports the `TokenValidator` struct.
///
/// This struct is used to validate SPL Token mints on the Solana blockchain.
use polar_bear_arc_forge_defi::validator::TokenValidator;

/// USDC on Solana mainnet - renounced authorities; expected: all checks pass.
///
/// This is the mint address for the USDC token on the Solana mainnet.
const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

/// Demonstrates the token validator by checking the health of the USDC mint.
///
/// Connects to the Solana mainnet RPC and performs a series of validation checks.
/// Expects all checks to pass, indicating the mint is safe to use.
///
/// Outputs a validation report with check results and risk score.
///
/// # Example
///
/// ```
/// use polar_bear_arc_forge_defi::validator::TokenValidator;
///
/// let validator = TokenValidator::new();
/// let report = validator.validate_mint(USDC_MINT).await?;
///
/// println!("{:?}", report);
/// ```
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("polar_bear_arc_forge_defi=info".parse()?),
        )
        .init();

    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║         ARC FORGE - TOKEN VALIDATOR DEMO                        ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
    println!();

    let validator = TokenValidator::new("https://api.mainnet-beta.solana.com");

    println!("Validating USDC mint: {USDC_MINT}");
    println!();

    let report = validator.validate(USDC_MINT).await?;

    println!("  Overall status : {}", report.overall_status);
    println!("  Risk score     : {}/100", report.risk_score);
    println!();

    for check in &report.checks {
        let icon = if check.passed { "✅" } else { "❌" };
        println!("  {icon}  {:25}  {}", check.name, check.message);
    }

    println!();
    println!("  RECOMMENDATION: {}", report.recommendation);
    println!();

    // Emit the full report as JSON (demonstrates serialisation)
    let json = serde_json::to_string_pretty(&report)?;
    println!("Full report JSON:");
    println!("{json}");

    Ok(())
}
