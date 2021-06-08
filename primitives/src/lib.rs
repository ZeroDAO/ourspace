#![cfg_attr(not(feature = "std"), no_std)]

use sp_runtime::{
    generic,
    traits::{IdentifyAccount, Verify},
    MultiSignature,
    Perbill
};

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// The type for looking up accounts. We don't expect more than 4 billion of them, but you
/// never know...
pub type AccountIndex = u32;

/// Balance of an account.
pub type Balance = u128;

/// Index of a transaction in the chain.
pub type Index = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// Digest item type.
pub type DigestItem = generic::DigestItem<Hash>;

pub type Amount = i128;

pub mod factor {
    use super::{BlockNumber, Balance, Perbill};

    /// Challenge staking amount.
    pub const CHALLENGE_STAKING_AMOUNT: Balance = 5_000;

    /// After this period, the proceeds can be claimed by other users.
    pub const RECEIVER_PROTECTION_PERIOD: BlockNumber = 10;

    /// After this period, no challenge can be launched.
    pub const CHALLENGE_PERIOD: BlockNumber = 10_000;

    /// When other users receive their earnings, they receive that percentage of the earnings.
    pub const PROXY_PICKUP_RATIO: Perbill = Perbill::from_percent(60);

    /// When the final reputation value obtained from the challenge is consistent with the 
    /// original reputation value, the accountant divides it into percentage values.
    pub const ANALYST_RATIO: Perbill = Perbill::from_percent(10);
}