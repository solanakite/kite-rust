//! Example: Token Extensions operations with Solana Kite
//!
//! Demonstrates creating Token Extensions mints with extensions, creating token accounts,
//! minting tokens, and transferring between accounts.

use litesvm::LiteSVM;
use solana_kite::assert_token_account_balance;
use solana_kite::create_wallet;
use solana_kite::token_extensions::{
    create_token_extensions_account, create_token_extensions_mint,
    mint_tokens_to_token_extensions_account, transfer_checked_token_extensions, MintExtension,
};
use solana_signer::Signer;

fn main() {
    let mut litesvm = LiteSVM::new();

    // Create wallets
    let authority = create_wallet(&mut litesvm, 2_000_000_000).unwrap();
    let user = create_wallet(&mut litesvm, 1_000_000_000).unwrap();

    // Create a Token Extensions mint with MintCloseAuthority and PermanentDelegate extensions
    let mint = create_token_extensions_mint(
        &mut litesvm,
        &authority,
        6,
        None,
        &[
            MintExtension::MintCloseAuthority {
                close_authority: authority.pubkey(),
            },
            MintExtension::PermanentDelegate {
                delegate: authority.pubkey(),
            },
        ],
    )
    .unwrap();
    println!("Created Token Extensions mint: {}", mint);

    // Create associated token accounts
    let authority_ata =
        create_token_extensions_account(&mut litesvm, &authority.pubkey(), &mint, &authority)
            .unwrap();
    let user_ata =
        create_token_extensions_account(&mut litesvm, &user.pubkey(), &mint, &user).unwrap();
    println!("Authority ATA: {}", authority_ata);
    println!("User ATA:      {}", user_ata);

    // Mint 1,000,000 tokens (with 6 decimals = 1.0 tokens)
    let mint_amount = 1_000_000;
    mint_tokens_to_token_extensions_account(
        &mut litesvm,
        &mint,
        &authority_ata,
        mint_amount,
        &authority,
    )
    .unwrap();
    println!("Minted {} tokens to authority", mint_amount);

    // Transfer 250,000 tokens to user
    let transfer_amount = 250_000;
    transfer_checked_token_extensions(
        &mut litesvm,
        &authority_ata,
        &mint,
        &user_ata,
        &authority,
        transfer_amount,
        6,
        &[], // no extra accounts needed (no transfer hook)
    )
    .unwrap();
    println!("Transferred {} tokens to user", transfer_amount);

    // Verify final balances
    assert_token_account_balance(
        &litesvm,
        &authority_ata,
        750_000,
        "Authority should have 750,000",
    );
    assert_token_account_balance(&litesvm, &user_ata, 250_000, "User should have 250,000");

    println!("All Token Extensions operations completed successfully!");
}
