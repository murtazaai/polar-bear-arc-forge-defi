# Contributing to polar-bear-arc-forge-defi

> **Polar Bear Systems** · Technology Lead: Murtaza Ali Imtiaz
> This repository is published under a restricted proprietary licence for
> portfolio and reference purposes.  See [LICENSE-PBS](./LICENSE-PBS) for permitted use.

---

## Development environment

### Prerequisites

| Tool | Version | Install |
|---|---|---|
| Rust stable toolchain | ≥ 1.93.1 (MSRV) | `rustup update stable` |
| `rustfmt` | (with toolchain) | `rustup component add rustfmt` |
| `clippy` | (with toolchain) | `rustup component add clippy` |

### Setup

```text
git clone https://github.com/murtazaai/polar-bear-arc-forge-defi
cd polar-bear-arc-forge-defi
cp .env.example .env
# Edit .env: set ANTHROPIC_API_KEY=sk-ant-... (only needed for ai-agent feature)
# Set SOLANA_RPC_URL if you want to use mainnet or a custom RPC
```

---

## Workflow

### Build

```text
cargo build                          # debug
cargo build --release                # optimised (use for benchmarks)
cargo build --features ai-agent      # include Rig AI agent module
```

### Run

```text
# Connectivity check (Solana RPC + Raydium API)
cargo run -- check

# Validate a live token mint (mainnet USDC)
cargo run -- --rpc-url https://api.mainnet-beta.solana.com \
    validate --mint EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v

# Full launch simulation (dry-run, devnet)
cargo run -- launch --symbol PBT --supply 1000000000000000 --sol 20 --burn-lp

# Help
cargo run -- --help
```

### Examples

```text
cargo run --example validator_demo              # live mainnet token validation
cargo run --example raydium_demo                # Raydium pool queries
cargo run --example agent_demo --features ai-agent   # requires ANTHROPIC_API_KEY
```

### Tests (no API key required)

```text
cargo test                                        # all deterministic tests
cargo test --test validator_tests                 # sniper-bot prevention checks
cargo test --test liquidity_tests                 # AMM model + anti-rug ratings
cargo test --test forge_tests                     # PEV loop integration
```

### Live provider tests (API key required)

```text
ANTHROPIC_API_KEY=sk-ant-... \
    cargo test --test providers --features ai-agent -- --ignored --test-threads=1
```

Use `--test-threads=1` to avoid concurrent API calls hitting rate limits.

### Format, lint, docs

```text
cargo fmt --all                                   # format
cargo fmt --all -- --check                        # CI format check
cargo clippy --all-targets -- -D warnings         # lint (CI mode)
cargo clippy --all-targets --features ai-agent -- -D warnings
cargo doc --open                                  # browse API docs locally
RUSTDOCFLAGS="--cfg docsrs -D warnings" cargo doc # CI docs check
```

---

## Code style

- **Edition**: Rust 2024
- **Max line width**: 100 characters (enforced by `rustfmt.toml`)
- **Imports**: `use rig_core::client::{CompletionClient, ProviderClient}` - both traits are
  required to call `.agent()` on any rig-core ≥ 0.36 Anthropic client
- **Doc comments**: `//!` for module-level docs; `///` for public items
- **Error handling**: always `anyhow::Result`; propagate with `?`; no `unwrap` in library code
- **Version pins**: use `^` (semver-compatible) for all dependencies; never `=` exact pins
- **Agent storage**: store `anthropic::Client` directly - never wrap in `Arc`

---

## Adding a new validation check

1. Add a `check_<name>()` function in `src/validator/token_validator.rs` following
   the existing pattern (returns `ValidationCheck`, documents the attack vector)
2. Add the new check to `run_all_checks()` in the same file
3. Add corresponding tests in `tests/validator_tests.rs`
4. Update the check table in `src/validator/mod.rs` and `docs/architecture.md`

---

## CI

The CI pipeline (`.github/workflows/ci.yml`) runs on every push and pull request:

1. `rustfmt --check` - enforces code style
2. `clippy -D warnings` - enforces lint rules (default + ai-agent features)
3. `cargo build --release` - ensures the release binary compiles
4. `cargo test --workspace` - runs all deterministic tests
5. `cargo doc` - ensures documentation compiles without warnings
6. MSRV check against Rust 1.93.1
