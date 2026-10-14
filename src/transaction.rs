//! Transaction utilities for sending Solana transactions.
//!
//! Every transaction kite builds, including the ones its token, Token
//! Extensions and transfer hook helpers send, is a v1 transaction (SIMD-0385),
//! the format live on mainnet beta since 15 September 2026 (epoch 1035).
//! [`create_wallet`](crate::create_wallet) funds a wallet with LiteSVM's
//! airdrop, which LiteSVM builds as a legacy transaction.
//!
//! A v1 transaction states its compute unit, loaded account data and heap
//! limits, and its priority fee, in a [`v1::TransactionConfig`] in the
//! message, not in ComputeBudget instructions: a ComputeBudget instruction in
//! a v1 transaction does nothing, so kite returns an error for one.
//!
//! An unset compute unit or loaded account data limit is zero, and SIMD-0385
//! leaves choosing them to whoever builds the transaction, so kite sets both in
//! every transaction it builds:
//!
//! - The compute unit limit is the one you pass
//!   [`send_transaction_from_instructions`] or kite's token, Token Extensions
//!   and transfer hook helpers. Pass `None` and kite simulates the transaction
//!   and uses the compute units the simulation used.
//! - The loaded account data limit is the most a transaction may load: see
//!   [`get_transaction_config`].

use crate::error::SolanaKiteError;
use litesvm::LiteSVM;
use solana_compute_budget::compute_budget_limits::{
    MAX_COMPUTE_UNIT_LIMIT, MAX_LOADED_ACCOUNTS_DATA_SIZE_BYTES,
};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_message::{v1, VersionedMessage};
use solana_pubkey::Pubkey;
use solana_sdk_ids::compute_budget;
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;

/// The [`v1::TransactionConfig`] kite gives a transaction: the compute unit
/// limit passed in, and Agave's `MAX_LOADED_ACCOUNTS_DATA_SIZE_BYTES` (64 MiB
/// in `solana-compute-budget` 4.2) of loaded account data, the most a
/// transaction may load. The heap and priority fee are left unset, so the
/// transaction gets a 32 KiB heap and pays no priority fee.
///
/// Kite sets the loaded account data limit to the most because a v1
/// transaction's fee does not depend on its limits: it is the signature fee
/// plus the priority fee, a fixed number of lamports. Pass
/// [`build_transaction_from_instructions`] your own config to set it lower.
///
/// ```rust
/// use solana_kite::get_transaction_config;
///
/// let config = get_transaction_config(50_000);
/// assert_eq!(config.compute_unit_limit, Some(50_000));
/// ```
pub const fn get_transaction_config(compute_unit_limit: u32) -> v1::TransactionConfig {
    v1::TransactionConfig::empty()
        .with_compute_unit_limit(compute_unit_limit)
        .with_loaded_accounts_data_size_limit(MAX_LOADED_ACCOUNTS_DATA_SIZE_BYTES.get())
}

/// Builds and signs a v1 transaction from instructions, with the given config.
///
/// Use this instead of [`send_transaction_from_instructions`] to read the
/// transaction's logs or compute units from `litesvm.send_transaction`, to
/// measure the transaction, or to set its limits yourself.
///
/// Signers may repeat: a fee payer that is also passed as a named signer is
/// signed for once.
///
/// # Errors
///
/// Returns [`SolanaKiteError::TransactionFailed`] if:
///
/// - an instruction is a ComputeBudget instruction. A v1 transaction ignores
///   them, so one would leave a test believing it set a limit or a priority
///   fee that it did not. Set those in `config` instead.
/// - the instructions do not compile into a v1 message.
/// - the signers do not match the signatures the message needs.
///
/// # Example
///
/// ```rust
/// use solana_kite::{
///     build_transaction_from_instructions, create_wallet, get_transaction_config,
/// };
/// use litesvm::LiteSVM;
/// use solana_signer::Signer;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut litesvm = LiteSVM::new();
/// let payer = create_wallet(&mut litesvm, 1_000_000_000)?;
/// let instructions = vec![]; // Your instructions here
///
/// let transaction = build_transaction_from_instructions(
///     &litesvm,
///     &instructions,
///     &[&payer],
///     &payer.pubkey(),
///     get_transaction_config(200_000),
/// )?;
/// let metadata = litesvm.send_transaction(transaction).unwrap();
/// println!("{:?}", metadata.logs);
/// # Ok(())
/// # }
/// ```
pub fn build_transaction_from_instructions(
    litesvm: &LiteSVM,
    instructions: &[Instruction],
    signers: &[&Keypair],
    fee_payer: &Pubkey,
    config: v1::TransactionConfig,
) -> Result<VersionedTransaction, SolanaKiteError> {
    if instructions
        .iter()
        .any(|instruction| compute_budget::check_id(&instruction.program_id))
    {
        return Err(SolanaKiteError::TransactionFailed(
            "ComputeBudget instructions do nothing in a v1 transaction: pass the compute unit \
             limit as compute_unit_limit, or set limits and the priority fee in the \
             TransactionConfig passed to build_transaction_from_instructions"
                .to_string(),
        ));
    }

    let message = v1::Message::try_compile_with_config(
        fee_payer,
        instructions,
        litesvm.latest_blockhash(),
        config,
    )
    .map_err(|error| {
        SolanaKiteError::TransactionFailed(format!("Failed to compile v1 message: {error}"))
    })?;

    // VersionedTransaction::try_new rejects a keypair passed twice as too many
    // signers, and callers often pass the fee payer as a named signer too.
    let mut unique_signers: Vec<&Keypair> = Vec::with_capacity(signers.len());
    for signer in signers {
        if !unique_signers
            .iter()
            .any(|unique_signer| unique_signer.pubkey() == signer.pubkey())
        {
            unique_signers.push(signer);
        }
    }

    VersionedTransaction::try_new(VersionedMessage::V1(message), &unique_signers).map_err(|error| {
        SolanaKiteError::TransactionFailed(format!("Failed to sign transaction: {error}"))
    })
}

/// Sends a transaction built from a vector of instructions.
///
/// This function creates a v1 transaction from the provided instructions, with
/// [`get_transaction_config`] and the given compute unit limit, signs it with
/// the given signers, and sends it through the LiteSVM instance. Kite's token,
/// Token Extensions and transfer hook helpers send their transactions with it.
///
/// # Arguments
///
/// * `litesvm` - Mutable reference to the LiteSVM instance
/// * `instructions` - Vector of instructions to include in the transaction
/// * `signers` - Array of keypairs that will sign the transaction
/// * `fee_payer` - Public key of the account that will pay transaction fees
/// * `compute_unit_limit` - The most compute units the transaction may use,
///   or `None` to simulate the transaction and use the compute units the
///   simulation used. A number you choose makes the test fail if the
///   transaction needs more, which `None` never does: a v1 transaction that
///   runs out of compute units fails.
///
/// # Returns
///
/// Returns `Ok(())` on successful transaction, or a [`SolanaKiteError`] on failure.
///
/// # Errors
///
/// This function will return an error if the transaction cannot be built, for
/// example because an instruction is a ComputeBudget instruction (see
/// [`build_transaction_from_instructions`]), or fails to send or execute.
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
///     Some(200_000),
/// )?;
/// # Ok(())
/// # }
/// ```
pub fn send_transaction_from_instructions(
    litesvm: &mut LiteSVM,
    instructions: Vec<Instruction>,
    signers: &[&Keypair],
    fee_payer: &Pubkey,
    compute_unit_limit: Option<u32>,
) -> Result<(), SolanaKiteError> {
    let compute_unit_limit = match compute_unit_limit {
        Some(compute_unit_limit) => compute_unit_limit,
        None => measure_compute_units(litesvm, &instructions, signers, fee_payer)?,
    };
    let transaction = build_transaction_from_instructions(
        litesvm,
        &instructions,
        signers,
        fee_payer,
        get_transaction_config(compute_unit_limit),
    )?;

    litesvm
        .send_transaction(transaction)
        .map(|_| ())
        .map_err(|error| SolanaKiteError::TransactionFailed(format!("{error:?}")))
}

/// The compute units a transaction of these instructions uses, measured by
/// simulating it with the most compute units a transaction may ask for, so the
/// simulation does not run out.
fn measure_compute_units(
    litesvm: &LiteSVM,
    instructions: &[Instruction],
    signers: &[&Keypair],
    fee_payer: &Pubkey,
) -> Result<u32, SolanaKiteError> {
    let transaction = build_transaction_from_instructions(
        litesvm,
        instructions,
        signers,
        fee_payer,
        get_transaction_config(MAX_COMPUTE_UNIT_LIMIT),
    )?;
    let simulation = litesvm
        .simulate_transaction(transaction)
        .map_err(|error| SolanaKiteError::TransactionFailed(format!("{error:?}")))?;
    // A transaction cannot use more than MAX_COMPUTE_UNIT_LIMIT, a u32.
    u32::try_from(simulation.meta.compute_units_consumed).map_err(|error| {
        SolanaKiteError::TransactionFailed(format!(
            "Simulated compute units do not fit in a u32: {error}"
        ))
    })
}
