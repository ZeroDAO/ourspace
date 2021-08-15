#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests;

use frame_support::{
    codec::{Decode, Encode},
    RuntimeDebug,
};
use sp_runtime::{
    traits::AtLeast32BitUnsigned, Perbill,
};
use sp_std::convert::TryInto;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

pub type AppId = [u8; 8];

/// Balance of an account.
pub type Balance = u128;

pub const PROXY_PERIOD: u64 = 20_000;

/// When other users receive their earnings, they receive that percentage of the earnings.
pub const PROXY_PICKUP_RATIO: Perbill = Perbill::from_perthousand(20);

/*
pub mod factor {
    use super::{Balance, BlockNumber, Perbill};

    /// Challenge staking amount.
    pub const CHALLENGE_STAKING_AMOUNT: Balance = 5_000;

    /// After this period, the proceeds can be claimed by other users.
    pub const RECEIVER_PROTECTION_PERIOD: BlockNumber = 10;

    /// After this period, no challenge can be launched.
    pub const CHALLENGE_PERIOD: BlockNumber = 10_000;



    pub const PROXY_PERIOD: BlockNumber = 20_000;

    /// When the final reputation value obtained from the challenge is consistent with the
    /// original reputation value, the accountant divides it into percentage values.
    pub const ANALYST_RATIO: Perbill = Perbill::from_percent(10);
}

*/

pub mod per_social_currency {
    use super::Perbill;

    pub const MIN_TRUST_COUNT: u32 = 150;
    /// Reserve the owner's free balance. The percentage can be adjusted by the community.
    pub const PRE_RESERVED: Perbill = Perbill::from_percent(10);

    /// Transfer to the social currency of users trusted by the owner.
    pub const PRE_SHARE: Perbill = Perbill::from_percent(10);

    /// Share to all users.
    pub const PRE_BURN: Perbill = Perbill::from_percent(10);

    /// Pathfinder's fee
    pub const PRE_FEE: Perbill = Perbill::from_percent(10);

    // Used to solve the verifier's Dilemma and refresh seed.
    // pub const PRE_REWARD: Perbill = Perbill::from_percent(10);
}

#[derive(Encode, Decode, Copy, Clone, PartialEq, Eq, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum TIRStep {
    FREE,
    SEED,
    REPUTATION,
}

impl Default for TIRStep {
    fn default() -> TIRStep {
        TIRStep::FREE
    }
}

pub mod fee {
    use super::*;

    pub trait ProxyFee
    where
        Self: Sized,
    {
        fn is_allowed_proxy<B: AtLeast32BitUnsigned>(last: B, now: B) -> bool;
        fn checked_with_fee<B: AtLeast32BitUnsigned>(
            &self,
            last: B,
            now: B,
        ) -> Option<(Self, Self)>;
        fn with_fee(&self) -> (Self, Self);
    }

    impl ProxyFee for Balance {
        fn is_allowed_proxy<B: AtLeast32BitUnsigned>(last: B, now: B) -> bool {
            let now_into = TryInto::<u64>::try_into(last).ok().unwrap();
            let last_into = TryInto::<u64>::try_into(now).ok().unwrap();
            last_into + PROXY_PERIOD > now_into
        }

        fn checked_with_fee<B: AtLeast32BitUnsigned>(
            &self,
            last: B,
            now: B,
        ) -> Option<(Self, Self)> {
            match Balance::is_allowed_proxy(last, now) {
                true => Some(self.with_fee()),
                false => None,
            }
        }

        fn with_fee(&self) -> (Self, Self) {
            let proxy_fee = PROXY_PICKUP_RATIO.mul_floor(*self);
            (proxy_fee, self.saturating_sub(proxy_fee))
        }
    }
}
