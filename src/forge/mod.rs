//! # ARC Forge
//!
//! Orchestrates the Perceive → Evaluate → Validate (PEV) loop for a
//! Solana token launch simulation.

/// Provides functionality for simulating a planned launch using the ARC Forge PEV loop.
pub mod arc_forge;

/// Exposes the [`ArcForgeLauncher`] struct for use in other modules.
pub use arc_forge::ArcForgeLauncher;
