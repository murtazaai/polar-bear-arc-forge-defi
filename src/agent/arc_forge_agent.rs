//! # ARC Forge AI Agent
//!
//! A `rig-core` agent that analyses [`LaunchSimulation`] reports and returns
//! a structured natural-language risk assessment via Claude.
//!
//! ## Required traits (rig-core ≥ 0.36)
//!
//! Both `CompletionClient` **and** `ProviderClient` must be in scope for
//! `.agent()` to resolve on `anthropic::Client`.  See `BUG-FIXES.md` Fix 2.
//!
//! ## Client storage
//!
//! `anthropic::Client` is stored directly - **not** wrapped in `Arc`.
//! Wrapping in `Arc` produces `Arc<Client>` on which `.agent()` cannot be
//! resolved because `CompletionClient` is implemented on `Client` directly,
//! not on `Arc<Client>`.  See `BUG-FIXES.md` Fix 1.
//!
//! ## Architecture
//!
//! ```text
//! LaunchSimulation (JSON)
//!     │
//!     ▼
//! ArcForgeAgent::analyse_simulation()
//!     │
//!     ├─ rig-core 0.37  CompletionClient + ProviderClient
//!     │  model: claude-sonnet-4-6
//!     │  preamble: DeFi security analyst
//!     │
//!     └─▶ natural-language risk assessment (≤ 300 words)
//! ```

use anyhow::Result;
use rig_core::client::ProviderClient;
use tracing::info;

use crate::types::LaunchSimulation;

// ── Model ─────────────────────────────────────────────────────────────────────

const AGENT_MODEL: &str = "claude-sonnet-4-6";

// ── Preamble ──────────────────────────────────────────────────────────────────

const PREAMBLE: &str = "\
You are an expert DeFi security analyst and Solana tokenomics specialist, \
operating as an ARC Forge launch analysis agent at Polar Bear Systems.

For every simulation report you receive, provide:
1. A concise risk assessment (3–5 sentences)
2. Specific concerns about the validation findings
3. Liquidity adequacy judgement (is the initial SOL allocation deep enough?)
4. Sniper-bot prevention effectiveness (freeze/mint authority analysis)
5. A final recommendation - LAUNCH, REVIEW, or BLOCK - with one sentence of reasoning

Be direct, technical, and concise.  Keep your entire response under 300 words.";

// ── ArcForgeAgent ─────────────────────────────────────────────────────────────

/// Rig (ARC) AI agent that analyses [`LaunchSimulation`] reports via Claude.
///
/// Requires the `ai-agent` feature and `ANTHROPIC_API_KEY` set in the
/// environment (or loaded from `.env` via `dotenvy`).
#[cfg(feature = "ai-agent")]
pub struct ArcForgeAgent {
    /// Anthropic client - stored directly, **not** wrapped in `Arc`.
    ///
    /// `Arc` is unnecessary: the client is consumed by `.agent()…build()` per
    /// call and never shared across tasks.  Wrapping in `Arc` produces
    /// `Arc<Client>` on which `.agent()` cannot be resolved (E0599).
    client: rig_core::providers::anthropic::Client,
}

#[cfg(feature = "ai-agent")]
impl ArcForgeAgent {
    /// Initialise the agent from `ANTHROPIC_API_KEY` in the environment.
    ///
    /// Calls `dotenvy::dotenv().ok()` so a `.env` file is automatically loaded
    /// in development without requiring the caller to do so.
    pub fn new() -> Result<Self> {
        use rig_core::providers::anthropic;
        let _ = dotenvy::dotenv();
        let client = anthropic::Client::from_env()?;
        Ok(Self { client })
    }

    /// Analyse a [`LaunchSimulation`] and return a natural-language assessment.
    ///
    /// The full simulation is serialised to JSON and sent as context to
    /// `claude-sonnet-4-6` via `rig-core`.  The agent returns its assessment
    /// as a plain string (≤ 300 words per the preamble).
    pub async fn analyse_simulation(&self, simulation: &LaunchSimulation) -> Result<String> {
        use rig_core::{
            client::CompletionClient,
            // client::ProviderClient,
            completion::Prompt,
        };

        let sim_json = serde_json::to_string_pretty(simulation)
            .unwrap_or_else(|_| "(serialisation failed)".to_string());

        let prompt = format!(
            "Analyse this ARC Forge launch simulation report:\n\n\
             ```json\n{sim_json}\n```\n\n\
             Provide your expert assessment following the format in your instructions."
        );

        info!(
            model = AGENT_MODEL,
            token = %simulation.config.token_symbol,
            "ArcForgeAgent - invoking rig-core agent"
        );

        // Build a fresh agent per call.  Both CompletionClient and ProviderClient
        // must be in scope for `.agent()` to resolve on anthropic::Client.
        let agent = self.client.agent(AGENT_MODEL).preamble(PREAMBLE).build();

        let response: String = agent.prompt(prompt.as_str()).await?;

        info!(
            response_len = response.len(),
            "ArcForgeAgent - analysis complete"
        );
        Ok(response)
    }
}

// ── Stub when feature is disabled ─────────────────────────────────────────────

/// No-op stub returned when the `ai-agent` feature is not compiled.
#[cfg(not(feature = "ai-agent"))]
pub struct ArcForgeAgent;

#[cfg(not(feature = "ai-agent"))]
impl ArcForgeAgent {
    /// Returns an informational message explaining how to enable the feature.
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    /// Always returns a placeholder message - no network call is made.
    pub async fn analyse_simulation(&self, _simulation: &LaunchSimulation) -> Result<String> {
        Ok("[Agent feature not compiled. \
             Rebuild with `--features ai-agent` and set ANTHROPIC_API_KEY \
             to enable rig-core AI analysis.]"
            .to_string())
    }
}
