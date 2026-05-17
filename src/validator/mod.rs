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

pub mod token_validator;

pub use token_validator::TokenValidator;
