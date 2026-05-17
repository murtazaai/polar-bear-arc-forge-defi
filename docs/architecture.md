# polar-bear-arc-forge-defi

## System Architecture

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
    ┌────────────▼──┐  ┌────▼────┐  ┌─▼────────┐ │
    │   rpc/        │  │ defi/   │  │ forge/   │ │
    │               │  │         │  │           │ │
    │ SolanaRpcClient│  │ Raydium │  │ ArcForge │ │
    │               │  │ Client  │  │ Launcher │ │
    │ getAccountInfo│  │         │  │           │ │
    │ getSlot       │  │ Deep    │  │ PERCEIVE  │ │
    │ getBalance    │  │ Liquidity│  │ EVALUATE │ │
    │               │  │ Protocol│  │ VALIDATE  │ │
    └────────────┬──┘  └────┬────┘  └─┬────────┘ │
                 │          │          │          │
    ┌────────────▼──────────▼──────────▼──────────▼──────┐
    │   validator/                                         │
    │                                                      │
    │   TokenValidator                                     │
    │   ┌──────────────────────────────────────────────┐  │
    │   │ 1. Freeze Authority  ← primary sniper vector │  │
    │   │ 2. Mint Authority    ← inflation vector       │  │
    │   │ 3. Mint Initialized  ← honey-pot guard        │  │
    │   │ 4. Decimals Sanity   ← price-display trick    │  │
    │   │ 5. Zero Supply Guard ← stealth-mint guard     │  │
    │   │ 6. Supply Upper Bound ← decimal trick guard   │  │
    │   └──────────────────────────────────────────────┘  │
    └──────────────────────────────────────────────────────┘
                               │
              ┌────────────────▼────────────────────────────┐
              │   LaunchSimulation (JSON audit record)        │
              │                                              │
              │   ValidationReport · LiquidityMetrics        │
              │   PevLoopSummary · agent_analysis            │
              │   dry_run = true (always)                    │
              └─────────────────────┬───────────────────────┘
                                    │
              ┌─────────────────────▼───────────────────────┐
              │   agent/  (feature = ai-agent)               │
              │                                              │
              │   ArcForgeAgent                              │
              │   rig-core 0.37                              │
              │   CompletionClient + ProviderClient          │
              │   claude-sonnet-4-6                          │
              │   preamble: DeFi security analyst            │
              │   output: LAUNCH | REVIEW | BLOCK            │
              └──────────────────────────────────────────────┘

External integrations (live, real APIs - read-only):
  ┌──────────────────────────┐   ┌──────────────────────────────┐
  │  Solana JSON-RPC         │   │  Raydium v3 REST API         │
  │  api.devnet/mainnet      │   │  api-v3.raydium.io           │
  │  getAccountInfo →        │   │  /pools/info/mint →          │
  │  82-byte mint decode     │   │  TVL, volume, APY, price     │
  └──────────────────────────┘   └──────────────────────────────┘
```

---

## Layer descriptions

### `rpc/` - Solana JSON-RPC client

Implements the three RPC methods needed by the pipeline without a `solana-sdk`
dependency.  The SPL Token mint account (82 bytes) is decoded manually using
published byte-offset constants, making the decoding logic transparent and
independently verifiable.

| Method | Purpose |
|--------|---------|
| `get_slot()` | Connectivity check |
| `get_mint_info(address)` | Fetch and decode SPL Token mint |
| `get_balance(address)` | SOL balance in lamports |

### `validator/` - Sniper-bot prevention

Six sequential checks are run against every token mint before ARC Forge proceeds.
Each check maps to a documented DeFi attack vector.  The checks are deterministic:
given the same `MintInfo`, the output is always identical.

### `defi/` - DeFi integrations

`RaydiumClient` queries the public Raydium v3 REST API for pool TVL, 24-hour
volume, and APY.  `DeepLiquidityProtocol` models the initial pool state using
the constant-product AMM formula and computes price impact curves and anti-rug ratings.

### `forge/` - PEV loop orchestrator

`ArcForgeLauncher` wires together the three stages of the Perceive → Evaluate → Validate
loop and emits a fully serialisable `LaunchSimulation` audit record.  The `dry_run`
field is always `true` - no SOL is ever spent.

### `agent/` - Rig (ARC) AI layer (optional)

`ArcForgeAgent` wraps a `rig-core 0.37` Anthropic client configured with a DeFi
security analyst preamble.  The full `LaunchSimulation` JSON is passed as context
and Claude returns a ≤ 300-word risk assessment with a `LAUNCH | REVIEW | BLOCK`
recommendation.

Both `CompletionClient` and `ProviderClient` must be in scope for `.agent()` to
resolve - see `BUG-FIXES.md` Fix 2.  The client is stored directly (not in `Arc`)
— see `BUG-FIXES.md` Fix 1.
