//! Live Anthropic provider tests - require `ANTHROPIC_API_KEY` + `--features ai-agent`.
//!
//! These tests are gated behind `#[ignore]` so they are **skipped in CI** when
//! the API key is absent.  Run them manually with:
//!
//! ```text
//! ANTHROPIC_API_KEY=sk-ant-... \
//!     cargo test --test providers --features ai-agent -- --ignored --test-threads=1
//! ```
//!
//! Use `--test-threads=1` to avoid concurrent API calls hitting rate limits.

/// Tests for the AI agent provider.
///
/// These tests verify the AI agent provider's ability to compute anti-rug ratings and
/// generate launch reviews based on the provided configuration.
///
/// These tests are **ignored in CI** and must be run manually with `ANTHROPIC_API_KEY` set.
#[cfg(feature = "ai-agent")]
mod agent_tests {
    /// Tests that the AI agent returns a non-empty analysis.
    use polar_bear_arc_forge_defi::{
        agent::ArcForgeAgent,
        forge::ArcForgeLauncher,
        types::{LaunchConfig, LiquidityConfig, SolanaNetwork},
    };

    /// Returns a safe launch configuration for testing.
    ///
    /// This configuration is safe to use in tests, as it does not require an API key.
    /// It uses a test token name and symbol, and a small total supply for quick execution.
    ///
    /// # Returns
    ///
    /// A `LaunchConfig` instance with the safe test configuration.
    fn safe_config() -> LaunchConfig {
        LaunchConfig {
            token_name: "Live Test Token".to_string(),
            token_symbol: "LTT".to_string(),
            total_supply: 1_000_000_000_000_000,
            decimals: 9,
            mint_authority_renounced: true,
            freeze_authority_renounced: true,
            liquidity: LiquidityConfig {
                initial_liquidity_sol: 20.0,
                token_allocation_pct: 10.0,
                burn_lp_tokens: true,
                lock_duration_days: 0,
                price_range_lower: 0.0,
                price_range_upper: 0.0,
            },
            network: SolanaNetwork::Devnet,
        }
    }

    /// Verify the agent returns a non-empty analysis string.
    ///
    /// This test runs the AI agent with a safe configuration and verifies that it returns a
    /// non-empty string.
    ///
    /// # Panics
    ///
    /// Panics if the agent call fails or returns an empty string.
    #[tokio::test]
    #[ignore = "requires ANTHROPIC_API_KEY and --features ai-agent"]
    async fn live_agent_returns_non_empty_analysis() {
        let launcher = ArcForgeLauncher::new("https://api.devnet.solana.com");
        let sim = launcher.simulate_planned_launch(safe_config());

        let agent = ArcForgeAgent::new().expect("ArcForgeAgent::new must succeed");
        let analysis = agent
            .analyse_simulation(&sim)
            .await
            .expect("agent call must succeed");

        assert!(!analysis.is_empty(), "analysis must not be empty");
    }

    /// Verify the analysis contains one of the expected recommendation strings.
    ///
    /// This test verifies that the AI agent returns an analysis that mentions one of the
    /// expected recommendation strings: LAUNCH, REVIEW, or BLOCK.
    ///
    /// # Panics
    ///
    /// Panics if the agent call fails or returns an empty string.
    #[tokio::test]
    #[ignore = "requires ANTHROPIC_API_KEY and --features ai-agent"]
    async fn live_agent_mentions_launch_review_or_block() {
        let launcher = ArcForgeLauncher::new("https://api.devnet.solana.com");
        let sim = launcher.simulate_planned_launch(safe_config());

        let agent = ArcForgeAgent::new().expect("ArcForgeAgent::new must succeed");
        let analysis = agent
            .analyse_simulation(&sim)
            .await
            .expect("agent call must succeed");

        let upper = analysis.to_uppercase();
        assert!(
            upper.contains("LAUNCH") || upper.contains("REVIEW") || upper.contains("BLOCK"),
            "analysis must contain LAUNCH, REVIEW, or BLOCK; got: {analysis}"
        );
    }
}
