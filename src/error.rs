//! Error types for Solana Kite operations.

use std::fmt;

/// Main error type for Solana Kite operations.
#[derive(Debug)]
pub enum SolanaKiteError {
    /// Transaction failed with an error message.
    TransactionFailed(String),
    /// Program deployment failed.
    ProgramDeploymentFailed(String),
    /// Token operation failed.
    TokenOperationFailed(String),
    /// Transfer hook operation failed.
    HookOperationFailed(String),
    /// Account operation failed.
    AccountOperationFailed(String),
}

impl fmt::Display for SolanaKiteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SolanaKiteError::TransactionFailed(msg) => {
                write!(f, "Transaction failed: {}", msg)
            }
            SolanaKiteError::ProgramDeploymentFailed(msg) => {
                write!(f, "Program deployment failed: {}", msg)
            }
            SolanaKiteError::TokenOperationFailed(msg) => {
                write!(f, "Token operation failed: {}", msg)
            }
            SolanaKiteError::HookOperationFailed(msg) => {
                write!(f, "Hook operation failed: {}", msg)
            }
            SolanaKiteError::AccountOperationFailed(msg) => {
                write!(f, "Account operation failed: {}", msg)
            }
        }
    }
}

impl std::error::Error for SolanaKiteError {}
