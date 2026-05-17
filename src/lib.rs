//! # polar-bear-arc-forge-defi
//!
//! ARC Forge DeFi platform - Solana token launch with sniper-bot prevention
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

pub mod agent;
pub mod defi;
pub mod forge;
pub mod rpc;
pub mod types;
pub mod validator;

// ── Convenience re-exports ────────────────────────────────────────────────────

pub use forge::ArcForgeLauncher;
pub use types::{
    LaunchConfig, LaunchSimulation, LiquidityConfig, LiquidityMetrics, MintInfo, PevLoopSummary,
    RaydiumPool, SolanaNetwork, ValidationCheck, ValidationReport, ValidationStatus,
};
pub use validator::TokenValidator;
