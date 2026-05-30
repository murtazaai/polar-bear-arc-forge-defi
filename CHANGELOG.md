# Changelog

All notable changes to `polar-bear-arc-forge-defi` are documented here.

This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

---

## [0.2.1] - 2026-05-30

### Fixed (crates.io publication readiness — Stage 08)

- **License** — replaced proprietary `LICENSE-PBS` with `MIT OR Apache-2.0`
  dual-license (`LICENSE-MIT` + `LICENSE-APACHE`); `cargo publish` requires a
  valid SPDX expression and the actual license files to be present
- **`Cargo.toml` `license` field** — changed from `"LicensePBS"` (invalid SPDX)
  to `"MIT OR Apache-2.0"` (valid SPDX per `cargo publish` requirements)
- **`Cargo.toml` `rust-version`** — corrected from `"1.93.1"` to `"1.85.0"`;
  1.85.0 is the minimum required for Rust 2024 edition per the process document
  (`rust-version = "1.85.0"`)
- **`Cargo.toml` missing `[[example]]` entries** — added `raydium_demo` and
  `validator_demo` entries (only `agent_demo` was declared; the other two
  examples existed in `examples/` but were undeclared)
- **`Cargo.toml` `exclude`** — added `.env`, `.env.example`, `.gitignore`,
  `keys/` to the `exclude` list to keep the crates.io tarball clean
- **`.clippy.toml`** — corrected project header from `polar-bear-hft-crypto`
  to `polar-bear-arc-forge-defi`; updated `msrv` to `"1.85.0"` to match
  the corrected `Cargo.toml` `rust-version`
- **`rustfmt.toml`** — corrected project header from `polar-bear-hft-crypto`
  to `polar-bear-arc-forge-defi`

### Added

- **`.github/workflows/ci.yml`** — full CI pipeline: `fmt → clippy → build →
  test → doc → msrv`; MSRV job pins Rust 1.85.0; live provider tests
  gated behind `#[ignore]` are never run in CI; doc step uses
  `RUSTDOCFLAGS="--cfg docsrs -D warnings"` mirroring `[package.metadata.docs.rs]`
- **`.zed/settings.json`** — rust-analyzer project-level config: clippy as
  check command, separate `target/rust-analyzer` dir, full inlay hints,
  proc macros enabled, `allFeatures = true` so `ai-agent`-gated items resolve

---

## [0.2.0] - 2026-05-17

### Added
- `rustfmt.toml` - code-style rules (100 cols, Rust 2024 edition, crate-level imports)
- `.clippy.toml` - Clippy config with MSRV 1.93.1 and complexity thresholds
- `LICENSE-PBS` - Polar Bear Systems proprietary licence
- `CHANGELOG.md` - this file
- `CONTRIBUTING.md` - contribution guide with full workflow
- `FILE_STRUCTURE.md` - annotated repository map
- `BUG-FIXES.md` - root-cause analysis of all resolved issues (7 fixes)
- `docs/architecture.md` - system architecture deep-dive with ASCII diagram
- `docs/defi_math.md` - liquidity mathematics and sniper-bot prevention theory
- `examples/validator_demo.rs` - standalone token validation demo (mainnet USDC)
- `examples/raydium_demo.rs` - Raydium v3 pool query demo
- `examples/agent_demo.rs` - Rig AI agent demo (requires `ai-agent` feature)
- `tests/validator_tests.rs` - 13 deterministic validator tests
- `tests/liquidity_tests.rs` - 11 deterministic liquidity tests
- `tests/forge_tests.rs` - 13 deterministic PEV loop integration tests
- `tests/providers.rs` - live Anthropic tests (all `#[ignore]`)
- `.github/workflows/ci.yml` - fmt → clippy → build → test → docs → MSRV
- `.zed/tasks.json` / `debug.json` - Zed IDE task and debug config
- Submodule structure: `src/rpc/`, `src/validator/`, `src/defi/`, `src/forge/`, `src/agent/`

### Changed
- `Cargo.toml` - upgraded to **Rust 2024 edition**; added `rust-version = "1.93.1"` (MSRV),
  `[package.metadata.docs.rs]`, and `[lints]` tables; feature renamed `agent` → `ai-agent`;
  all version pins made explicit with `^`; bumped `thiserror` to `^2`; replaced `dotenv` with
  `dotenvy ^0.15`; added `[profile.release]` with LTO + single-codegen-unit; added
  `[[example]]` entry with `required-features = ["ai-agent"]` for `agent_demo`
- `src/agent/` - fixed three critical compilation issues (see `BUG-FIXES.md` Fix 1–3):
  removed `Arc<Client>` wrapper; added `CompletionClient + ProviderClient` imports;
  corrected crate path from `rig::` to `rig_core::`
- `.gitignore` - consolidated with focused Rust-only ignore rules matching `polar-bear-hft-crypto`
- `.env.example` - simplified to project-relevant variables only

---

## [0.1.0] - 2026-05-16

Initial release:

- Solana JSON-RPC client (no `solana-sdk` dependency; manual 82-byte mint decoder)
- `TokenValidator` with 6 sniper-bot / rug-pull prevention checks
- `RaydiumClient` (Raydium v3 REST API)
- `DeepLiquidityProtocol` (constant-product AMM model, depth score, anti-rug ratings)
- `ArcForgeLauncher` (PEV loop orchestrator)
- `ArcForgeAgent` (rig-core integration, behind `agent` feature flag)
- `polar-bear-arc-forge` CLI: validate, raydium, launch, agent, check
- JSON-serialisable `LaunchSimulation` audit record
- GitHub Actions CI: fmt → clippy → build → test
