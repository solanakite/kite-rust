# Solana Kite 🪁

[![Crates.io](https://img.shields.io/crates/v/solana-kite.svg)](https://crates.io/crates/solana-kite)
[![Documentation](https://docs.rs/solana-kite/badge.svg)](https://docs.rs/solana-kite)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE.md)

A Rust library that works great with [LiteSVM](https://litesvm.org) for testing your Solana programs. High-level abstractions for common Solana operations: wallets, transactions, tokens, Token Extensions, transfer hooks, PDAs, and program deployment.

## Features

- 🚀 **Program Deployment**: Deploy programs from files or bytes (`include_bytes!`)
- 💸 **Transaction Utilities**: Send v1 transactions (SIMD-0385) from instructions with proper signing
- 🪙 **Token Operations**: Create mints, ATAs, mint tokens, check balances
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
solana-kite = "0.7"
```

**Compatibility:** Solana 3.x / Anchor 1.2 / LiteSVM 0.18 / Rust 1.97.1+

## Quick Start

```rust
use solana_kite::{create_wallet, create_token_mint, create_associated_token_account, mint_tokens_to_token_account};
use litesvm::LiteSVM;
use solana_signer::Signer;

let mut svm = LiteSVM::new();

// Create a funded wallet
let authority = create_wallet(&mut svm, 1_000_000_000).unwrap();

// Create a token mint (6 decimals, like USDC). The last argument of every function
// that sends a transaction is its compute unit limit: None simulates the
// transaction to find it.
let mint = create_token_mint(&mut svm, &authority, 6, None, None).unwrap();

// Create associated token account
let ata = create_associated_token_account(&mut svm, &authority.pubkey(), &mint, &authority, None).unwrap();

// Mint 1000 tokens
mint_tokens_to_token_account(&mut svm, &mint, &ata, 1_000_000_000, &authority, None).unwrap();
```

## API Overview

### Wallets

```rust
use solana_kite::{create_wallet, create_wallets};

let wallet = create_wallet(&mut svm, 1_000_000_000)?; // 1 SOL
let wallets = create_wallets(&mut svm, 5, 1_000_000_000)?; // 5 wallets, 1 SOL each
```

### SOL Balances

```rust
use solana_kite::{get_sol_balance, assert_sol_balance};

let balance = get_sol_balance(&svm, &wallet.pubkey()); // returns lamports, 0 if account missing
assert_sol_balance(&svm, &wallet.pubkey(), 1_000_000_000, "Should have 1 SOL");
```

### Transactions

```rust
use solana_kite::send_transaction_from_instructions;

send_transaction_from_instructions(
    &mut svm,
    vec![instruction1, instruction2],
    &[&payer, &other_signer],
    &payer.pubkey(),
    Some(200_000), // compute unit limit, or None to simulate and use what the simulation used
)?;
```

Every transaction kite builds is a v1 transaction (SIMD-0385, live on mainnet beta since 15 September
2026), including the ones its token, Token Extensions and transfer hook helpers send. `create_wallet`
funds a wallet with LiteSVM's airdrop, which LiteSVM builds as a legacy transaction.

A v1 transaction holds up to 4,096 bytes. Its compute unit, loaded account data and heap limits, and
its priority fee, are part of the transaction itself, in a `TransactionConfig` in the message, and an
unset compute unit or loaded account data limit is zero. So:

- **The compute unit limit is the last argument** of `send_transaction_from_instructions` and of every
  token, Token Extensions and transfer hook helper: `compute_unit_limit: Option<u32>`.
  - `Some(limit)` sends the transaction with that limit. Choose a number your instructions run in, and
    the test fails if they need more: a transaction that runs out of compute units fails.
  - `None` simulates the transaction in LiteSVM and sends it with the compute units the simulation
    used. It never fails for running out, so it does not check how many compute units your program
    uses.
- **Kite sets the loaded account data limit** to the most a transaction may load, Agave's
  `MAX_LOADED_ACCOUNTS_DATA_SIZE_BYTES` (64 MiB in `solana-compute-budget` 4.2). The heap is 32 KiB and
  there is no priority fee. See `get_transaction_config`.
- **ComputeBudget instructions are rejected.** A ComputeBudget instruction in a v1 transaction does
  nothing, so kite returns an error for one rather than let a test believe it set a limit or priority
  fee.

To read logs or compute units, measure a transaction, or set every limit yourself, build it with
`build_transaction_from_instructions` and send it through LiteSVM:

```rust
use solana_kite::{build_transaction_from_instructions, get_transaction_config};

let transaction = build_transaction_from_instructions(
    &svm,
    &instructions,
    &[&payer],
    &payer.pubkey(),
    get_transaction_config(50_000),
)?;
let metadata = svm.send_transaction(transaction).unwrap();
println!("{} compute units", metadata.compute_units_consumed);
```

### Token Operations

```rust
use solana_kite::{
    create_token_mint, create_associated_token_account, get_token_account_address,
    mint_tokens_to_token_account, get_token_account_balance, assert_token_account_balance,
};

let mint = create_token_mint(&mut svm, &authority, 9, None, None)?;

// Pre-compute the ATA address before creating it
let ata_address = get_token_account_address(&owner.pubkey(), &mint);

let ata = create_associated_token_account(&mut svm, &owner.pubkey(), &mint, &payer, None)?;
assert_eq!(ata_address, ata);
mint_tokens_to_token_account(&mut svm, &mint, &ata, 1_000_000_000, &authority, None)?;
assert_token_account_balance(&svm, &ata, 1_000_000_000, "Should have 1B tokens");
```

### Token Extensions

Create mints with extensions: transfer hooks, transfer fees, permanent delegates, non-transferable tokens, and more:

```rust
use solana_kite::{
    create_token_extensions_mint, create_token_extensions_account,
    get_token_extensions_account_address, mint_tokens_to_token_extensions_account,
    transfer_checked_token_extensions, MintExtension, TokenAccountState,
};

// Create a mint with transfer fee extension
let mint = create_token_extensions_mint(
    &mut svm,
    &authority,
    6,
    None,
    &[MintExtension::TransferFee {
        fee_basis_points: 100, // 1%
        maximum_fee: 1_000_000,
    }],
    None,
)?;

// Pre-compute an ATA address before creating it (e.g. to pass to a program instruction)
let sender_ata_address = get_token_extensions_account_address(&sender.pubkey(), &mint);

// Create token accounts, mint, and transfer
let sender_ata = create_token_extensions_account(&mut svm, &sender.pubkey(), &mint, &payer, None)?;
let receiver_ata = create_token_extensions_account(&mut svm, &receiver.pubkey(), &mint, &payer, None)?;
mint_tokens_to_token_extensions_account(&mut svm, &mint, &sender_ata, 1_000_000, &authority, None)?;
transfer_checked_token_extensions(&mut svm, &sender_ata, &mint, &receiver_ata, &sender, 500_000, 6, &[], None)?;

```

**Supported extensions:**
- `TransferHook` - attach a hook program to transfers
- `TransferFee` - automatic fee collection on transfers
- `MintCloseAuthority` - allow closing the mint account
- `PermanentDelegate` - irrevocable delegate authority
- `NonTransferable` - soulbound tokens
- `DefaultAccountState` - new token accounts start in a given `TokenAccountState` (Frozen, Initialized, or Uninitialized)
- `InterestBearing` - display interest rate on token
- `MetadataPointer` - point to onchain metadata

### Transfer Hooks

```rust
use solana_kite::{
    build_hook_accounts, get_hook_accounts_address, initialize_hook_accounts, HookAccount,
};

// The extra accounts your hook program reads on every transfer
let hook_accounts: Vec<HookAccount> = vec![/* ... */];

// Initialize the ExtraAccountMetaList account for your hook program
initialize_hook_accounts(
    &mut svm,
    &hook_program_id,
    &mint,
    &authority,
    &hook_accounts,
    None,
)?;

// The ExtraAccountMetaList PDA, e.g. to pass to your hook program's instructions
let extra_account_meta_list_address = get_hook_accounts_address(&mint, &hook_program_id);

// Build the account list to pass as `hook_accounts` to transfer_checked_token_extensions
let transfer_hook_account_metas = build_hook_accounts(&mint, &hook_program_id, &hook_accounts);
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
use solana_kite::{get_pda_and_bump, Seed};

let (pda, bump) = get_pda_and_bump(
    &seeds!["user-account", user_address, 42u64],
    &program_id,
);
```

### Account Utilities

```rust
use solana_kite::check_account_is_closed;

check_account_is_closed(&svm, &account_pubkey, "Account should be closed after this instruction");
```

## Error Handling

Every function that can fail returns `Result<T, SolanaKiteError>`:

```rust
use solana_kite::SolanaKiteError;

match some_operation() {
    Ok(result) => println!("Success: {:?}", result),
    Err(SolanaKiteError::TransactionFailed(message)) => eprintln!("Transaction failed: {message}"),
    Err(SolanaKiteError::TokenOperationFailed(message)) => eprintln!("Token operation failed: {message}"),
    Err(error) => eprintln!("Error: {error}"),
}
```

## Examples

```bash
cargo run --example basic_usage
cargo run --example token_operations
cargo run --example token_extensions_operations
cargo run --example transfer_hook_operations
```

## Testing

```bash
cargo test
```

## License

MIT - see [LICENSE.md](LICENSE.md).

---

Made with ❤️ for the Solana ecosystem
