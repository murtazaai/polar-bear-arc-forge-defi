//! # arc-forge CLI
//!
//! ```text
//! USAGE:
//!   arc-forge [OPTIONS] <COMMAND>
//!
//! COMMANDS:
//!   validate   Fetch a live Solana mint and run sniper-bot prevention checks
//!   raydium    Query Raydium v3 API for pool liquidity data
//!   launch     Run a full ARC Forge launch simulation (always dry-run)
//!   agent      Launch simulation + Rig (ARC) AI agent analysis
//!   check      Connectivity check: Solana RPC slot + Raydium API ping
//! ```

use anyhow::Result;
use clap::{Parser, Subcommand};
use polar_bear_arc_forge_defi::{
    agent::ArcForgeAgent,
    defi::RaydiumClient,
    forge::ArcForgeLauncher,
    rpc::SolanaRpcClient,
    types::{LaunchConfig, LiquidityConfig, SolanaNetwork, ValidationStatus},
    validator::TokenValidator,
};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "arc-forge",
    version,
    about = "ARC Forge DeFi Platform - Solana token launch with sniper-bot prevention\n\
             and deep initial liquidity, powered by Rig (ARC) AI agents.\n\n\
             https://github.com/murtazaai/polar-bear-arc-forge-defi",
    long_about = None,
)]
struct Cli {
    /// Solana RPC endpoint (overrides `SOLANA_RPC_URL` env var).
    #[arg(
        long,
        env = "SOLANA_RPC_URL",
        default_value = "https://api.devnet.solana.com",
        global = true
    )]
    rpc_url: String,

    /// Log level: error | warn | info | debug | trace
    #[arg(long, env = "RUST_LOG", default_value = "info", global = true)]
    log_level: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Fetch a live SPL Token mint and run all sniper-bot prevention checks.
    Validate {
        /// SPL Token mint address (base58-encoded).
        #[arg(short, long)]
        mint: String,
        /// Emit JSON instead of human-readable output.
        #[arg(long)]
        json: bool,
    },

    /// Query the Raydium v3 REST API for pool data.
    Raydium {
        /// Token mint to query (defaults to native SOL).
        #[arg(
            short,
            long,
            default_value = "So11111111111111111111111111111111111111112"
        )]
        mint: String,
        /// Pool type filter: all | standard | concentrated
        #[arg(long, default_value = "all")]
        pool_type: String,
        /// Maximum number of pools to return (1–20).
        #[arg(long, default_value = "5")]
        limit: u32,
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },

    /// Run a full ARC Forge launch simulation (always dry-run, no SOL spent).
    Launch {
        /// Total token supply in smallest units.
        #[arg(long, default_value = "1000000000000000")]
        supply: u64,
        /// Token decimal places (standard: 9).
        #[arg(long, default_value = "9")]
        decimals: u8,
        /// Token name.
        #[arg(long, default_value = "ARC Forge Demo Token")]
        name: String,
        /// Token ticker symbol.
        #[arg(long, default_value = "ARCD")]
        symbol: String,
        /// SOL for the initial liquidity pool.
        #[arg(long, default_value = "20.0")]
        sol: f64,
        /// Percentage of supply in the initial LP pool.
        #[arg(long, default_value = "10.0")]
        lp_pct: f64,
        /// Burn LP tokens (permanent, strongest anti-rug).
        #[arg(long)]
        burn_lp: bool,
        /// Days to lock LP tokens (ignored when --burn-lp is set).
        #[arg(long, default_value = "0")]
        lock_days: u32,
        /// Simulate against an existing on-chain mint instead of a planned config.
        #[arg(long)]
        mint: Option<String>,
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },

    /// Launch simulation + Rig (ARC) AI agent analysis.
    ///
    /// Requires --features ai-agent and `ANTHROPIC_API_KEY`.
    Agent {
        /// Total token supply.
        #[arg(long, default_value = "1000000000000000")]
        supply: u64,
        /// Token decimal places.
        #[arg(long, default_value = "9")]
        decimals: u8,
        /// Token name.
        #[arg(long, default_value = "ARC Forge Demo Token")]
        name: String,
        /// Token symbol.
        #[arg(long, default_value = "ARCD")]
        symbol: String,
        /// SOL for initial liquidity.
        #[arg(long, default_value = "20.0")]
        sol: f64,
        /// LP supply allocation percentage.
        #[arg(long, default_value = "10.0")]
        lp_pct: f64,
        /// Burn LP tokens.
        #[arg(long)]
        burn_lp: bool,
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },

    /// Connectivity check: Solana RPC current slot + Raydium API ping.
    Check,
}

// ── main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let subscriber = FmtSubscriber::builder()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&cli.log_level)),
        )
        .with_target(false)
        .compact()
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global tracing subscriber");

    let _ = dotenvy::dotenv();

    match cli.command {
        // ── validate ──────────────────────────────────────────────────────────
        Commands::Validate { mint, json } => {
            let report = TokenValidator::new(&cli.rpc_url).validate(&mint).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print_validation(&report);
            }
        }

        // ── raydium ───────────────────────────────────────────────────────────
        Commands::Raydium {
            mint,
            pool_type,
            limit,
            json,
        } => {
            let client = RaydiumClient::new();
            if json {
                let pools = client.get_pools_for_mint(&mint, &pool_type, limit).await?;
                println!("{}", serde_json::to_string_pretty(&pools)?);
            } else {
                let summary = client.pool_health_summary(&mint).await;
                print_raydium(&summary);
            }
        }

        // ── launch ────────────────────────────────────────────────────────────
        Commands::Launch {
            supply,
            decimals,
            name,
            symbol,
            sol,
            lp_pct,
            burn_lp,
            lock_days,
            mint,
            json,
        } => {
            let cfg = build_config(
                name, symbol, supply, decimals, sol, lp_pct, burn_lp, lock_days,
            );
            let launcher = ArcForgeLauncher::new(&cli.rpc_url);
            let sim = match mint {
                Some(ref addr) => launcher.simulate_existing_mint(addr, cfg).await?,
                None => launcher.simulate_planned_launch(cfg),
            };
            if json {
                println!("{}", serde_json::to_string_pretty(&sim)?);
            } else {
                sim.print_report();
            }
        }

        // ── agent ─────────────────────────────────────────────────────────────
        Commands::Agent {
            supply,
            decimals,
            name,
            symbol,
            sol,
            lp_pct,
            burn_lp,
            json,
        } => {
            let cfg = build_config(name, symbol, supply, decimals, sol, lp_pct, burn_lp, 0);
            let launcher = ArcForgeLauncher::new(&cli.rpc_url);
            let mut sim = launcher.simulate_planned_launch(cfg);

            println!("\n🤖  Invoking Rig (ARC) agent via rig-core…");
            match ArcForgeAgent::new() {
                Ok(agent) => match agent.analyse_simulation(&sim).await {
                    Ok(analysis) => sim.agent_analysis = Some(analysis),
                    Err(e) => {
                        eprintln!("⚠️  Agent error: {e}");
                        sim.agent_analysis = Some(format!("Agent error: {e}"));
                    }
                },
                Err(e) => eprintln!(
                    "⚠️  Could not initialise agent: {e}\n   Ensure ANTHROPIC_API_KEY is set."
                ),
            }

            if json {
                println!("{}", serde_json::to_string_pretty(&sim)?);
            } else {
                sim.print_report();
            }
        }

        // ── check ─────────────────────────────────────────────────────────────
        Commands::Check => {
            println!("🔍  Connectivity check\n");
            print!("  Solana RPC ({})  … ", cli.rpc_url);
            match SolanaRpcClient::new(&cli.rpc_url).get_slot().await {
                Ok(slot) => println!("✅  slot = {slot}"),
                Err(e) => println!("❌  {e}"),
            }
            print!("  Raydium v3 API                  … ");
            match RaydiumClient::new()
                .get_pools_for_mint("So11111111111111111111111111111111111111112", "all", 1)
                .await
            {
                Ok(pools) => println!("✅  {} pool(s) returned for SOL", pools.len()),
                Err(e) => println!("❌  {e}"),
            }
            println!("\n  All checks complete.");
        }
    }

    Ok(())
}

// ── Display helpers ───────────────────────────────────────────────────────────

fn print_validation(report: &polar_bear_arc_forge_defi::ValidationReport) {
    let wide = "═".repeat(70);
    let thin = "─".repeat(70);
    println!("\n{wide}");
    println!("  TOKEN VALIDATION REPORT - ARC Forge Sniper-Bot Prevention");
    println!("{wide}");
    println!("  Mint   : {}", report.mint_address);
    println!("  Time   : {}", report.timestamp);
    println!("  Status : {}", report.overall_status);
    println!("  Risk   : {}/100", report.risk_score);
    println!("{thin}");
    for c in &report.checks {
        let icon = if c.passed { "✅" } else { "❌" };
        let label = match c.status {
            ValidationStatus::Safe => "SAFE     ",
            ValidationStatus::Warning => "WARNING  ",
            ValidationStatus::Dangerous => "DANGEROUS",
        };
        println!("  {icon}  [{label}]  {:25}", c.name);
        println!("             └─ {}", c.message);
    }
    println!("{thin}");
    println!("  RECOMMENDATION: {}", report.recommendation);
    println!("{wide}\n");
}

fn print_raydium(summary: &polar_bear_arc_forge_defi::defi::raydium::PoolHealthSummary) {
    let wide = "═".repeat(70);
    let thin = "─".repeat(70);
    println!("\n{wide}");
    println!("  RAYDIUM POOL HEALTH - {}", summary.token_mint);
    println!("{wide}");
    if let Some(ref e) = summary.error {
        println!("  ⚠️  {e}");
    } else {
        println!("  Pools       : {}", summary.pool_count);
        println!("  Total TVL   : ${:.2}", summary.total_tvl_usd);
        println!("  Volume 24h  : ${:.2}", summary.total_volume_24h_usd);
        println!("  Best APY    : {:.2}%", summary.best_apy);
        println!("{thin}");
        for (i, p) in summary.pools.iter().enumerate() {
            println!(
                "  #{} {}  {}/{}  TVL ${:.0}  Vol ${:.0}  Price {:.6}",
                i + 1,
                &p.pool_id[..8.min(p.pool_id.len())],
                p.base_symbol,
                p.quote_symbol,
                p.liquidity_usd,
                p.volume_24h_usd,
                p.price,
            );
        }
    }
    println!("{wide}\n");
}

// ── Config builder ────────────────────────────────────────────────────────────

fn build_config(
    name: String,
    symbol: String,
    supply: u64,
    decimals: u8,
    sol: f64,
    lp_pct: f64,
    burn_lp: bool,
    lock_days: u32,
) -> LaunchConfig {
    LaunchConfig {
        token_name: name,
        token_symbol: symbol,
        total_supply: supply,
        decimals,
        mint_authority_renounced: true,
        freeze_authority_renounced: true,
        liquidity: LiquidityConfig {
            initial_liquidity_sol: sol,
            token_allocation_pct: lp_pct,
            burn_lp_tokens: burn_lp,
            lock_duration_days: lock_days,
            price_range_lower: 0.0,
            price_range_upper: 0.0,
        },
        network: SolanaNetwork::Devnet,
    }
}
