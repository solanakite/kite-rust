//! Program Derived Address (PDA) utilities with type-safe seed handling.

use solana_pubkey::Pubkey;

/// Represents different types of seeds that can be used for PDA generation.
///
/// This enum provides type-safe seed handling for generating Program Derived Addresses.
/// It supports common data types used as seeds in Solana programs.
#[derive(Debug, Clone)]
pub enum Seed {
    /// String seed value
    String(String),
    /// Raw bytes seed value
    Bytes(Vec<u8>),
    /// 64-bit unsigned integer seed value (stored as little-endian bytes)
    U64(u64),
    /// Public key seed value
    Address(Pubkey),
}

impl Seed {
    /// Converts the seed to its byte representation.
    ///
    /// # Returns
    ///
    /// Returns a vector of bytes representing the seed value.
    fn to_bytes(&self) -> Vec<u8> {
        match self {
            Seed::String(string_value) => string_value.as_bytes().to_vec(),
            Seed::Bytes(byte_vector) => byte_vector.clone(),
            Seed::U64(number) => number.to_le_bytes().to_vec(),
            Seed::Address(address) => address.to_bytes().to_vec(),
        }
    }
}

/// Generates a Program Derived Address (PDA) and its bump seed.
///
/// This function takes a slice of seeds and a program ID to generate a PDA.
/// PDAs are deterministic addresses that are derived from seeds and are not
/// on the Ed25519 curve, making them safe for programs to sign for.
///
/// # Arguments
///
/// * `seeds` - Array of seed values used to derive the PDA
/// * `program_id` - The program ID that will own the PDA
///
/// # Returns
///
/// Returns a tuple containing:
/// - The derived public key (PDA)
/// - The bump seed (a value that ensures the address is off-curve)
///
/// # Example
///
/// ```rust
/// use solana_kite::{get_pda_and_bump, Seed, seeds};
/// use solana_pubkey::Pubkey;
///
/// let program_id = Pubkey::new_unique();
/// let user_address = Pubkey::new_unique();
///
/// // Using the seeds! macro for convenience
/// let seed_vec = seeds!["user-account", user_address, 42u64];
/// let (pda, bump) = get_pda_and_bump(&seed_vec, &program_id);
///
/// // Or manually creating seeds
/// let manual_seeds = vec![
///     Seed::String("user-account".to_string()),
///     Seed::Address(user_address),
///     Seed::U64(42),
/// ];
/// let (pda2, bump2) = get_pda_and_bump(&manual_seeds, &program_id);
///
/// assert_eq!(pda, pda2);
/// assert_eq!(bump, bump2);
/// ```
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
/// use solana_kite::{seeds, get_pda_and_bump};
/// use solana_pubkey::Pubkey;
///
/// let program_id = Pubkey::new_unique();
/// let user_addr = Pubkey::new_unique();
/// let offer_id = 123u64;
///
/// // Before (explicit):
/// let seeds_explicit = vec!["offer".into(), offer_id.into(), user_addr.into()];
///
/// // After (with macro):
/// let seeds_macro = seeds!["offer", offer_id, user_addr];
///
/// let (pda1, bump1) = get_pda_and_bump(&seeds_explicit, &program_id);
/// let (pda2, bump2) = get_pda_and_bump(&seeds_macro, &program_id);
///
/// assert_eq!(pda1, pda2);
/// assert_eq!(bump1, bump2);
/// ```
#[macro_export]
macro_rules! seeds {
    ($($seed:expr),* $(,)?) => {
        vec![$($seed.into()),*]
    };
}

// Implement From traits for convenient seed creation

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_conversions() {
        let str_seed: Seed = "test".into();
        let string_seed: Seed = "test".to_string().into();
        let u64_seed: Seed = 42u64.into();
        let pubkey_seed: Seed = Pubkey::new_unique().into();
        let bytes_seed: Seed = vec![1, 2, 3].into();
        let slice_seed: Seed = [1, 2, 3].as_slice().into();

        match str_seed {
            Seed::String(s) => assert_eq!(s, "test"),
            _ => panic!("Expected string seed"),
        }

        match string_seed {
            Seed::String(s) => assert_eq!(s, "test"),
            _ => panic!("Expected string seed"),
        }

        match u64_seed {
            Seed::U64(n) => assert_eq!(n, 42),
            _ => panic!("Expected u64 seed"),
        }

        match pubkey_seed {
            Seed::Address(_) => {} // Just verify it's the right variant
            _ => panic!("Expected address seed"),
        }

        match bytes_seed {
            Seed::Bytes(b) => assert_eq!(b, vec![1, 2, 3]),
            _ => panic!("Expected bytes seed"),
        }

        match slice_seed {
            Seed::Bytes(b) => assert_eq!(b, vec![1, 2, 3]),
            _ => panic!("Expected bytes seed"),
        }
    }

    #[test]
    fn test_seeds_macro() {
        let pubkey = Pubkey::new_unique();
        let seeds_macro: Vec<Seed> = seeds!["test", 42u64, pubkey];
        let seeds_manual: Vec<Seed> = vec!["test".into(), 42u64.into(), pubkey.into()];

        assert_eq!(seeds_macro.len(), seeds_manual.len());
        assert_eq!(seeds_macro.len(), 3);
    }
}
