//! # Solana RPC
//!
//! Lightweight Solana JSON-RPC 2.0 client built on `reqwest`.
//!
//! Decodes SPL Token mint accounts from raw on-chain bytes without a
//! `solana-sdk` dependency, keeping compile times fast and avoiding
//! crate-version conflicts with `rig-core`.

/// Provides a lightweight Solana JSON-RPC 2.0 client built on `reqwest`.
pub mod solana;

/// Exposes the [`SolanaRpcClient`] struct for use in other modules.
pub use solana::SolanaRpcClient;
