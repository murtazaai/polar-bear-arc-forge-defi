//! # polar-bear-arc-forge-defi
//!
//! ARC Forge `DeFi` platform - Solana token launch with sniper-bot prevention
//! and deep initial liquidity, powered by [Rig (ARC)](https://rig.rs) AI agents.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                    polar-bear-arc-forge-defi                        │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │  rpc::SolanaRpcClient      Solana JSON-RPC (no solana-sdk dep)     │
//! │  validator::TokenValidator  6 sniper-bot prevention checks          │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │  defi::RaydiumClient        Raydium v3 REST API (pool TVL / APY)   │
//! │  defi::DeepLiquidityProtocol  AMM model, depth score, anti-rug     │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │  forge::ArcForgeLauncher    PEV loop orchestrator                  │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │  agent::ArcForgeAgent       rig-core 0.37 (feature = ai-agent)     │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use polar_bear_arc_forge_defi::{
//!     forge::ArcForgeLauncher,
//!     types::LaunchConfig,
//! };
//!
//! let launcher = ArcForgeLauncher::new("https://api.devnet.solana.com");
//! let sim = launcher.simulate_planned_launch(LaunchConfig::default());
//! sim.print_report();
//! ```

/// Re-exports from the `agent` module.
///
/// This module re-exports the `ArcForgeAgent` struct and related types from the `agent` module.
pub mod agent;
/// Re-exports from the `defi` module.
///
/// This module re-exports the `defi` module and related types from the `defi` module.
pub mod defi;
/// Re-exports from the `forge` module.
///
/// This module re-exports the `forge` module and related types from the `forge` module.
pub mod forge;
/// Re-exports from the `rpc` module.
///
/// This module re-exports the `rpc` module and related types from the `rpc` module.
pub mod rpc;
/// Re-exports from the `types` module.
///
/// This module re-exports the `types` module and related types from the `types` module.
pub mod types;
/// Re-exports from the `validator` module.
///
/// This module re-exports the `validator` module and related types from the `validator` module.
pub mod validator;

/// Convenience re-exports from the `forge` module.
///
/// This module re-exports the `ArcForgeLauncher` struct and related types from the `forge`
/// module.
pub use forge::ArcForgeLauncher;
/// Convenience re-exports from the `types` module.
///
/// This module re-exports the `types` module and related types from the `types` module.
pub use types::{
    LaunchConfig, LaunchSimulation, LiquidityConfig, LiquidityMetrics, MintInfo, PevLoopSummary,
    RaydiumPool, SolanaNetwork, ValidationCheck, ValidationReport, ValidationStatus,
};
/// Convenience re-exports from the `validator` module.
///
/// This module re-exports the `TokenValidator` struct and related types from the `validator`
/// module.
pub use validator::TokenValidator;
