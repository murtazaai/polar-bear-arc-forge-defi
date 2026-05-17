# DeFi Mathematics - ARC Forge

Mathematical foundations for the `DeepLiquidityProtocol` and `TokenValidator`
implementations.

---

## 1. Constant-Product AMM (x · y = k)

Raydium and all Uniswap-style AMMs hold two token reserves `x` (token) and `y`
(quote, e.g. SOL) such that their product is a constant `k`:

```
x · y = k
```

### 1.1 Initial Pool State

Given:

| Symbol | Meaning |
|--------|---------|
| `S_sol` | SOL deposited into the LP pool |
| `P_sol` | SOL price in USD (constant: `165.0`) |
| `T_pool` | tokens deposited (= `total_supply × allocation_pct / 100`) |
| `T_total` | total token supply (adjusted for decimals) |

Initial state:

```
y₀ = S_sol × P_sol          (USD value of the SOL side)
x₀ = T_pool / 10^decimals   (token side, decimal-adjusted)
k  = x₀ · y₀
```

Initial price per token:

```
P₀ = y₀ / x₀  =  (S_sol × P_sol) / T_pool_adjusted
```

Estimated fully-diluted market cap at launch:

```
MCap = P₀ × T_total
```

### 1.2 Price Impact

For a buy of `Δy` USD into the pool, the constant-product formula gives the
fraction of the pool's value displaced:

```
price_impact (%) = Δy / (pool_value + Δy) × 100
```

where `pool_value = y₀ × 2` (both sides of the symmetric pool at initialisation).

This is an **upper bound** on the real-world impact; concentrated liquidity pools
(CLMM) achieve better capital efficiency within a price range.

**Worked example** (10 SOL pool, `P_sol = $165`):

```
pool_value_usd = 10 × 165 × 2 = $3 300

$1 000 buy impact = 1 000 / (3 300 + 1 000) × 100 = 23.3 %
$10 000 buy impact = 10 000 / (3 300 + 10 000) × 100 = 75.2 %
```

At 10 SOL the pool is easily manipulated.  A 50 SOL pool:

```
pool_value_usd = 50 × 165 × 2 = $16 500

$1 000 buy impact = 1 000 / 17 500 × 100 = 5.7 %
$10 000 buy impact = 10 000 / 26 500 × 100 = 37.7 %
```

This is the quantitative basis for the depth-score thresholds in
`DeepLiquidityProtocol::depth_score()`.

---

## 2. Liquidity Depth Score

The depth score `D ∈ [0, 100]` is a monotonically increasing step function of
`S_sol`:

| SOL range | Score | Rationale |
|-----------|-------|-----------|
| `< 1 SOL` | 15 | Trivially manipulable; $1K buy causes >30% impact |
| `1–5 SOL` | 40 | Small project; susceptible to moderate buys |
| `5–20 SOL` | 60 | Acceptable floor; $1K buy ≈ 6–15% impact |
| `20–100 SOL` | 80 | Solid depth; aligns with successful Raydium launches |
| `≥ 100 SOL` | 95 | Institutional depth; $10K buy < 6% impact |

Score 60+ combined with LP burn produces the **DIAMOND** anti-rug rating.

---

## 3. Anti-Rug Rating

The rating encodes both the depth score and LP token disposition:

```
if burn_lp AND depth ≥ 60  →  ⭐⭐⭐⭐⭐  DIAMOND
if burn_lp AND depth  < 60  →  ⭐⭐⭐⭐   GOLD
if lock ≥ 180d AND depth ≥ 60  →  ⭐⭐⭐  SILVER
if lock ≥ 30d              →  ⭐⭐    BRONZE
else                        →  ⭐     RISKY
```

Burning LP tokens is strictly stronger than locking them: a burned LP position
is provably irremovable from the AMM pool forever.  A locked position can be
unlocked when the timelock expires.

---

## 4. Launch Readiness Score

The readiness score `R ∈ [0, 100]` aggregates risk:

```
R = 100
  − (risk_score × 0.7)       # validation: max deduction 70
  − depth_deduction           # liquidity depth: 0 | 5 | 15 | 25
  − lp_deduction              # 20 if no burn and lock < 30 days
  clamped to [0, 100]
```

| `R` | Interpretation |
|-----|----------------|
| ≥ 80 | `VALIDATED` - clear for launch |
| 60–79 | Marginal - address liquidity or validation issues |
| < 60 | `BLOCKED` - critical issues must be resolved |

---

## 5. Sniper-Bot Prevention Theory

### 5.1 Freeze Authority Attack

```
1. Deployer deploys token with freeze_authority = <deployer_key>
2. Sniper bot detects new liquidity add (mempool monitoring)
3. Sniper executes a large buy at the open
4. Deployer calls freeze_account(sniper_account = False)
   - but also freezes all other buyer accounts
5. Only the sniper can sell; deployer calls sell
6. Sniper and deployer share the exit liquidity
7. All other holders are frozen - unable to sell
```

**Mitigation**: ARC Forge requires `freeze_authority = None` and verifies this
from the live on-chain mint account before any launch proceeds.

### 5.2 Mint Authority Inflation Attack

```
1. Deployer retains mint_authority after launch
2. After holders buy in, deployer mints 10× the original supply to self
3. Each existing holder's share is diluted by 10×
4. Deployer dumps the freshly minted tokens into the pool
5. Price collapses; original holders cannot exit at a profit
```

**Mitigation**: ARC Forge requires `mint_authority = None` (renounced).

### 5.3 Stealth Mint (Zero Supply + Active Mint Authority)

```
1. Token deployed with supply = 0 and mint_authority = <deployer>
2. Mint is listed on aggregators (it looks like a real token address)
3. Deployer mints a large private allocation after LP is set up
4. Private allocation is dumped into the pool immediately after public launch
```

**Mitigation**: Check 5 in `TokenValidator` flags `supply == 0` with active
`mint_authority` as `Dangerous`.

### 5.4 Decimal Trick Attack

```
1. Token deployed with decimals = 0 (or > 18)
2. On DEX UI: 1 token appears to cost $0.000001
3. Users see a "cheap" token and buy in bulk
4. After accumulation, decimals are exploited to show $1 per unit
5. Deployer sells at the apparent $1/token price
```

**Mitigation**: Check 4 flags `decimals < 6` or `> 18` as `Dangerous`.

---

## 6. Constant-Product vs. CLMM

| Pool type | Formula | Capital efficiency |
|-----------|---------|-------------------|
| Full-range AMM | `x · y = k` | Low - price distributes liquidity across `[0, ∞)` |
| CLMM (Raydium v3) | `x · y = k` over `[P_lower, P_upper]` | High - all liquidity within the range |

ARC Forge models the **full-range AMM** as a conservative lower bound.
Real Raydium v3 concentrated positions will produce lower price impact within
their configured range, meaning the ARC Forge impact estimates are
*worst-case conservative* - a safer default for launch planning.

---

## References

- Uniswap v2 whitepaper: <https://uniswap.org/whitepaper.pdf>
- Raydium CLMM documentation: <https://docs.raydium.io/raydium/liquidity-providers/providing-concentrated-liquidity-clmm>
- SPL Token program source: <https://github.com/solana-labs/solana-program-library/blob/master/token/program/src/state.rs>
- Solana JSON-RPC API: <https://docs.solana.com/api/http>
