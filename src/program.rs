//! Program deployment utilities for Solana programs.

use crate::error::SolanaKiteError;
use litesvm::LiteSVM;
use solana_pubkey::Pubkey;
use std::fs;

/// Deploys a program to the LiteSVM test environment.
///
/// This function reads a program binary from the filesystem and deploys it to the
/// specified program ID in the LiteSVM instance. The program will be marked as executable
/// and owned by the BPF loader.
///
/// # Arguments
///
/// * `litesvm` - Mutable reference to the LiteSVM instance
/// * `program_id` - The public key where the program should be deployed
/// * `program_path` - Path to the compiled program binary (.so file)
///
/// # Returns
///
/// Returns `Ok(())` on successful deployment, or a [`SolanaKiteError`] on failure.
///
/// # Errors
///
/// This function will return an error if:
/// - The program binary file cannot be read
/// - The program deployment to LiteSVM fails
///
/// # Example
///
/// ```rust
/// use solana_kite::deploy_program;
/// use litesvm::LiteSVM;
/// use solana_pubkey::Pubkey;
///
/// let mut litesvm = LiteSVM::new();
/// let program_id = Pubkey::new_unique();
/// 
/// // Deploy a program (this would fail in tests without an actual .so file)
/// // deploy_program(&mut litesvm, &program_id, "./target/deploy/my_program.so")?;
/// ```
pub fn deploy_program(
    litesvm: &mut LiteSVM,
    program_id: &Pubkey,
    program_path: &str,
) -> Result<(), SolanaKiteError> {
    let program_bytes = fs::read(program_path)
        .map_err(|e| SolanaKiteError::ProgramDeploymentFailed(format!("Failed to read program binary at {}: {}", program_path, e)))?;
    
    litesvm
        .set_account(
            *program_id,
            solana_account::Account {
                lamports: litesvm.minimum_balance_for_rent_exemption(program_bytes.len()),
                data: program_bytes,
                owner: solana_program::bpf_loader::ID,
                executable: true,
                rent_epoch: 0,
            },
        )
        .map_err(|e| SolanaKiteError::ProgramDeploymentFailed(format!("Failed to deploy program: {:?}", e)))?;
    
    Ok(())
}