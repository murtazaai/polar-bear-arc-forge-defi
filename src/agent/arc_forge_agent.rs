//! # ARC Forge AI Agent
//!
//! A `rig-core` agent that analyses `LaunchSimulation` reports and returns
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

/// The `ArcForgeAgent` is responsible for analysing Solana token launch simulations
/// and providing natural-language risk assessments.
use anyhow::Result;
/// The `ArcForgeAgent` is responsible for analysing Solana token launch simulations
/// and providing natural-language risk assessments.
///
/// It uses a language model to analyse simulation results and generate a concise risk
/// assessment.
///
/// The agent is configured with a preamble that defines its role and capabilities.
#[allow(unused_imports)]
use tracing::info;

/// The `LaunchSimulation` type defines the structure of a launch simulation report.
/// It contains all the data needed to analyse a token launch simulation and generate a risk
/// assessment.
use crate::types::LaunchSimulation;

/// The model used by the ARC Forge agent for natural-language risk assessment.
///
/// This is a constant that defines the language model to use for generating risk assessments.
/// It is used to configure the agent's language model capabilities.
#[allow(dead_code)]
const AGENT_MODEL: &str = "claude-sonnet-4-6";

/// The preamble for the ARC Forge agent, providing context for risk assessment.
///
/// This preamble defines the agent's role and capabilities, including its ability
/// to analyze Solana token launch simulations and provide natural-language risk assessments.
/// It is used to configure the agent's language model capabilities.
#[allow(dead_code)]
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

/// Rig (ARC) AI agent that analyses `LaunchSimulation` reports via Claude.
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

/// Default implementation for `ArcForgeAgent`, initialising from the environment.
///
/// Requires `ANTHROPIC_API_KEY` to be set in the environment or loaded from `.env`.
///
/// Calls `dotenvy::dotenv().ok()` so a `.env` file is automatically loaded
/// in development without requiring the caller to do so.
///
/// Returns an error if `ANTHROPIC_API_KEY` is not set.
#[cfg(feature = "ai-agent")]
impl ArcForgeAgent {
    /// Initialise the agent from `ANTHROPIC_API_KEY` in the environment.
    ///
    /// Calls `dotenvy::dotenv().ok()` so a `.env` file is automatically loaded
    /// in development without requiring the caller to do so.
    pub fn new() -> Result<Self> {
        use rig_core::{client::ProviderClient, providers::anthropic};
        let _ = dotenvy::dotenv();
        let client = anthropic::Client::from_env()?;
        Ok(Self { client })
    }

    /// Analyse a `LaunchSimulation` and return a natural-language assessment.
    ///
    /// The full simulation is serialised to JSON and sent as context to
    /// `claude-sonnet-4-6` via `rig-core`.  The agent returns its assessment
    /// as a plain string (≤ 300 words per the preamble).
    ///
    /// The simulation is sent as a JSON string to the agent's language model,
    /// which returns a natural-language assessment of the simulation's risk.
    ///
    /// Returns an error if the simulation cannot be analysed or if the agent fails to respond.
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
        //
        // The agent is configured with the `AGENT_MODEL` and `PREAMBLE` constants,
        // which define the model capabilities and agent role.
        let agent = self.client.agent(AGENT_MODEL).preamble(PREAMBLE).build();

        let response: String = agent.prompt(prompt.as_str()).await?;

        info!(
            response_len = response.len(),
            "ArcForgeAgent - analysis complete"
        );
        Ok(response)
    }
}

/// No-op stub returned when the `ai-agent` feature is not compiled.
///
/// Calls to `ArcForgeAgent` methods will return a placeholder message
/// explaining how to enable the feature.
///
/// Returns an error if `ANTHROPIC_API_KEY` is not set.
#[cfg(not(feature = "ai-agent"))]
pub struct ArcForgeAgent;

/// Returns an error if `ANTHROPIC_API_KEY` is not set.
///
/// Always returns a placeholder message - no network call is made.
#[cfg(not(feature = "ai-agent"))]
impl ArcForgeAgent {
    /// Returns an informational message explaining how to enable the feature.
    ///
    /// Always returns `Ok(Self)` - no network call is made.
    ///
    /// Always returns the same placeholder message.
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    /// Always returns a placeholder message - no network call is made.
    ///
    /// `async` is kept so this stub is a drop-in for the real implementation
    /// and call sites compile without `#[cfg]` guards.
    ///
    /// Always returns `Ok` - no error is returned.
    ///
    /// Always returns the same placeholder message.
    #[allow(clippy::unused_async)]
    pub async fn analyse_simulation(&self, _simulation: &LaunchSimulation) -> Result<String> {
        Ok("[Agent feature not compiled. \
             Rebuild with `--features ai-agent` and set ANTHROPIC_API_KEY \
             to enable rig-core AI analysis.]"
            .to_string())
    }
}
