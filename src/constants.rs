//! Constants shared across token and token extensions modules.

use solana_pubkey::Pubkey;

/// SPL Token mint account data size in bytes.
/// https://docs.rs/spl-token/latest/spl_token/state/struct.Mint.html
pub const SPL_TOKEN_MINT_SIZE: usize = 82;

/// Byte offset where the amount field begins in the SPL Token account layout.
/// https://docs.rs/spl-token/latest/spl_token/state/struct.Account.html
pub const TOKEN_ACCOUNT_AMOUNT_OFFSET: usize = 64;

/// Byte offset where the amount field ends in the SPL Token account layout.
/// https://docs.rs/spl-token/latest/spl_token/state/struct.Account.html
pub const TOKEN_ACCOUNT_AMOUNT_END: usize = 72;

/// The System program ID.
pub const SYSTEM_PROGRAM_ID: Pubkey = solana_pubkey::pubkey!("11111111111111111111111111111111");
