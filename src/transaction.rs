//! Transaction utilities for sending Solana transactions.

use crate::error::SolanaKiteError;
use litesvm::LiteSVM;
use solana_keypair::Keypair;
use solana_message::Message;
use solana_pubkey::Pubkey;
use solana_transaction::Transaction;

/// Sends a transaction built from a vector of instructions.
///
/// This function creates a transaction from the provided instructions, signs it with
/// the given signers, and sends it through the LiteSVM instance.
///
/// # Arguments
///
/// * `litesvm` - Mutable reference to the LiteSVM instance
/// * `instructions` - Vector of instructions to include in the transaction
/// * `signers` - Array of keypairs that will sign the transaction
/// * `fee_payer` - Public key of the account that will pay transaction fees
///
/// # Returns
///
/// Returns `Ok(())` on successful transaction, or a [`SolanaKiteError`] on failure.
///
/// # Errors
///
/// This function will return an error if the transaction fails to send or execute.
///
/// # Example
///
/// ```rust
/// use solana_kite::{send_transaction_from_instructions, create_wallet};
/// use litesvm::LiteSVM;
/// use solana_signer::Signer;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut litesvm = LiteSVM::new();
/// let payer = create_wallet(&mut litesvm, 1_000_000_000)?;
/// let instructions = vec![]; // Your instructions here
/// 
/// send_transaction_from_instructions(
///     &mut litesvm,
///     instructions,
///     &[&payer],
///     &payer.pubkey(),
/// )?;
/// # Ok(())
/// # }
/// ```
pub fn send_transaction_from_instructions(
    litesvm: &mut LiteSVM,
    instructions: Vec<solana_instruction::Instruction>,
    signers: &[&Keypair],
    fee_payer: &Pubkey,
) -> Result<(), SolanaKiteError> {
    let recent_blockhash = litesvm.latest_blockhash();
    let message = Message::new(&instructions, Some(fee_payer));
    let mut transaction = Transaction::new_unsigned(message);
    transaction.sign(signers, recent_blockhash);
    
    litesvm
        .send_transaction(transaction)
        .map(|_| ())
        .map_err(|e| SolanaKiteError::TransactionFailed(format!("{:?}", e)))
}