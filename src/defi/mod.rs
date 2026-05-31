//! # `DeFi` integrations
//!
//! - `raydium` - Raydium v3 REST API client (pool TVL, volume, APY)
//! - `liquidity` - Deep initial liquidity protocol (constant-product AMM model)

/// The [`DeepLiquidityProtocol`] struct provides a client for interacting with the Deep
/// Initial Liquidity Protocol on the Solana blockchain.
///
/// This module is compiled conditionally via the `defi` feature flag.
pub mod liquidity;

/// The [`RaydiumClient`] struct provides a client for interacting with the Raydium v3 REST API.
///
/// This module is compiled conditionally via the `defi` feature flag.
pub mod raydium;

/// Re-exports the [`DeepLiquidityProtocol`] struct from the `liquidity` module.
///
/// This module is compiled conditionally via the `defi` feature flag.
pub use liquidity::DeepLiquidityProtocol;
/// Re-exports the [`RaydiumClient`] struct from the `raydium` module.
///
/// This module is compiled conditionally via the `defi` feature flag.
pub use raydium::{RaydiumClient, SOL_MINT, USDC_MINT};
