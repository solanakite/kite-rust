//! Token Extensions operations for Solana.
//!
//! This module provides helpers for creating and interacting with Token Extensions mints
//! and accounts. All instructions are built from raw bytes to avoid depending on
//! the `spl-token-extensions` crate, which can cause version conflicts with `anchor-lang`.
//!
//! # Supported Extensions
//!
//! - **TransferHook** — attach a program that runs on every transfer
//! - **TransferFee** — automatic fee collection on transfers
//! - **MintCloseAuthority** — allow closing a mint account
//! - **PermanentDelegate** — irrevocable delegate for all token accounts
//! - **NonTransferable** — soulbound tokens that cannot be transferred
//! - **DefaultAccountState** — configure the default state for newly created token accounts
//! - **InterestBearing** — onchain interest rate display
//! - **MetadataPointer** — point to an onchain metadata account
//!
//! # Example
//!
//! ```rust
//! use solana_kite::token_extensions::{create_token_extensions_mint, MintExtension};
//! use solana_kite::create_wallet;
//! use litesvm::LiteSVM;
//! use solana_pubkey::Pubkey;
//! use solana_signer::Signer;
//!
//! let mut litesvm = LiteSVM::new();
//! let authority = create_wallet(&mut litesvm, 1_000_000_000).unwrap();
//!
//! let mint = create_token_extensions_mint(
//!     &mut litesvm,
//!     &authority,
//!     6,
//!     None,
//!     &[MintExtension::MintCloseAuthority {
//!         close_authority: authority.pubkey(),
//!     }],
//! ).unwrap();
//! ```

use crate::constants::{SPL_TOKEN_MINT_SIZE, SYSTEM_PROGRAM_ID};
use crate::error::SolanaKiteError;
use crate::transaction::send_transaction_from_instructions;
use litesvm::LiteSVM;
use solana_instruction::account_meta::AccountMeta;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

/// The Token Extensions program ID.
pub const TOKEN_EXTENSIONS_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

/// The Associated Token Account program ID (shared with Token Extensions).
const ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

/// The size all Token Extensions account types are padded to before extension data is appended.
/// This equals the Token Account layout size (165 bytes); Mint data (82 bytes) is zero-padded
/// to reach this size.
const TOKEN_EXTENSIONS_BASE_SIZE: usize = 165;

/// Size of the AccountType discriminator byte that separates base data from TLV.
const ACCOUNT_TYPE_SIZE: usize = 1;

/// Size overhead per TLV extension entry: 2 bytes for type + 2 bytes for length.
const TLV_HEADER_SIZE: usize = 4;

// Instruction discriminators for the Token Extensions program.
// Source: https://github.com/solana-program/token-extensions/blob/main/interface/src/instruction.rs
const INSTRUCTION_INITIALIZE_MINT2: u8 = 20;
const INSTRUCTION_MINT_TO: u8 = 7;
const INSTRUCTION_TRANSFER_CHECKED: u8 = 12;
const INSTRUCTION_INITIALIZE_NON_TRANSFERABLE_MINT: u8 = 32;
const INSTRUCTION_TRANSFER_HOOK: u8 = 36;
const INSTRUCTION_TRANSFER_FEE: u8 = 26;
const INSTRUCTION_MINT_CLOSE_AUTHORITY: u8 = 25;
const INSTRUCTION_PERMANENT_DELEGATE: u8 = 35;
const INSTRUCTION_DEFAULT_ACCOUNT_STATE: u8 = 28;
const INSTRUCTION_INTEREST_BEARING_MINT: u8 = 33;
const INSTRUCTION_METADATA_POINTER: u8 = 39;

/// The state a newly created token account will start in when a mint has the
/// `DefaultAccountState` extension.
#[derive(Debug, Clone, Copy)]
pub enum TokenAccountState {
    /// Account exists but is not yet initialised. Rarely used intentionally.
    Uninitialized = 0,
    /// Account is active and can send/receive tokens.
    Initialized = 1,
    /// Account is frozen and cannot send or receive tokens until thawed by the freeze authority.
    Frozen = 2,
}

/// Extensions that can be applied to a Token Extensions mint at creation time.
///
/// Each variant corresponds to a Token Extensions extension that modifies mint behaviour.
/// Extensions are initialized *before* the mint itself, following the Token Extensions
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
        /// The state all new token accounts for this mint will start in.
        initial_state: TokenAccountState,
    },
    /// Display an interest rate on token balances (cosmetic, no actual accrual).
    InterestBearing {
        /// Authority that can update the rate.
        rate_authority: Pubkey,
        /// Interest rate in basis points.
        rate: i16,
    },
    /// Point to an onchain metadata account.
    MetadataPointer {
        /// Authority that can update the pointer.
        authority: Pubkey,
        /// Address of the metadata account.
        metadata_address: Pubkey,
    },
}

impl MintExtension {
    /// Returns the onchain data size of this extension (not counting the 4-byte TLV header).
    /// These sizes match the Pod structs in spl-token-extensions-interface exactly.
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
    /// Instruction formats are matched exactly to the spl-token-extensions-interface source.
    fn build_init_instruction(&self, mint: &Pubkey, mint_authority: &Pubkey) -> Instruction {
        match self {
            MintExtension::TransferHook { program_id } => {
                // Initialize (sub=0) + data:
                // authority(32) + program_id(32) as OptionalNonZeroPubkey (zero = None)
                let mut data = Vec::with_capacity(2 + 64);
                data.push(INSTRUCTION_TRANSFER_HOOK);
                data.push(0); // Initialize sub-instruction
                data.extend_from_slice(&mint_authority.to_bytes()); // authority
                data.extend_from_slice(&program_id.to_bytes()); // program_id
                Instruction {
                    program_id: TOKEN_EXTENSIONS_PROGRAM_ID,
                    accounts: vec![AccountMeta::new(*mint, false)],
                    data,
                }
            }
            MintExtension::TransferFee {
                fee_basis_points,
                maximum_fee,
            } => {
                // InitializeTransferFeeConfig (sub=0) + data:
                // config_authority(COption<Pubkey>) + withdraw_authority(COption<Pubkey>)
                // + fee_basis_points(u16) + maximum_fee(u64)
                // COption<Pubkey>: [0] for None, [1, pubkey(32)] for Some
                let mut data = Vec::with_capacity(2 + 1 + 32 + 1 + 32 + 2 + 8);
                data.push(INSTRUCTION_TRANSFER_FEE);
                data.push(0); // InitializeTransferFeeConfig sub-instruction
                              // config_authority: Some(mint_authority)
                data.push(1);
                data.extend_from_slice(&mint_authority.to_bytes());
                // withdraw_authority: Some(mint_authority)
                data.push(1);
                data.extend_from_slice(&mint_authority.to_bytes());
                data.extend_from_slice(&fee_basis_points.to_le_bytes());
                data.extend_from_slice(&maximum_fee.to_le_bytes());
                Instruction {
                    program_id: TOKEN_EXTENSIONS_PROGRAM_ID,
                    accounts: vec![AccountMeta::new(*mint, false)],
                    data,
                }
            }
            MintExtension::MintCloseAuthority { close_authority } => {
                // COption<Pubkey>: [1, pubkey(32)] for Some
                let mut data = Vec::with_capacity(1 + 1 + 32);
                data.push(INSTRUCTION_MINT_CLOSE_AUTHORITY);
                data.push(1); // COption::Some
                data.extend_from_slice(&close_authority.to_bytes());
                Instruction {
                    program_id: TOKEN_EXTENSIONS_PROGRAM_ID,
                    accounts: vec![AccountMeta::new(*mint, false)],
                    data,
                }
            }
            MintExtension::PermanentDelegate { delegate } => {
                let mut data = Vec::with_capacity(1 + 32);
                data.push(INSTRUCTION_PERMANENT_DELEGATE);
                data.extend_from_slice(&delegate.to_bytes());
                Instruction {
                    program_id: TOKEN_EXTENSIONS_PROGRAM_ID,
                    accounts: vec![AccountMeta::new(*mint, false)],
                    data,
                }
            }
            MintExtension::NonTransferable => Instruction {
                program_id: TOKEN_EXTENSIONS_PROGRAM_ID,
                accounts: vec![AccountMeta::new(*mint, false)],
                data: vec![INSTRUCTION_INITIALIZE_NON_TRANSFERABLE_MINT],
            },
            MintExtension::DefaultAccountState { initial_state } => {
                // Initialize (sub=0) + state(1)
                Instruction {
                    program_id: TOKEN_EXTENSIONS_PROGRAM_ID,
                    accounts: vec![AccountMeta::new(*mint, false)],
                    data: vec![INSTRUCTION_DEFAULT_ACCOUNT_STATE, 0, *initial_state as u8],
                }
            }
            MintExtension::InterestBearing {
                rate_authority,
                rate,
            } => {
                // Initialize (sub=0) + data: rate_authority(32) + rate(i16 LE)
                let mut data = Vec::with_capacity(2 + 32 + 2);
                data.push(INSTRUCTION_INTEREST_BEARING_MINT);
                data.push(0); // Initialize sub-instruction
                data.extend_from_slice(&rate_authority.to_bytes()); // OptionalNonZeroPubkey
                data.extend_from_slice(&rate.to_le_bytes());
                Instruction {
                    program_id: TOKEN_EXTENSIONS_PROGRAM_ID,
                    accounts: vec![AccountMeta::new(*mint, false)],
                    data,
                }
            }
            MintExtension::MetadataPointer {
                authority,
                metadata_address,
            } => {
                // Initialize (sub=0) + data:
                // authority(32) + metadata_address(32) as OptionalNonZeroPubkey
                let mut data = Vec::with_capacity(2 + 64);
                data.push(INSTRUCTION_METADATA_POINTER);
                data.push(0); // Initialize sub-instruction
                data.extend_from_slice(&authority.to_bytes());
                data.extend_from_slice(&metadata_address.to_bytes());
                Instruction {
                    program_id: TOKEN_EXTENSIONS_PROGRAM_ID,
                    accounts: vec![AccountMeta::new(*mint, false)],
                    data,
                }
            }
        }
    }
}

fn calculate_mint_size(extensions: &[MintExtension]) -> usize {
    if extensions.is_empty() {
        // Without extensions, a Token Extensions mint is the same size as SPL Token
        return SPL_TOKEN_MINT_SIZE;
    }
    let extension_data_size: usize = extensions.iter().map(|ext| ext.tlv_size()).sum();
    TOKEN_EXTENSIONS_BASE_SIZE + ACCOUNT_TYPE_SIZE + extension_data_size
}

/// Creates a new Token Extensions mint with the specified extensions.
///
/// Extensions are initialized before the mint itself, as required by the
/// Token Extensions program. The `mint_authority` is both the payer and the
/// mint authority for the new mint.
///
/// # Arguments
///
/// * `litesvm` - Mutable reference to the LiteSVM instance
/// * `mint_authority` - Keypair that pays for creation and becomes the mint authority
/// * `decimals` - Number of decimal places for the token (0–9)
/// * `mint` - Optional custom public key for the mint. If `None`, a unique address will be generated
/// * `extensions` - Slice of extensions to enable on this mint
///
/// # Returns
///
/// The public key of the newly created Token Extensions mint.
///
/// # Errors
///
/// Returns an error if the mint account cannot be created or the initialization transaction fails.
///
/// # Note
///
/// This function does not set a freeze authority. Using
/// `MintExtension::DefaultAccountState { initial_state: TokenAccountState::Frozen }` will cause
/// transfers to fail at runtime because there is no freeze authority to thaw accounts.
///
/// # Example
///
/// ```rust
/// use solana_kite::token_extensions::{create_token_extensions_mint, MintExtension};
/// use solana_kite::create_wallet;
/// use litesvm::LiteSVM;
/// use solana_pubkey::Pubkey;
/// use solana_signer::Signer;
///
/// let mut litesvm = LiteSVM::new();
/// let authority = create_wallet(&mut litesvm, 1_000_000_000).unwrap();
/// let hook_program = Pubkey::new_unique();
///
/// let mint = create_token_extensions_mint(
///     &mut litesvm,
///     &authority,
///     6,
///     None,
///     &[
///         MintExtension::TransferHook { program_id: hook_program },
///         MintExtension::MintCloseAuthority { close_authority: authority.pubkey() },
///     ],
/// ).unwrap();
/// ```
pub fn create_token_extensions_mint(
    litesvm: &mut LiteSVM,
    mint_authority: &Keypair,
    decimals: u8,
    mint: Option<Pubkey>,
    extensions: &[MintExtension],
) -> Result<Pubkey, SolanaKiteError> {
    let mint = mint.unwrap_or(Pubkey::new_unique());
    let mint_size = calculate_mint_size(extensions);
    let rent = litesvm.minimum_balance_for_rent_exemption(mint_size);

    // Pre-allocate the account owned by Token Extensions
    litesvm
        .set_account(
            mint,
            solana_account::Account {
                lamports: rent,
                data: vec![0u8; mint_size],
                owner: TOKEN_EXTENSIONS_PROGRAM_ID,
                executable: false,
                rent_epoch: 0,
            },
        )
        .map_err(|e| {
            SolanaKiteError::TokenOperationFailed(format!(
                "Failed to create Token Extensions mint account: {}",
                e
            ))
        })?;

    // Build all instructions: extension inits first, then InitializeMint2
    let mut instructions: Vec<Instruction> = extensions
        .iter()
        .map(|ext| ext.build_init_instruction(&mint, &mint_authority.pubkey()))
        .collect();

    // [discriminator, decimals, mint_authority(32), freeze_authority COption: 0=None]
    let mut init_mint_data = Vec::with_capacity(1 + 1 + 32 + 1);
    init_mint_data.push(INSTRUCTION_INITIALIZE_MINT2);
    init_mint_data.push(decimals);
    init_mint_data.extend_from_slice(&mint_authority.pubkey().to_bytes());
    init_mint_data.push(0); // freeze_authority: None

    instructions.push(Instruction {
        program_id: TOKEN_EXTENSIONS_PROGRAM_ID,
        accounts: vec![AccountMeta::new(mint, false)],
        data: init_mint_data,
    });

    send_transaction_from_instructions(
        litesvm,
        instructions,
        &[mint_authority],
        &mint_authority.pubkey(),
    )?;

    Ok(mint)
}

/// Creates an associated token account for a Token Extensions mint.
///
/// Derives the ATA address from the owner and mint, then creates it using
/// the Associated Token Account program (which supports both Token and Token Extensions).
///
/// # Arguments
///
/// * `litesvm` - Mutable reference to the LiteSVM instance
/// * `owner` - Public key of the account that will own the token account
/// * `mint` - Public key of the Token Extensions mint
/// * `payer` - Keypair that pays for account creation
///
/// # Returns
///
/// The public key of the created associated token account.
///
/// # Errors
///
/// Returns an error if the account creation transaction fails.
///
/// # Example
///
/// ```rust
/// use solana_kite::token_extensions::{create_token_extensions_mint, create_token_extensions_account};
/// use solana_kite::create_wallet;
/// use litesvm::LiteSVM;
/// use solana_signer::Signer;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut litesvm = LiteSVM::new();
/// let authority = create_wallet(&mut litesvm, 1_000_000_000)?;
/// let mint = create_token_extensions_mint(&mut litesvm, &authority, 6, None, &[])?;
/// let ata = create_token_extensions_account(&mut litesvm, &authority.pubkey(), &mint, &authority)?;
/// # Ok(())
/// # }
/// ```
pub fn create_token_extensions_account(
    litesvm: &mut LiteSVM,
    owner: &Pubkey,
    mint: &Pubkey,
    payer: &Keypair,
) -> Result<Pubkey, SolanaKiteError> {
    let associated_token_account = get_token_extensions_account_address(owner, mint);

    // CreateAssociatedTokenAccount: the ATA program derives the instruction type
    // from the account count / layout — no explicit discriminator byte needed.
    let instruction = Instruction {
        program_id: ASSOCIATED_TOKEN_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(payer.pubkey(), true), // funding account (signer)
            AccountMeta::new(associated_token_account, false), // ATA to create
            AccountMeta::new_readonly(*owner, false), // wallet address
            AccountMeta::new_readonly(*mint, false), // token mint
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false), // system program
            AccountMeta::new_readonly(TOKEN_EXTENSIONS_PROGRAM_ID, false), // token program
        ],
        data: Vec::new(), // CreateAssociatedTokenAccount has no data
    };

    send_transaction_from_instructions(litesvm, vec![instruction], &[payer], &payer.pubkey())?;

    Ok(associated_token_account)
}

/// Mints tokens to a Token Extensions token account.
///
/// # Arguments
///
/// * `litesvm` - Mutable reference to the LiteSVM instance
/// * `mint` - Public key of the Token Extensions mint
/// * `token_account` - Public key of the destination token account
/// * `amount` - Number of tokens to mint (in base units)
/// * `mint_authority` - Keypair with mint authority
///
/// # Errors
///
/// Returns an error if the minting transaction fails.
///
/// # Example
///
/// ```rust
/// use solana_kite::token_extensions::{
///     create_token_extensions_mint, create_token_extensions_account,
///     mint_tokens_to_token_extensions_account,
/// };
/// use solana_kite::create_wallet;
/// use litesvm::LiteSVM;
/// use solana_signer::Signer;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut litesvm = LiteSVM::new();
/// let authority = create_wallet(&mut litesvm, 1_000_000_000)?;
/// let mint = create_token_extensions_mint(&mut litesvm, &authority, 6, None, &[])?;
/// let ata = create_token_extensions_account(&mut litesvm, &authority.pubkey(), &mint, &authority)?;
/// mint_tokens_to_token_extensions_account(&mut litesvm, &mint, &ata, 1_000_000, &authority)?;
/// # Ok(())
/// # }
/// ```
pub fn mint_tokens_to_token_extensions_account(
    litesvm: &mut LiteSVM,
    mint: &Pubkey,
    token_account: &Pubkey,
    amount: u64,
    mint_authority: &Keypair,
) -> Result<(), SolanaKiteError> {
    let mut data = vec![INSTRUCTION_MINT_TO];
    data.extend_from_slice(&amount.to_le_bytes());

    let instruction = Instruction {
        program_id: TOKEN_EXTENSIONS_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*mint, false),
            AccountMeta::new(*token_account, false),
            AccountMeta::new_readonly(mint_authority.pubkey(), true),
        ],
        data,
    };

    send_transaction_from_instructions(
        litesvm,
        vec![instruction],
        &[mint_authority],
        &mint_authority.pubkey(),
    )?;

    Ok(())
}

/// Transfers tokens between Token Extensions accounts using TransferChecked.
///
/// TransferChecked is required for Token Extensions tokens (especially those with
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
/// * `hook_accounts` - Additional accounts required by the transfer hook (pass `&[]` if none)
///
/// # Errors
///
/// Returns an error if the transfer transaction fails.
///
/// # Example
///
/// ```rust
/// use solana_kite::token_extensions::{
///     create_token_extensions_mint, create_token_extensions_account,
///     mint_tokens_to_token_extensions_account, transfer_checked_token_extensions,
/// };
/// use solana_kite::create_wallet;
/// use litesvm::LiteSVM;
/// use solana_signer::Signer;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut litesvm = LiteSVM::new();
/// let authority = create_wallet(&mut litesvm, 1_000_000_000)?;
/// let user = create_wallet(&mut litesvm, 1_000_000_000)?;
/// let mint = create_token_extensions_mint(&mut litesvm, &authority, 6, None, &[])?;
/// let source = create_token_extensions_account(&mut litesvm, &authority.pubkey(), &mint, &authority)?;
/// let destination = create_token_extensions_account(&mut litesvm, &user.pubkey(), &mint, &authority)?;
/// mint_tokens_to_token_extensions_account(&mut litesvm, &mint, &source, 1_000_000, &authority)?;
/// transfer_checked_token_extensions(&mut litesvm, &source, &mint, &destination, &authority, 500_000, 6, &[])?;
/// # Ok(())
/// # }
/// ```
#[allow(clippy::too_many_arguments)] // All parameters map directly to TransferChecked instruction fields
pub fn transfer_checked_token_extensions(
    litesvm: &mut LiteSVM,
    source: &Pubkey,
    mint: &Pubkey,
    destination: &Pubkey,
    authority: &Keypair,
    amount: u64,
    decimals: u8,
    hook_accounts: &[AccountMeta],
) -> Result<(), SolanaKiteError> {
    let mut data = vec![INSTRUCTION_TRANSFER_CHECKED];
    data.extend_from_slice(&amount.to_le_bytes());
    data.push(decimals);

    let mut accounts = vec![
        AccountMeta::new(*source, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new(*destination, false),
        AccountMeta::new_readonly(authority.pubkey(), true),
    ];
    accounts.extend_from_slice(hook_accounts);

    let instruction = Instruction {
        program_id: TOKEN_EXTENSIONS_PROGRAM_ID,
        accounts,
        data,
    };

    send_transaction_from_instructions(
        litesvm,
        vec![instruction],
        &[authority],
        &authority.pubkey(),
    )?;

    Ok(())
}

/// Derives the associated token account address for a Token Extensions mint.
///
/// Useful for pre-computing the address before calling
/// [`create_token_extensions_account`], e.g. to pass it to a program instruction
/// before the account exists onchain.
///
/// # Example
///
/// ```rust
/// use solana_kite::token_extensions::get_token_extensions_account_address;
/// use litesvm::LiteSVM;
/// use solana_pubkey::Pubkey;
///
/// let owner = Pubkey::new_unique();
/// let mint = Pubkey::new_unique();
/// let ata = get_token_extensions_account_address(&owner, &mint);
/// ```
#[must_use]
pub fn get_token_extensions_account_address(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    spl_associated_token_account::get_associated_token_address_with_program_id(
        owner,
        mint,
        &TOKEN_EXTENSIONS_PROGRAM_ID,
    )
}
