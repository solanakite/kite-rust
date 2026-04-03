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
    /// Account operation failed.
    AccountOperationFailed(String),
    /// I/O error occurred.
    IoError(std::io::Error),
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
            SolanaKiteError::AccountOperationFailed(msg) => {
                write!(f, "Account operation failed: {}", msg)
            }
            SolanaKiteError::IoError(err) => {
                write!(f, "I/O error: {}", err)
            }
        }
    }
}

impl std::error::Error for SolanaKiteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SolanaKiteError::IoError(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for SolanaKiteError {
    fn from(err: std::io::Error) -> Self {
        SolanaKiteError::IoError(err)
    }
}
