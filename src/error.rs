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
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SolanaKiteError::TransactionFailed(message) => {
                write!(formatter, "Transaction failed: {}", message)
            }
            SolanaKiteError::ProgramDeploymentFailed(message) => {
                write!(formatter, "Program deployment failed: {}", message)
            }
            SolanaKiteError::TokenOperationFailed(message) => {
                write!(formatter, "Token operation failed: {}", message)
            }
            SolanaKiteError::HookOperationFailed(message) => {
                write!(formatter, "Hook operation failed: {}", message)
            }
            SolanaKiteError::AccountOperationFailed(message) => {
                write!(formatter, "Account operation failed: {}", message)
            }
        }
    }
}

impl std::error::Error for SolanaKiteError {}
