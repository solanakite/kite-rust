//! Transfer hook helpers for Token Extensions.
//!
//! When a Token Extensions mint has the TransferHook extension, every transfer
//! invokes a program-defined hook. That hook program must store an
//! `ExtraAccountMetaList` account that tells the Token Extensions runtime which
//! additional accounts to pass into the hook.
//!
//! This module provides a helper to initialise that account.

use crate::error::SolanaKiteError;
use crate::token_extensions::TOKEN_EXTENSIONS_PROGRAM_ID;
use litesvm::LiteSVM;
use solana_instruction::account_meta::AccountMeta;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_message::Message;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::Transaction;

// ─── ExtraAccountMeta ────────────────────────────────────────────────────────

/// Describes an additional account that the transfer hook program requires.
///
/// The Token Extensions runtime reads these from the `ExtraAccountMetaList` PDA
/// and appends them to the CPI into the hook program.
#[derive(Debug, Clone)]
pub struct ExtraAccountMeta {
    /// The account's public key.
    pub pubkey: Pubkey,
    /// Whether the account must sign.
    pub is_signer: bool,
    /// Whether the account is writable.
    pub is_writable: bool,
}

impl ExtraAccountMeta {
    /// Serialise to the 35-byte on-chain format:
    ///   discriminator (1) + address_config (32) + is_signer (1) + is_writable (1)
    ///
    /// Discriminator 0 means "literal pubkey" (not a PDA seed derivation).
    fn to_bytes(&self) -> [u8; 35] {
        let mut buf = [0u8; 35];
        buf[0] = 0; // discriminator: literal address
        buf[1..33].copy_from_slice(&self.pubkey.to_bytes());
        buf[33] = self.is_signer as u8;
        buf[34] = self.is_writable as u8;
        buf
    }
}

/// Derives the ExtraAccountMetaList PDA for a given mint and hook program.
///
/// The PDA seeds are: `["extra-account-metas", mint]` with the hook program as
/// the deriving program.
pub fn get_extra_account_metas_address(mint: &Pubkey, hook_program_id: &Pubkey) -> Pubkey {
    let (address, _bump) = Pubkey::find_program_address(
        &[b"extra-account-metas", mint.as_ref()],
        hook_program_id,
    );
    address
}

/// Initialises the `ExtraAccountMetaList` account for a transfer hook.
///
/// This sends a transaction to the hook program telling it to write the
/// extra account metas into the PDA. The hook program must implement the
/// `spl-transfer-hook-interface` `InitializeExtraAccountMetaList` instruction
/// (discriminator `[43, 34, 13, 49, 167, 88, 235, 235]`).
///
/// # Arguments
///
/// * `litesvm` - Mutable reference to the LiteSVM instance
/// * `hook_program_id` - The transfer hook program ID
/// * `mint` - The Token Extensions mint with the TransferHook extension
/// * `authority` - The mint authority / payer keypair
/// * `extra_metas` - Slice of extra account metas the hook needs
///
/// # Errors
///
/// Returns an error if the initialisation transaction fails.
pub fn initialize_extra_account_meta_list(
    litesvm: &mut LiteSVM,
    hook_program_id: &Pubkey,
    mint: &Pubkey,
    authority: &Keypair,
    extra_metas: &[ExtraAccountMeta],
) -> Result<(), SolanaKiteError> {
    let extra_account_metas_address = get_extra_account_metas_address(mint, hook_program_id);

    // Build the instruction data:
    //   anchor discriminator (8 bytes) + serialised ExtraAccountMetaList
    //
    // The spl-transfer-hook-interface InitializeExtraAccountMetaList discriminator
    // is sha256("spl-transfer-hook-interface:execute")[..8], but the standard
    // Anchor convention uses: [43, 34, 13, 49, 167, 88, 235, 235]
    let discriminator: [u8; 8] = [43, 34, 13, 49, 167, 88, 235, 235];

    // Serialise the extra metas as a length-prefixed array:
    //   u32 length + N * 35-byte entries
    let mut data = Vec::with_capacity(8 + 4 + extra_metas.len() * 35);
    data.extend_from_slice(&discriminator);
    data.extend_from_slice(&(extra_metas.len() as u32).to_le_bytes());
    for meta in extra_metas {
        data.extend_from_slice(&meta.to_bytes());
    }

    let system_program_id = solana_pubkey::pubkey!("11111111111111111111111111111111");

    let instruction = Instruction {
        program_id: *hook_program_id,
        accounts: vec![
            AccountMeta::new(extra_account_metas_address, false), // extra account metas PDA
            AccountMeta::new_readonly(*mint, false),               // mint
            AccountMeta::new(authority.pubkey(), true),             // authority / payer
            AccountMeta::new_readonly(system_program_id, false),   // system program
        ],
        data,
    };

    let message = Message::new(&[instruction], Some(&authority.pubkey()));
    let mut transaction = Transaction::new_unsigned(message);
    let blockhash = litesvm.latest_blockhash();
    transaction.sign(&[authority], blockhash);

    litesvm.send_transaction(transaction).map_err(|e| {
        SolanaKiteError::TokenOperationFailed(format!(
            "Failed to initialize extra account meta list: {:?}",
            e
        ))
    })?;

    Ok(())
}

/// Builds the extra accounts needed for a Token Extensions transfer with a transfer hook.
///
/// Returns the account metas that should be passed as `extra_accounts` to
/// [`crate::token_extensions::transfer_checked_token_extensions`]. This includes the
/// ExtraAccountMetaList PDA, the hook program, and any user-defined extra accounts.
pub fn build_transfer_hook_extra_accounts(
    mint: &Pubkey,
    hook_program_id: &Pubkey,
    extra_metas: &[ExtraAccountMeta],
) -> Vec<AccountMeta> {
    let extra_account_metas_address = get_extra_account_metas_address(mint, hook_program_id);

    let mut accounts: Vec<AccountMeta> = extra_metas
        .iter()
        .map(|meta| {
            if meta.is_writable {
                AccountMeta::new(meta.pubkey, meta.is_signer)
            } else {
                AccountMeta::new_readonly(meta.pubkey, meta.is_signer)
            }
        })
        .collect();

    // The transfer hook runtime appends these at the end
    accounts.push(AccountMeta::new_readonly(*hook_program_id, false));
    accounts.push(AccountMeta::new_readonly(
        extra_account_metas_address,
        false,
    ));
    // Token Extensions also passes the Token Extensions program itself as the last account
    accounts.push(AccountMeta::new_readonly(TOKEN_EXTENSIONS_PROGRAM_ID, false));

    accounts
}
