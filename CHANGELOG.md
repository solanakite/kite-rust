# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-04-02

### Breaking Changes

- **Solana 3.x**: All Solana dependencies bumped from 2.x to 3.0 (matching Anchor 1.0.0-rc.5)
- **LiteSVM 0.11**: Bumped from 0.7 to 0.11.0
- **SPL crates**: spl-token 8→9, spl-associated-token-account 7→8
- **Removed deprecated `TestError`**: Use `SolanaKiteError` instead (deprecated since 0.1.0)

### Added

- **Token-2022 module** (`token_2022`):
  - `create_token_2022_mint` — create mints with any combination of 8 extension types
  - `create_token_2022_account` — create ATAs for Token-2022 mints
  - `mint_tokens_to_account_2022` — mint tokens via Token-2022
  - `transfer_checked_token_2022` — TransferChecked with extra account support (for hooks)
  - `get_token_2022_balance` / `assert_token_2022_balance` — balance helpers
  - `MintExtension` enum: TransferHook, TransferFee, MintCloseAuthority, PermanentDelegate, NonTransferable, DefaultAccountState, InterestBearing, MetadataPointer
- **Transfer hook module** (`transfer_hook`):
  - `ExtraAccountMeta` — describes additional accounts for transfer hooks
  - `get_extra_account_metas_address` — derive the ExtraAccountMetaList PDA
  - `initialize_extra_account_meta_list` — initialise the PDA for a hook program
  - `build_transfer_hook_extra_accounts` — build extra accounts for transfers
- **`deploy_program_bytes`** — deploy programs from `&[u8]` (for `include_bytes!` workflows)
- **Token-2022 example** (`examples/token_2022_operations.rs`)
- **11 new Token-2022 integration tests** covering all extension types

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
  - Now returns Result<Keypair, SolanaKiteError> to Result<Pubkey, SolanaKiteError> - now returns just the public key instead of the full keypair
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
- Backward compatibility with legacy `TestError` type

[Unreleased]: https://github.com/solanakite/kite-rust/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/solanakite/kite-rust/releases/tag/v0.1.0