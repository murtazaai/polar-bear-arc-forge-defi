# Bug Fixes

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

This matches the canonical import in all official rig 0.36+ examples and
documentation, and in `polar-bear-hft-crypto/src/agent/hft_agent.rs`.

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
