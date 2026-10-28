# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.7.0] - 2026-10-28

### Breaking Changes

- **LiteSVM 0.18**: Bumped from 0.16.0 to 0.18.1, adding support for Agave 4.3
- **Rust 1.97.1**: litesvm 0.18 needs Rust 1.97.1, so kite now sets `rust-version = "1.97.1"`
- **`solana-compute-budget` 4.3**: moved from 4.2 to the version litesvm 0.18 uses

### Notes

- `solana-instruction` 3.5 re-exports `solana-instruction` 4.0, which serializes with wincode 0.6. `solana-loader-v3-interface` 8.0.1, which litesvm 0.18 depends on, still uses wincode 0.5 and fails to compile against it. 8.1.1 fixes this: a project with 8.0.1 in its `Cargo.lock` should run `cargo update -p solana-loader-v3-interface`.
- The tests' `wincode` dev-dependency moved from 0.5 to 0.6, the version `solana-transaction` now uses.
- Anchor 1.2 programs still build alongside kite.
- No source changes were required. All 2 unit tests, 30 integration tests and 26 doc-tests pass.

## [0.6.1] - 2026-10-15

### Fixed

- **No program entrypoints**: `spl-token` and `spl-associated-token-account` are built with their `no-entrypoint` feature. Without it each defined an `entrypoint` symbol, so a test binary linking kite and an Anchor program failed with `duplicate symbol: entrypoint`. Anchor's IDL build links the tests, so `anchor build` failed for `compression/cnft-vault` in solana-program-examples. Kite only calls their instruction builders; the token programs themselves run inside LiteSVM.

## [0.6.0] - 2026-10-14

### Breaking Changes

- **v1 transactions**: `send_transaction_from_instructions` now sends a v1 transaction (SIMD-0385, live on mainnet beta since 15 September 2026, epoch 1035) instead of a legacy one, so every helper that sends a transaction does too: `create_token_mint`, `create_associated_token_account`, `mint_tokens_to_token_account`, the Token Extensions helpers and `initialize_hook_accounts`. A v1 transaction holds up to 4,096 bytes rather than 1,232. `create_wallet` still funds wallets with LiteSVM's airdrop, which LiteSVM builds as a legacy transaction.
- **Every transaction states its limits**: a v1 transaction carries its limits and priority fee in a `TransactionConfig`, and an unset compute unit or loaded account data limit is zero, so kite sets both in every transaction it builds. Where the compute unit limit comes from:
  - `send_transaction_from_instructions`, `create_token_mint`, `create_associated_token_account`, `mint_tokens_to_token_account`, `create_token_extensions_mint`, `create_token_extensions_account`, `mint_tokens_to_token_extensions_account`, `transfer_checked_token_extensions` and `initialize_hook_accounts` take a new last argument, `compute_unit_limit: Option<u32>`.
  - `Some(limit)` uses that limit, so a test fails if its transaction needs more.
  - `None` simulates the transaction and sends it with the compute units the simulation used.
  - Kite sets the loaded account data limit to 64 MiB, the most a transaction may load.
- **ComputeBudget instructions are rejected**: a v1 transaction ignores them, so `build_transaction_from_instructions`, and so `send_transaction_from_instructions`, return `SolanaKiteError::TransactionFailed` for one, rather than let a test believe it set a limit or priority fee. Remove ComputeBudget instructions, and pass the compute unit limit as `compute_unit_limit`, or set limits and a priority fee in the config passed to `build_transaction_from_instructions`.

### Added

- `build_transaction_from_instructions`: builds and signs a v1 transaction with a given config, for tests that read logs or compute units, measure the transaction, or set every limit themselves. A keypair passed twice signs once.
- `get_transaction_config`: the config kite gives a transaction with a given compute unit limit, with 64 MiB of loaded account data, a 32 KiB heap and no priority fee.
- `solana-compute-budget` 4.2 dependency: kite reads the loaded account data limit from Agave's `MAX_LOADED_ACCOUNTS_DATA_SIZE_BYTES`, and simulates a transaction with `MAX_COMPUTE_UNIT_LIMIT` when no compute unit limit is passed. litesvm 0.16 already depends on the same version.
- `solana-sdk-ids` 3.1 dependency: kite identifies ComputeBudget instructions by the ComputeBudget program ID. litesvm 0.16 already depends on the same version.

### Notes

- All 2 unit tests, 30 integration tests and 26 doc-tests pass.

## [0.5.0] - 2026-09-08

### Breaking Changes

- **LiteSVM 0.16**: Bumped from 0.13.1 to 0.16.0, adding support for Agave 4.2
- **Solana 4.x SDK crates**: `solana-account` (3.4.0 → 4.3.0), `solana-message` (3.1.0 → 4.2.4) and `solana-transaction` (3.1.0 → 4.1.5) moved to their 4.x lines to match litesvm 0.16

### Notes

- Anchor 1.2.0 depends on `solana-loader-v3-interface ^6.1.1`, which needs `solana-instruction ^3.3.0`. litesvm 0.13.1 pinned `solana-instruction = "=3.2.0"`, so Anchor 1.2.0 programs could not use solana-kite 0.4.0 in their tests. litesvm 0.16.0 wants `solana-instruction ~3.4.0`, which the existing `>=3.2, <4` range already allows.
- No source changes were required. All 2 unit tests, 22 integration tests and 24 doc-tests pass.

## [0.4.0] - 2026-07-01

### Breaking Changes

- **LiteSVM 0.13**: Bumped from 0.11.0 to 0.13.1, adding support for Agave 4.0
- **Relaxed `solana-instruction`**: Loosened from `^3.3.0` to `>=3.2, <4` so Cargo can satisfy litesvm 0.13.1's exact `=3.2.0` pin on `solana-instruction`

### Notes

- No source changes were required — the litesvm public API surface used by this crate (`LiteSVM::new`, `latest_blockhash`, `set_account`, `minimum_balance_for_rent_exemption`, `send_transaction`) is unchanged from 0.11 to 0.13. All 22 integration tests and 24 doc-tests pass.

## [0.3.0] - 2026-04-06

### Breaking Changes

- **Solana 3.x**: All Solana dependencies bumped from 2.x to 3.0 (matching Anchor 1.0.0-rc.5)
- **LiteSVM 0.11**: Bumped from 0.7 to 0.11.0
- **SPL crates**: spl-token 8→9, spl-associated-token-account 7→8
- **Removed deprecated `TestError`**: Use `SolanaKiteError` instead (deprecated since 0.1.0)
- **Renamed functions** for consistency — "token account" and "token extensions account" are now used consistently:
  - `mint_tokens_to_account` → `mint_tokens_to_token_account`
  - `assert_token_balance` → `assert_token_account_balance`
- **Unified token balance helpers**: `get_token_account_balance` and `assert_token_account_balance` work for both Classic Token and Token Extensions accounts — no separate helpers needed (the base account layout is identical)
- **New error variant** `SolanaKiteError::HookOperationFailed` for transfer hook failures (previously `TokenOperationFailed`)

### Changed

- Updated `keccak` transitive dependency from 0.1.5 (yanked) to 0.1.6, which fixes undefined behaviour in the ARMv8 inline assembly backend

### Added

- **Token Extensions module** (`token_extensions`):
  - `create_token_extensions_mint(litesvm, mint_authority, decimals, mint, extensions)` — create mints with any combination of 8 extension types; `mint` is an optional custom address (pass `None` to generate one)
  - `create_token_extensions_account` — create ATAs for Token Extensions mints
  - `mint_tokens_to_token_extensions_account` — mint tokens via Token Extensions
  - `transfer_checked_token_extensions` — TransferChecked with hook account support
  - `get_token_extensions_account_address` — derive a Token Extensions ATA address without creating it
  - `MintExtension` enum: TransferHook, TransferFee, MintCloseAuthority, PermanentDelegate, NonTransferable, DefaultAccountState, InterestBearing, MetadataPointer
  - `TokenAccountState` enum: Frozen, Initialized, Uninitialized (used with `DefaultAccountState`)
- **Transfer hook module** (`transfer_hook`):
  - `HookAccount` — describes an account a hook program requires on every transfer
  - `get_hook_accounts_address` — derive the ExtraAccountMetaList PDA address
  - `initialize_hook_accounts` — initialise the hook program's account list PDA
  - `build_hook_accounts` — build the accounts to pass into a hook-aware transfer
- **`get_token_account_address`** — derive a Classic Token Program ATA address without creating it (mirrors `get_token_extensions_account_address`)
- **`deploy_program_bytes`** — deploy programs from `&[u8]` (for `include_bytes!` workflows)
- **`get_sol_balance` / `assert_sol_balance`** — check SOL balances in lamports (parallel to the token balance helpers)
- **Token Extensions example** (`examples/token_extensions_operations.rs`)
- **Transfer hook example** (`examples/transfer_hook_operations.rs`) — end-to-end walkthrough of mint creation, ExtraAccountMetaList setup, and hook-aware `TransferChecked`
- **11 new Token Extensions integration tests** covering all extension types

## [0.2.1] - 2025-10-09

### Changed

- Changed dependencies to match Anchor 0.32.0 requirements
- Changed litesvm to 0.7.x
- Changed Solana dependencies to specific versions required by Anchor 0.32.0:
  - solana-account: 2.2.1
  - solana-instruction: 2.3.0
  - solana-pubkey: 2.4.0
  - solana-message: 2.4.0
  - solana-transaction: 2.2.3
- Changed SPL token dependencies:
  - spl-token: 8.0.0
  - spl-associated-token-account: 7.0.0
- Changed tokio to 1.47

## [0.2.0] - 2025-10-09

### Changed

- Update LiteSVM and Solana dependencies to latest supported by `spl-token` crate.
- `create_token_mint()` (thanks @M-Daeva)
  - Added option for a custom mint address
  - Changed return type from `Result<Keypair, SolanaKiteError>` to `Result<Pubkey, SolanaKiteError>` - now returns just the public key instead of the full keypair
  - Used Pubkey::new_unique(): Instead of Keypair::new() for generating random addresses
- `create_associated_token_account()` (thanks @M-Daeva!)
  - Changed `owner` parameter from &Keypair to &Pubkey - as you only need the public key, not the full keypair
  - Removed owner from signing: The owner no longer needs to sign the transaction since only the payer needs to sign for ATA creation.

## [0.1.0] - 2025-01-26

### Added
- Initial release of Solana Kite
- Wallet creation and management utilities (`create_wallet`, `create_wallets`)
- Token operations:
  - Token mint creation (`create_token_mint`)
  - Associated token account creation (`create_associated_token_account`)
  - Token minting (`mint_tokens_to_account`)
  - Balance checking (`get_token_account_balance`, `assert_token_balance`)
- Transaction utilities (`send_transaction_from_instructions`)
- Program deployment utilities (`deploy_program`)
- Program Derived Address (PDA) utilities:
  - Type-safe seed handling (`Seed` enum)
  - PDA generation (`get_pda_and_bump`)
  - Convenient `seeds!` macro
- Comprehensive error handling (`SolanaKiteError`)
- Account management utilities (`check_account_is_closed`)
- Extensive documentation and examples
- Integration tests
- Support for Solana 2.1.x and SPL Token libraries

### Features
- Full rustdoc documentation
- Two comprehensive examples (basic usage and token operations)
- Integration test suite
- Support for LiteSVM test environment
- Type-safe error handling
- `TestError` type alias for backward compatibility (deprecated — use `SolanaKiteError`)

[0.3.0]: https://github.com/solanakite/kite-rust/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/solanakite/kite-rust/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/solanakite/kite-rust/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/solanakite/kite-rust/releases/tag/v0.1.0