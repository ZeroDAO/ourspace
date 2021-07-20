#![cfg_attr(not(feature = "std"), no_std)]

use sp_runtime::{
    generic,
    traits::{IdentifyAccount, Verify},
    MultiSignature, Perbill,
};

/// An index to a block.
pub type BlockNumber = u64;

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
    use super::{Balance, BlockNumber, Perbill};

    /// Challenge staking amount.
    pub const CHALLENGE_STAKING_AMOUNT: Balance = 5_000;

    /// After this period, the proceeds can be claimed by other users.
    pub const RECEIVER_PROTECTION_PERIOD: BlockNumber = 10;

    /// After this period, no challenge can be launched.
    pub const CHALLENGE_PERIOD: BlockNumber = 10_000;

    /// When other users receive their earnings, they receive that percentage of the earnings.
    pub const PROXY_PICKUP_RATIO: Perbill = Perbill::from_perthousand(20);

    pub const PROXY_PERIOD: BlockNumber = 20_000;

    /// When the final reputation value obtained from the challenge is consistent with the
    /// original reputation value, the accountant divides it into percentage values.
    pub const ANALYST_RATIO: Perbill = Perbill::from_percent(10);
}

pub mod fee {
    use super::*;

    pub trait ProxyFee
    where
        Self: Sized,
    {
        fn check(last: BlockNumber,now: BlockNumber) -> bool;
        fn checked_proxy_fee(&self, last: BlockNumber, now: BlockNumber) -> Option<Self>;
    }

    impl ProxyFee for Balance {
        fn check(last: BlockNumber, now: BlockNumber) -> bool {
            last + factor::PROXY_PERIOD < now
        }

        fn checked_proxy_fee(&self, last: BlockNumber, now: BlockNumber) -> Option<Self> {
            match Self::check(last,now) {
                true => Some(factor::PROXY_PICKUP_RATIO.mul_floor(*self)),
                false => None,
            }
        }
    }
}
