//! Basic usage example for Solana Kite
//! 
//! This example demonstrates the core functionality of the solana_kite library,
//! including wallet creation, program deployment, and transaction sending.

use litesvm::LiteSVM;
use solana_kite::{
    create_wallet, send_transaction_from_instructions, SolanaKiteError,
};
use solana_pubkey::Pubkey;
use solana_signer::Signer;

fn main() -> Result<(), SolanaKiteError> {
    println!("🪁 Solana Kite Basic Usage Example");
    println!("==================================");

    // Initialize the LiteSVM test environment
    let mut litesvm = LiteSVM::new();
    println!("✅ LiteSVM initialized");

    // Create a wallet with 1 SOL
    let wallet = create_wallet(&mut litesvm, 1_000_000_000)?;
    println!("✅ Created wallet: {}", wallet.pubkey());
    println!("   Balance: 1 SOL");

    // Check wallet balance
    let balance = litesvm.get_balance(&wallet.pubkey()).unwrap_or(0);
    println!("✅ Wallet balance verified: {} lamports", balance);

    // Generate a program ID (in a real scenario, this would be your actual program)
    let program_id = Pubkey::new_unique();
    println!("✅ Generated program ID: {}", program_id);

    // Note: Program deployment would require an actual .so file
    // This is just to demonstrate the API
    println!("📝 Program deployment example:");
    println!("   deploy_program(&mut litesvm, &program_id, \"./target/deploy/my_program.so\")?;");

    // Example of sending a transaction (empty instruction list for demo)
    let instructions = vec![]; // In practice, you'd have actual instructions here
    
    println!("📝 Transaction sending example:");
    println!("   send_transaction_from_instructions(&mut litesvm, instructions, &[&wallet], &wallet.pubkey())?;");
    
    // Actually send the empty transaction (this will succeed)
    send_transaction_from_instructions(&mut litesvm, instructions, &[&wallet], &wallet.pubkey())?;
    println!("✅ Sent transaction successfully");

    println!("🎉 Basic usage example completed successfully!");
    Ok(())
}