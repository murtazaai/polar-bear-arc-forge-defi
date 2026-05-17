//! # DeFi integrations
//!
//! - [`raydium`] - Raydium v3 REST API client (pool TVL, volume, APY)
//! - [`liquidity`] - Deep initial liquidity protocol (constant-product AMM model)

pub mod liquidity;
pub mod raydium;

pub use liquidity::DeepLiquidityProtocol;
pub use raydium::{RaydiumClient, SOL_MINT, USDC_MINT};
