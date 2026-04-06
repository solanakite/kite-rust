//! Integration tests for Solana Kite

use litesvm::LiteSVM;
use solana_kite::{
    assert_sol_balance, assert_token_account_balance, build_hook_accounts, check_account_is_closed,
    create_associated_token_account, create_token_mint, create_wallet, create_wallets,
    get_hook_accounts_address, get_pda_and_bump, get_sol_balance, get_token_account_address,
    get_token_account_balance, mint_tokens_to_token_account, seeds, send_transaction_from_instructions,
    HookAccount, Seed,
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
fn test_sol_balance_helpers() {
    let mut litesvm = LiteSVM::new();

    let wallet = create_wallet(&mut litesvm, 1_000_000_000).unwrap();
    assert_eq!(get_sol_balance(&litesvm, &wallet.pubkey()), 1_000_000_000);
    assert_sol_balance(&litesvm, &wallet.pubkey(), 1_000_000_000, "Should have 1 SOL");

    // Non-existent account returns 0
    let missing = Pubkey::new_unique();
    assert_eq!(get_sol_balance(&litesvm, &missing), 0);
}

#[test]
fn test_token_operations() {
    let mut litesvm = LiteSVM::new();

    let mint_authority = create_wallet(&mut litesvm, 1_000_000_000).unwrap();
    let user = create_wallet(&mut litesvm, 1_000_000_000).unwrap();

    let mint = create_token_mint(&mut litesvm, &mint_authority, 9, None).unwrap();

    let token_account = create_associated_token_account(
        &mut litesvm,
        &user.pubkey(),
        &mint,
        &user,
    )
    .unwrap();

    let initial_balance = get_token_account_balance(&litesvm, &token_account).unwrap();
    assert_eq!(initial_balance, 0);

    let mint_amount = 1_000_000_000;
    mint_tokens_to_token_account(
        &mut litesvm,
        &mint,
        &token_account,
        mint_amount,
        &mint_authority,
    )
    .unwrap();

    let final_balance = get_token_account_balance(&litesvm, &token_account).unwrap();
    assert_eq!(final_balance, mint_amount);

    assert_token_account_balance(&litesvm, &token_account, mint_amount, "Balance should match");
}

#[test]
fn test_get_token_account_address() {
    let mut litesvm = LiteSVM::new();
    let authority = create_wallet(&mut litesvm, 1_000_000_000).unwrap();
    let mint = create_token_mint(&mut litesvm, &authority, 6, None).unwrap();

    let predicted = get_token_account_address(&authority.pubkey(), &mint);
    let actual = create_associated_token_account(&mut litesvm, &authority.pubkey(), &mint, &authority).unwrap();
    assert_eq!(predicted, actual);
}

#[test]
fn test_transaction_sending() {
    let mut litesvm = LiteSVM::new();
    let wallet = create_wallet(&mut litesvm, 1_000_000_000).unwrap();

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

    let seeds_vec = seeds!["user-account", user_address, 42u64];
    let (pda1, bump1) = get_pda_and_bump(&seeds_vec, &program_id);

    let manual_seeds = vec![
        Seed::String("user-account".to_string()),
        Seed::Address(user_address),
        Seed::U64(42),
    ];
    let (pda2, bump2) = get_pda_and_bump(&manual_seeds, &program_id);

    assert_eq!(pda1, pda2);
    assert_eq!(bump1, bump2);

    let bytes_seeds = seeds![b"prefix".as_slice(), 123u64];
    let (pda3, _bump3) = get_pda_and_bump(&bytes_seeds, &program_id);

    assert_ne!(pda1, pda3);
}

#[test]
fn test_account_closure_check() {
    let litesvm = LiteSVM::new();
    let non_existent_account = Pubkey::new_unique();
    check_account_is_closed(&litesvm, &non_existent_account, "Account should be closed");
}

#[test]
#[should_panic(expected = "Account should still be open")]
fn test_account_closure_check_panics_for_open_account() {
    let mut litesvm = LiteSVM::new();
    let authority = create_wallet(&mut litesvm, 1_000_000_000).unwrap();
    // A mint account has non-empty data, so check_account_is_closed should panic
    let mint = create_token_mint(&mut litesvm, &authority, 6, None).unwrap();
    check_account_is_closed(&litesvm, &mint, "Account should still be open");
}

#[test]
fn test_hook_account_helpers() {
    let mint = Pubkey::new_unique();
    let hook_program = Pubkey::new_unique();

    let pda = get_hook_accounts_address(&mint, &hook_program);
    assert_ne!(pda, mint);
    assert_ne!(pda, hook_program);

    // Deterministic — same inputs give same address
    assert_eq!(get_hook_accounts_address(&mint, &hook_program), pda);

    // build_hook_accounts always appends hook program + PDA + Token Extensions program
    let accounts = build_hook_accounts(&mint, &hook_program, &[]);
    assert_eq!(accounts.len(), 3);

    let state_account = Pubkey::new_unique();
    let accounts_with_extra = build_hook_accounts(
        &mint,
        &hook_program,
        &[HookAccount { pubkey: state_account, is_signer: false, is_writable: true }],
    );
    assert_eq!(accounts_with_extra.len(), 4);
    assert_eq!(accounts_with_extra[0].pubkey, state_account);
}

#[test]
fn test_multiple_token_mints() {
    let mut litesvm = LiteSVM::new();

    let mint_authority = create_wallet(&mut litesvm, 2_000_000_000).unwrap();
    let user = create_wallet(&mut litesvm, 1_000_000_000).unwrap();

    let specified_mint = Pubkey::new_unique();
    let mint_6 = create_token_mint(&mut litesvm, &mint_authority, 6, Some(specified_mint)).unwrap();
    let mint_9 = create_token_mint(&mut litesvm, &mint_authority, 9, None).unwrap();

    assert_eq!(mint_6, specified_mint);

    let account_6 =
        create_associated_token_account(&mut litesvm, &user.pubkey(), &mint_6, &user).unwrap();
    let account_9 =
        create_associated_token_account(&mut litesvm, &user.pubkey(), &mint_9, &user).unwrap();

    mint_tokens_to_token_account(&mut litesvm, &mint_6, &account_6, 1_000_000, &mint_authority).unwrap();
    mint_tokens_to_token_account(
        &mut litesvm,
        &mint_9,
        &account_9,
        1_000_000_000,
        &mint_authority,
    )
    .unwrap();

    assert_token_account_balance(&litesvm, &account_6, 1_000_000, "6-decimal token balance");
    assert_token_account_balance(
        &litesvm,
        &account_9,
        1_000_000_000,
        "9-decimal token balance",
    );
}

// ─── Token Extensions Tests ────────────────────────────────────────────────────────

mod token_extensions_tests {
    use litesvm::LiteSVM;
    use solana_kite::token_extensions::{
        create_token_extensions_account, create_token_extensions_mint,
        get_token_extensions_account_address, mint_tokens_to_token_extensions_account,
        transfer_checked_token_extensions, MintExtension, TokenAccountState,
    };
    use solana_kite::{assert_token_account_balance, create_wallet, get_token_account_balance};
    use solana_signer::Signer;

    #[test]
    fn test_create_mint_with_close_authority() {
        let mut litesvm = LiteSVM::new();
        let authority = create_wallet(&mut litesvm, 1_000_000_000).unwrap();

        let mint = create_token_extensions_mint(
            &mut litesvm,
            &authority,
            6,
            None,
        &[MintExtension::MintCloseAuthority {
                close_authority: authority.pubkey(),
            }],
        )
        .unwrap();

        // Verify the mint account exists and is owned by Token Extensions
        let account = litesvm.get_account(&mint).unwrap();
        assert_eq!(
            account.owner,
            solana_kite::token_extensions::TOKEN_EXTENSIONS_PROGRAM_ID
        );
    }

    #[test]
    fn test_create_mint_with_permanent_delegate() {
        let mut litesvm = LiteSVM::new();
        let authority = create_wallet(&mut litesvm, 1_000_000_000).unwrap();

        let mint = create_token_extensions_mint(
            &mut litesvm,
            &authority,
            9,
            None,
        &[MintExtension::PermanentDelegate {
                delegate: authority.pubkey(),
            }],
        )
        .unwrap();

        let account = litesvm.get_account(&mint).unwrap();
        assert_eq!(
            account.owner,
            solana_kite::token_extensions::TOKEN_EXTENSIONS_PROGRAM_ID
        );
    }

    #[test]
    fn test_create_non_transferable_mint() {
        let mut litesvm = LiteSVM::new();
        let authority = create_wallet(&mut litesvm, 1_000_000_000).unwrap();

        let mint = create_token_extensions_mint(
            &mut litesvm,
            &authority,
            0,
            None,
        &[MintExtension::NonTransferable],
        )
        .unwrap();

        let account = litesvm.get_account(&mint).unwrap();
        assert_eq!(
            account.owner,
            solana_kite::token_extensions::TOKEN_EXTENSIONS_PROGRAM_ID
        );
    }

    #[test]
    fn test_create_mint_with_multiple_extensions() {
        let mut litesvm = LiteSVM::new();
        let authority = create_wallet(&mut litesvm, 1_000_000_000).unwrap();

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

        let account = litesvm.get_account(&mint).unwrap();
        assert_eq!(
            account.owner,
            solana_kite::token_extensions::TOKEN_EXTENSIONS_PROGRAM_ID
        );
    }

    #[test]
    fn test_token_extensions_mint_and_transfer() {
        let mut litesvm = LiteSVM::new();
        let authority = create_wallet(&mut litesvm, 2_000_000_000).unwrap();
        let user = create_wallet(&mut litesvm, 1_000_000_000).unwrap();

        // Create mint with close authority extension
        let mint = create_token_extensions_mint(
            &mut litesvm,
            &authority,
            6,
            None,
        &[MintExtension::MintCloseAuthority {
                close_authority: authority.pubkey(),
            }],
        )
        .unwrap();

        // Create token accounts
        let authority_ata =
            create_token_extensions_account(&mut litesvm, &authority.pubkey(), &mint, &authority)
                .unwrap();
        let user_ata =
            create_token_extensions_account(&mut litesvm, &user.pubkey(), &mint, &user).unwrap();

        // Mint tokens to authority's account
        let mint_amount = 1_000_000; // 1 token (6 decimals)
        mint_tokens_to_token_extensions_account(
            &mut litesvm,
            &mint,
            &authority_ata,
            mint_amount,
            &authority,
        )
        .unwrap();

        assert_token_account_balance(
            &litesvm,
            &authority_ata,
            mint_amount,
            "Authority should have minted tokens",
        );

        // Transfer half to user
        let transfer_amount = 500_000;
        transfer_checked_token_extensions(
            &mut litesvm,
            &authority_ata,
            &mint,
            &user_ata,
            &authority,
            transfer_amount,
            6, // decimals
            &[],
        )
        .unwrap();

        assert_token_account_balance(
            &litesvm,
            &authority_ata,
            mint_amount - transfer_amount,
            "Authority balance after transfer",
        );
        assert_token_account_balance(
            &litesvm,
            &user_ata,
            transfer_amount,
            "User balance after transfer",
        );
    }

    #[test]
    fn test_token_extensions_balance_helpers() {
        let mut litesvm = LiteSVM::new();
        let authority = create_wallet(&mut litesvm, 1_000_000_000).unwrap();

        let mint =
            create_token_extensions_mint(&mut litesvm, &authority, 6, None,
        &[MintExtension::NonTransferable])
                .unwrap();

        let ata =
            create_token_extensions_account(&mut litesvm, &authority.pubkey(), &mint, &authority)
                .unwrap();

        // Initial balance should be zero
        let balance = get_token_account_balance(&litesvm, &ata).unwrap();
        assert_eq!(balance, 0);

        // Mint and check
        mint_tokens_to_token_extensions_account(&mut litesvm, &mint, &ata, 42_000, &authority).unwrap();
        assert_token_account_balance(&litesvm, &ata, 42_000, "Should have 42000 tokens");
    }

    #[test]
    fn test_create_mint_with_transfer_fee() {
        let mut litesvm = LiteSVM::new();
        let authority = create_wallet(&mut litesvm, 2_000_000_000).unwrap();

        // 1% fee, max 1000 base units
        let mint = create_token_extensions_mint(
            &mut litesvm,
            &authority,
            6,
            None,
        &[MintExtension::TransferFee {
                fee_basis_points: 100,
                maximum_fee: 1000,
            }],
        )
        .unwrap();

        let account = litesvm.get_account(&mint).unwrap();
        assert_eq!(
            account.owner,
            solana_kite::token_extensions::TOKEN_EXTENSIONS_PROGRAM_ID
        );
    }

    #[test]
    fn test_create_mint_with_metadata_pointer() {
        let mut litesvm = LiteSVM::new();
        let authority = create_wallet(&mut litesvm, 1_000_000_000).unwrap();
        let metadata_address = solana_pubkey::Pubkey::new_unique();

        let mint = create_token_extensions_mint(
            &mut litesvm,
            &authority,
            6,
            None,
        &[MintExtension::MetadataPointer {
                authority: authority.pubkey(),
                metadata_address,
            }],
        )
        .unwrap();

        let account = litesvm.get_account(&mint).unwrap();
        assert_eq!(
            account.owner,
            solana_kite::token_extensions::TOKEN_EXTENSIONS_PROGRAM_ID
        );
    }

    #[test]
    fn test_create_mint_with_interest_bearing() {
        let mut litesvm = LiteSVM::new();
        let authority = create_wallet(&mut litesvm, 1_000_000_000).unwrap();

        // 5% interest rate (500 basis points)
        let mint = create_token_extensions_mint(
            &mut litesvm,
            &authority,
            6,
            None,
        &[MintExtension::InterestBearing {
                rate_authority: authority.pubkey(),
                rate: 500,
            }],
        )
        .unwrap();

        let account = litesvm.get_account(&mint).unwrap();
        assert_eq!(
            account.owner,
            solana_kite::token_extensions::TOKEN_EXTENSIONS_PROGRAM_ID
        );
    }

    #[test]
    fn test_create_mint_with_default_account_state() {
        let mut litesvm = LiteSVM::new();
        let authority = create_wallet(&mut litesvm, 1_000_000_000).unwrap();

        // Default state: Initialized (1)
        // Note: Frozen (2) requires a freeze authority on the mint,
        // which create_token_extensions_mint doesn't set by default.
        let mint = create_token_extensions_mint(
            &mut litesvm,
            &authority,
            6,
            None,
        &[MintExtension::DefaultAccountState { initial_state: TokenAccountState::Initialized }],
        )
        .unwrap();

        let account = litesvm.get_account(&mint).unwrap();
        assert_eq!(
            account.owner,
            solana_kite::token_extensions::TOKEN_EXTENSIONS_PROGRAM_ID
        );
    }

    #[test]
    fn test_create_mint_with_transfer_hook() {
        let mut litesvm = LiteSVM::new();
        let authority = create_wallet(&mut litesvm, 1_000_000_000).unwrap();
        let hook_program = solana_pubkey::Pubkey::new_unique();

        let mint = create_token_extensions_mint(
            &mut litesvm,
            &authority,
            6,
            None,
        &[MintExtension::TransferHook {
                program_id: hook_program,
            }],
        )
        .unwrap();

        let account = litesvm.get_account(&mint).unwrap();
        assert_eq!(
            account.owner,
            solana_kite::token_extensions::TOKEN_EXTENSIONS_PROGRAM_ID
        );
    }

    #[test]
    fn test_get_token_extensions_account_address() {
        let mut litesvm = LiteSVM::new();
        let authority = create_wallet(&mut litesvm, 1_000_000_000).unwrap();
        let mint = create_token_extensions_mint(&mut litesvm, &authority, 6, None,
        &[]).unwrap();

        let predicted = get_token_extensions_account_address(&authority.pubkey(), &mint);
        let actual = create_token_extensions_account(&mut litesvm, &authority.pubkey(), &mint, &authority).unwrap();

        assert_eq!(predicted, actual);
    }
}
