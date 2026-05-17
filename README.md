# polar-bear-arc-forge-defi

**ARC Forge DeFi Platform** - Solana token launch with sniper-bot prevention
and deep initial liquidity, powered by [Rig (ARC)](https://rig.rs) AI agents.

[![Rust](https://img.shields.io/badge/Rust-1.93.1+-orange)](https://www.rust-lang.org)
[![Edition](https://img.shields.io/badge/Edition-2024-blue)](https://doc.rust-lang.org/edition-guide/)
[![rig-core](https://img.shields.io/badge/rig--core-0.37-purple)](https://rig.rs)
[![Solana](https://img.shields.io/badge/Solana-Devnet%2FMainnet-9945FF)](https://solana.com)
[![License: PBS](https://img.shields.io/badge/License-PBS%20Proprietary-red)](LICENSE-PBS)

> Built by **[Murtaza Ali Imtiaz](https://github.com/murtazaai)** · Technology Lead · Polar Bear Systems · July 2019–Present

---


> for token launches with built-in sniper-bot prevention and deep initial
> liquidity on Solana."*
>
> **All operations are DRY-RUN only.** No SOL is spent. No transactions are
> submitted to any network.  The token validator connects to the live Solana
> RPC and reads real on-chain data.  The Raydium client calls the real public
> Raydium v3 REST API.  The Rig (ARC) agent invokes the real Anthropic API
> when `ANTHROPIC_API_KEY` is set.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                     polar-bear-arc-forge-defi                           │
│              ARC Forge DeFi Platform · Rig (ARC) · Solana               │
└─────────────────────────────────────────────────────────────────────────┘
                                   │
          ┌────────────────────────▼────────────────────────┐
          │          CLI Entry Point  (main.rs)              │
          │  [validate | raydium | launch | agent | check]  │
          └──────┬──────────┬──────────┬──────────┬─────────┘
                 │          │          │          │
     ┌───────────▼──┐  ┌────▼────┐  ┌─▼────────┐ │
     │   rpc/        │  │ defi/   │  │ forge/   │ │
     │ SolanaRpcClient│  │ Raydium │  │ArcForge  │ │
     │ getAccountInfo│  │ + Deep  │  │Launcher  │ │
     │ manual decode │  │Liquidity│  │PEV loop  │ │
     └───────────┬──┘  └────┬────┘  └─┬────────┘ │
                 │          │          │          │
     ┌───────────▼──────────▼──────────▼──────────▼───────┐
     │   validator/ - 6 sniper-bot prevention checks        │
     │   freeze authority · mint authority · decimals · …   │
     └──────────────────────┬──────────────────────────────┘
                            │
              ┌─────────────▼──────────────────────────────┐
              │   LaunchSimulation  (JSON audit record)      │
              └─────────────────────┬──────────────────────┘
                                    │
              ┌─────────────────────▼───────────────────────┐
              │   agent/  (feature = ai-agent)               │
              │   ArcForgeAgent · rig-core 0.37              │
              │   claude-sonnet-4-6 · LAUNCH|REVIEW|BLOCK    │
              └──────────────────────────────────────────────┘
```

---

## Sniper-Bot Prevention - Validation Checks

| # | Check | Attack Vector | ARC Forge Policy |
|---|-------|---------------|-----------------|
| 1 | **Freeze Authority** | Deployer freezes all holders; only sniper can sell | Must be `None` |
| 2 | **Mint Authority** | Post-launch supply inflation dilutes all holders | Must be `None` |
| 3 | **Mint Initialized** | Uninitialised account is not a real token | Must be `true` |
| 4 | **Decimals Sanity** | Non-standard decimals used for price-display tricks | Must be 6–9 |
| 5 | **Zero Supply Guard** | `supply=0` + active mint authority = stealth-mint honey-pot | Supply must be > 0 |
| 6 | **Supply Upper Bound** | Astronomical supply creates decimal manipulation | ≤ 1 quadrillion |

---

## Deep Liquidity Protocol - Anti-Rug Ratings

| Rating | Condition | Description |
|--------|-----------|-------------|
| ⭐⭐⭐⭐⭐ DIAMOND | LP burned + ≥20 SOL | Permanent, deep - mathematically rug-proof |
| ⭐⭐⭐⭐ GOLD | LP burned, shallow | Permanent but easy to manipulate |
| ⭐⭐⭐ SILVER | 180+ day lock + deep | Good, but lock expires |
| ⭐⭐ BRONZE | 30+ day lock | Moderate risk post-lock |
| ⭐ RISKY | No burn, no lock | ARC Forge will not proceed |

See `docs/defi_math.md` for the constant-product AMM math behind these ratings.

---

## Stack

| Layer | Technology | Purpose |
|-------|------------|---------|
| AI Agent | [rig-core 0.37](https://rig.rs) | LLM orchestration, PEV loop AI layer |
| LLM | [claude-sonnet-4-6](https://anthropic.com) | Launch analysis and risk assessment |
| Async | [tokio 1.x](https://tokio.rs) | Async runtime |
| Blockchain | Solana (devnet / mainnet) | Token mint data via JSON-RPC |
| DeFi Data | [Raydium v3 API](https://api-v3.raydium.io) | Pool TVL, volume, APY |
| HTTP | reqwest 0.12 | Solana RPC + Raydium REST calls |
| CLI | clap 4.x | Command-line interface |
| Serialisation | serde + serde_json | JSON output for all reports |

---

## Prerequisites

```text
rustup install stable          # Rust ≥ 1.93.1
rustup component add rustfmt clippy
```

Optional for the AI agent:

```text
export ANTHROPIC_API_KEY="sk-ant-..."
```

## Quick Start

```text
git clone https://github.com/murtazaai/polar-bear-arc-forge-defi
cd polar-bear-arc-forge-defi
cp .env.example .env
cargo build --release
cargo test
```

---

## Usage

### Connectivity Check

```text
cargo run -- check
```

```
  Solana RPC (https://api.devnet.solana.com)  … ✅  slot = 312840567
  Raydium v3 API                              … ✅  5 pool(s) returned for SOL
```

### Validate a Live Token Mint

```text
# USDC on mainnet - all checks should pass
cargo run -- --rpc-url https://api.mainnet-beta.solana.com \
    validate --mint EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
```

```
  ✅  [SAFE     ]  Freeze Authority
  ✅  [SAFE     ]  Mint Authority
  ✅  [SAFE     ]  Mint Initialized
  ✅  [SAFE     ]  Decimals Sanity
  ✅  [SAFE     ]  Zero Supply Guard
  ✅  [SAFE     ]  Supply Upper Bound
  RECOMMENDATION: All checks passed. Safe to launch via ARC Forge.
```

### Query Raydium Pools

```text
cargo run -- raydium \
    --mint So11111111111111111111111111111111111111112 \
    --limit 3
```

### Full Launch Simulation (dry-run)

```text
cargo run -- launch \
    --name "Polar Bear Token" \
    --symbol PBT \
    --supply 1000000000000000 \
    --sol 20 \
    --lp-pct 10 \
    --burn-lp
```

JSON output:

```text
cargo run -- launch --symbol PBT --supply 1000000000000000 --sol 20 --burn-lp \
    --json | jq '{readiness: .launch_readiness_score, sniper: .sniper_bot_prevention_active}'
```

### Rig (ARC) Agent Analysis

Requires `ANTHROPIC_API_KEY`.

```text
cargo run --features ai-agent -- agent \
    --name "Polar Bear Token" --symbol PBT --sol 20 --burn-lp
```

The agent receives the full `LaunchSimulation` JSON as context and returns a
structured risk assessment with a `LAUNCH | REVIEW | BLOCK` recommendation via
`rig-core` and `claude-sonnet-4-6`.

---

## Tests

```text
cargo test                                        # all deterministic tests (no network)
cargo test --test validator_tests                 # sniper-bot prevention
cargo test --test liquidity_tests                 # AMM model + anti-rug ratings
cargo test --test forge_tests                     # PEV loop integration

# Live provider tests (requires ANTHROPIC_API_KEY)
ANTHROPIC_API_KEY=sk-ant-... \
    cargo test --test providers --features ai-agent -- --ignored --test-threads=1
```

---

## Test Inventory

| Test file | Tests | Network |
|-----------|-------|---------|
| `tests/validator_tests.rs` | 13 | None |
| `tests/liquidity_tests.rs` | 11 | None |
| `tests/forge_tests.rs` | 13 | None |
| `tests/providers.rs` | 2 (#[ignore]) | Anthropic API |

---

## North Star

**Taskforce**: *"Greenfield DeFi platforms including ARC Forge
for token launches with built-in sniper-bot prevention and deep initial liquidity
on Solana."*

### STAR

**Situation**: Polar Bear Systems' DeFi clients needed token launch infrastructure
protecting buyers from sniper bots, rug pulls, and unfair initial liquidity - the
three most common Solana launch exploits.

**Task**: Architect and implement the full ARC Forge pipeline: on-chain token
validation, deep initial liquidity strategy, and an AI agent layer for autonomous
launch governance - all in Rust on the Rig (ARC) framework.

**Action**:
- Built `TokenValidator` reading live SPL Token mint accounts via Solana JSON-RPC
  without a `solana-sdk` dependency; checks freeze authority (primary sniper vector),
  mint authority (inflation vector), and four additional risk signals
- Implemented `DeepLiquidityProtocol` using the constant-product AMM formula
  (`x · y = k`) to compute price-impact curves and anti-rug ratings; enforces LP
  burn or 180+ day lock before launch proceeds
- Integrated Raydium v3 REST API for real-time pool TVL, volume, and APY data
- Orchestrated the Perceive → Evaluate → Validate (PEV) loop in `ArcForgeLauncher`,
  producing a JSON-serialisable `LaunchSimulation` audit record for every decision
- Wrapped `claude-sonnet-4-6` via `rig-core 0.37` (fixing three compile-time
  bugs: `Arc<Client>` wrapper, missing `CompletionClient + ProviderClient` trait
  imports, wrong `rig::` vs `rig_core::` crate path)

**Result**: A transparent, open-source, live-data pipeline that: enforces
deterministic safety gates, produces a fully auditable JSON report per launch
decision, and extends to real SOL spend by removing the dry-run flag. 100% Rust,
tokio-native, zero Python.

---

## Related Repositories

| Repo | Details |
|------|-----------|
| [polar-bear-rig-hft](https://github.com/murtazaai/polar-bear-rig-hft) | rig-core HFT + PEV loop |
| [polar-bear-rig-onchain](https://github.com/murtazaai/polar-bear-rig-onchain) | rig-onchain-kit + SignerContext |
| [polar-bear-hft-crypto](https://github.com/murtazaai/polar-bear-hft-crypto) | ECDSA/Ed25519 + 7-exchange auth |
| **[polar-bear-arc-forge-defi](https://github.com/murtazaai/polar-bear-arc-forge-defi)** | **ARC Forge + sniper-bot prevention ← this repo** |

---

## References

- [Rig (ARC) Framework](https://rig.rs) - Rust Inference Gateway
- [0xPlaygrounds/rig](https://github.com/0xPlaygrounds/rig) - GitHub source
- [Raydium v3 API](https://api-v3.raydium.io) - DEX pool data
- [Solana JSON-RPC](https://docs.solana.com/api/http) - On-chain data
- [SPL Token Program](https://spl.solana.com/token) - Mint account layout
- [Uniswap v2 whitepaper](https://uniswap.org/whitepaper.pdf) - AMM formula
- [System Architecture](./docs/architecture.md) - System architecture overview
- [DeFi Math](./docs/defi_math.md) - ARC Forge liquidity mathematics

---

## License

PBS License: [LICENSE-PBS](./LICENSE-PBS)

---

## Author

**Murtaza Ali Imtiaz**

- LinkedIn: [LinkedIn](https://linkedin.com/in/murtazai)
- GitHub: [@murtazaai](https://github.com/murtazaai)
- Portfolio: [murtazai.com](https://murtazai.com)
