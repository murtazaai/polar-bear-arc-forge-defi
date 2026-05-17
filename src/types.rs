//! # Shared types
//!
//! All data structures shared across the `rpc`, `validator`, `defi`, `forge`,
//! and `agent` modules.  Every type is `Serialize + Deserialize` so the full
//! [`LaunchSimulation`] report can be emitted as a JSON audit record.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

// ── Solana mint ───────────────────────────────────────────────────────────────

/// SPL Token mint account decoded from raw Solana on-chain data.
///
/// The 82-byte mint layout is parsed manually by [`crate::rpc::SolanaRpcClient`]
/// without a `solana-sdk` dependency.  See [`crate::rpc::solana`] for the
/// byte-offset constants and decoding logic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintInfo {
    /// Base58-encoded mint address.
    pub address: String,
    /// Total supply in the smallest unit (pre-decimal adjustment).
    pub supply: u64,
    /// Decimal places (standard Solana tokens: 6–9).
    pub decimals: u8,
    /// Whether the mint account has been initialised on-chain.
    pub is_initialized: bool,
    /// If `Some`, this pubkey can mint additional tokens - inflation risk.
    pub mint_authority: Option<String>,
    /// If `Some`, this pubkey can freeze any holder account - sniper-bot vector.
    pub freeze_authority: Option<String>,
}

// ── Validation ────────────────────────────────────────────────────────────────

/// Risk level assigned to each [`ValidationCheck`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValidationStatus {
    /// No risk detected for this check.
    Safe,
    /// Potential risk; review before launch.
    Warning,
    /// Critical risk; ARC Forge blocks launch.
    Dangerous,
}

impl fmt::Display for ValidationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Safe => write!(f, "✅ SAFE"),
            Self::Warning => write!(f, "⚠️  WARNING"),
            Self::Dangerous => write!(f, "🚨 DANGEROUS"),
        }
    }
}

/// Result of a single sniper-bot / rug-pull prevention check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationCheck {
    /// Human-readable check name (e.g. `"Freeze Authority"`).
    pub name: String,
    /// `true` if the check passed (no risk detected).
    pub passed: bool,
    /// Severity level for this check result.
    pub status: ValidationStatus,
    /// Explanation of what was found and why it matters.
    pub message: String,
}

/// Full token validation report produced by [`crate::validator::TokenValidator`].
///
/// JSON-serialisable and publishable as transparent, and on-chain-verifiable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    /// The mint address that was validated.
    pub mint_address: String,
    /// UTC timestamp of the validation query.
    pub timestamp: DateTime<Utc>,
    /// Aggregate risk level across all checks.
    pub overall_status: ValidationStatus,
    /// Individual check results.
    pub checks: Vec<ValidationCheck>,
    /// Composite risk score: 0 = safest, 100 = maximum danger.
    pub risk_score: u8,
    /// Plain-English recommendation for the launch team.
    pub recommendation: String,
}

// ── Raydium ───────────────────────────────────────────────────────────────────

/// Raydium liquidity pool data from the public v3 REST API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumPool {
    /// On-chain pool ID (base58 address).
    pub pool_id: String,
    /// Base token mint address.
    pub base_mint: String,
    /// Quote token mint address.
    pub quote_mint: String,
    /// Base token ticker symbol (e.g. `"SOL"`).
    pub base_symbol: String,
    /// Quote token ticker symbol (e.g. `"USDC"`).
    pub quote_symbol: String,
    /// Total value locked in USD.
    pub liquidity_usd: f64,
    /// 24-hour trading volume in USD.
    pub volume_24h_usd: f64,
    /// Annualised percentage yield for LP providers.
    pub apy: f64,
    /// Current token price in quote-token units.
    pub price: f64,
}

// ── Liquidity config ──────────────────────────────────────────────────────────

/// Configuration for ARC Forge's deep initial liquidity provisioning.
///
/// Burning LP tokens (`burn_lp_tokens = true`) is the strongest anti-rug signal:
/// it makes the initial liquidity permanent and provably irremovable on-chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityConfig {
    /// SOL allocated to the initial LP position.
    pub initial_liquidity_sol: f64,
    /// Percentage of total token supply placed into the LP pool (0–100).
    pub token_allocation_pct: f64,
    /// Burn LP tokens on receipt - permanent liquidity, strongest anti-rug.
    pub burn_lp_tokens: bool,
    /// Days to lock LP tokens if not burning (0 when `burn_lp_tokens` is true).
    pub lock_duration_days: u32,
    /// CLMM lower price bound (USD); set to 0 for full-range AMM pool.
    pub price_range_lower: f64,
    /// CLMM upper price bound (USD); set to 0 for full-range AMM pool.
    pub price_range_upper: f64,
}

impl LiquidityConfig {
    /// Conservative launch defaults: burn LP, full-range AMM, 10 % supply in pool.
    #[must_use]
    pub fn conservative_defaults(initial_sol: f64) -> Self {
        Self {
            initial_liquidity_sol: initial_sol,
            token_allocation_pct: 10.0,
            burn_lp_tokens: true,
            lock_duration_days: 0,
            price_range_lower: 0.0,
            price_range_upper: 0.0,
        }
    }
}

/// Calculated metrics produced by [`crate::defi::DeepLiquidityProtocol`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityMetrics {
    /// The configuration used for these calculations.
    pub config: LiquidityConfig,
    /// Number of tokens placed into the LP pool (raw units).
    pub tokens_in_pool: u64,
    /// Estimated launch price in USD per token.
    pub estimated_initial_price_usd: f64,
    /// Estimated fully-diluted market cap in USD at launch price.
    pub estimated_market_cap_usd: f64,
    /// Liquidity depth score 0–100; higher = harder to manipulate.
    pub liquidity_depth_score: u8,
    /// Constant-product price impact for a $1 000 buy order (percent).
    pub price_impact_1k_usd_buy_pct: f64,
    /// Constant-product price impact for a $10 000 buy order (percent).
    pub price_impact_10k_usd_buy_pct: f64,
    /// Human-readable anti-rug rating (⭐ RISKY → ⭐⭐⭐⭐⭐ DIAMOND).
    pub anti_rug_rating: String,
}

// ── Launch config ─────────────────────────────────────────────────────────────

/// Target Solana network for a launch simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SolanaNetwork {
    /// Mainnet-beta - real SOL, real tokens.
    Mainnet,
    /// Devnet - free test SOL, no real value.
    Devnet,
    /// Testnet - pre-release validator network.
    Testnet,
}

impl SolanaNetwork {
    /// JSON-RPC endpoint URL for this network.
    #[must_use]
    pub fn rpc_url(&self) -> &str {
        match self {
            Self::Mainnet => "https://api.mainnet-beta.solana.com",
            Self::Devnet => "https://api.devnet.solana.com",
            Self::Testnet => "https://api.testnet.solana.com",
        }
    }

    /// Short network label used in log output and reports.
    #[must_use]
    pub fn label(&self) -> &str {
        match self {
            Self::Mainnet => "mainnet-beta",
            Self::Devnet => "devnet",
            Self::Testnet => "testnet",
        }
    }
}

/// Full configuration for an ARC Forge token launch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchConfig {
    /// Human-readable token name (e.g. `"Polar Bear Token"`).
    pub token_name: String,
    /// Ticker symbol (e.g. `"PBT"`).
    pub token_symbol: String,
    /// Total supply in the smallest unit (e.g. `1_000_000_000_000_000` = 1 M tokens at 9 dec).
    pub total_supply: u64,
    /// Decimal places (standard: 9).
    pub decimals: u8,
    /// Mint authority renounced at launch - no future inflation possible.
    pub mint_authority_renounced: bool,
    /// Freeze authority renounced at launch - no account freezing possible.
    pub freeze_authority_renounced: bool,
    /// Liquidity provisioning configuration.
    pub liquidity: LiquidityConfig,
    /// Target network for this launch simulation.
    pub network: SolanaNetwork,
}

impl Default for LaunchConfig {
    fn default() -> Self {
        Self {
            token_name: "ARC Forge Demo Token".to_string(),
            token_symbol: "ARCD".to_string(),
            total_supply: 1_000_000_000_000_000,
            decimals: 9,
            mint_authority_renounced: true,
            freeze_authority_renounced: true,
            liquidity: LiquidityConfig::conservative_defaults(10.0),
            network: SolanaNetwork::Devnet,
        }
    }
}

// ── PEV loop ──────────────────────────────────────────────────────────────────

/// Perceive → Evaluate → Validate loop summary.
///
/// ARC agents operate in a stateful PEV loop:
///
/// - **Perceive** - read on-chain and off-chain state
/// - **Evaluate** - reason about risk, opportunity, and configuration
/// - **Validate** - confirm all safety gates before any state-changing action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PevLoopSummary {
    /// What the agent observed (on-chain mint data, pool state).
    pub perceive: String,
    /// What the agent analysed (risk score, liquidity metrics).
    pub evaluate: String,
    /// What the agent confirmed (safety gates, readiness decision).
    pub validate: String,
}

// ── Full launch simulation ────────────────────────────────────────────────────

/// Complete output of an ARC Forge launch simulation (always dry-run).
///
/// This is the primary artefact for the `polar-bear-arc-forge-defi`
/// repository.  It is JSON-serialisable and captures every decision the system
/// made together with the on-chain data it was based on.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchSimulation {
    /// The launch configuration used for this simulation.
    pub config: LaunchConfig,
    /// Token validation report from the on-chain sniper-bot prevention checks.
    pub validation_report: ValidationReport,
    /// Deep liquidity metrics computed from the constant-product AMM model.
    pub liquidity_metrics: LiquidityMetrics,
    /// `true` when all sniper-bot prevention gates passed.
    pub sniper_bot_prevention_active: bool,
    /// Launch readiness score 0–100; ≥ 80 clears for launch.
    pub launch_readiness_score: u8,
    /// Always `true` - no real SOL is spent in this repository.
    pub dry_run: bool,
    /// UTC timestamp of this simulation run.
    pub timestamp: DateTime<Utc>,
    /// PEV loop narrative summary.
    pub pev_loop_summary: PevLoopSummary,
    /// Optional natural-language analysis from the Rig (ARC) AI agent.
    pub agent_analysis: Option<String>,
}
