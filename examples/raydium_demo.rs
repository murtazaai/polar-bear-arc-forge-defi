//! # Raydium Pool Query Demo
//!
//! Demonstrates the Raydium v3 REST API integration - queries live pool
//! data for native SOL and USDC.
//!
//! ## Usage
//!
//! ```text
//! cargo run --example raydium_demo
//! ```
//!
//! No API key required.  Connects to the public Raydium v3 API.

/// Re-exports the `RaydiumClient` struct.
use polar_bear_arc_forge_defi::defi::{RaydiumClient, SOL_MINT, USDC_MINT};

/// Demonstrates the Raydium v3 REST API integration - queries live pool
/// data for native SOL and USDC.
///
/// No API key required.  Connects to the public Raydium v3 API.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("polar_bear_arc_forge_defi=info".parse()?),
        )
        .init();

    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║         ARC FORGE - RAYDIUM POOL QUERY DEMO                     ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
    println!();

    let client = RaydiumClient::new();

    // ── SOL pools ─────────────────────────────────────────────────────────────
    println!("── SOL pools (top 3 by TVL) ──────────────────────────────────────");
    let summary = client.pool_health_summary(SOL_MINT).await;
    print_summary(&summary);

    println!();

    // ── USDC pools ────────────────────────────────────────────────────────────
    println!("── USDC pools (top 3 by TVL) ─────────────────────────────────────");
    let usdc_pools = client.get_pools_for_mint(USDC_MINT, "all", 3).await;
    match usdc_pools {
        Ok(pools) => {
            for (i, p) in pools.iter().enumerate() {
                println!(
                    "  #{} {:8}  {}/{}  TVL: ${:.0}  Vol24h: ${:.0}  APY: {:.1}%",
                    i + 1,
                    &p.pool_id[..8.min(p.pool_id.len())],
                    p.base_symbol,
                    p.quote_symbol,
                    p.liquidity_usd,
                    p.volume_24h_usd,
                    p.apy
                );
            }
        }
        Err(e) => eprintln!("  ⚠  Raydium API error: {e}"),
    }

    println!();
    println!("✓  Raydium demo complete.");
    Ok(())
}

/// Prints a formatted summary of the Raydium pool health data.
///
/// # Arguments
///
/// * `s` - The `PoolHealthSummary` to print.
///
/// # Examples
///
/// ```
/// let summary = raydium::get_pool_health_summary("SOL");
/// print_summary(&summary);
/// ```
fn print_summary(s: &polar_bear_arc_forge_defi::defi::raydium::PoolHealthSummary) {
    if let Some(ref e) = s.error {
        eprintln!("  ⚠  {e}");
        return;
    }
    println!("  Pool count  : {}", s.pool_count);
    println!("  Total TVL   : ${:.2}", s.total_tvl_usd);
    println!("  Volume 24h  : ${:.2}", s.total_volume_24h_usd);
    println!("  Best APY    : {:.2}%", s.best_apy);
    for (i, p) in s.pools.iter().take(3).enumerate() {
        println!(
            "  #{} {:8}  {}/{}  TVL: ${:.0}  Price: {:.6}",
            i + 1,
            &p.pool_id[..8.min(p.pool_id.len())],
            p.base_symbol,
            p.quote_symbol,
            p.liquidity_usd,
            p.price
        );
    }
}
