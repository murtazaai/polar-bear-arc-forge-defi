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

/// Convenience re-exports from the `anyhow` module.
///
/// This module re-exports the `Result` type from the `anyhow` module.
use anyhow::Result;
/// Convenience re-exports from the `clap` module.
///
/// This module re-exports the `Parser` and `Subcommand` types from the `clap` module.
use clap::{Parser, Subcommand};
/// Convenience re-exports from the `tracing_subscriber` module.
///
/// This module re-exports the `EnvFilter` and `FmtSubscriber` types from the
/// `tracing_subscriber` module.
use polar_bear_arc_forge_defi::{
    agent::ArcForgeAgent,
    defi::RaydiumClient,
    forge::ArcForgeLauncher,
    rpc::SolanaRpcClient,
    types::{LaunchConfig, LiquidityConfig, SolanaNetwork, ValidationStatus},
    validator::TokenValidator,
};
/// Convenience re-exports from the `tracing_subscriber` module.
use tracing_subscriber::{EnvFilter, FmtSubscriber};

#[derive(Parser)]
#[command(
    name = "arc-forge",
    version,
    about = "ARC Forge DeFi Platform - Solana token launch with sniper-bot prevention\n\
             and deep initial liquidity, powered by Rig (ARC) AI agents.\n\n\
             https://github.com/murtazaai/polar-bear-arc-forge-defi",
    long_about = None,
)]
/// CLI for the ARC Forge `DeFi` platform.
struct Cli {
    /// Solana RPC endpoint (overrides `SOLANA_RPC_URL` env var).
    #[arg(
        long,
        env = "SOLANA_RPC_URL",
        default_value = "https://api.devnet.solana.com",
        global = true
    )]
    /// Solana RPC endpoint URL.
    rpc_url: String,

    /// Log level: error | warn | info | debug | trace
    #[arg(long, env = "RUST_LOG", default_value = "info", global = true)]
    log_level: String,

    /// Subcommand to execute.
    #[command(subcommand)]
    command: Commands,
}

/// Subcommands for the ARC Forge `DeFi` platform.
///
/// Each subcommand corresponds to a specific ARC Forge `DeFi` platform operation.
#[derive(Subcommand)]
enum Commands {
    /// Fetch a live SPL Token mint and run all sniper-bot prevention checks.
    ///
    /// This command validates a given SPL Token mint address and runs all necessary checks to
    /// prevent sniper-bot activity.
    ///
    /// # Arguments
    ///
    /// * `mint` - SPL Token mint address (base58-encoded).
    /// * `json` - Emit JSON instead of human-readable output.
    Validate {
        /// SPL Token mint address (base58-encoded).
        ///
        /// This is the mint address of the SPL Token to validate.
        #[arg(short, long)]
        mint: String,
        /// Emit JSON instead of human-readable output.
        ///
        /// This flag enables JSON output instead of the default human-readable format.
        #[arg(long)]
        json: bool,
    },

    /// Query the Raydium v3 REST API for pool data.
    ///
    /// This command fetches pool data from the Raydium v3 REST API and displays it in the console.
    ///
    /// # Arguments
    ///
    /// * `mint` - Token mint to query (defaults to native SOL).
    /// * `pool_type` - Pool type filter: all | standard | concentrated.
    /// * `limit` - Maximum number of pools to return (1–20).
    /// * `json` - Emit JSON.
    ///
    /// # Examples
    ///
    /// ```
    /// $ arc-forge raydium --mint So11111111111111111111111111111111111111112 --pool-type standard --limit 10 --json
    /// ```
    Raydium {
        /// Token mint to query (defaults to native SOL).
        ///
        /// This is the base58-encoded mint address of the token to query.
        ///
        /// Defaults to `So11111111111111111111111111111111111111112` (native SOL).
        #[arg(
            short,
            long,
            default_value = "So11111111111111111111111111111111111111112"
        )]
        mint: String,
        /// Pool type filter: all | standard | concentrated
        ///
        /// Defaults to `all`.
        ///
        /// Valid values are `all`, `standard`, and `concentrated`.
        #[arg(long, default_value = "all")]
        pool_type: String,
        /// Maximum number of pools to return (1–20).
        ///
        /// Defaults to `5`.
        #[arg(long, default_value = "5")]
        limit: u32,
        /// Emit JSON.
        ///
        /// Defaults to `false`.
        ///
        /// If `true`, the output will be emitted as JSON.
        #[arg(long)]
        json: bool,
    },

    /// Run a full ARC Forge launch simulation (always dry-run, no SOL spent).
    ///
    /// This command simulates a full ARC Forge launch, including token creation, liquidity pool
    /// creation, and token distribution.
    ///
    /// If `json` is `true`, the output will be emitted as JSON.
    ///
    /// If `json` is `false`, the output will be emitted as a human-readable summary.
    Launch {
        /// Total token supply in smallest units.
        ///
        /// Defaults to `1000000000000000` (1 million tokens).
        #[arg(long, default_value = "1000000000000000")]
        supply: u64,
        /// Token decimal places (standard: 9).
        ///
        /// Defaults to `9`.
        ///
        /// Must be between `0` and `18` (inclusive).
        #[arg(long, default_value = "9")]
        decimals: u8,
        /// Token name.
        ///
        /// Defaults to `"ARC Forge Demo Token"`.
        #[arg(long, default_value = "ARC Forge Demo Token")]
        name: String,
        /// Token ticker symbol.
        ///
        /// Defaults to `"ARCD"`.
        #[arg(long, default_value = "ARCD")]
        symbol: String,
        /// SOL for the initial liquidity pool.
        ///
        /// Defaults to `20.0`.
        #[arg(long, default_value = "20.0")]
        sol: f64,
        /// Percentage of supply in the initial LP pool.
        ///
        /// Defaults to `10.0`.
        #[arg(long, default_value = "10.0")]
        lp_pct: f64,
        /// Burn LP tokens (permanent, strongest anti-rug).
        ///
        /// Defaults to `false`.
        #[arg(long)]
        burn_lp: bool,
        /// Days to lock LP tokens (ignored when --burn-lp is set).
        ///
        /// Defaults to `0`.
        #[arg(long, default_value = "0")]
        lock_days: u32,
        /// Simulate against an existing on-chain mint instead of a planned config.
        ///
        /// Defaults to `None`.
        #[arg(long)]
        mint: Option<String>,
        /// Emit JSON.
        ///
        /// Defaults to `false`.
        #[arg(long)]
        json: bool,
    },

    /// Launch simulation + Rig (ARC) AI agent analysis.
    ///
    /// Requires --features ai-agent and `ANTHROPIC_API_KEY`.
    ///
    /// Defaults to `false`.
    Agent {
        /// Total token supply.
        ///
        /// Defaults to `1000000000000000`.
        #[arg(long, default_value = "1000000000000000")]
        supply: u64,
        /// Token decimal places.
        ///
        /// Defaults to `9`.
        #[arg(long, default_value = "9")]
        decimals: u8,
        /// Token name.
        ///
        /// Defaults to `ARC Forge Demo Token`.
        #[arg(long, default_value = "ARC Forge Demo Token")]
        name: String,
        /// Token symbol.
        ///
        /// Defaults to `ARCD`.
        #[arg(long, default_value = "ARCD")]
        symbol: String,
        /// SOL for initial liquidity.
        ///
        /// Defaults to `20.0`.
        #[arg(long, default_value = "20.0")]
        sol: f64,
        /// LP supply allocation percentage.
        ///
        /// Defaults to `10.0`.
        #[arg(long, default_value = "10.0")]
        lp_pct: f64,
        /// Burn LP tokens.
        ///
        /// Defaults to `false`.
        #[arg(long)]
        burn_lp: bool,
        /// Emit JSON.
        ///
        /// Defaults to `false`.
        #[arg(long)]
        json: bool,
    },

    /// Connectivity check: Solana RPC current slot + Raydium API ping.
    ///
    /// Defaults to `false`.
    Check,
}

/// Launch a new SPL Token on Solana using the ARC Forge protocol.
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
        Commands::Validate { mint, json } => run_validate(&cli.rpc_url, &mint, json).await?,
        Commands::Raydium {
            mint,
            pool_type,
            limit,
            json,
        } => {
            run_raydium(&mint, &pool_type, limit, json).await?;
        }
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
            run_launch(&cli.rpc_url, cfg, mint, json).await?;
        }
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
            run_agent(
                &cli.rpc_url,
                name,
                symbol,
                supply,
                decimals,
                sol,
                lp_pct,
                burn_lp,
                json,
            )
            .await?;
        }
        Commands::Check => run_check(&cli.rpc_url).await,
    }

    Ok(())
}

/// Validate a SPL Token on Solana using the ARC Forge protocol.
///
/// Defaults to `false` for JSON output.
///
/// Returns a validation report as JSON if `json` is `true`, otherwise prints a human-readable
/// summary.
async fn run_validate(rpc_url: &str, mint: &str, json: bool) -> Result<()> {
    let report = TokenValidator::new(rpc_url).validate(mint).await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_validation(&report);
    }
    Ok(())
}

/// Fetch Raydium pools for a given SPL Token and pool type.
///
/// Defaults to `false` for JSON output.
///
/// Returns a list of pools as JSON if `json` is `true`, otherwise prints a human-readable summary.
async fn run_raydium(mint: &str, pool_type: &str, limit: u32, json: bool) -> Result<()> {
    let client = RaydiumClient::new();
    if json {
        let pools = client.get_pools_for_mint(mint, pool_type, limit).await?;
        println!("{}", serde_json::to_string_pretty(&pools)?);
    } else {
        let summary = client.pool_health_summary(mint).await;
        print_raydium(&summary);
    }
    Ok(())
}

/// Launch a new SPL Token on Solana using the ARC Forge protocol.
///
/// Defaults to `false` for JSON output.
///
/// Returns a launch report as JSON if `json` is `true`, otherwise prints a human-readable summary.
async fn run_launch(
    rpc_url: &str,
    cfg: LaunchConfig,
    mint: Option<String>,
    json: bool,
) -> Result<()> {
    let launcher = ArcForgeLauncher::new(rpc_url);
    let sim = match mint {
        Some(ref addr) => launcher.simulate_existing_mint(addr, cfg).await?,
        None => launcher.simulate_planned_launch(cfg),
    };
    if json {
        println!("{}", serde_json::to_string_pretty(&sim)?);
    } else {
        sim.print_report();
    }
    Ok(())
}

/// Run the agent for a given SPL Token on Solana using the ARC Forge protocol.
///
/// Defaults to `false` for JSON output.
///
/// Returns an agent report as JSON if `json` is `true`, otherwise prints a human-readable summary.
async fn run_agent(
    rpc_url: &str,
    name: String,
    symbol: String,
    supply: u64,
    decimals: u8,
    sol: f64,
    lp_pct: f64,
    burn_lp: bool,
    json: bool,
) -> Result<()> {
    let cfg = build_config(name, symbol, supply, decimals, sol, lp_pct, burn_lp, 0);
    let launcher = ArcForgeLauncher::new(rpc_url);
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
        Err(e) => {
            eprintln!("⚠️  Could not initialise agent: {e}\n   Ensure ANTHROPIC_API_KEY is set.");
        }
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&sim)?);
    } else {
        sim.print_report();
    }
    Ok(())
}


/// Run the connectivity check for a given Solana RPC URL and Raydium v3 API.
///
/// Prints a human-readable summary of the checks.
///
/// Returns `Ok(())` if all checks pass, otherwise returns an error.
async fn run_check(rpc_url: &str) {
    println!("🔍  Connectivity check\n");
    print!("  Solana RPC ({rpc_url})  … ");
    match SolanaRpcClient::new(rpc_url).get_slot().await {
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

/// Prints a human-readable summary of the validation report.
///
/// The report includes the mint address, timestamp, overall status, risk score, and individual
/// check results.
///
/// # Arguments
///
/// * `report` - The validation report to print.
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

/// Prints a human-readable summary of the Raydium pool health summary.
///
/// # Arguments
///
/// * `summary` - The Raydium pool health summary to print.
///
/// # Returns
///
/// * `()` - This function does not return a value.
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

/// Builds a [`LaunchConfig`] from the given token details and parameters.
///
/// # Arguments
///
/// * `name` - The name of the token.
/// * `symbol` - The symbol of the token.
/// * `supply` - The total supply of the token.
/// * `decimals` - The number of decimals of the token.
/// * `sol` - The initial liquidity in SOL.
/// * `lp_pct` - The token allocation percentage in the liquidity pool.
/// * `burn_lp` - Whether to burn LP tokens.
/// * `lock_days` - The duration of the lock in days.
///
/// # Returns
///
/// * `LaunchConfig` - The built launch configuration.
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
