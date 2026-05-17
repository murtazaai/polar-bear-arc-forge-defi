//! # Rig (ARC) AI agent integration
//!
//! Requires the `ai-agent` feature and a valid `ANTHROPIC_API_KEY`.
//!
//! ```text
//! cargo build --features ai-agent
//! export ANTHROPIC_API_KEY=sk-ant-...
//! ```

pub mod arc_forge_agent;

pub use arc_forge_agent::ArcForgeAgent;
