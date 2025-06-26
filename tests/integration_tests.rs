//! Integration tests for Solana Kite

use litesvm::LiteSVM;
use solana_kite::{
    create_wallet, create_wallets, create_token_mint, create_associated_token_account,
    mint_tokens_to_account, get_token_account_balance, assert_token_balance,
    send_transaction_from_instructions, get_pda_and_bump, seeds, Seed, check_account_is_closed,
};
use solana_pubkey::Pubkey;
use solana_signer::Signer;

#[test]
fn test_wallet_creation() {
    let mut litesvm = LiteSVM::new();
    
    // Test single wallet creation
    let wallet = create_wallet(&mut litesvm, 1_000_000_000).unwrap();
    let balance = litesvm.get_balance(&wallet.pubkey()).unwrap();
    assert_eq!(balance, 1_000_000_000);
    
    // Test multiple wallet creation
    let wallets = create_wallets(&mut litesvm, 3, 500_000_000).unwrap();
    assert_eq!(wallets.len(), 3);
    
    for wallet in &wallets {
        let balance = litesvm.get_balance(&wallet.pubkey()).unwrap();
        assert_eq!(balance, 500_000_000);
    }
}

#[test]
fn test_token_operations() {
    let mut litesvm = LiteSVM::new();
    
    // Create test accounts
    let mint_authority = create_wallet(&mut litesvm, 1_000_000_000).unwrap();
    let user = create_wallet(&mut litesvm, 1_000_000_000).unwrap();
    
    // Create token mint
    let mint = create_token_mint(&mut litesvm, &mint_authority, 9).unwrap();
    
    // Create associated token account
    let token_account = create_associated_token_account(
        &mut litesvm,
        &user,
        &mint.pubkey(),
        &user,
    ).unwrap();
    
    // Check initial balance
    let initial_balance = get_token_account_balance(&litesvm, &token_account).unwrap();
    assert_eq!(initial_balance, 0);
    
    // Mint tokens
    let mint_amount = 1_000_000_000;
    mint_tokens_to_account(
        &mut litesvm,
        &mint.pubkey(),
        &token_account,
        mint_amount,
        &mint_authority,
    ).unwrap();
    
    // Verify balance
    let final_balance = get_token_account_balance(&litesvm, &token_account).unwrap();
    assert_eq!(final_balance, mint_amount);
    
    // Test assertion helper
    assert_token_balance(&litesvm, &token_account, mint_amount, "Balance should match");
}

#[test]
fn test_transaction_sending() {
    let mut litesvm = LiteSVM::new();
    let wallet = create_wallet(&mut litesvm, 1_000_000_000).unwrap();
    
    // Send empty transaction (should succeed)
    let instructions = vec![];
    let result = send_transaction_from_instructions(
        &mut litesvm,
        instructions,
        &[&wallet],
        &wallet.pubkey(),
    );
    
    assert!(result.is_ok());
}

#[test]
fn test_pda_generation() {
    let program_id = Pubkey::new_unique();
    let user_address = Pubkey::new_unique();
    
    // Test with different seed types
    let seeds_vec = seeds!["user-account", user_address, 42u64];
    let (pda1, bump1) = get_pda_and_bump(&seeds_vec, &program_id);
    
    // Test manual seed creation
    let manual_seeds = vec![
        Seed::String("user-account".to_string()),
        Seed::Address(user_address),
        Seed::U64(42),
    ];
    let (pda2, bump2) = get_pda_and_bump(&manual_seeds, &program_id);
    
    // Should be identical
    assert_eq!(pda1, pda2);
    assert_eq!(bump1, bump2);
    
    // Test with bytes
    let bytes_seeds = seeds![b"prefix".as_slice(), 123u64];
    let (pda3, _bump3) = get_pda_and_bump(&bytes_seeds, &program_id);
    
    // Should be different from the first PDA
    assert_ne!(pda1, pda3);
}

#[test]
fn test_account_closure_check() {
    let litesvm = LiteSVM::new();
    let non_existent_account = Pubkey::new_unique();
    
    // Should not panic for non-existent account
    check_account_is_closed(&litesvm, &non_existent_account, "Account should be closed");
}

#[test]
fn test_seed_conversions() {
    // Test all From implementations
    let _str_seed: Seed = "test".into();
    let _string_seed: Seed = "test".to_string().into();
    let _u64_seed: Seed = 42u64.into();
    let _pubkey_seed: Seed = Pubkey::new_unique().into();
    let _bytes_seed: Seed = vec![1, 2, 3].into();
    let _slice_seed: Seed = [1, 2, 3].as_slice().into();
}

#[test]
fn test_multiple_token_mints() {
    let mut litesvm = LiteSVM::new();
    
    let mint_authority = create_wallet(&mut litesvm, 2_000_000_000).unwrap();
    let user = create_wallet(&mut litesvm, 1_000_000_000).unwrap();
    
    // Create multiple token mints with different decimals
    let mint_6_decimals = create_token_mint(&mut litesvm, &mint_authority, 6).unwrap();
    let mint_9_decimals = create_token_mint(&mut litesvm, &mint_authority, 9).unwrap();
    
    // Create token accounts for each mint
    let account_6 = create_associated_token_account(
        &mut litesvm,
        &user,
        &mint_6_decimals.pubkey(),
        &user,
    ).unwrap();
    
    let account_9 = create_associated_token_account(
        &mut litesvm,
        &user,
        &mint_9_decimals.pubkey(),
        &user,
    ).unwrap();
    
    // Mint different amounts to each account
    mint_tokens_to_account(
        &mut litesvm,
        &mint_6_decimals.pubkey(),
        &account_6,
        1_000_000, // 1 token with 6 decimals
        &mint_authority,
    ).unwrap();
    
    mint_tokens_to_account(
        &mut litesvm,
        &mint_9_decimals.pubkey(),
        &account_9,
        1_000_000_000, // 1 token with 9 decimals
        &mint_authority,
    ).unwrap();
    
    // Verify balances
    assert_token_balance(&litesvm, &account_6, 1_000_000, "6-decimal token balance");
    assert_token_balance(&litesvm, &account_9, 1_000_000_000, "9-decimal token balance");
}