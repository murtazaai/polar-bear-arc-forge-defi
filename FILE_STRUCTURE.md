# Repository File Structure

```
polar-bear-arc-forge-defi/
│
│  ── Root tooling & meta ────────────────────────────────────────────
├── Cargo.toml             Rust 2024 edition; all dependencies; lints; release profile
├── Cargo.lock             Committed (binary crate); delete + regenerate on dep changes
├── rustfmt.toml           Code-style rules (100 cols, 2024 edition, crate-level imports)
├── .clippy.toml           Clippy config (MSRV 1.93.1, complexity thresholds)
├── .gitignore             Focused Rust-only ignore file; secrets never committed
├── .env.example           Template: ANTHROPIC_API_KEY, SOLANA_RPC_URL, DRY_RUN
├── LICENSE-PBS            Polar Bear Systems proprietary licence
├── README.md              Project overview, architecture, quick-start
├── CHANGELOG.md           Version history (Semantic Versioning)
├── CONTRIBUTING.md        Dev setup, workflow, code-style, CI description
├── BUG-FIXES.md           Root-cause analysis of 7 resolved issues
├── FILE_STRUCTURE.md      This file
│
│  ── GitHub Actions CI ──────────────────────────────────────────────
├── .github/
│   └── workflows/
│       └── ci.yml         fmt → clippy → build → test → docs → MSRV (1.93.1)
│
│  ── Zed IDE config ─────────────────────────────────────────────────
├── .zed/
│   ├── tasks.json         Cargo build / test / check / example tasks
│   └── debug.json         CodeLLDB debug launch config
│
│  ── Documentation ──────────────────────────────────────────────────
├── docs/
│   ├── architecture.md    System architecture diagram and layer descriptions
│   └── defi_math.md       Constant-product AMM math; sniper-bot prevention theory
│
│  ── Standalone runnable examples ───────────────────────────────────
│  (cargo run --example <name>; no API key needed unless noted)
├── examples/
│   ├── validator_demo.rs  Live USDC mainnet validation - all 6 checks displayed
│   ├── raydium_demo.rs    Raydium v3 SOL + USDC pool query with TVL / APY
│   └── agent_demo.rs      Rig AI agent analysis (--features ai-agent required)
│
│  ── Library source ─────────────────────────────────────────────────
├── src/
│   ├── lib.rs             Crate root; //! architecture diagram; public re-exports
│   ├── main.rs            Binary entry point; clap CLI (validate/raydium/launch/agent/check)
│   ├── types.rs           All shared types (MintInfo, ValidationReport, LaunchSimulation, …)
│   │
│   ├── rpc/               Solana JSON-RPC client
│   │   ├── mod.rs         Module declaration; re-exports SolanaRpcClient
│   │   └── solana.rs      getSlot / getAccountInfo / getBalance; manual 82-byte mint decoder
│   │
│   ├── validator/         Sniper-bot prevention checks
│   │   ├── mod.rs         Module declaration; check summary table; re-exports TokenValidator
│   │   └── token_validator.rs  6 checks: freeze authority, mint authority, decimals, …
│   │
│   ├── defi/              DeFi integrations
│   │   ├── mod.rs         Module declaration; re-exports RaydiumClient, DeepLiquidityProtocol
│   │   ├── raydium.rs     Raydium v3 REST API client; PoolHealthSummary
│   │   └── liquidity.rs   Constant-product AMM model; depth score; anti-rug ratings
│   │
│   ├── forge/             ARC Forge PEV loop orchestrator
│   │   ├── mod.rs         Module declaration; re-exports ArcForgeLauncher
│   │   └── arc_forge.rs   Perceive → Evaluate → Validate; LaunchSimulation output
│   │
│   └── agent/             Rig (ARC) AI agent (feature = ai-agent)
│       ├── mod.rs         Module declaration (feature-gated); re-exports ArcForgeAgent
│       └── arc_forge_agent.rs  rig-core 0.37; claude-sonnet-4-6; no Arc<Client>
│
│  ── Integration tests (no API key required) ─────────────────────────
├── tests/
│   ├── validator_tests.rs    13 tests - freeze/mint authority, decimals, risk score
│   ├── liquidity_tests.rs    11 tests - price, AMM model, anti-rug ratings, depth score
│   ├── forge_tests.rs        13 tests - PEV loop, JSON round-trip, readiness score
│   │
│   └── providers.rs          Live Anthropic tests - gated behind #[ignore]
│                             Run: cargo test --test providers --features ai-agent -- --ignored
```

---

## Key design decisions

| Decision | Rationale |
|---|---|
| Lib + bin targets | Integration tests are external crates; lib exposes `polar_bear_arc_forge_defi::` |
| Rust 2024 edition | Matches the rig upstream repository and MSRV 1.93.1 |
| Submodule structure | Each domain has its own directory with `mod.rs` re-exports; mirrors `polar-bear-hft-crypto` |
| No `solana-sdk` | Avoids version conflicts with `rig-core`; manual 82-byte mint decoder is transparent |
| `ai-agent` feature flag | Keeps `rig-core` out of the default build; mirrors rig-hft naming convention |
| `anthropic::Client` direct | No `Arc<Client>` wrapper; `CompletionClient` trait not implemented on `Arc<Client>` |
| `^` semver pins | Avoids resolver conflicts from over-constraining transitive deps |
| `dry_run = true` always | No real SOL is ever spent; enforced in `ArcForgeLauncher::build()` |
| `#[ignore]` on live tests | Prevents CI failures when `ANTHROPIC_API_KEY` is absent |
| `strip = "debuginfo"` | Reduces binary size; mirrors rig release profile |
