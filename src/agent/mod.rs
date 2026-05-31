//! # Rig (ARC) AI agent integration
//!
//! Requires the `ai-agent` feature and a valid `ANTHROPIC_API_KEY`.
//!
//! ```text
//! cargo build --features ai-agent
//! export ANTHROPIC_API_KEY=sk-ant-...
//! ```

/// The `ArcForgeAgent` is an AI agent that uses the ARC (Anthropic) API to analyse simulations.
pub mod arc_forge_agent;

/// Re-exports the `ArcForgeAgent` struct.
pub use arc_forge_agent::ArcForgeAgent;
