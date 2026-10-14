//! Integration tests for Solana Kite

use litesvm::LiteSVM;
use solana_compute_budget::compute_budget_limits::MAX_COMPUTE_UNIT_LIMIT;
use solana_compute_budget_interface::ComputeBudgetInstruction;
use solana_instruction::Instruction;
use solana_kite::{
    assert_sol_balance, assert_token_account_balance, build_hook_accounts,
    build_transaction_from_instructions, check_account_is_closed, create_associated_token_account,
    create_token_mint, create_wallet, create_wallets, get_hook_accounts_address, get_pda_and_bump,
    get_sol_balance, get_token_account_address, get_token_account_balance, get_transaction_config,
    mint_tokens_to_token_account, seeds, send_transaction_from_instructions, HookAccount, Seed,
    SolanaKiteError,
};
use solana_message::{v1, VersionedMessage};
use solana_packet::PACKET_DATA_SIZE;
use solana_system_interface::instruction::transfer;

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
    assert_sol_balance(
        &litesvm,
        &wallet.pubkey(),
        1_000_000_000,
        "Should have 1 SOL",
    );

    // Non-existent account returns 0
    let missing = Pubkey::new_unique();
    assert_eq!(get_sol_balance(&litesvm, &missing), 0);
}

#[test]
fn test_token_operations() {
    let mut litesvm = LiteSVM::new();

    let mint_authority = create_wallet(&mut litesvm, 1_000_000_000).unwrap();
    let user = create_wallet(&mut litesvm, 1_000_000_000).unwrap();

    let mint = create_token_mint(&mut litesvm, &mint_authority, 9, None, None).unwrap();

    let token_account =
        create_associated_token_account(&mut litesvm, &user.pubkey(), &mint, &user, None).unwrap();

    let initial_balance = get_token_account_balance(&litesvm, &token_account).unwrap();
    assert_eq!(initial_balance, 0);

    let mint_amount = 1_000_000_000;
    mint_tokens_to_token_account(
        &mut litesvm,
        &mint,
        &token_account,
        mint_amount,
        &mint_authority,
        None,
    )
    .unwrap();

    let final_balance = get_token_account_balance(&litesvm, &token_account).unwrap();
    assert_eq!(final_balance, mint_amount);

    assert_token_account_balance(
        &litesvm,
        &token_account,
        mint_amount,
        "Balance should match",
    );
}

#[test]
fn test_get_token_account_address() {
    let mut litesvm = LiteSVM::new();
    let authority = create_wallet(&mut litesvm, 1_000_000_000).unwrap();
    let mint = create_token_mint(&mut litesvm, &authority, 6, None, None).unwrap();

    let predicted = get_token_account_address(&authority.pubkey(), &mint);
    let actual =
        create_associated_token_account(&mut litesvm, &authority.pubkey(), &mint, &authority, None)
            .unwrap();
    assert_eq!(predicted, actual);
}

#[test]
fn test_transaction_sending() {
    let mut litesvm = LiteSVM::new();
    let wallet = create_wallet(&mut litesvm, 1_000_000_000).unwrap();

    // A transaction with no instructions uses no compute units, so a limit of
    // 0 is enough.
    let instructions = vec![];
    let result = send_transaction_from_instructions(
        &mut litesvm,
        instructions,
        &[&wallet],
        &wallet.pubkey(),
        Some(0),
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
    let mint = create_token_mint(&mut litesvm, &authority, 6, None, None).unwrap();
    check_account_is_closed(&litesvm, &mint, "Account should still be open");
}

#[test]
fn test_hook_account_helpers() {
    let mint = Pubkey::new_unique();
    let hook_program = Pubkey::new_unique();

    let pda = get_hook_accounts_address(&mint, &hook_program);
    assert_ne!(pda, mint);
    assert_ne!(pda, hook_program);

    // Deterministic - same inputs give same address
    assert_eq!(get_hook_accounts_address(&mint, &hook_program), pda);

    // build_hook_accounts always appends hook program + PDA + Token Extensions program
    let accounts = build_hook_accounts(&mint, &hook_program, &[]);
    assert_eq!(accounts.len(), 3);

    let state_account = Pubkey::new_unique();
    let accounts_with_extra = build_hook_accounts(
        &mint,
        &hook_program,
        &[HookAccount {
            pubkey: state_account,
            is_signer: false,
            is_writable: true,
        }],
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
    let mint_6 =
        create_token_mint(&mut litesvm, &mint_authority, 6, Some(specified_mint), None).unwrap();
    let mint_9 = create_token_mint(&mut litesvm, &mint_authority, 9, None, None).unwrap();

    assert_eq!(mint_6, specified_mint);

    let account_6 =
        create_associated_token_account(&mut litesvm, &user.pubkey(), &mint_6, &user, None)
            .unwrap();
    let account_9 =
        create_associated_token_account(&mut litesvm, &user.pubkey(), &mint_9, &user, None)
            .unwrap();

    mint_tokens_to_token_account(
        &mut litesvm,
        &mint_6,
        &account_6,
        1_000_000,
        &mint_authority,
        None,
    )
    .unwrap();
    mint_tokens_to_token_account(
        &mut litesvm,
        &mint_9,
        &account_9,
        1_000_000_000,
        &mint_authority,
        None,
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
            None,
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
            None,
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
            None,
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
            None,
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
            None,
        )
        .unwrap();

        // Create token accounts
        let authority_ata = create_token_extensions_account(
            &mut litesvm,
            &authority.pubkey(),
            &mint,
            &authority,
            None,
        )
        .unwrap();
        let user_ata =
            create_token_extensions_account(&mut litesvm, &user.pubkey(), &mint, &user, None)
                .unwrap();

        // Mint tokens to authority's account
        let mint_amount = 1_000_000; // 1 token (6 decimals)
        mint_tokens_to_token_extensions_account(
            &mut litesvm,
            &mint,
            &authority_ata,
            mint_amount,
            &authority,
            None,
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
            None,
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

        let mint = create_token_extensions_mint(
            &mut litesvm,
            &authority,
            6,
            None,
            &[MintExtension::NonTransferable],
            None,
        )
        .unwrap();

        let ata = create_token_extensions_account(
            &mut litesvm,
            &authority.pubkey(),
            &mint,
            &authority,
            None,
        )
        .unwrap();

        // Initial balance should be zero
        let balance = get_token_account_balance(&litesvm, &ata).unwrap();
        assert_eq!(balance, 0);

        // Mint and check
        mint_tokens_to_token_extensions_account(
            &mut litesvm,
            &mint,
            &ata,
            42_000,
            &authority,
            None,
        )
        .unwrap();
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
            None,
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
            None,
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
            None,
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
            &[MintExtension::DefaultAccountState {
                initial_state: TokenAccountState::Initialized,
            }],
            None,
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
            None,
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
        let mint =
            create_token_extensions_mint(&mut litesvm, &authority, 6, None, &[], None).unwrap();

        let predicted = get_token_extensions_account_address(&authority.pubkey(), &mint);
        let actual = create_token_extensions_account(
            &mut litesvm,
            &authority.pubkey(),
            &mint,
            &authority,
            None,
        )
        .unwrap();

        assert_eq!(predicted, actual);
    }
}

/// More compute units than a System Program transfer uses (150 in LiteSVM 0.16).
const TRANSFER_COMPUTE_UNIT_LIMIT: u32 = 1_000;

#[test]
fn test_get_transaction_config() {
    let config = get_transaction_config(50_000);
    assert_eq!(config.compute_unit_limit, Some(50_000));
    assert_eq!(
        config.loaded_accounts_data_size_limit,
        Some(64 * 1024 * 1024)
    );
    assert_eq!(config.heap_size, None);
    assert_eq!(config.priority_fee, None);
}

#[test]
fn test_build_transaction_from_instructions_builds_v1() {
    let mut litesvm = LiteSVM::new();
    let payer = create_wallet(&mut litesvm, 1_000_000_000).unwrap();
    let recipient = Pubkey::new_unique();
    let instructions = vec![transfer(&payer.pubkey(), &recipient, 1_000_000)];

    // Pass the payer twice, as callers do when the fee payer is also a named
    // signer. The transaction holds one signature for the payer.
    let transaction = build_transaction_from_instructions(
        &litesvm,
        &instructions,
        &[&payer, &payer],
        &payer.pubkey(),
        get_transaction_config(TRANSFER_COMPUTE_UNIT_LIMIT),
    )
    .unwrap();
    assert_eq!(transaction.signatures.len(), 1);
    let VersionedMessage::V1(message) = &transaction.message else {
        panic!("expected a v1 message");
    };
    assert_eq!(
        message.config,
        get_transaction_config(TRANSFER_COMPUTE_UNIT_LIMIT)
    );
    assert_eq!(wincode::serialize(&transaction).unwrap()[0], v1::V1_PREFIX);

    litesvm.send_transaction(transaction).unwrap();
    assert_eq!(get_sol_balance(&litesvm, &recipient), 1_000_000);
}

#[test]
fn test_build_transaction_from_instructions_rejects_missing_signer() {
    let mut litesvm = LiteSVM::new();
    let payer = create_wallet(&mut litesvm, 1_000_000_000).unwrap();
    let sender = create_wallet(&mut litesvm, 1_000_000_000).unwrap();
    let instructions = vec![transfer(&sender.pubkey(), &Pubkey::new_unique(), 1_000_000)];

    let result = build_transaction_from_instructions(
        &litesvm,
        &instructions,
        &[&payer],
        &payer.pubkey(),
        get_transaction_config(TRANSFER_COMPUTE_UNIT_LIMIT),
    );
    let Err(SolanaKiteError::TransactionFailed(message)) = result else {
        panic!("expected a TransactionFailed error, got {result:?}");
    };
    assert!(message.starts_with("Failed to sign transaction"));
}

#[test]
fn test_send_transaction_rejects_compute_budget_instruction() {
    let mut litesvm = LiteSVM::new();
    let payer = create_wallet(&mut litesvm, 1_000_000_000).unwrap();
    let recipient = Pubkey::new_unique();
    let instructions = vec![
        ComputeBudgetInstruction::set_compute_unit_limit(100_000),
        transfer(&payer.pubkey(), &recipient, 1_000_000),
    ];

    let result = send_transaction_from_instructions(
        &mut litesvm,
        instructions,
        &[&payer],
        &payer.pubkey(),
        Some(TRANSFER_COMPUTE_UNIT_LIMIT),
    );
    let Err(SolanaKiteError::TransactionFailed(message)) = result else {
        panic!("expected a TransactionFailed error, got {result:?}");
    };
    assert!(message.starts_with("ComputeBudget instructions do nothing in a v1 transaction"));

    // Nothing was sent.
    assert_eq!(get_sol_balance(&litesvm, &recipient), 0);
}

#[test]
fn test_build_transaction_from_instructions_applies_compute_unit_limit() {
    let mut litesvm = LiteSVM::new();
    let payer = create_wallet(&mut litesvm, 1_000_000_000).unwrap();
    let recipient = Pubkey::new_unique();
    let instructions = vec![transfer(&payer.pubkey(), &recipient, 1_000_000)];
    let send_with_compute_unit_limit = |litesvm: &mut LiteSVM, compute_unit_limit: u32| {
        let transaction = build_transaction_from_instructions(
            litesvm,
            &instructions,
            &[&payer],
            &payer.pubkey(),
            get_transaction_config(compute_unit_limit),
        )
        .unwrap();
        litesvm.send_transaction(transaction)
    };

    let transfer_compute_units = send_with_compute_unit_limit(&mut litesvm, MAX_COMPUTE_UNIT_LIMIT)
        .unwrap()
        .compute_units_consumed;
    let transfer_compute_units = u32::try_from(transfer_compute_units).unwrap();

    // One compute unit short of what the transfer costs fails, and moves no
    // lamports.
    assert!(send_with_compute_unit_limit(&mut litesvm, transfer_compute_units - 1).is_err());
    assert_eq!(get_sol_balance(&litesvm, &recipient), 1_000_000);

    // Exactly what the transfer costs succeeds.
    send_with_compute_unit_limit(&mut litesvm, transfer_compute_units).unwrap();
    assert_eq!(get_sol_balance(&litesvm, &recipient), 2_000_000);
}

#[test]
fn test_send_transaction_larger_than_legacy_limit() {
    let mut litesvm = LiteSVM::new();
    let payer = create_wallet(&mut litesvm, 1_000_000_000).unwrap();
    let recipients: Vec<Pubkey> = (0..30).map(|_| Pubkey::new_unique()).collect();
    let instructions: Vec<Instruction> = recipients
        .iter()
        .map(|recipient| transfer(&payer.pubkey(), recipient, 1_000_000))
        .collect();

    // Thirty transfers to thirty new accounts is too large for a legacy
    // transaction.
    let compute_unit_limit = 30 * TRANSFER_COMPUTE_UNIT_LIMIT;
    let transaction = build_transaction_from_instructions(
        &litesvm,
        &instructions,
        &[&payer],
        &payer.pubkey(),
        get_transaction_config(compute_unit_limit),
    )
    .unwrap();
    assert!(wincode::serialize(&transaction).unwrap().len() > PACKET_DATA_SIZE);

    send_transaction_from_instructions(
        &mut litesvm,
        instructions,
        &[&payer],
        &payer.pubkey(),
        Some(compute_unit_limit),
    )
    .unwrap();
    for recipient in &recipients {
        assert_eq!(get_sol_balance(&litesvm, recipient), 1_000_000);
    }
}

#[test]
fn test_helper_with_too_low_compute_unit_limit_fails() {
    let mut litesvm = LiteSVM::new();
    let authority = create_wallet(&mut litesvm, 1_000_000_000).unwrap();
    let mint = create_token_mint(&mut litesvm, &authority, 6, None, None).unwrap();
    let token_account =
        create_associated_token_account(&mut litesvm, &authority.pubkey(), &mint, &authority, None)
            .unwrap();

    // Minting tokens needs more than one compute unit.
    let result = mint_tokens_to_token_account(
        &mut litesvm,
        &mint,
        &token_account,
        1_000_000,
        &authority,
        Some(1),
    );
    assert!(result.is_err());
    assert_eq!(
        get_token_account_balance(&litesvm, &token_account).unwrap(),
        0
    );

    // 20,000 is more than the Classic Token Program's mint_to uses.
    mint_tokens_to_token_account(
        &mut litesvm,
        &mint,
        &token_account,
        1_000_000,
        &authority,
        Some(20_000),
    )
    .unwrap();
    assert_eq!(
        get_token_account_balance(&litesvm, &token_account).unwrap(),
        1_000_000
    );
}

#[test]
fn test_send_transaction_without_compute_unit_limit_simulates() {
    let mut litesvm = LiteSVM::new();
    let payer = create_wallet(&mut litesvm, 1_000_000_000).unwrap();
    let recipient = Pubkey::new_unique();

    send_transaction_from_instructions(
        &mut litesvm,
        vec![transfer(&payer.pubkey(), &recipient, 1_000_000)],
        &[&payer],
        &payer.pubkey(),
        None,
    )
    .unwrap();
    assert_eq!(get_sol_balance(&litesvm, &recipient), 1_000_000);

    // A transaction that fails in simulation is not sent.
    let result = send_transaction_from_instructions(
        &mut litesvm,
        vec![transfer(&payer.pubkey(), &recipient, 10_000_000_000)],
        &[&payer],
        &payer.pubkey(),
        None,
    );
    assert!(result.is_err());
    assert_eq!(get_sol_balance(&litesvm, &recipient), 1_000_000);
}
