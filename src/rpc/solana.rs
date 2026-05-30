//! # Solana JSON-RPC client
//!
//! Implements the subset of the [Solana JSON-RPC 2.0 API](https://docs.solana.com/api/http)
//! required by the ARC Forge pipeline: `getSlot`, `getAccountInfo`, and
//! `getBalance`.
//!
//! ## Design
//!
//! - No `solana-sdk` dependency - avoids version conflicts with `rig-core`.
//! - All RPC calls are logged at `DEBUG` level for full transparency.
//! - The SPL Token mint account (82 bytes) is decoded from base64 manually using published
//!   byte-offset constants from the SPL source.
//!
//! ## SPL Token Mint layout (82 bytes)
//!
//! ```text
//! [0..4]   mint_authority option tag  (0 = None, 1 = Some)
//! [4..36]  mint_authority pubkey      (32 bytes)
//! [36..44] supply                     (u64 little-endian)
//! [44]     decimals                   (u8)
//! [45]     is_initialized             (bool)
//! [46..50] freeze_authority option tag
//! [50..82] freeze_authority pubkey    (32 bytes)
//! ```
//!
//! Reference:
//! <https://github.com/solana-labs/solana-program-library/blob/master/token/program/src/state.rs>

use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result, anyhow};
use base64::{Engine, engine::general_purpose::STANDARD as B64};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::{debug, warn};

use crate::types::MintInfo;

// ── Mint layout offsets ───────────────────────────────────────────────────────

const MINT_LEN: usize = 82;
const MINT_AUTH_TAG_OFF: usize = 0;
const SUPPLY_OFF: usize = 36;
const DECIMALS_OFF: usize = 44;
const INITIALIZED_OFF: usize = 45;
const FREEZE_AUTH_TAG_OFF: usize = 46;
const FREEZE_AUTH_KEY_OFF: usize = 50;

// ── Well-known program IDs ────────────────────────────────────────────────────

const SPL_TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const SPL_TOKEN_2022: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";

// ── JSON-RPC wire types ───────────────────────────────────────────────────────

#[derive(Serialize)]
struct RpcRequest<'a> {
    jsonrpc: &'a str,
    id: u64,
    method: &'a str,
    params: Value,
}

#[derive(Deserialize)]
struct RpcResponse<T> {
    #[allow(dead_code)]
    jsonrpc: String,
    #[allow(dead_code)]
    id: u64,
    result: Option<T>,
    error: Option<RpcError>,
}

#[derive(Deserialize)]
struct RpcError {
    code: i64,
    message: String,
}

#[derive(Deserialize)]
struct AccountInfoResult {
    value: Option<AccountValue>,
}

#[derive(Deserialize)]
struct AccountValue {
    data: Vec<String>,
    #[serde(rename = "lamports")]
    _lamports: u64,
    owner: String,
    #[serde(rename = "executable")]
    _executable: bool,
}

#[derive(Deserialize)]
struct BalanceResult {
    value: u64,
}

// ── SolanaRpcClient ───────────────────────────────────────────────────────────

/// Lightweight Solana JSON-RPC client.
///
/// Wraps `reqwest::Client` and exposes the three methods needed by the
/// ARC Forge pipeline: [`get_slot`], [`get_mint_info`], and [`get_balance`].
pub struct SolanaRpcClient {
    http: Client,
    rpc_url: String,
    next_id: AtomicU64,
}

impl SolanaRpcClient {
    /// Create a new client pointed at `rpc_url`.
    ///
    /// # Panics
    ///
    /// Panics if the underlying `reqwest` client cannot be constructed.  In
    /// practice this is infallible for the configuration used here (no TLS
    /// customisation, no invalid header values), so the panic should never
    /// trigger at runtime.
    pub fn new(rpc_url: impl Into<String>) -> Self {
        Self {
            http: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("reqwest Client construction is infallible"),
            rpc_url: rpc_url.into(),
            next_id: AtomicU64::new(1),
        }
    }

    // ── Public API ────────────────────────────────────────────────────────────

    /// Return the current confirmed slot - useful as a connectivity check.
    pub async fn get_slot(&self) -> Result<u64> {
        self.call::<u64>("getSlot", json!([])).await
    }

    /// Fetch and decode the SPL Token mint account for `mint_address`.
    ///
    /// Returns `Err` if the account does not exist, is not owned by the
    /// SPL Token program, or the account data cannot be decoded.
    pub async fn get_mint_info(&self, mint_address: &str) -> Result<MintInfo> {
        let params = json!([
            mint_address,
            { "encoding": "base64", "commitment": "confirmed" }
        ]);

        let result: AccountInfoResult = self
            .call("getAccountInfo", params)
            .await
            .with_context(|| format!("getAccountInfo failed for {mint_address}"))?;

        let account = result
            .value
            .ok_or_else(|| anyhow!("Account {mint_address} not found on-chain"))?;

        if account.owner != SPL_TOKEN_PROGRAM && account.owner != SPL_TOKEN_2022 {
            warn!(owner = %account.owner, "Account is not owned by an SPL Token program");
            return Err(anyhow!(
                "Account {mint_address} is not an SPL Token mint (owner: {})",
                account.owner
            ));
        }

        let raw_b64 = account
            .data
            .first()
            .ok_or_else(|| anyhow!("Account data array is empty"))?;

        let raw = B64
            .decode(raw_b64)
            .context("Failed to base64-decode mint account data")?;

        decode_mint(&raw, mint_address)
    }

    /// Return the confirmed SOL balance of `address` in lamports.
    pub async fn get_balance(&self, address: &str) -> Result<u64> {
        let params = json!([address, { "commitment": "confirmed" }]);
        let result: BalanceResult = self.call("getBalance", params).await?;
        Ok(result.value)
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    async fn call<T: for<'de> Deserialize<'de>>(&self, method: &str, params: Value) -> Result<T> {
        let req = RpcRequest {
            jsonrpc: "2.0",
            id: self.next_id.fetch_add(1, Ordering::Relaxed),
            method,
            params,
        };

        debug!(method, rpc_url = %self.rpc_url, "Solana RPC →");

        let resp = self
            .http
            .post(&self.rpc_url)
            .json(&req)
            .send()
            .await
            .with_context(|| format!("HTTP POST to {} failed", self.rpc_url))?;

        let body: RpcResponse<T> = resp
            .json()
            .await
            .context("Failed to deserialise Solana RPC response")?;

        if let Some(err) = body.error {
            return Err(anyhow!("RPC error {}: {}", err.code, err.message));
        }

        body.result
            .ok_or_else(|| anyhow!("RPC returned null result for method={method}"))
    }
}

// ── Mint decoding ─────────────────────────────────────────────────────────────

/// Decode a raw ≥ 82-byte SPL Token mint account into a [`MintInfo`].
fn decode_mint(data: &[u8], address: &str) -> Result<MintInfo> {
    if data.len() < MINT_LEN {
        return Err(anyhow!(
            "Mint data too short: {} bytes (expected ≥ {})",
            data.len(),
            MINT_LEN
        ));
    }

    let mint_authority = read_coption_pubkey(data, MINT_AUTH_TAG_OFF, 4);

    let supply = u64::from_le_bytes(
        data[SUPPLY_OFF..SUPPLY_OFF + 8]
            .try_into()
            .context("Failed to read supply bytes")?,
    );

    let decimals = data[DECIMALS_OFF];
    let is_initialized = data[INITIALIZED_OFF] != 0;
    let freeze_authority = read_coption_pubkey(data, FREEZE_AUTH_TAG_OFF, FREEZE_AUTH_KEY_OFF);

    debug!(
        address,
        supply,
        decimals,
        mint_authority = ?mint_authority,
        freeze_authority = ?freeze_authority,
        "Decoded SPL mint"
    );

    Ok(MintInfo {
        address: address.to_owned(),
        supply,
        decimals,
        is_initialized,
        mint_authority,
        freeze_authority,
    })
}

/// Read a Solana `COption<Pubkey>` from `data[tag_off..]`.
///
/// Layout: `[tag: u32 le][key: 32 bytes]`
/// `tag == 1` → `Some(base58(key))`; otherwise `None`.
fn read_coption_pubkey(data: &[u8], tag_off: usize, key_off: usize) -> Option<String> {
    let tag = u32::from_le_bytes(data[tag_off..tag_off + 4].try_into().ok()?);
    if tag == 1 {
        Some(bs58::encode(&data[key_off..key_off + 32]).into_string())
    } else {
        None
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn mint_bytes(
        mint_auth: bool,
        supply: u64,
        decimals: u8,
        initialized: bool,
        freeze_auth: bool,
    ) -> Vec<u8> {
        let mut d = vec![0u8; MINT_LEN];
        if mint_auth {
            d[MINT_AUTH_TAG_OFF..MINT_AUTH_TAG_OFF + 4].copy_from_slice(&1u32.to_le_bytes());
            d[4..36].fill(0xAB);
        }
        d[SUPPLY_OFF..SUPPLY_OFF + 8].copy_from_slice(&supply.to_le_bytes());
        d[DECIMALS_OFF] = decimals;
        d[INITIALIZED_OFF] = u8::from(initialized);
        if freeze_auth {
            d[FREEZE_AUTH_TAG_OFF..FREEZE_AUTH_TAG_OFF + 4].copy_from_slice(&1u32.to_le_bytes());
            d[FREEZE_AUTH_KEY_OFF..FREEZE_AUTH_KEY_OFF + 32].fill(0xCD);
        }
        d
    }

    #[test]
    fn decode_safe_mint_no_authorities() {
        let data = mint_bytes(false, 1_000_000_000, 9, true, false);
        let mint = decode_mint(&data, "SafeMint111").unwrap();
        assert_eq!(mint.supply, 1_000_000_000);
        assert_eq!(mint.decimals, 9);
        assert!(mint.is_initialized);
        assert!(mint.mint_authority.is_none());
        assert!(mint.freeze_authority.is_none());
    }

    #[test]
    fn decode_mint_with_freeze_authority() {
        let data = mint_bytes(false, 500, 6, true, true);
        let mint = decode_mint(&data, "FrozenMint111").unwrap();
        assert!(mint.freeze_authority.is_some());
        assert!(mint.mint_authority.is_none());
    }

    #[test]
    fn decode_mint_with_both_authorities() {
        let data = mint_bytes(true, 0, 9, true, true);
        let mint = decode_mint(&data, "DangerMint111").unwrap();
        assert!(mint.mint_authority.is_some());
        assert!(mint.freeze_authority.is_some());
        assert_eq!(mint.supply, 0);
    }

    #[test]
    fn decode_short_data_returns_err() {
        let data = vec![0u8; 10];
        assert!(decode_mint(&data, "short").is_err());
    }

    #[test]
    fn system_program_pubkey_encodes_correctly() {
        let zeros = vec![0u8; 32];
        assert_eq!(
            bs58::encode(&zeros).into_string(),
            "11111111111111111111111111111111"
        );
    }
}
