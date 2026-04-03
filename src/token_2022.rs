//! Token-2022 (Token Extensions) operations for Solana.
//!
//! This module provides helpers for creating and interacting with Token-2022 mints
//! and accounts. All instructions are built from raw bytes to avoid depending on
//! the `spl-token-2022` crate, which can cause version conflicts with `anchor-lang`.
//!
//! # Supported Extensions
//!
//! - **TransferHook** — attach a program that runs on every transfer
//! - **TransferFee** — automatic fee collection on transfers
//! - **MintCloseAuthority** — allow closing a mint account
//! - **PermanentDelegate** — irrevocable delegate for all token accounts
//! - **NonTransferable** — soulbound tokens that cannot be transferred
//! - **DefaultAccountState** — new token accounts start frozen
//! - **InterestBearing** — on-chain interest rate display
//! - **MetadataPointer** — point to an on-chain metadata account
//!
//! # Example
//!
//! ```rust
//! use solana_kite::token_2022::{create_token_2022_mint, MintExtension};
//! use solana_kite::create_wallet;
//! use litesvm::LiteSVM;
//! use solana_pubkey::Pubkey;
//! use solana_signer::Signer;
//!
//! let mut litesvm = LiteSVM::new();
//! let authority = create_wallet(&mut litesvm, 1_000_000_000).unwrap();
//!
//! let mint = create_token_2022_mint(
//!     &mut litesvm,
//!     &authority,
//!     6,
//!     &[MintExtension::MintCloseAuthority {
//!         close_authority: authority.pubkey(),
//!     }],
//! ).unwrap();
//! ```

use crate::error::SolanaKiteError;
use litesvm::LiteSVM;
use solana_instruction::account_meta::AccountMeta;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_message::Message;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::Transaction;

// ─── Program IDs ─────────────────────────────────────────────────────────────

/// The Token-2022 program ID.
pub const TOKEN_2022_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

/// The Associated Token Account program ID (shared with Token-2022).
const ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

/// The System program ID.
const SYSTEM_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("11111111111111111111111111111111");

// ─── Account Layout Constants ────────────────────────────────────────────────

/// Base size for Account (not Mint). The Token-2022 extension framework pads
/// all account types (including Mints) to this size before appending extensions.
const BASE_ACCOUNT_LENGTH: usize = 165;

/// Size of the AccountType discriminator byte that separates base data from TLV.
const ACCOUNT_TYPE_SIZE: usize = 1;

/// Minimum size for a mint with extensions: base (165) + account type (1) + TLV data.
/// The Mint data (82 bytes) is followed by 83 bytes of zero-padding up to 165.
const BASE_ACCOUNT_AND_TYPE_LENGTH: usize = BASE_ACCOUNT_LENGTH + ACCOUNT_TYPE_SIZE;

/// Size overhead per TLV extension entry: 2 bytes for type + 2 bytes for length.
const TLV_HEADER_SIZE: usize = 4;

// ─── Extension Types ─────────────────────────────────────────────────────────

/// Extensions that can be applied to a Token-2022 mint at creation time.
///
/// Each variant corresponds to a Token-2022 extension that modifies mint behaviour.
/// Extensions are initialized *before* the mint itself, following the Token-2022
/// protocol requirement.
#[derive(Debug, Clone)]
pub enum MintExtension {
    /// Attach a transfer hook program that executes on every token transfer.
    TransferHook {
        /// The program ID that implements the transfer hook interface.
        program_id: Pubkey,
    },
    /// Automatically collect fees on token transfers.
    TransferFee {
        /// Fee in basis points (1 bp = 0.01%). Max 10000 (100%).
        fee_basis_points: u16,
        /// Maximum fee in token base units (caps the percentage-based fee).
        maximum_fee: u64,
    },
    /// Allow the mint account to be closed (reclaiming rent).
    MintCloseAuthority {
        /// The authority that can close the mint.
        close_authority: Pubkey,
    },
    /// Set an irrevocable delegate for all token accounts of this mint.
    PermanentDelegate {
        /// The permanent delegate address.
        delegate: Pubkey,
    },
    /// Make tokens non-transferable (soulbound).
    NonTransferable,
    /// Set the default state for newly created token accounts.
    DefaultAccountState {
        /// 0 = Uninitialized, 1 = Initialized, 2 = Frozen.
        state: u8,
    },
    /// Display an interest rate on token balances (cosmetic, no actual accrual).
    InterestBearing {
        /// Authority that can update the rate.
        rate_authority: Pubkey,
        /// Interest rate in basis points.
        rate: i16,
    },
    /// Point to an on-chain metadata account.
    MetadataPointer {
        /// Authority that can update the pointer.
        authority: Pubkey,
        /// Address of the metadata account.
        metadata_address: Pubkey,
    },
}

impl MintExtension {
    /// Returns the on-chain data size of this extension (not counting the 4-byte TLV header).
    /// These sizes match the Pod structs in spl-token-2022-interface exactly.
    fn data_size(&self) -> usize {
        match self {
            // TransferHook: authority(32) + program_id(32)
            MintExtension::TransferHook { .. } => 64,
            // TransferFeeConfig: config_authority(32) + withdraw_authority(32) + withheld(8)
            //   + older_fee(epoch:8 + max:8 + bps:2) + newer_fee(epoch:8 + max:8 + bps:2) = 108
            MintExtension::TransferFee { .. } => 108,
            // MintCloseAuthority: close_authority(32)
            MintExtension::MintCloseAuthority { .. } => 32,
            // PermanentDelegate: delegate(32)
            MintExtension::PermanentDelegate { .. } => 32,
            // NonTransferable: zero-sized marker
            MintExtension::NonTransferable => 0,
            // DefaultAccountState: state(1)
            MintExtension::DefaultAccountState { .. } => 1,
            // InterestBearingConfig: authority(32) + init_ts(8) + pre_avg_rate(2) + last_ts(8) + rate(2) = 52
            MintExtension::InterestBearing { .. } => 52,
            // MetadataPointer: authority(32) + metadata_address(32)
            MintExtension::MetadataPointer { .. } => 64,
        }
    }

    /// Returns the total TLV entry size (header + data).
    fn tlv_size(&self) -> usize {
        TLV_HEADER_SIZE + self.data_size()
    }

    /// Builds the initialization instruction for this extension.
    ///
    /// Instruction formats are matched exactly to the spl-token-2022-interface source.
    fn build_init_instruction(&self, mint: &Pubkey, mint_authority: &Pubkey) -> Instruction {
        match self {
            MintExtension::TransferHook { program_id } => {
                // TransferHookExtension (prefix=36) + Initialize (sub=0) + data
                // Data: authority(32) + program_id(32)
                // OptionalNonZeroPubkey: the raw 32 bytes (zero = None)
                let mut data = Vec::with_capacity(2 + 64);
                data.push(36); // TransferHookExtension
                data.push(0);  // Initialize sub-instruction
                data.extend_from_slice(&mint_authority.to_bytes()); // authority
                data.extend_from_slice(&program_id.to_bytes());     // program_id
                Instruction {
                    program_id: TOKEN_2022_PROGRAM_ID,
                    accounts: vec![AccountMeta::new(*mint, false)],
                    data,
                }
            }
            MintExtension::TransferFee {
                fee_basis_points,
                maximum_fee,
            } => {
                // TransferFeeExtension (prefix=26) + InitializeTransferFeeConfig (sub=0) + data
                // Data: config_authority(COption<Pubkey>) + withdraw_authority(COption<Pubkey>)
                //       + fee_basis_points(u16) + maximum_fee(u64)
                // COption<Pubkey>: [0] for None, [1, pubkey(32)] for Some
                let mut data = Vec::with_capacity(2 + 1 + 32 + 1 + 32 + 2 + 8);
                data.push(26); // TransferFeeExtension
                data.push(0);  // InitializeTransferFeeConfig sub-instruction
                // config_authority: Some(mint_authority)
                data.push(1);
                data.extend_from_slice(&mint_authority.to_bytes());
                // withdraw_authority: Some(mint_authority)
                data.push(1);
                data.extend_from_slice(&mint_authority.to_bytes());
                data.extend_from_slice(&fee_basis_points.to_le_bytes());
                data.extend_from_slice(&maximum_fee.to_le_bytes());
                Instruction {
                    program_id: TOKEN_2022_PROGRAM_ID,
                    accounts: vec![AccountMeta::new(*mint, false)],
                    data,
                }
            }
            MintExtension::MintCloseAuthority { close_authority } => {
                // InitializeMintCloseAuthority (instruction 25) + COption<Pubkey>
                // COption<Pubkey>: [1, pubkey(32)] for Some
                let mut data = Vec::with_capacity(1 + 1 + 32);
                data.push(25); // InitializeMintCloseAuthority
                data.push(1);  // COption::Some
                data.extend_from_slice(&close_authority.to_bytes());
                Instruction {
                    program_id: TOKEN_2022_PROGRAM_ID,
                    accounts: vec![AccountMeta::new(*mint, false)],
                    data,
                }
            }
            MintExtension::PermanentDelegate { delegate } => {
                // InitializePermanentDelegate (instruction 35) + delegate(32)
                let mut data = Vec::with_capacity(1 + 32);
                data.push(35); // InitializePermanentDelegate
                data.extend_from_slice(&delegate.to_bytes());
                Instruction {
                    program_id: TOKEN_2022_PROGRAM_ID,
                    accounts: vec![AccountMeta::new(*mint, false)],
                    data,
                }
            }
            MintExtension::NonTransferable => {
                // InitializeNonTransferableMint (instruction 32)
                Instruction {
                    program_id: TOKEN_2022_PROGRAM_ID,
                    accounts: vec![AccountMeta::new(*mint, false)],
                    data: vec![32],
                }
            }
            MintExtension::DefaultAccountState { state } => {
                // DefaultAccountStateExtension (prefix=28) + Initialize (sub=0) + state(1)
                Instruction {
                    program_id: TOKEN_2022_PROGRAM_ID,
                    accounts: vec![AccountMeta::new(*mint, false)],
                    data: vec![28, 0, *state],
                }
            }
            MintExtension::InterestBearing {
                rate_authority,
                rate,
            } => {
                // InterestBearingMintExtension (prefix=33) + Initialize (sub=0) + data
                // Data: rate_authority(32) + rate(i16 as 2 LE bytes)
                // Uses encode_instruction which puts: [prefix, sub, pod_data...]
                let mut data = Vec::with_capacity(2 + 32 + 2);
                data.push(33); // InterestBearingMintExtension
                data.push(0);  // Initialize sub-instruction
                data.extend_from_slice(&rate_authority.to_bytes()); // OptionalNonZeroPubkey
                data.extend_from_slice(&rate.to_le_bytes());
                Instruction {
                    program_id: TOKEN_2022_PROGRAM_ID,
                    accounts: vec![AccountMeta::new(*mint, false)],
                    data,
                }
            }
            MintExtension::MetadataPointer {
                authority,
                metadata_address,
            } => {
                // MetadataPointerExtension (prefix=39) + Initialize (sub=0) + data
                // Data: authority(32) + metadata_address(32) — OptionalNonZeroPubkey
                let mut data = Vec::with_capacity(2 + 64);
                data.push(39); // MetadataPointerExtension
                data.push(0);  // Initialize sub-instruction
                data.extend_from_slice(&authority.to_bytes());
                data.extend_from_slice(&metadata_address.to_bytes());
                Instruction {
                    program_id: TOKEN_2022_PROGRAM_ID,
                    accounts: vec![AccountMeta::new(*mint, false)],
                    data,
                }
            }
        }
    }
}

// ─── Mint Account Size ───────────────────────────────────────────────────────

fn calculate_mint_size(extensions: &[MintExtension]) -> usize {
    if extensions.is_empty() {
        // Without extensions, a Token-2022 mint is the same size as SPL Token
        return 82;
    }
    let extension_data_size: usize = extensions.iter().map(|ext| ext.tlv_size()).sum();
    BASE_ACCOUNT_AND_TYPE_LENGTH + extension_data_size
}

// ─── Public API ──────────────────────────────────────────────────────────────

/// Creates a new Token-2022 mint with the specified extensions.
///
/// Extensions are initialized before the mint itself, as required by the
/// Token-2022 program. The `mint_authority` is both the payer and the
/// mint authority for the new mint.
///
/// # Arguments
///
/// * `litesvm` - Mutable reference to the LiteSVM instance
/// * `mint_authority` - Keypair that pays for creation and becomes the mint authority
/// * `decimals` - Number of decimal places for the token (0–9)
/// * `extensions` - Slice of extensions to enable on this mint
///
/// # Returns
///
/// The public key of the newly created Token-2022 mint.
///
/// # Example
///
/// ```rust
/// use solana_kite::token_2022::{create_token_2022_mint, MintExtension};
/// use solana_kite::create_wallet;
/// use litesvm::LiteSVM;
/// use solana_pubkey::Pubkey;
/// use solana_signer::Signer;
///
/// let mut litesvm = LiteSVM::new();
/// let authority = create_wallet(&mut litesvm, 1_000_000_000).unwrap();
/// let hook_program = Pubkey::new_unique();
///
/// let mint = create_token_2022_mint(
///     &mut litesvm,
///     &authority,
///     6,
///     &[
///         MintExtension::TransferHook { program_id: hook_program },
///         MintExtension::MintCloseAuthority { close_authority: authority.pubkey() },
///     ],
/// ).unwrap();
/// ```
pub fn create_token_2022_mint(
    litesvm: &mut LiteSVM,
    mint_authority: &Keypair,
    decimals: u8,
    extensions: &[MintExtension],
) -> Result<Pubkey, SolanaKiteError> {
    let mint = Pubkey::new_unique();
    let mint_size = calculate_mint_size(extensions);
    let rent = litesvm.minimum_balance_for_rent_exemption(mint_size);

    // Pre-allocate the account owned by Token-2022
    litesvm
        .set_account(
            mint,
            solana_account::Account {
                lamports: rent,
                data: vec![0u8; mint_size],
                owner: TOKEN_2022_PROGRAM_ID,
                executable: false,
                rent_epoch: 0,
            },
        )
        .map_err(|e| {
            SolanaKiteError::TokenOperationFailed(format!(
                "Failed to create Token-2022 mint account: {:?}",
                e
            ))
        })?;

    // Build all instructions: extension inits first, then InitializeMint2
    let mut instructions: Vec<Instruction> = extensions
        .iter()
        .map(|ext| ext.build_init_instruction(&mint, &mint_authority.pubkey()))
        .collect();

    // InitializeMint2 (instruction 20): [20, decimals, mint_authority(32), freeze_option]
    // COption<Pubkey>: [0] for None
    let mut init_mint_data = Vec::with_capacity(1 + 1 + 32 + 1);
    init_mint_data.push(20); // InitializeMint2
    init_mint_data.push(decimals);
    init_mint_data.extend_from_slice(&mint_authority.pubkey().to_bytes());
    init_mint_data.push(0); // freeze_authority: None

    instructions.push(Instruction {
        program_id: TOKEN_2022_PROGRAM_ID,
        accounts: vec![AccountMeta::new(mint, false)],
        data: init_mint_data,
    });

    let message = Message::new(&instructions, Some(&mint_authority.pubkey()));
    let mut transaction = Transaction::new_unsigned(message);
    let blockhash = litesvm.latest_blockhash();
    transaction.sign(&[mint_authority], blockhash);

    litesvm.send_transaction(transaction).map_err(|e| {
        SolanaKiteError::TokenOperationFailed(format!(
            "Failed to initialize Token-2022 mint: {:?}",
            e
        ))
    })?;

    Ok(mint)
}

/// Creates an associated token account for a Token-2022 mint.
///
/// Derives the ATA address from the owner and mint, then creates it using
/// the Associated Token Account program (which supports both Token and Token-2022).
///
/// # Arguments
///
/// * `litesvm` - Mutable reference to the LiteSVM instance
/// * `owner` - Public key of the account that will own the token account
/// * `mint` - Public key of the Token-2022 mint
/// * `payer` - Keypair that pays for account creation
///
/// # Returns
///
/// The public key of the created associated token account.
pub fn create_token_2022_account(
    litesvm: &mut LiteSVM,
    owner: &Pubkey,
    mint: &Pubkey,
    payer: &Keypair,
) -> Result<Pubkey, SolanaKiteError> {
    let associated_token_account = get_associated_token_address_2022(owner, mint);

    // CreateAssociatedTokenAccount: the ATA program derives the instruction type
    // from the account count / layout — no explicit discriminator byte needed.
    let instruction = Instruction {
        program_id: ASSOCIATED_TOKEN_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(payer.pubkey(), true),                  // funding account (signer)
            AccountMeta::new(associated_token_account, false),       // ATA to create
            AccountMeta::new_readonly(*owner, false),                // wallet address
            AccountMeta::new_readonly(*mint, false),                 // token mint
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),     // system program
            AccountMeta::new_readonly(TOKEN_2022_PROGRAM_ID, false), // token program
        ],
        data: vec![], // CreateAssociatedTokenAccount has no data
    };

    let message = Message::new(&[instruction], Some(&payer.pubkey()));
    let mut transaction = Transaction::new_unsigned(message);
    let blockhash = litesvm.latest_blockhash();
    transaction.sign(&[payer], blockhash);

    litesvm.send_transaction(transaction).map_err(|e| {
        SolanaKiteError::TokenOperationFailed(format!(
            "Failed to create Token-2022 associated token account: {:?}",
            e
        ))
    })?;

    Ok(associated_token_account)
}

/// Mints tokens to a Token-2022 token account.
///
/// # Arguments
///
/// * `litesvm` - Mutable reference to the LiteSVM instance
/// * `mint` - Public key of the Token-2022 mint
/// * `token_account` - Public key of the destination token account
/// * `amount` - Number of tokens to mint (in base units)
/// * `mint_authority` - Keypair with mint authority
pub fn mint_tokens_to_account_2022(
    litesvm: &mut LiteSVM,
    mint: &Pubkey,
    token_account: &Pubkey,
    amount: u64,
    mint_authority: &Keypair,
) -> Result<(), SolanaKiteError> {
    // MintTo instruction (7): [7, amount(8)]
    let mut data = vec![7u8];
    data.extend_from_slice(&amount.to_le_bytes());

    let instruction = Instruction {
        program_id: TOKEN_2022_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*mint, false),
            AccountMeta::new(*token_account, false),
            AccountMeta::new_readonly(mint_authority.pubkey(), true),
        ],
        data,
    };

    let message = Message::new(&[instruction], Some(&mint_authority.pubkey()));
    let mut transaction = Transaction::new_unsigned(message);
    let blockhash = litesvm.latest_blockhash();
    transaction.sign(&[mint_authority], blockhash);

    litesvm.send_transaction(transaction).map_err(|e| {
        SolanaKiteError::TokenOperationFailed(format!(
            "Failed to mint Token-2022 tokens: {:?}",
            e
        ))
    })?;

    Ok(())
}

/// Transfers tokens between Token-2022 accounts using TransferChecked.
///
/// TransferChecked is required for Token-2022 tokens (especially those with
/// transfer hooks, transfer fees, or other extensions that need to inspect
/// the transfer).
///
/// # Arguments
///
/// * `litesvm` - Mutable reference to the LiteSVM instance
/// * `source` - Source token account
/// * `mint` - Token mint address
/// * `destination` - Destination token account
/// * `authority` - Transfer authority keypair
/// * `amount` - Amount to transfer (in base units)
/// * `decimals` - Mint decimals (must match the mint's configured decimals)
/// * `extra_accounts` - Additional account metas required by transfer hooks
pub fn transfer_checked_token_2022(
    litesvm: &mut LiteSVM,
    source: &Pubkey,
    mint: &Pubkey,
    destination: &Pubkey,
    authority: &Keypair,
    amount: u64,
    decimals: u8,
    extra_accounts: &[AccountMeta],
) -> Result<(), SolanaKiteError> {
    // TransferChecked instruction (12): [12, amount(8), decimals(1)]
    let mut data = vec![12u8];
    data.extend_from_slice(&amount.to_le_bytes());
    data.push(decimals);

    let mut accounts = vec![
        AccountMeta::new(*source, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new(*destination, false),
        AccountMeta::new_readonly(authority.pubkey(), true),
    ];
    accounts.extend_from_slice(extra_accounts);

    let instruction = Instruction {
        program_id: TOKEN_2022_PROGRAM_ID,
        accounts,
        data,
    };

    let message = Message::new(&[instruction], Some(&authority.pubkey()));
    let mut transaction = Transaction::new_unsigned(message);
    let blockhash = litesvm.latest_blockhash();
    transaction.sign(&[authority], blockhash);

    litesvm.send_transaction(transaction).map_err(|e| {
        SolanaKiteError::TokenOperationFailed(format!(
            "Failed to transfer Token-2022 tokens: {:?}",
            e
        ))
    })?;

    Ok(())
}

// ─── Utility Functions ───────────────────────────────────────────────────────

/// Derives the associated token address for a Token-2022 mint.
///
/// Uses the same derivation as the standard ATA program, but with the
/// Token-2022 program ID.
pub fn get_associated_token_address_2022(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    let (address, _bump) = Pubkey::find_program_address(
        &[
            owner.as_ref(),
            TOKEN_2022_PROGRAM_ID.as_ref(),
            mint.as_ref(),
        ],
        &ASSOCIATED_TOKEN_PROGRAM_ID,
    );
    address
}

/// Gets the token balance of a Token-2022 token account.
///
/// The account layout is the same as SPL Token for the base fields:
/// amount is at bytes 64..72 (u64, little-endian).
pub fn get_token_2022_balance(
    litesvm: &LiteSVM,
    token_account: &Pubkey,
) -> Result<u64, SolanaKiteError> {
    let account = litesvm.get_account(token_account).ok_or_else(|| {
        SolanaKiteError::TokenOperationFailed("Token-2022 account not found".to_string())
    })?;

    let data = &account.data;
    if data.len() < 72 {
        return Err(SolanaKiteError::TokenOperationFailed(
            "Invalid Token-2022 account data length".to_string(),
        ));
    }

    // SPL Token account layout: amount is at bytes 64..72
    let amount = u64::from_le_bytes(
        data[64..72].try_into().map_err(|_| {
            SolanaKiteError::TokenOperationFailed(
                "Failed to parse Token-2022 token amount".to_string(),
            )
        })?,
    );

    Ok(amount)
}

/// Asserts that a Token-2022 token account has the expected balance.
///
/// Convenience wrapper around [`get_token_2022_balance`] for test assertions.
pub fn assert_token_2022_balance(
    litesvm: &LiteSVM,
    token_account: &Pubkey,
    expected_balance: u64,
    message: &str,
) {
    let actual_balance =
        get_token_2022_balance(litesvm, token_account).expect("Failed to get Token-2022 balance");
    assert_eq!(actual_balance, expected_balance, "{}", message);
}
