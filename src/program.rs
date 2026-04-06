//! Program deployment utilities for Solana programs.

use crate::error::SolanaKiteError;
use litesvm::LiteSVM;
use solana_pubkey::Pubkey;
use std::fs;

/// Deploys a program to the LiteSVM test environment from a file path.
///
/// Reads a compiled program binary (.so file) from disk and deploys it.
/// For deploying from in-memory bytes (e.g. from `include_bytes!`), use
/// [`deploy_program_bytes`] instead.
///
/// # Arguments
///
/// * `litesvm` - Mutable reference to the LiteSVM instance
/// * `program_id` - The public key where the program should be deployed
/// * `program_path` - Path to the compiled program binary (.so file)
///
/// # Errors
///
/// Returns an error if the file cannot be read or the deployment fails.
///
/// # Example
///
/// ```rust,no_run
/// use solana_kite::deploy_program;
/// use litesvm::LiteSVM;
/// use solana_pubkey::Pubkey;
///
/// let mut litesvm = LiteSVM::new();
/// let program_id = Pubkey::new_unique();
/// deploy_program(&mut litesvm, &program_id, "./target/deploy/my_program.so").unwrap();
/// ```
pub fn deploy_program(
    litesvm: &mut LiteSVM,
    program_id: &Pubkey,
    program_path: &str,
) -> Result<(), SolanaKiteError> {
    let program_bytes = fs::read(program_path).map_err(|e| {
        SolanaKiteError::ProgramDeploymentFailed(format!(
            "Failed to read program binary at {}: {}",
            program_path, e
        ))
    })?;

    deploy_program_bytes(litesvm, program_id, &program_bytes)
}

/// Deploys a program to the LiteSVM test environment from raw bytes.
///
/// This is useful when you have the program binary embedded via `include_bytes!`
/// or loaded from a non-filesystem source.
///
/// # Arguments
///
/// * `litesvm` - Mutable reference to the LiteSVM instance
/// * `program_id` - The public key where the program should be deployed
/// * `program_bytes` - The compiled program binary as a byte slice
///
/// # Errors
///
/// Returns an error if the deployment fails.
///
/// # Example
///
/// ```rust,no_run
/// use solana_kite::deploy_program_bytes;
/// use litesvm::LiteSVM;
/// use solana_pubkey::Pubkey;
///
/// let mut litesvm = LiteSVM::new();
/// let program_id = Pubkey::new_unique();
/// # let program_bytes: &[u8] = &[];
/// deploy_program_bytes(&mut litesvm, &program_id, program_bytes).unwrap();
/// ```
pub fn deploy_program_bytes(
    litesvm: &mut LiteSVM,
    program_id: &Pubkey,
    program_bytes: &[u8],
) -> Result<(), SolanaKiteError> {
    litesvm
        .set_account(
            *program_id,
            solana_account::Account {
                lamports: litesvm.minimum_balance_for_rent_exemption(program_bytes.len()),
                data: program_bytes.to_vec(),
                owner: solana_program::bpf_loader::ID,
                executable: true,
                rent_epoch: 0,
            },
        )
        .map_err(|e| {
            SolanaKiteError::ProgramDeploymentFailed(format!(
                "Failed to deploy program: {}",
                e
            ))
        })?;

    Ok(())
}
