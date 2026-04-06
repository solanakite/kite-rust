//! Example: Transfer hook setup with Solana Kite
//!
//! Demonstrates how to configure a Token Extensions mint with a transfer hook,
//! set up the ExtraAccountMetaList PDA, and prepare extra accounts for a
//! hook-aware transfer.
//!
//! In a real test you would first deploy your hook program with `deploy_program`
//! or `deploy_program_bytes`, then use its program ID here. This example uses a
//! placeholder program ID and skips the transfer step because LiteSVM would
//! reject a CPI into a program that hasn't been deployed.

use litesvm::LiteSVM;
use solana_kite::{
    build_hook_accounts, create_token_extensions_account, create_token_extensions_mint,
    create_wallet, get_hook_accounts_address, mint_tokens_to_token_extensions_account,
    transfer_checked_token_extensions, HookAccount, MintExtension,
};
use solana_signer::Signer;

fn main() {
    let mut litesvm = LiteSVM::new();

    // In a real test: deploy your hook program and use its ID here.
    //   let hook_program_id = Pubkey::new_unique();
    //   deploy_program_bytes(&mut litesvm, &hook_program_id, include_bytes!("../target/deploy/my_hook.so")).unwrap();
    //
    // For this demonstration we use a pre-loaded program that LiteSVM ships
    // built-in: the system program, which accepts any instruction and is always
    // present. We set hook_accounts to empty so no CPI into our hook is needed.
    let hook_program_id = solana_pubkey::pubkey!("11111111111111111111111111111111");

    let authority = create_wallet(&mut litesvm, 2_000_000_000).unwrap();
    let sender = create_wallet(&mut litesvm, 1_000_000_000).unwrap();
    let receiver = create_wallet(&mut litesvm, 1_000_000_000).unwrap();

    // 1. Create a mint with the TransferHook extension pointing at our hook program.
    let mint = create_token_extensions_mint(
        &mut litesvm,
        &authority,
        6,
        None,
        &[MintExtension::TransferHook {
            program_id: hook_program_id,
        }],
    )
    .unwrap();
    println!("Created mint with TransferHook extension: {}", mint);

    // 2. Show where the ExtraAccountMetaList PDA will live.
    let hook_accounts_address = get_hook_accounts_address(&mint, &hook_program_id);
    println!("ExtraAccountMetaList PDA: {}", hook_accounts_address);

    // 3. Declare the accounts your hook program needs on every transfer.
    //    Empty here because the system program hook requires none.
    let hook_accounts: Vec<HookAccount> = vec![];

    // In a real test with a real hook program, uncomment this:
    //   initialize_hook_accounts(
    //       &mut litesvm,
    //       &hook_program_id,
    //       &mint,
    //       &authority,
    //       &hook_accounts,
    //   ).unwrap();
    //
    // With a custom hook that needs extra accounts you would add entries like:
    //   hook_accounts.push(HookAccount {
    //       pubkey: some_state_account,
    //       is_signer: false,
    //       is_writable: true,
    //   });

    // 4. Create token accounts and mint tokens.
    let sender_ata =
        create_token_extensions_account(&mut litesvm, &sender.pubkey(), &mint, &authority).unwrap();
    let receiver_ata =
        create_token_extensions_account(&mut litesvm, &receiver.pubkey(), &mint, &authority)
            .unwrap();

    mint_tokens_to_token_extensions_account(
        &mut litesvm,
        &mint,
        &sender_ata,
        1_000_000,
        &authority,
    )
    .unwrap();
    println!("Minted 1,000,000 tokens to sender");

    // 5. Build the accounts to pass into the transfer, then transfer.
    //    build_hook_accounts collects your hook_accounts plus the hook program
    //    ID and ExtraAccountMetaList PDA that Token Extensions always appends.
    let extra_accounts = build_hook_accounts(&mint, &hook_program_id, &hook_accounts);

    transfer_checked_token_extensions(
        &mut litesvm,
        &sender_ata,
        &mint,
        &receiver_ata,
        &sender,
        500_000,
        6,
        &extra_accounts,
    )
    .unwrap();
    println!("Transferred 500,000 tokens from sender to receiver via hook-aware transfer");

    println!("Transfer hook example complete.");
}
