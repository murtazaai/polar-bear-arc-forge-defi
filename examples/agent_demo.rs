//! # Rig (ARC) ARC Forge Agent Demo
//!
//! Demonstrates the `ArcForgeAgent` - a rig-core 0.37 agent wrapping
//! `claude-sonnet-4-6` that analyses launch simulations and returns a
//! structured `DeFi` security assessment.
//!
//! ## Requirements
//!
//! ```text
//! export ANTHROPIC_API_KEY=sk-ant-...
//! cargo run --example agent_demo --features ai-agent
//! ```
//!
//! ## Architecture
//!
//! ```text
//! LaunchSimulation (JSON)
//!     │
//!     ▼
//! ArcForgeAgent::analyse_simulation()
//!     │
//!     ├─ rig-core 0.37  CompletionClient + ProviderClient
//!     │  model: claude-sonnet-4-6
//!     │  preamble: DeFi security analyst
//!     │
//!     └─▶ risk assessment (≤ 300 words)
//!           LAUNCH | REVIEW | BLOCK recommendation
//! ```

#[cfg(feature = "ai-agent")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use polar_bear_arc_forge_defi::{
        agent::ArcForgeAgent,
        forge::ArcForgeLauncher,
        types::{LaunchConfig, LiquidityConfig, SolanaNetwork},
    };

    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("polar_bear_arc_forge_defi=info".parse()?),
        )
        .init();

    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║     RIG (ARC) ARC FORGE AGENT DEMO  ·  claude-sonnet-4-6        ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
    println!();

    let launcher = ArcForgeLauncher::new("https://api.devnet.solana.com");
    let agent = ArcForgeAgent::new()?;

    let scenarios: &[(&str, &str, bool, f64)] = &[
        ("Polar Bear Token", "PBT", true, 50.0), // safe: burn LP, deep liquidity
        ("Risky Launch Token", "RLT", false, 1.0), // risky: no burn, shallow
    ];

    for (name, symbol, burn, sol) in scenarios {
        let config = LaunchConfig {
            token_name: name.to_string(),
            token_symbol: symbol.to_string(),
            total_supply: 1_000_000_000_000_000,
            decimals: 9,
            mint_authority_renounced: true,
            freeze_authority_renounced: true,
            liquidity: LiquidityConfig {
                initial_liquidity_sol: *sol,
                token_allocation_pct: 10.0,
                burn_lp_tokens: *burn,
                lock_duration_days: 0,
                price_range_lower: 0.0,
                price_range_upper: 0.0,
            },
            network: SolanaNetwork::Devnet,
        };

        println!("── Scenario: {name} ({symbol}) ─────────────────────────────────────────────");

        let sim = launcher.simulate_planned_launch(config);
        println!(
            "   Readiness: {}/100  Sniper prevention: {}",
            sim.launch_readiness_score, sim.sniper_bot_prevention_active
        );

        match agent.analyse_simulation(&sim).await {
            Ok(analysis) => println!("\n{analysis}\n"),
            Err(e) => eprintln!("  ⚠  Agent error: {e}\n"),
        }
    }

    println!("✓  Agent demo complete.");
    Ok(())
}

#[cfg(not(feature = "ai-agent"))]
fn main() {
    eprintln!("This example requires the `ai-agent` feature.");
    eprintln!("Run with: cargo run --example agent_demo --features ai-agent");
    std::process::exit(1);
}
