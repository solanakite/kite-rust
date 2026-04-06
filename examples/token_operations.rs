//! Token operations example for Solana Kite
//! 
//! This example demonstrates token-related functionality including:
//! - Creating token mints
//! - Creating associated token accounts
//! - Minting tokens
//! - Checking token balances

use litesvm::LiteSVM;
use solana_kite::{
    create_wallet, create_token_mint, create_associated_token_account,
    mint_tokens_to_token_account, get_token_account_balance, assert_token_account_balance,
    SolanaKiteError,
};
use solana_signer::Signer;

fn main() -> Result<(), SolanaKiteError> {
    println!("🪁 Solana Kite Token Operations Example");
    println!("=======================================");

    // Initialize the LiteSVM test environment
    let mut litesvm = LiteSVM::new();
    println!("✅ LiteSVM initialized");

    // Create wallets
    let mint_authority = create_wallet(&mut litesvm, 1_000_000_000)?;
    let user = create_wallet(&mut litesvm, 1_000_000_000)?;
    println!("✅ Created mint authority: {}", mint_authority.pubkey());
    println!("✅ Created user wallet: {}", user.pubkey());

    // Create a token mint with 6 decimals (like USDC)
    let mint = create_token_mint(&mut litesvm, &mint_authority, 6, None)?;
    println!("✅ Created token mint: {}", mint);
    println!("   Decimals: 6");
    println!("   Mint authority: {}", mint_authority.pubkey());

    // Create an associated token account for the user
    let user_token_account = create_associated_token_account(
        &mut litesvm,
        &user.pubkey(),
        &mint,
        &user,
    )?;
    println!("✅ Created associated token account: {}", user_token_account);

    // Check initial balance (should be 0)
    let initial_balance = get_token_account_balance(&litesvm, &user_token_account)?;
    println!("✅ Initial token balance: {} (raw units)", initial_balance);
    assert_eq!(initial_balance, 0);

    // Mint 1000 tokens (1000 * 10^6 = 1,000,000,000 base units)
    let mint_amount = 1_000_000_000; // 1000 tokens with 6 decimals
    mint_tokens_to_token_account(
        &mut litesvm,
        &mint,
        &user_token_account,
        mint_amount,
        &mint_authority,
    )?;
    println!("✅ Minted {} base units to user account", mint_amount);

    // Check the balance after minting
    let final_balance = get_token_account_balance(&litesvm, &user_token_account)?;
    println!("✅ Final token balance: {} base units", final_balance);
    println!("   That's {} tokens (with 6 decimals)", final_balance as f64 / 1_000_000.0);

    // Assert the balance is correct
    assert_token_account_balance(&litesvm, &user_token_account, mint_amount, "Balance should match minted amount");
    println!("✅ Balance assertion passed");

    // Mint more tokens to demonstrate cumulative balance
    let additional_mint = 500_000_000; // 500 more tokens
    mint_tokens_to_token_account(
        &mut litesvm,
        &mint,
        &user_token_account,
        additional_mint,
        &mint_authority,
    )?;
    println!("✅ Minted additional {} base units", additional_mint);

    let total_balance = get_token_account_balance(&litesvm, &user_token_account)?;
    let expected_total = mint_amount + additional_mint;
    println!("✅ Total balance after second mint: {} base units", total_balance);
    println!("   That's {} tokens (with 6 decimals)", total_balance as f64 / 1_000_000.0);
    
    assert_token_account_balance(&litesvm, &user_token_account, expected_total, "Total balance should be cumulative");
    println!("✅ Cumulative balance assertion passed");

    println!("🎉 Token operations example completed successfully!");
    println!("📊 Summary:");
    println!("   - Created 1 token mint with 6 decimals");
    println!("   - Created 1 associated token account");
    println!("   - Performed 2 mint operations");
    println!("   - Final balance: {} tokens", total_balance as f64 / 1_000_000.0);

    Ok(())
}