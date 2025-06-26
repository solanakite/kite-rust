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

/// Legacy error type for backward compatibility.
/// 
/// This is kept for backward compatibility with existing code.
/// New code should use [`SolanaKiteError`] instead.
#[deprecated(since = "0.1.0", note = "Use SolanaKiteError instead")]
#[derive(Debug)]
pub enum TestError {
    /// Transaction failed with an error message.
    TransactionFailed(String),
}

#[allow(deprecated)]
impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TestError::TransactionFailed(msg) => {
                write!(f, "Transaction failed: {}", msg)
            }
        }
    }
}

#[allow(deprecated)]
impl std::error::Error for TestError {}

#[allow(deprecated)]
impl From<TestError> for SolanaKiteError {
    fn from(err: TestError) -> Self {
        match err {
            TestError::TransactionFailed(msg) => SolanaKiteError::TransactionFailed(msg),
        }
    }
}