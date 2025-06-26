# Solana Kite 🪁

[![Crates.io](https://img.shields.io/crates/v/solana-kite.svg)](https://crates.io/crates/solana-kite)
[![Documentation](https://docs.rs/solana-kite/badge.svg)](https://docs.rs/solana-kite)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE.md)

> [!NOTE]
> This is a new Rust port of [Solana Kite](https://solanakite.org)! It works well but it may have some bugs - please [report them](https://github.com/solanakite/kite-rust/issues)!

A Rust library that works great with [LiteSVM](https://litesvm.org) for testing your Solana programs. Solana Kite offers high-level abstractions for common Solana operations like program deployment, transaction sending, token operations, and account management.

## Features

- 🚀 **Program Deployment**: Deploy programs to test environments
- 💸 **Transaction Utilities**: Send transactions from instructions with proper signing
- 🪙 **Token Operations**: Create mints, associated token accounts, and mint tokens
- 👛 **Account Management**: Create wallets, check balances, and manage account state
- 🔑 **PDA Utilities**: Generate Program Derived Addresses with type-safe seed handling
- 🛡️ **Error Handling**: Comprehensive error types for robust error handling
- 📚 **Well Documented**: Extensive documentation and examples

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
solana-kite = "0.1.0"
```

## Quick Start

```rust
use solana_kite::{create_wallet, create_token_mint, create_associated_token_account, mint_tokens_to_account};
use litesvm::LiteSVM;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize test environment
    let mut litesvm = LiteSVM::new();
    
    // Create wallets
    let mint_authority = create_wallet(&mut litesvm, 1_000_000_000)?; // 1 SOL
    let user = create_wallet(&mut litesvm, 1_000_000_000)?; // 1 SOL
    
    // Create a token mint (6 decimals, like USDC)
    let mint = create_token_mint(&mut litesvm, &mint_authority, 6)?;
    
    // Create associated token account for user
    let user_token_account = create_associated_token_account(
        &mut litesvm,
        &user,
        &mint.pubkey(),
        &user,
    )?;
    
    // Mint 1000 tokens to user
    mint_tokens_to_account(
        &mut litesvm,
        &mint.pubkey(),
        &user_token_account,
        1_000_000_000, // 1000 tokens with 6 decimals
        &mint_authority,
    )?;
    
    println!("🎉 Successfully minted tokens!");
    Ok(())
}
```

## API Overview

### Wallet Operations

```rust
use solana_kite::{create_wallet, create_wallets};

// Create a single wallet with 1 SOL
let wallet = create_wallet(&mut litesvm, 1_000_000_000)?;

// Create multiple wallets
let wallets = create_wallets(&mut litesvm, 5, 1_000_000_000)?; // 5 wallets, 1 SOL each
```

### Token Operations

```rust
use solana_kite::{
    create_token_mint, create_associated_token_account, 
    mint_tokens_to_account, get_token_account_balance, assert_token_balance
};

// Create token mint with 9 decimals
let mint = create_token_mint(&mut litesvm, &mint_authority, 9)?;

// Create associated token account
let token_account = create_associated_token_account(&mut litesvm, &owner, &mint.pubkey(), &payer)?;

// Mint tokens
mint_tokens_to_account(&mut litesvm, &mint.pubkey(), &token_account, 1_000_000_000, &mint_authority)?;

// Check balance
let balance = get_token_account_balance(&litesvm, &token_account)?;

// Assert balance (useful for testing)
assert_token_balance(&litesvm, &token_account, 1_000_000_000, "Should have 1B tokens");
```

### Program Derived Addresses (PDAs)

```rust
use solana_kite::{get_pda_and_bump, seeds, Seed};
use solana_pubkey::Pubkey;

let program_id = Pubkey::new_unique();
let user_address = Pubkey::new_unique();

// Using the convenient seeds! macro
let seed_vec = seeds!["user-account", user_address, 42u64];
let (pda, bump) = get_pda_and_bump(&seed_vec, &program_id);

// Or create seeds manually
let manual_seeds = vec![
    Seed::String("user-account".to_string()),
    Seed::Address(user_address),
    Seed::U64(42),
];
let (pda2, bump2) = get_pda_and_bump(&manual_seeds, &program_id);
```

### Transaction Sending

```rust
use solana_kite::send_transaction_from_instructions;

let instructions = vec![/* your instructions */];
send_transaction_from_instructions(
    &mut litesvm,
    instructions,
    &[&signer1, &signer2],
    &fee_payer.pubkey(),
)?;
```

### Program Deployment

```rust
use solana_kite::deploy_program;

deploy_program(
    &mut litesvm,
    &program_id,
    "./target/deploy/my_program.so",
)?;
```

## Error Handling

Solana Kite provides comprehensive error handling through the `SolanaKiteError` enum:

```rust
use solana_kite::SolanaKiteError;

match some_operation() {
    Ok(result) => println!("Success: {:?}", result),
    Err(SolanaKiteError::TransactionFailed(msg)) => eprintln!("Transaction failed: {}", msg),
    Err(SolanaKiteError::TokenOperationFailed(msg)) => eprintln!("Token operation failed: {}", msg),
    Err(SolanaKiteError::AccountOperationFailed(msg)) => eprintln!("Account operation failed: {}", msg),
    Err(e) => eprintln!("Other error: {}", e),
}
```

## Examples

The repository includes comprehensive examples:

- **Basic Usage**: `cargo run --example basic_usage`
- **Token Operations**: `cargo run --example token_operations`

## Testing

Run the test suite:

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run integration tests specifically
cargo test --test integration_tests
```

## Features

The crate supports the following Cargo features:

- `default`: Standard functionality
- `testing`: Additional testing utilities (currently empty, reserved for future use)

## Documentation

Full API documentation is available on [docs.rs](https://docs.rs/solana_kite).

Generate local documentation:

```bash
cargo doc --open
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

### Development Setup

1. Clone the repository
2. Install Rust (if not already installed)
3. Run tests: `cargo test`
4. Run examples: `cargo run --example basic_usage`

## License

This project is licensed under the MIT License. See [LICENSE.md](LICENSE.md) for details.

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for details about changes in each version.

---

Made with ❤️ for the Solana ecosystem