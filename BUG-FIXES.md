# Bug Fixes

---

## Fix 8 - Invalid SPDX license `LicensePBS` blocks `cargo publish`

**File**: `Cargo.toml`, `LICENSE-PBS` → `LICENSE-MIT` + `LICENSE-APACHE`

**Root Cause**: `v0.1.0` / `v0.2.0` used a proprietary `LICENSE-PBS` file and
set `license = "LicensePBS"` in `Cargo.toml`.  `cargo publish` requires a valid
[SPDX 2.x](https://spdx.org/licenses/) expression.  `"LicensePBS"` is not in the
SPDX license list and causes an immediate publish rejection:

```
error: invalid license `LicensePBS`
```

Additionally, the `AND/OR` operator is not a valid SPDX combinator — only `AND`,
`OR`, and `WITH` are defined.

**Fix**: Replace `LICENSE-PBS` with `LICENSE-MIT` and `LICENSE-APACHE`.  Set:

```toml
# Before (broken)
license = "LicensePBS"

# After (correct)
license = "MIT OR Apache-2.0"
```

Both physical license files must be present in the repository root; crates.io
validates their existence during publish.

---

## Fix 9 - Wrong MSRV `1.93.1` — Rust 2024 minimum is `1.85.0`

**File**: `Cargo.toml`, `.clippy.toml`

**Root Cause**: `v0.2.0` set `rust-version = "1.93.1"` and `msrv = "1.93.1"` in
`.clippy.toml`.  Rust 2024 edition requires a minimum of **1.85.0** (stabilised
February 2025).  Advertising `1.93.1` overstates the MSRV, may block users on
older stable toolchains, and deviates from the process document standard
(`rust-version = "1.85.0"`).

**Fix**:

```toml
# Cargo.toml - Before (overstated)
rust-version = "1.93.1"

# Cargo.toml - After (correct Rust 2024 minimum)
rust-version = "1.85.0"
```

```toml
# .clippy.toml - Before
msrv = "1.93.1"

# .clippy.toml - After
msrv = "1.85.0"
```

The CI MSRV job was updated to pin `dtolnay/rust-toolchain@1.85.0` accordingly.

---

## Fix 10 - Missing `[[example]]` declarations for `raydium_demo` and `validator_demo`

**File**: `Cargo.toml`

**Root Cause**: `v0.2.0` added `examples/raydium_demo.rs` and
`examples/validator_demo.rs` to the repository but only declared `agent_demo` in
`Cargo.toml` under `[[example]]`.  Without explicit declarations Cargo auto-discovers
examples, but they are invisible to docs.rs, `cargo publish` metadata, and the
Zed task palette's `--example` args.  Also, the `exclude` field was absent, meaning
development files (`.env`, `.gitignore`) were included in the crates.io tarball.

**Fix**: Add both missing `[[example]]` entries and an `exclude` list:

```toml
# Before (only agent_demo declared)
[[example]]
name              = "agent_demo"
required-features = ["ai-agent"]

# After (all three declared)
[[example]]
name              = "agent_demo"
required-features = ["ai-agent"]

[[example]]
name = "raydium_demo"

[[example]]
name = "validator_demo"

# Added
exclude = [".env", ".env.example", ".gitignore", "keys/"]
```

---

## Fix 11 - Copy-paste project header in `.clippy.toml` and `rustfmt.toml`

**Files**: `.clippy.toml`, `rustfmt.toml`

**Root Cause**: Both files were copied from `polar-bear-hft-crypto` and retained
that project's name in the comment header.  While functionally harmless, it creates
confusion during audits and violates the standard that every config file correctly
identifies its owning project.

**Fix**: Updated both headers to read `polar-bear-arc-forge-defi`.

---

## Fix 12 - Missing `.github/workflows/ci.yml` and `.zed/settings.json`

**Files**: `.github/workflows/ci.yml`, `.zed/settings.json`

**Root Cause**: The `CHANGELOG.md` for `v0.2.0` lists `ci.yml` as added, but no
`ci.yml` was present in the archive.  `.zed/settings.json` was referenced in the
`FILE_STRUCTURE.md` and the process document but also absent.

**Fix**:
- Added `.github/workflows/ci.yml` with the canonical 6-job pipeline:
  `fmt → clippy → build → test → doc → msrv`.  The MSRV job pins
  `dtolnay/rust-toolchain@1.85.0`.  `SKIP_LLM` is not needed here (no
  `SKIP_LLM` pattern in this codebase); live provider tests use `#[ignore]`
  and are skipped automatically.
- Added `.zed/settings.json` with rust-analyzer wired to clippy, separate
  `target/rust-analyzer` dir, full inlay hints, proc macros, and
  `allFeatures = true` so `ai-agent`-gated symbols resolve in the IDE.

---

## Fix 1 - `Arc<anthropic::Client>` in `ArcForgeAgent`

**File**: `src/agent/arc_forge_agent.rs`

**Root Cause**: The original `v0.1.0` code wrapped `anthropic::Client::from_env()`
in `Arc::new(...)`, producing `Arc<anthropic::Client>`.  The `.agent()` method is
defined on the `CompletionClient` trait, which is implemented by `anthropic::Client`
directly - not by `Arc<anthropic::Client>`.  Rust's method resolution cannot find
`.agent()` on the `Arc` wrapper, yielding E0599.

Additionally, `Arc` was unnecessary: the `client` field is consumed by
`.agent().preamble().build()` in the same method call and never shared across tasks.

**Fix**: Remove `Arc::new(...)`, remove `use std::sync::Arc`, and store
`client: anthropic::Client` directly.

```rust
// Before (broken)
use std::sync::Arc;
pub struct ArcForgeAgent { client: Arc<anthropic::Client> }
let client = Arc::new(anthropic::Client::from_env());

// After (correct)
pub struct ArcForgeAgent { client: anthropic::Client }
let client = anthropic::Client::from_env()?;
```

---

## Fix 2 - Missing `CompletionClient` + `ProviderClient` trait imports

**File**: `src/agent/arc_forge_agent.rs`

**Root Cause**: In rig-core ≥ 0.36, `.agent()` is a method on the `CompletionClient`
trait, not an inherent method on `anthropic::Client`.  Without bringing both
`CompletionClient` and `ProviderClient` into scope via `use`, the compiler cannot
resolve the method call even though `Client<AnthropicExt>` implements both traits.

**Fix**: Add both traits to the `use rig_core::{…}` import:

```rust
use rig_core::{
    client::{CompletionClient, ProviderClient},
    completion::Prompt,
    providers::anthropic,
};
```

---

## Fix 3 - Wrong crate path: `rig::` instead of `rig_core::`

**File**: `src/agent/arc_forge_agent.rs`

**Root Cause**: The `v0.1.0` agent used `use rig::providers::anthropic` and
`use rig::completion::Prompt`.  The Cargo dependency is named `rig-core` (hyphen),
which Rust resolves to the crate root `rig_core` (underscore).  There is no
re-export crate named `rig` in this project.

**Fix**: Replace all `rig::` paths with `rig_core::`.

```rust
// Before (broken)
use rig::providers::anthropic;
use rig::completion::Prompt;

// After (correct)
use rig_core::{
    client::{CompletionClient, ProviderClient},
    completion::Prompt,
    providers::anthropic,
};
```

---

## Fix 4 - Exact `=` version pins in `Cargo.toml`

**File**: `Cargo.toml`

**Root Cause**: `v0.1.0` used exact pins (`"0.37"`, `"1"`, `"4"`) without the
semver-compatible `^` prefix.  Cargo treats bare version strings as `^`-compatible
by default, but the intent was ambiguous and inconsistent with the `rig-hft-crypto`
standard.

**Fix**: Add explicit `^` to all dependency versions (e.g. `"^0.37"`, `"^1"`,
`"^4"`).  This makes semver compatibility explicit and mirrors the rig upstream
and `polar-bear-hft-crypto` conventions.

---

## Fix 5 - `dotenv` → `dotenvy`

**File**: `Cargo.toml`, `src/agent/arc_forge_agent.rs`

**Root Cause**: `v0.1.0` depended on the unmaintained `dotenv` crate.
`dotenvy` is the actively-maintained fork used by the rig upstream and
`polar-bear-hft-crypto`.

**Fix**: Replace `dotenv = "^0.15"` with `dotenvy = "^0.15"` and update all
call sites from `dotenv::dotenv()` to `dotenvy::dotenv()`.

---

## Fix 6 - `thiserror` pinned to `1.0` instead of `^2`

**File**: `Cargo.toml`

**Root Cause**: `thiserror` 2.x includes ergonomic improvements and is the
version used by the rig upstream.  Pinning to `1.0` was unnecessarily
restrictive.

**Fix**: `thiserror = "^2"`.

---

## Fix 7 - Flat `src/*.rs` - no submodule organisation

**Files**: all source files

**Root Cause**: `v0.1.0` placed all modules at the crate root as flat `.rs`
files, making it difficult to extend individual components and inconsistent with
the `polar-bear-hft-crypto` reference structure.

**Fix**: Reorganised into focused submodules with their own `mod.rs`:

```
src/rpc/           SolanaRpcClient
src/validator/     TokenValidator (6 sniper-bot checks)
src/defi/          RaydiumClient + DeepLiquidityProtocol
src/forge/         ArcForgeLauncher (PEV loop)
src/agent/         ArcForgeAgent (rig-core; feature-gated)
src/types.rs       All shared types (unchanged location)
```

Each `mod.rs` re-exports its public surface, keeping import paths short.
