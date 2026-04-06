//! Transfer hook helpers for Token Extensions.
//!
//! When a Token Extensions mint has the TransferHook extension, every transfer
//! invokes a program-defined hook. That hook program must store an
//! `ExtraAccountMetaList` account that tells the Token Extensions runtime which
//! additional accounts to pass into the hook.
//!
//! This module provides a helper to initialise that account.

use crate::constants::SYSTEM_PROGRAM_ID;
use crate::error::SolanaKiteError;
use crate::token_extensions::TOKEN_EXTENSIONS_PROGRAM_ID;
use crate::transaction::send_transaction_from_instructions;
use litesvm::LiteSVM;
use solana_instruction::account_meta::AccountMeta;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

/// The `InitializeExtraAccountMetaList` instruction discriminator from
/// spl-transfer-hook-interface.
/// Source: https://github.com/solana-program/transfer-hook/blob/main/interface/src/instruction.rs
const INITIALIZE_EXTRA_ACCOUNT_META_LIST_DISCRIMINATOR: [u8; 8] =
    [43, 34, 13, 49, 167, 88, 235, 235];

/// Serialized size of one `HookAccount` entry in the onchain format:
/// discriminator(1) + address_config(32) + is_signer(1) + is_writable(1).
const EXTRA_ACCOUNT_META_SERIALIZED_SIZE: usize = 35;

/// Describes an additional account that the transfer hook program requires.
///
/// The Token Extensions runtime reads these from the `ExtraAccountMetaList` PDA
/// and appends them to the CPI into the hook program.
#[derive(Debug, Clone)]
pub struct HookAccount {
    /// The account's public key.
    pub pubkey: Pubkey,
    /// Whether the account must sign.
    pub is_signer: bool,
    /// Whether the account is writable.
    pub is_writable: bool,
}

impl HookAccount {
    /// Serialise to the onchain format:
    ///   discriminator (1) + address_config (32) + is_signer (1) + is_writable (1)
    ///
    /// Discriminator 0 means "literal pubkey" (not a PDA seed derivation).
    fn to_bytes(&self) -> [u8; EXTRA_ACCOUNT_META_SERIALIZED_SIZE] {
        let mut buf = [0u8; EXTRA_ACCOUNT_META_SERIALIZED_SIZE];
        buf[0] = 0; // discriminator: literal address
        buf[1..33].copy_from_slice(&self.pubkey.to_bytes());
        buf[33] = self.is_signer as u8;
        buf[34] = self.is_writable as u8;
        buf
    }
}

/// Derives the ExtraAccountMetaList PDA address for a given mint and hook program.
///
/// The PDA seeds are: `["extra-account-metas", mint]` with the hook program as
/// the deriving program.
#[must_use]
pub fn get_hook_accounts_address(mint: &Pubkey, hook_program_id: &Pubkey) -> Pubkey {
    let (address, _bump) = Pubkey::find_program_address(
        &[b"extra-account-metas", mint.as_ref()],
        hook_program_id,
    );
    address
}

/// Initialises the ExtraAccountMetaList PDA for a transfer hook program.
///
/// Sends a transaction to the hook program telling it to write the
/// hook accounts into its PDA. The hook program must implement the
/// `spl-transfer-hook-interface` `InitializeExtraAccountMetaList` instruction handler
/// (discriminator `[43, 34, 13, 49, 167, 88, 235, 235]`).
///
/// # Arguments
///
/// * `litesvm` - Mutable reference to the LiteSVM instance
/// * `hook_program_id` - The transfer hook program ID
/// * `mint` - The Token Extensions mint with the TransferHook extension
/// * `authority` - The mint authority / payer keypair
/// * `hook_accounts` - Accounts the hook program requires on every transfer
///
/// # Errors
///
/// Returns an error if the initialisation transaction fails.
///
/// # Example
///
/// ```rust,no_run
/// use solana_kite::{create_wallet, deploy_program_bytes};
/// use solana_kite::transfer_hook::{initialize_hook_accounts, HookAccount};
/// use litesvm::LiteSVM;
/// use solana_pubkey::Pubkey;
/// use solana_signer::Signer;
///
/// let mut litesvm = LiteSVM::new();
/// let authority = create_wallet(&mut litesvm, 1_000_000_000).unwrap();
/// let hook_program_id = Pubkey::new_unique();
/// let program_bytes = include_bytes!("../target/deploy/my_hook.so");
/// deploy_program_bytes(&mut litesvm, &hook_program_id, program_bytes).unwrap();
/// let mint = Pubkey::new_unique(); // a Token Extensions mint with the TransferHook extension
///
/// initialize_hook_accounts(&mut litesvm, &hook_program_id, &mint, &authority, &[]).unwrap();
/// ```
pub fn initialize_hook_accounts(
    litesvm: &mut LiteSVM,
    hook_program_id: &Pubkey,
    mint: &Pubkey,
    authority: &Keypair,
    hook_accounts: &[HookAccount],
) -> Result<(), SolanaKiteError> {
    let hook_accounts_address = get_hook_accounts_address(mint, hook_program_id);

    // Serialise as: discriminator(8) + u32 entry count + N * EXTRA_ACCOUNT_META_SERIALIZED_SIZE
    let mut data = Vec::with_capacity(
        8 + 4 + hook_accounts.len() * EXTRA_ACCOUNT_META_SERIALIZED_SIZE,
    );
    data.extend_from_slice(&INITIALIZE_EXTRA_ACCOUNT_META_LIST_DISCRIMINATOR);
    data.extend_from_slice(&(hook_accounts.len() as u32).to_le_bytes());
    for account in hook_accounts {
        data.extend_from_slice(&account.to_bytes());
    }

    let instruction = Instruction {
        program_id: *hook_program_id,
        accounts: vec![
            AccountMeta::new(hook_accounts_address, false),      // ExtraAccountMetaList PDA
            AccountMeta::new_readonly(*mint, false),              // mint
            AccountMeta::new(authority.pubkey(), true),           // authority / payer
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),  // system program
        ],
        data,
    };

    send_transaction_from_instructions(
        litesvm,
        vec![instruction],
        &[authority],
        &authority.pubkey(),
    )
    .map_err(|e| match e {
        SolanaKiteError::TransactionFailed(msg) => SolanaKiteError::HookOperationFailed(msg),
        other => other,
    })?;

    Ok(())
}

/// Builds the accounts needed to pass into a hook-aware transfer.
///
/// Returns the accounts to pass as `hook_accounts` to
/// [`crate::token_extensions::transfer_checked_token_extensions`]. This includes
/// any user-defined [`HookAccount`]s plus the hook program ID and ExtraAccountMetaList
/// PDA that Token Extensions appends to every hook CPI.
#[must_use]
pub fn build_hook_accounts(
    mint: &Pubkey,
    hook_program_id: &Pubkey,
    hook_accounts: &[HookAccount],
) -> Vec<AccountMeta> {
    let hook_accounts_address = get_hook_accounts_address(mint, hook_program_id);

    let mut accounts: Vec<AccountMeta> = hook_accounts
        .iter()
        .map(|account| {
            if account.is_writable {
                AccountMeta::new(account.pubkey, account.is_signer)
            } else {
                AccountMeta::new_readonly(account.pubkey, account.is_signer)
            }
        })
        .collect();

    // The transfer hook runtime appends these at the end
    accounts.push(AccountMeta::new_readonly(*hook_program_id, false));
    accounts.push(AccountMeta::new_readonly(hook_accounts_address, false));
    // Token Extensions also passes the Token Extensions program itself as the last account
    accounts.push(AccountMeta::new_readonly(TOKEN_EXTENSIONS_PROGRAM_ID, false));

    accounts
}
