use litesvm::LiteSVM;
use solana_keypair::Keypair;
use solana_message::Message;
// Note Pubkeys may not be Pubkeys,his should be thought of as an Address like Address in Solana Kit.
// TODO: fix upstream
// https://github.com/anza-xyz/solana-sdk/issues/204
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::Transaction;
use spl_associated_token_account::instruction::create_associated_token_account as create_ata_instruction;
use spl_token::instruction::mint_to;
use std::fs;

#[derive(Debug)]
pub enum TestError {
    TransactionFailed(String),
}

impl std::fmt::Display for TestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestError::TransactionFailed(message) => {
                write!(formatter, "Transaction failed: {}", message)
            }
        }
    }
}

impl std::error::Error for TestError {}

pub fn deploy_program(litesvm: &mut LiteSVM, program_id: &Pubkey, program_path: &str) {
    let program_bytes = fs::read(program_path).expect("Failed to read program binary");
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
        .expect("Failed to deploy program");
}

pub fn send_transaction_from_instructions(
    litesvm: &mut LiteSVM,
    instructions: Vec<solana_instruction::Instruction>,
    signers: &[&Keypair],
    fee_payer: &Pubkey,
) -> Result<(), TestError> {
    let recent_blockhash = litesvm.latest_blockhash();
    let message = Message::new(&instructions, Some(fee_payer));
    let mut transaction = Transaction::new_unsigned(message);
    transaction.sign(signers, recent_blockhash);
    litesvm
        .send_transaction(transaction)
        .map(|_| ())
        .map_err(|e| TestError::TransactionFailed(format!("{:?}", e)))
}

pub fn create_token_mint(litesvm: &mut LiteSVM, mint_authority: &Keypair, decimals: u8) -> Keypair {
    let mint = Keypair::new();
    let rent = litesvm.minimum_balance_for_rent_exemption(82);

    litesvm
        .set_account(
            mint.pubkey(),
            solana_account::Account {
                lamports: rent,
                data: vec![0u8; 82],
                owner: spl_token::ID,
                executable: false,
                rent_epoch: 0,
            },
        )
        .unwrap();

    let initialize_mint_instruction = spl_token::instruction::initialize_mint(
        &spl_token::ID,
        &mint.pubkey(),
        &mint_authority.pubkey(),
        None,
        decimals,
    )
    .unwrap();

    let message = Message::new(
        &[initialize_mint_instruction],
        Some(&mint_authority.pubkey()),
    );
    let mut transaction = Transaction::new_unsigned(message);
    let blockhash = litesvm.latest_blockhash();
    transaction.sign(&[mint_authority], blockhash);
    litesvm.send_transaction(transaction).unwrap();

    mint
}


pub fn create_associated_token_account(
    litesvm: &mut LiteSVM,
    owner: &Keypair,
    mint: &Pubkey,
    payer: &Keypair,
) -> Pubkey {
    let associated_token_account =
        spl_associated_token_account::get_associated_token_address(&owner.pubkey(), mint);

    let create_ata_instruction = create_ata_instruction(
        &payer.pubkey(),
        &owner.pubkey(),
        mint,
        &spl_token::id(),
    );

    let message = Message::new(&[create_ata_instruction], Some(&payer.pubkey()));
    let mut transaction = Transaction::new_unsigned(message);
    let blockhash = litesvm.latest_blockhash();
    transaction.sign(&[payer], blockhash);
    litesvm.send_transaction(transaction).unwrap();

    associated_token_account
}

pub fn mint_tokens_to_account(
    litesvm: &mut LiteSVM,
    mint: &Pubkey,
    token_account: &Pubkey,
    amount: u64,
    mint_authority: &Keypair,
) {
    let mint_to_instruction = mint_to(
        &spl_token::id(),
        mint,
        token_account,
        &mint_authority.pubkey(),
        &[],
        amount,
    )
    .unwrap();

    let message = Message::new(&[mint_to_instruction], Some(&mint_authority.pubkey()));
    let mut transaction = Transaction::new_unsigned(message);
    let blockhash = litesvm.latest_blockhash();
    transaction.sign(&[mint_authority], blockhash);
    litesvm.send_transaction(transaction).unwrap();
}

pub fn get_token_account_balance(litesvm: &LiteSVM, token_account: &Pubkey) -> u64 {
    let account = litesvm
        .get_account(token_account)
        .expect("Token account not found");
    let data = &account.data;
    // SPL Token account layout: amount is at bytes 64..72 (u64, little endian)
    let amount_bytes = &data[64..72];
    u64::from_le_bytes(amount_bytes.try_into().expect("Failed to parse amount"))
}

pub fn assert_token_balance(
    litesvm: &LiteSVM,
    token_account: &Pubkey,
    expected_balance: u64,
    message: &str,
) {
    let actual_balance = get_token_account_balance(litesvm, token_account);
    assert_eq!(actual_balance, expected_balance, "{}", message);
}

/// Verifies that an account is closed (either doesn't exist or has empty data)
pub fn check_account_is_closed(litesvm: &LiteSVM, account: &Pubkey, message: &str) {
    let account_data = litesvm.get_account(account);
    assert!(
        account_data.is_none() || account_data.unwrap().data.is_empty(),
        "{}",
        message
    );
}

pub fn create_wallet(litesvm: &mut LiteSVM, airdrop_amount: u64) -> Keypair {
    let wallet = Keypair::new();
    litesvm
        .airdrop(&wallet.pubkey(), airdrop_amount)
        .expect("Failed to airdrop to wallet");
    wallet
}

pub fn create_wallets(litesvm: &mut LiteSVM, count: usize, airdrop_amount: u64) -> Vec<Keypair> {
    (0..count)
        .map(|_| create_wallet(litesvm, airdrop_amount))
        .collect()
}

pub enum Seed {
    String(String),
    Bytes(Vec<u8>),
    U64(u64),
    Address(Pubkey),
}

impl Seed {
    fn to_bytes(&self) -> Vec<u8> {
        match self {
            Seed::String(string_value) => string_value.as_bytes().to_vec(),
            Seed::Bytes(byte_vector) => byte_vector.clone(),
            Seed::U64(number) => number.to_le_bytes().to_vec(),
            Seed::Address(address) => address.to_bytes().to_vec(),
        }
    }
}

pub fn get_pda_and_bump(seeds: &[Seed], program_id: &Pubkey) -> (Pubkey, u8) {
    let seed_bytes: Vec<Vec<u8>> = seeds.iter().map(|seed| seed.to_bytes()).collect();
    let seed_slices: Vec<&[u8]> = seed_bytes.iter().map(|v| v.as_slice()).collect();
    Pubkey::find_program_address(&seed_slices, program_id)
}

/// Syntactic sugar for creating seed vectors with automatic type conversion.
///
/// This macro expands to `vec![seed1.into(), seed2.into(), ...]` - it's purely
/// for reducing boilerplate and doesn't perform any compile-time magic.
///
/// # Examples
///
/// ```rust
/// // Before (explicit):
/// let seeds = vec!["offer".into(), offer_id.into(), user_addr.into()];
///
/// // After (with macro):
/// let seeds = seeds!["offer", offer_id, user_addr];
/// ```
#[macro_export]
macro_rules! seeds {
    ($($seed:expr),* $(,)?) => {
        vec![$($seed.into()),*]
    };
}

impl From<&str> for Seed {
    fn from(value: &str) -> Self {
        Seed::String(value.to_string())
    }
}

impl From<String> for Seed {
    fn from(value: String) -> Self {
        Seed::String(value)
    }
}

impl From<u64> for Seed {
    fn from(value: u64) -> Self {
        Seed::U64(value)
    }
}

impl From<Pubkey> for Seed {
    fn from(value: Pubkey) -> Self {
        Seed::Address(value)
    }
}

impl From<Vec<u8>> for Seed {
    fn from(value: Vec<u8>) -> Self {
        Seed::Bytes(value)
    }
}

impl From<&[u8]> for Seed {
    fn from(value: &[u8]) -> Self {
        Seed::Bytes(value.to_vec())
    }
}
