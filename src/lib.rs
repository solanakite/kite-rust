//! # Solana Kite
//!
//! A Rust library that works great with [LiteSVM](https://litesvm.org) for testing your Solana programs.
//! This crate offers high-level abstractions for common Solana operations like program deployment,
//! transaction sending, token operations, and account management.
//!
//! ## Features
//!
//! - **Program Deployment**: Deploy programs to a test environment (from files or bytes)
//! - **Transaction Utilities**: Send transactions from instructions with proper signing
//! - **Token Operations**: Create mints, associated token accounts, and mint tokens
//! - **Token-2022 Support**: Create Token-2022 mints with extensions, transfer hooks, and more
//! - **Account Management**: Create wallets, check balances, and manage account state
//! - **PDA Utilities**: Generate Program Derived Addresses with type-safe seed handling
//!
//! ## Example
//!
//! ```rust
//! use solana_kite::{create_wallet, create_token_mint};
//! use litesvm::LiteSVM;
//!
//! let mut litesvm = LiteSVM::new();
//! let wallet = create_wallet(&mut litesvm, 1_000_000_000).unwrap(); // 1 SOL
//! let mint = create_token_mint(&mut litesvm, &wallet, 6, None).unwrap(); // 6 decimals
//! ```

pub mod error;
pub mod pda;
pub mod program;
pub mod token;
pub mod token_2022;
pub mod transaction;
pub mod transfer_hook;
pub mod wallet;

pub use error::SolanaKiteError;
pub use pda::{get_pda_and_bump, Seed};
pub use program::{deploy_program, deploy_program_bytes};
pub use token::{
    assert_token_balance, create_associated_token_account, create_token_mint,
    get_token_account_balance, mint_tokens_to_account,
};
pub use transaction::send_transaction_from_instructions;
pub use wallet::{create_wallet, create_wallets};

// The seeds! macro is automatically available at the crate root due to #[macro_export]

/// Verifies that an account is closed (either doesn't exist or has empty data).
///
/// # Arguments
///
/// * `litesvm` - The LiteSVM instance to query
/// * `account` - The account address to check
/// * `message` - Error message to display if the account is not closed
///
/// # Panics
///
/// Panics if the account exists and has non-empty data, with the provided message.
///
/// # Example
///
/// ```rust
/// use solana_kite::check_account_is_closed;
/// use litesvm::LiteSVM;
/// use solana_pubkey::Pubkey;
///
/// let litesvm = LiteSVM::new();
/// let account = Pubkey::new_unique();
/// check_account_is_closed(&litesvm, &account, "Account should be closed");
/// ```
pub fn check_account_is_closed(
    litesvm: &litesvm::LiteSVM,
    account: &solana_pubkey::Pubkey,
    message: &str,
) {
    let account_data = litesvm.get_account(account);
    assert!(
        account_data.is_none() || account_data.unwrap().data.is_empty(),
        "{}",
        message
    );
}
