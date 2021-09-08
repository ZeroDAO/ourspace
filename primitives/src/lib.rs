#![cfg_attr(not(feature = "std"), no_std)]

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

#[derive(Encode, Debug, Decode, Eq, PartialEq, Copy, Clone, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum CurrencyId {
    ZDAO,
    SOCI,
}

pub type AppId = [u8; 8];

/// Balance of an account.
pub type Balance = u128;

pub const PROXY_PERIOD: u64 = 500;

/// When other users receive their earnings, they receive that percentage of the earnings.
pub const PROXY_PICKUP_RATIO: Perbill = Perbill::from_perthousand(20);

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
    Free,
    Seed,
    Reputation,
}

impl Default for TIRStep {
    fn default() -> TIRStep {
        TIRStep::Free
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
            let now_into = TryInto::<u64>::try_into(now).ok().unwrap();
            let last_into = TryInto::<u64>::try_into(last).ok().unwrap();
            last_into + PROXY_PERIOD < now_into
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

pub fn appro_ln(value: u32) -> u32 {
    if value < 3 {
        1
    }else if value < 8 {
        2
    }else if value < 21 {
        3
    }else if value < 55 {
        4
    }else if value < 149 {
        5
    }else if value < 404 {
        6
    }else if value < 1097 {
        7
    }else {
        8
    }
}
