//! # Raydium v3 REST API client
//!
//! Queries the public [Raydium v3 API](https://api-v3.raydium.io/) for pool
//! information, liquidity depth, and 24-hour volume data.  No authentication
//! required - all data is public.
//!
//! ## Endpoints used
//!
//! | Path | Purpose |
//! |------|---------|
//! | `GET /pools/info/mint` | Pool list by token mint, sorted by TVL |
//!
//! Reference: <https://docs.raydium.io>

use anyhow::{Context, Result, anyhow};
use reqwest::Client;
use serde::Deserialize;
use tracing::{debug, info};

use crate::types::RaydiumPool;

// ── Well-known mints ──────────────────────────────────────────────────────────

/// Native SOL wrapped mint address on Solana mainnet.
pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
/// USDC mint address on Solana mainnet.
pub const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

const RAYDIUM_API_BASE: &str = "https://api-v3.raydium.io";

// ── Raydium API response shapes ───────────────────────────────────────────────

#[derive(Deserialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    #[serde(default)]
    msg: Option<String>,
}

#[derive(Deserialize)]
struct PoolPage {
    data: Vec<RawPool>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawPool {
    id: String,
    #[serde(rename = "mintA")]
    mint_a: RawMintMeta,
    #[serde(rename = "mintB")]
    mint_b: RawMintMeta,
    #[serde(default)]
    tvl: f64,
    #[serde(default)]
    day: Option<DayStats>,
    #[serde(default)]
    price: f64,
}

#[derive(Deserialize)]
struct RawMintMeta {
    address: String,
    #[serde(default)]
    symbol: String,
}

#[derive(Deserialize)]
struct DayStats {
    #[serde(default)]
    volume: f64,
    #[serde(default)]
    apr: f64,
}

// ── PoolHealthSummary ─────────────────────────────────────────────────────────

/// Aggregated health metrics for a token across all its Raydium pools.
#[derive(Debug, serde::Serialize)]
pub struct PoolHealthSummary {
    /// The token mint that was queried.
    pub token_mint: String,
    /// Number of active pools found.
    pub pool_count: usize,
    /// Sum of TVL across all pools in USD.
    pub total_tvl_usd: f64,
    /// Sum of 24-hour volume across all pools in USD.
    pub total_volume_24h_usd: f64,
    /// Highest APY across all pools.
    pub best_apy: f64,
    /// Individual pool details.
    pub pools: Vec<RaydiumPool>,
    /// Set if the API call failed or returned no pools.
    pub error: Option<String>,
}

// ── RaydiumClient ─────────────────────────────────────────────────────────────

/// Raydium v3 REST API client.
pub struct RaydiumClient {
    http: Client,
}

impl Default for RaydiumClient {
    fn default() -> Self {
        Self::new()
    }
}

impl RaydiumClient {
    /// Create a new client with a 20-second request timeout.
    pub fn new() -> Self {
        Self {
            http: Client::builder()
                .timeout(std::time::Duration::from_secs(20))
                .user_agent("polar-bear-arc-forge-defi/0.2.0")
                .build()
                .expect("reqwest Client construction is infallible"),
        }
    }

    /// Fetch pools for `mint`, filtered by `pool_type`, sorted by TVL descending.
    ///
    /// `pool_type` accepts `"all"`, `"standard"`, or `"concentrated"`.
    pub async fn get_pools_for_mint(
        &self,
        mint: &str,
        pool_type: &str,
        page_size: u32,
    ) -> Result<Vec<RaydiumPool>> {
        let url = format!(
            "{RAYDIUM_API_BASE}/pools/info/mint\
             ?mint1={mint}\
             &poolType={pool_type}\
             &poolSortField=liquidity\
             &sortType=desc\
             &pageSize={page_size}\
             &page=1"
        );

        info!(mint, pool_type, "Querying Raydium v3 API");
        debug!(url = %url, "Raydium API request");

        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .context("Raydium API HTTP request failed")?;

        let status = resp.status();
        debug!(status = %status, "Raydium API response");

        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Raydium API HTTP {status}: {body}"));
        }

        let api: ApiResponse<PoolPage> = resp
            .json()
            .await
            .context("Failed to parse Raydium API JSON")?;

        if !api.success {
            return Err(anyhow!(
                "Raydium API returned success=false: {}",
                api.msg.unwrap_or_default()
            ));
        }

        Ok(api
            .data
            .map(|p| p.data)
            .unwrap_or_default()
            .into_iter()
            .map(into_pool)
            .collect())
    }

    /// Return an aggregated [`PoolHealthSummary`] for `token_mint`.
    ///
    /// Errors from the API are captured in [`PoolHealthSummary::error`] rather
    /// than propagated, so callers can render partial results.
    pub async fn pool_health_summary(&self, token_mint: &str) -> PoolHealthSummary {
        match self.get_pools_for_mint(token_mint, "all", 10).await {
            Ok(pools) if !pools.is_empty() => {
                let total_tvl = pools.iter().map(|p| p.liquidity_usd).sum();
                let total_vol = pools.iter().map(|p| p.volume_24h_usd).sum();
                let best_apy = pools.iter().map(|p| p.apy).fold(0.0_f64, f64::max);
                PoolHealthSummary {
                    token_mint: token_mint.to_owned(),
                    pool_count: pools.len(),
                    total_tvl_usd: total_tvl,
                    total_volume_24h_usd: total_vol,
                    best_apy,
                    pools,
                    error: None,
                }
            }
            Ok(_) => PoolHealthSummary {
                token_mint: token_mint.to_owned(),
                pool_count: 0,
                total_tvl_usd: 0.0,
                total_volume_24h_usd: 0.0,
                best_apy: 0.0,
                pools: vec![],
                error: Some("No pools found for this token on Raydium".to_string()),
            },
            Err(e) => PoolHealthSummary {
                token_mint: token_mint.to_owned(),
                pool_count: 0,
                total_tvl_usd: 0.0,
                total_volume_24h_usd: 0.0,
                best_apy: 0.0,
                pools: vec![],
                error: Some(e.to_string()),
            },
        }
    }
}

// ── Conversion ────────────────────────────────────────────────────────────────

fn into_pool(raw: RawPool) -> RaydiumPool {
    RaydiumPool {
        pool_id: raw.id,
        base_mint: raw.mint_a.address,
        quote_mint: raw.mint_b.address,
        base_symbol: raw.mint_a.symbol,
        quote_symbol: raw.mint_b.symbol,
        liquidity_usd: raw.tvl,
        volume_24h_usd: raw.day.as_ref().map_or(0.0, |d| d.volume),
        apy: raw.day.as_ref().map_or(0.0, |d| d.apr),
        price: raw.price,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn into_pool_maps_fields_correctly() {
        let raw = RawPool {
            id: "pool-abc".to_string(),
            mint_a: RawMintMeta {
                address: "TokenMint111".to_string(),
                symbol: "TKN".to_string(),
            },
            mint_b: RawMintMeta {
                address: SOL_MINT.to_string(),
                symbol: "SOL".to_string(),
            },
            tvl: 250_000.0,
            day: Some(DayStats {
                volume: 80_000.0,
                apr: 55.5,
            }),
            price: 0.00042,
        };
        let pool = into_pool(raw);
        assert_eq!(pool.pool_id, "pool-abc");
        assert_eq!(pool.base_symbol, "TKN");
        assert_eq!(pool.quote_mint, SOL_MINT);
        assert!((pool.liquidity_usd - 250_000.0).abs() < f64::EPSILON);
        assert!((pool.apy - 55.5).abs() < f64::EPSILON);
    }

    #[test]
    fn into_pool_handles_missing_day_stats() {
        let raw = RawPool {
            id: "p".to_string(),
            mint_a: RawMintMeta {
                address: "A".to_string(),
                symbol: "A".to_string(),
            },
            mint_b: RawMintMeta {
                address: "B".to_string(),
                symbol: "B".to_string(),
            },
            tvl: 0.0,
            day: None,
            price: 0.0,
        };
        let pool = into_pool(raw);
        assert!((pool.volume_24h_usd - 0.0).abs() < f64::EPSILON);
        assert!((pool.apy - 0.0).abs() < f64::EPSILON);
    }
}
