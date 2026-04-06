//! Token operations for SPL tokens on Solana.

use crate::constants::{
    SPL_TOKEN_MINT_SIZE, TOKEN_ACCOUNT_AMOUNT_END, TOKEN_ACCOUNT_AMOUNT_OFFSET,
};
use crate::error::SolanaKiteError;
use crate::transaction::send_transaction_from_instructions;
use litesvm::LiteSVM;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use spl_associated_token_account::instruction::create_associated_token_account as create_ata_instruction;
use spl_token::instruction::mint_to;

/// Creates a new SPL token mint with the specified mint authority and decimals.
///
/// This function creates a new token mint account with proper rent exemption and
/// initializes it as an SPL token mint. You can optionally specify a custom mint
/// address, or let the function generate a unique one.
///
/// # Arguments
///
/// * `litesvm` - Mutable reference to the LiteSVM instance
/// * `mint_authority` - Keypair that will have authority to mint tokens
/// * `decimals` - Number of decimal places for the token (SPL Token enforces a maximum of 9)
/// * `mint` - Optional custom public key for the mint. If None, a unique address will be generated
///
/// # Returns
///
/// Returns the public key of the newly created mint.
///
/// # Errors
///
/// This function will return an error if the mint creation or initialization fails.
///
/// # Example
///
/// ```rust
/// use solana_kite::{create_token_mint, create_wallet};
/// use litesvm::LiteSVM;
/// use solana_pubkey::Pubkey;
/// use solana_signer::Signer;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut litesvm = LiteSVM::new();
/// let mint_authority = create_wallet(&mut litesvm, 1_000_000_000)?;
///
/// // Create a mint with auto-generated address
/// let mint_pubkey = create_token_mint(&mut litesvm, &mint_authority, 6, None)?;
///
/// // Or create a mint with a specific address
/// let custom_mint = Pubkey::new_unique();
/// let mint_pubkey2 = create_token_mint(&mut litesvm, &mint_authority, 6, Some(custom_mint))?;
/// assert_eq!(mint_pubkey2, custom_mint);
/// # Ok(())
/// # }
/// ```
pub fn create_token_mint(
    litesvm: &mut LiteSVM,
    mint_authority: &Keypair,
    decimals: u8,
    mint: Option<Pubkey>,
) -> Result<Pubkey, SolanaKiteError> {
    let mint = mint.unwrap_or(Pubkey::new_unique());
    let rent = litesvm.minimum_balance_for_rent_exemption(SPL_TOKEN_MINT_SIZE);

    litesvm
        .set_account(
            mint,
            solana_account::Account {
                lamports: rent,
                data: vec![0u8; SPL_TOKEN_MINT_SIZE],
                owner: spl_token::ID,
                executable: false,
                rent_epoch: 0,
            },
        )
        .map_err(|e| {
            SolanaKiteError::TokenOperationFailed(format!("Failed to create mint account: {e}"))
        })?;

    let initialize_mint_instruction = spl_token::instruction::initialize_mint(
        &spl_token::ID,
        &mint,
        &mint_authority.pubkey(),
        None,
        decimals,
    )
    .map_err(|e| {
        SolanaKiteError::TokenOperationFailed(format!(
            "Failed to create initialize mint instruction: {e}"
        ))
    })?;

    send_transaction_from_instructions(
        litesvm,
        vec![initialize_mint_instruction],
        &[mint_authority],
        &mint_authority.pubkey(),
    )?;

    Ok(mint)
}

/// Creates an associated token account for the given owner and mint.
///
/// This function creates an associated token account (ATA) which is a deterministic
/// address derived from the owner and mint addresses. The payer funds the account
/// creation and signs the transaction.
///
/// # Arguments
///
/// * `litesvm` - Mutable reference to the LiteSVM instance
/// * `owner` - Public key of the account that will own the token account
/// * `mint` - Public key of the token mint
/// * `payer` - Keypair that will pay for the account creation and sign the transaction
///
/// # Returns
///
/// Returns the public key of the created associated token account.
///
/// # Errors
///
/// This function will return an error if the account creation fails.
///
/// # Example
///
/// ```rust
/// use solana_kite::{create_token_mint, create_associated_token_account, create_wallet};
/// use litesvm::LiteSVM;
/// use solana_keypair::Keypair;
/// use solana_signer::Signer;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut litesvm = LiteSVM::new();
/// let owner_wallet = create_wallet(&mut litesvm, 1_000_000_000)?;
/// let payer_wallet = create_wallet(&mut litesvm, 1_000_000_000)?;
/// let mint_authority = create_wallet(&mut litesvm, 1_000_000_000)?;
/// let mint_pubkey = create_token_mint(&mut litesvm, &mint_authority, 6, None)?;
///
/// let token_account = create_associated_token_account(
///     &mut litesvm,
///     &owner_wallet.pubkey(),
///     &mint_pubkey,
///     &payer_wallet,
/// )?;
/// # Ok(())
/// # }
/// ```
pub fn create_associated_token_account(
    litesvm: &mut LiteSVM,
    owner: &Pubkey,
    mint: &Pubkey,
    payer: &Keypair,
) -> Result<Pubkey, SolanaKiteError> {
    let associated_token_account =
        spl_associated_token_account::get_associated_token_address(owner, mint);

    let instruction = create_ata_instruction(&payer.pubkey(), owner, mint, &spl_token::ID);

    send_transaction_from_instructions(litesvm, vec![instruction], &[payer], &payer.pubkey())?;

    Ok(associated_token_account)
}

/// Mints tokens to a specified token account.
///
/// # Arguments
///
/// * `litesvm` - Mutable reference to the LiteSVM instance
/// * `mint` - Public key of the token mint
/// * `token_account` - Public key of the destination token account
/// * `amount` - Number of tokens to mint (in base units)
/// * `mint_authority` - Keypair with mint authority
///
/// # Errors
///
/// This function will return an error if the minting transaction fails.
///
/// # Example
///
/// ```rust
/// use solana_kite::{create_token_mint, create_associated_token_account, mint_tokens_to_token_account, create_wallet};
/// use litesvm::LiteSVM;
/// use solana_signer::Signer;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut litesvm = LiteSVM::new();
/// let mint_authority = create_wallet(&mut litesvm, 1_000_000_000)?;
/// let owner = create_wallet(&mut litesvm, 1_000_000_000)?;
/// let mint = create_token_mint(&mut litesvm, &mint_authority, 6, None)?;
/// let token_account = create_associated_token_account(&mut litesvm, &owner.pubkey(), &mint, &owner)?;
///
/// mint_tokens_to_token_account(
///     &mut litesvm,
///     &mint,
///     &token_account,
///     1_000_000, // 1 token with 6 decimals
///     &mint_authority,
/// )?;
/// # Ok(())
/// # }
/// ```
pub fn mint_tokens_to_token_account(
    litesvm: &mut LiteSVM,
    mint: &Pubkey,
    token_account: &Pubkey,
    amount: u64,
    mint_authority: &Keypair,
) -> Result<(), SolanaKiteError> {
    let instruction = mint_to(
        &spl_token::ID,
        mint,
        token_account,
        &mint_authority.pubkey(),
        &[],
        amount,
    )
    .map_err(|e| {
        SolanaKiteError::TokenOperationFailed(format!("Failed to create mint_to instruction: {e}"))
    })?;

    send_transaction_from_instructions(
        litesvm,
        vec![instruction],
        &[mint_authority],
        &mint_authority.pubkey(),
    )?;

    Ok(())
}

/// Gets the token balance of a token account.
///
/// Works for both Classic Token Program and Token Extensions accounts — both
/// share the same base account layout, with the amount at the same byte offset.
///
/// # Arguments
///
/// * `litesvm` - Reference to the LiteSVM instance
/// * `token_account` - Public key of the token account to query
///
/// # Returns
///
/// Returns the token balance as a u64 in base units.
///
/// # Errors
///
/// This function will return an error if the token account doesn't exist or
/// the balance cannot be parsed.
///
/// # Example
///
/// ```rust
/// use solana_kite::{create_token_mint, create_associated_token_account,
///     get_token_account_balance, create_wallet};
/// use litesvm::LiteSVM;
/// use solana_signer::Signer;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut litesvm = LiteSVM::new();
/// let authority = create_wallet(&mut litesvm, 1_000_000_000)?;
/// let mint = create_token_mint(&mut litesvm, &authority, 6, None)?;
/// let ata = create_associated_token_account(&mut litesvm, &authority.pubkey(), &mint, &authority)?;
/// let balance = get_token_account_balance(&litesvm, &ata)?;
/// assert_eq!(balance, 0);
/// # Ok(())
/// # }
/// ```
#[must_use = "call assert_token_account_balance if you want a panicking assertion"]
pub fn get_token_account_balance(
    litesvm: &LiteSVM,
    token_account: &Pubkey,
) -> Result<u64, SolanaKiteError> {
    let account = litesvm.get_account(token_account).ok_or_else(|| {
        SolanaKiteError::TokenOperationFailed("Token account not found".to_string())
    })?;

    let data = &account.data;
    if data.len() < TOKEN_ACCOUNT_AMOUNT_END {
        return Err(SolanaKiteError::TokenOperationFailed(
            "Invalid token account data length".to_string(),
        ));
    }

    let amount = u64::from_le_bytes(
        data[TOKEN_ACCOUNT_AMOUNT_OFFSET..TOKEN_ACCOUNT_AMOUNT_END]
            .try_into()
            .map_err(|_| {
                SolanaKiteError::TokenOperationFailed("Failed to parse token amount".to_string())
            })?,
    );

    Ok(amount)
}

/// Derives the associated token account address for a Classic Token Program mint.
///
/// Useful for pre-computing the address before calling
/// [`create_associated_token_account`], e.g. to pass it to a program instruction
/// before the account exists onchain.
///
/// # Example
///
/// ```rust
/// use solana_kite::{create_token_mint, create_associated_token_account,
///     get_token_account_address, create_wallet};
/// use litesvm::LiteSVM;
/// use solana_signer::Signer;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut litesvm = LiteSVM::new();
/// let authority = create_wallet(&mut litesvm, 1_000_000_000)?;
/// let mint = create_token_mint(&mut litesvm, &authority, 6, None)?;
///
/// let predicted = get_token_account_address(&authority.pubkey(), &mint);
/// let actual = create_associated_token_account(&mut litesvm, &authority.pubkey(), &mint, &authority)?;
/// assert_eq!(predicted, actual);
/// # Ok(())
/// # }
/// ```
#[must_use]
pub fn get_token_account_address(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    spl_associated_token_account::get_associated_token_address(owner, mint)
}

/// Asserts that a token account has the expected balance.
///
/// Works for both Classic Token Program and Token Extensions accounts.
/// Convenience wrapper around [`get_token_account_balance`] for test assertions.
///
/// # Panics
///
/// Panics if the actual balance doesn't match the expected balance, with the provided message.
///
/// # Example
///
/// ```rust
/// use solana_kite::{create_wallet, create_token_mint, create_associated_token_account,
///     mint_tokens_to_token_account, assert_token_account_balance};
/// use litesvm::LiteSVM;
/// use solana_signer::Signer;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut litesvm = LiteSVM::new();
/// let authority = create_wallet(&mut litesvm, 1_000_000_000)?;
/// let mint = create_token_mint(&mut litesvm, &authority, 6, None)?;
/// let ata = create_associated_token_account(&mut litesvm, &authority.pubkey(), &mint, &authority)?;
/// mint_tokens_to_token_account(&mut litesvm, &mint, &ata, 1_000_000, &authority)?;
/// assert_token_account_balance(&litesvm, &ata, 1_000_000, "Should have 1 token");
/// # Ok(())
/// # }
/// ```
pub fn assert_token_account_balance(
    litesvm: &LiteSVM,
    token_account: &Pubkey,
    expected_balance: u64,
    message: &str,
) {
    let actual_balance = get_token_account_balance(litesvm, token_account)
        .unwrap_or_else(|e| panic!("{}: {}", message, e));
    assert_eq!(actual_balance, expected_balance, "{}", message);
}
