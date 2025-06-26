//! Wallet creation and management utilities.

use crate::error::SolanaKiteError;
use litesvm::LiteSVM;
use solana_keypair::Keypair;
use solana_signer::Signer;

/// Creates a new wallet (keypair) and airdrops SOL to it.
///
/// This function generates a new keypair and funds it with the specified amount
/// of lamports via an airdrop in the LiteSVM test environment.
///
/// # Arguments
///
/// * `litesvm` - Mutable reference to the LiteSVM instance
/// * `airdrop_amount` - Amount of lamports to airdrop to the new wallet
///
/// # Returns
///
/// Returns the newly created and funded keypair.
///
/// # Errors
///
/// This function will return an error if the airdrop fails.
///
/// # Example
///
/// ```rust
/// use solana_kite::create_wallet;
/// use litesvm::LiteSVM;
///
/// let mut litesvm = LiteSVM::new();
/// let wallet = create_wallet(&mut litesvm, 1_000_000_000)?; // 1 SOL
/// # Ok::<(), solana_kite::SolanaKiteError>(())
/// ```
pub fn create_wallet(
    litesvm: &mut LiteSVM,
    airdrop_amount: u64,
) -> Result<Keypair, SolanaKiteError> {
    let wallet = Keypair::new();
    litesvm
        .airdrop(&wallet.pubkey(), airdrop_amount)
        .map_err(|e| SolanaKiteError::AccountOperationFailed(format!("Failed to airdrop to wallet: {:?}", e)))?;
    Ok(wallet)
}

/// Creates multiple wallets with the same airdrop amount.
///
/// This is a convenience function for creating multiple funded wallets at once,
/// useful for testing scenarios that require multiple participants.
///
/// # Arguments
///
/// * `litesvm` - Mutable reference to the LiteSVM instance
/// * `count` - Number of wallets to create
/// * `airdrop_amount` - Amount of lamports to airdrop to each wallet
///
/// # Returns
///
/// Returns a vector of newly created and funded keypairs.
///
/// # Errors
///
/// This function will return an error if any airdrop fails.
///
/// # Example
///
/// ```rust
/// use solana_kite::create_wallets;
/// use litesvm::LiteSVM;
///
/// let mut litesvm = LiteSVM::new();
/// let wallets = create_wallets(&mut litesvm, 3, 1_000_000_000)?; // 3 wallets with 1 SOL each
/// assert_eq!(wallets.len(), 3);
/// # Ok::<(), solana_kite::SolanaKiteError>(())
/// ```
pub fn create_wallets(
    litesvm: &mut LiteSVM,
    count: usize,
    airdrop_amount: u64,
) -> Result<Vec<Keypair>, SolanaKiteError> {
    let mut wallets = Vec::with_capacity(count);
    for _ in 0..count {
        let wallet = create_wallet(litesvm, airdrop_amount)?;
        wallets.push(wallet);
    }
    Ok(wallets)
}