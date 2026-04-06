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
//! - **Token Extensions Support**: Create Token Extensions mints with extensions, transfer hooks, and more
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

mod constants;
pub mod error;
pub mod pda;
pub mod program;
pub mod token;
pub mod token_extensions;
pub mod transaction;
pub mod transfer_hook;
pub mod wallet;

pub use error::SolanaKiteError;
pub use pda::{get_pda_and_bump, Seed};
pub use program::{deploy_program, deploy_program_bytes};
pub use token::{
    assert_token_account_balance, create_associated_token_account, create_token_mint,
    get_token_account_address, get_token_account_balance, mint_tokens_to_token_account,
};
pub use token_extensions::{
    create_token_extensions_account, create_token_extensions_mint,
    get_token_extensions_account_address, mint_tokens_to_token_extensions_account,
    transfer_checked_token_extensions, MintExtension, TokenAccountState,
};
pub use transaction::send_transaction_from_instructions;
pub use transfer_hook::{
    build_hook_accounts, get_hook_accounts_address, initialize_hook_accounts, HookAccount,
};
pub use wallet::{assert_sol_balance, check_account_is_closed, create_wallet, create_wallets, get_sol_balance};
