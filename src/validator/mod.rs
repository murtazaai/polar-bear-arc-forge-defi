//! # Token validator
//!
//! Sniper-bot prevention and rug-pull risk assessment.
//!
//! | # | Check | Attack vector mitigated |
//! |---|-------|------------------------|
//! | 1 | Freeze Authority | Deployer can freeze holder accounts |
//! | 2 | Mint Authority | Deployer can inflate supply post-launch |
//! | 3 | Mint Initialized | Uninitialized account is not a real token |
//! | 4 | Decimals Sanity | Non-standard decimals enable price-display tricks |
//! | 5 | Zero Supply Guard | Supply=0 + live mint authority = stealth-mint honey-pot |
//! | 6 | Supply Upper Bound | Astronomical supply creates decimal-trick manipulation |

/// Validates a token account against a set of security checks.
///
/// See the [Solana RPC API documentation](https://docs.solana.com/developing/clients/jsonrpc-api) for more information.
pub mod token_validator;

/// Re-exports the [`TokenValidator`] struct.
///
/// This is the main struct used to validate token accounts.
pub use token_validator::TokenValidator;
