# Solana Kite 🪁

[![Crates.io](https://img.shields.io/crates/v/solana-kite.svg)](https://crates.io/crates/solana-kite)
[![Documentation](https://docs.rs/solana-kite/badge.svg)](https://docs.rs/solana-kite)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE.md)

A Rust library that works great with [LiteSVM](https://litesvm.org) for testing your Solana programs. High-level abstractions for common Solana operations — wallets, transactions, SPL tokens, Token Extensions, transfer hooks, PDAs, and program deployment.

## Features

- 🚀 **Program Deployment**: Deploy programs from files or bytes (`include_bytes!`)
- 💸 **Transaction Utilities**: Send transactions from instructions with proper signing
- 🪙 **SPL Token Operations**: Create mints, ATAs, mint tokens, check balances
- 🔐 **Token Extensions**: Create mints with transfer hooks, transfer fees, permanent delegates, non-transferable tokens, and more
- 🪝 **Transfer Hook Support**: ExtraAccountMetaList setup and transfer helpers
- 👛 **Wallet Management**: Create funded wallets in one call
- 🔑 **PDA Utilities**: Type-safe seed handling with the `seeds!` macro
- 📚 **Well Documented**: Extensive docs and examples

## Installation

```
cargo add --dev solana-kite
```

or add to your `Cargo.toml`:

```toml
[dev-dependencies]
solana-kite = "0.3.0"
```

**Compatibility:** Solana 3.x / Anchor 1.0 / LiteSVM 0.11

## Quick Start

```rust
use solana_kite::{create_wallet, create_token_mint, create_associated_token_account, mint_tokens_to_account};
use litesvm::LiteSVM;
use solana_signer::Signer;

let mut svm = LiteSVM::new();

// Create a funded wallet
let authority = create_wallet(&mut svm, 1_000_000_000).unwrap();

// Create a token mint (6 decimals, like USDC)
let mint = create_token_mint(&mut svm, &authority, 6, None).unwrap();

// Create associated token account
let ata = create_associated_token_account(&mut svm, &authority.pubkey(), &mint, &authority).unwrap();

// Mint 1000 tokens
mint_tokens_to_account(&mut svm, &mint, &ata, 1_000_000_000, &authority).unwrap();
```

## API Overview

### Wallets

```rust
use solana_kite::{create_wallet, create_wallets};

let wallet = create_wallet(&mut svm, 1_000_000_000)?; // 1 SOL
let wallets = create_wallets(&mut svm, 5, 1_000_000_000)?; // 5 wallets, 1 SOL each
```

### Transactions

```rust
use solana_kite::send_transaction_from_instructions;

send_transaction_from_instructions(
    &mut svm,
    vec![instruction1, instruction2],
    &[&payer, &other_signer],
    &payer.pubkey(),
)?;
```

### SPL Token Operations

```rust
use solana_kite::{
    create_token_mint, create_associated_token_account,
    mint_tokens_to_account, get_token_account_balance, assert_token_balance,
};

let mint = create_token_mint(&mut svm, &authority, 9, None)?;
let ata = create_associated_token_account(&mut svm, &owner.pubkey(), &mint, &payer)?;
mint_tokens_to_account(&mut svm, &mint, &ata, 1_000_000_000, &authority)?;
assert_token_balance(&svm, &ata, 1_000_000_000, "Should have 1B tokens");
```

### Token Extensions

Create mints with extensions — transfer hooks, transfer fees, permanent delegates, non-transferable tokens, and more:

```rust
use solana_kite::{
    create_token_extensions_mint, create_token_extensions_account,
    mint_tokens_to_token_extensions_account, transfer_checked_token_extensions,
    MintExtension,
};

// Create a mint with transfer fee extension
let mint = create_token_extensions_mint(
    &mut svm,
    &authority,
    6,
    &[MintExtension::TransferFee {
        fee_basis_points: 100, // 1%
        maximum_fee: 1_000_000,
    }],
)?;

// Create token accounts, mint, and transfer
let sender_ata = create_token_extensions_account(&mut svm, &sender.pubkey(), &mint, &payer)?;
let receiver_ata = create_token_extensions_account(&mut svm, &receiver.pubkey(), &mint, &payer)?;
mint_tokens_to_token_extensions_account(&mut svm, &mint, &sender_ata, 1_000_000, &authority)?;
transfer_checked_token_extensions(&mut svm, &sender_ata, &mint, &receiver_ata, &sender, 500_000, 6, &[])?;
```

**Supported extensions:**
- `TransferHook` — attach a hook program to transfers
- `TransferFee` — automatic fee collection on transfers
- `MintCloseAuthority` — allow closing the mint account
- `PermanentDelegate` — irrevocable delegate authority
- `NonTransferable` — soulbound tokens
- `DefaultAccountState` — new token accounts start frozen/initialized
- `InterestBearing` — display interest rate on token
- `MetadataPointer` — point to on-chain metadata

### Transfer Hooks

```rust
use solana_kite::{
    initialize_extra_account_meta_list, get_extra_account_metas_address,
    build_transfer_hook_extra_accounts, ExtraAccountMeta,
};

// Initialize the ExtraAccountMetaList PDA for your hook program
initialize_extra_account_meta_list(
    &mut svm,
    &hook_program_id,
    &mint,
    &authority,
    &extra_metas,
)?;

// Get extra accounts for a transfer
let extra_accounts = build_transfer_hook_extra_accounts(
    &svm,
    &hook_program_id,
    &mint,
)?;
```

### Program Deployment

```rust
use solana_kite::{deploy_program, deploy_program_bytes};

// From a file path
deploy_program(&mut svm, &program_id, "./target/deploy/my_program.so")?;

// From bytes (works with include_bytes!)
let bytes = include_bytes!("../target/deploy/my_program.so");
deploy_program_bytes(&mut svm, &program_id, bytes)?;
```

### PDAs

```rust
use solana_kite::{get_pda_and_bump, seeds, Seed};

let (pda, bump) = get_pda_and_bump(
    &seeds!["user-account", user_address, 42u64],
    &program_id,
);
```

## Error Handling

All functions return `Result<T, SolanaKiteError>`:

```rust
use solana_kite::SolanaKiteError;

match some_operation() {
    Ok(result) => println!("Success: {:?}", result),
    Err(SolanaKiteError::TransactionFailed(msg)) => eprintln!("Tx failed: {}", msg),
    Err(SolanaKiteError::TokenOperationFailed(msg)) => eprintln!("Token op failed: {}", msg),
    Err(e) => eprintln!("Error: {}", e),
}
```

## Examples

```bash
cargo run --example basic_usage
cargo run --example token_operations
cargo run --example token_extensions_operations
```

## Testing

```bash
cargo test
```

## License

MIT — see [LICENSE.md](LICENSE.md).

---

Made with ❤️ for the Solana ecosystem
