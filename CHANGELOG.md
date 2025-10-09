# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Nothing yet.

## [0.2.0]

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