// Copyright 2021 ZeroDAO
// 
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// 
//     http://www.apache.org/licenses/LICENSE-2.0
// 
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # ZdToken Module
//!
//! ## 介绍
//!
//! The ZdToken module is used to manage social currency of users, staking and system rewards.
//! All funds are held in a `SocialPool` rather than being sent to the user in real time, 
//! which is more efficient for social currency and staking, which require frequent interaction. 
//!
//! ### Implementations
//!
//! The ZdToken module implements the following trait :
//!
//! - `MultiBaseToken` - Application management of system currency.
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! - `transfer_social` - An interface for sending social currency to a particular user.
//! - `claim` - The user withdraws the funds from `pending` to the balance.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::{pallet_prelude::*, transactional};
use frame_system::{ensure_signed, pallet_prelude::*};
use sp_runtime::{
    traits::{MaybeSerializeDeserialize, Member, Saturating, StaticLookup, Zero},
    DispatchResult,
};
use sp_std::convert::{TryFrom, TryInto};

use zd_primitives::{per_social_currency, Balance};
use zd_support::MultiBaseToken;

use orml_traits::{
    arithmetic::{self, Signed},
    MultiCurrency,
};

mod mock;
mod tests;
pub mod weights;

pub use module::*;
pub use weights::WeightInfo;

/// balance information for an account.
#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug)]
pub struct SocialAccount<Balance> {
    /// Non-reserved part of the balance. There may still be restrictions on
    /// this, but it is the total pool what may in principle be transferred,
    /// reserved.
    ///
    /// In some frequent interaction scenarios, the system will mark the funds 
    /// as `pending` instead of sending them directly to the user's balance. 
    /// The user will need to withdraw the `pending` funds to the balance 
    /// themselves.
    #[codec(compact)]
    pub pending: Balance,
    /// Balance of social tokens.
    #[codec(compact)]
    pub social: Balance,
}

impl<Balance: Saturating + Copy + Ord> SocialAccount<Balance> {
    /// The total balance in this account ignoring any frozen.
    fn total(&self) -> Balance {
        self.pending.saturating_add(self.social)
    }
}

#[frame_support::pallet]
pub mod module {

    use super::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The currency ID type
        type CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize + Ord;

        type Amount: Signed
            + TryInto<Balance>
            + TryFrom<Balance>
            + Parameter
            + Member
            + arithmetic::SimpleArithmetic
            + Default
            + Copy
            + MaybeSerializeDeserialize;

        type Currency: MultiCurrency<
            Self::AccountId,
            CurrencyId = Self::CurrencyId,
            Balance = Balance,
        >;

        /// Which currency to use.
        #[pallet::constant]
        type BaceToken: Get<Self::CurrencyId>;

        /// Address of the pool.
        #[pallet::constant]
        type SocialPool: Get<Self::AccountId>;

        /// Weight information for extrinsics in this module.
        type WeightInfo: WeightInfo;
    }

    #[pallet::error]
    pub enum Error<T> {
        Overflow,
        /// Bonus too low
        BonusTooLow,
        /// Total staking amount too low
        StakingAmountTooLow,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Token transfer_social success. \[from, to, amount\]
        TransferSocial(T::AccountId, T::AccountId, Balance),
        /// Transferr `pending` Tokens to `free` \[who\]
        Claim(T::AccountId),
    }

    #[pallet::pallet]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

    #[pallet::storage]
    #[pallet::getter(fn get_bonus)]
    pub type Bonus<T: Config> = StorageValue<_, Balance, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn accounts)]
    pub type Accounts<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, SocialAccount<Balance>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn total_staking)]
    pub type TotalStaking<T: Config> = StorageValue<_, Balance, ValueQuery>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Transfer some balance to another social-currency account
        ///
        /// The dispatch origin for this call must be `Signed` by the
        /// transactor.
        #[pallet::weight(T::WeightInfo::transfer_social())]
        #[transactional]
        pub fn transfer_social(
            origin: OriginFor<T>,
            dest: <T::Lookup as StaticLookup>::Source,
            #[pallet::compact] amount: Balance,
        ) -> DispatchResultWithPostInfo {
            let from = ensure_signed(origin)?;
            let to = T::Lookup::lookup(dest)?;
            <Self as MultiBaseToken<_, _>>::transfer_social(&from, &to, amount)?;
            Self::deposit_event(Event::TransferSocial(from, to, amount));
            Ok(().into())
        }

        /// Extract the caller `pending` to the `free` balance.
        #[pallet::weight(T::WeightInfo::claim())]
        #[transactional]
        pub fn claim(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            <Self as MultiBaseToken<_, _>>::claim(&who)?;
            Self::deposit_event(Event::Claim(who));
            Ok(().into())
        }
    }
}

impl<T: Config> Pallet<T> {
    /// Set social balance of `who` to a new value.
    ///
    /// Note this will not maintain total issuance, and the caller is
    /// expected to do it.
    pub(crate) fn set_social_balance(who: &T::AccountId, amount: Balance) {
        <Accounts<T>>::mutate(who, |account| {
            account.social = amount;
        });
    }

    pub(crate) fn share_and_reserv(
        from: &T::AccountId,
        trustees: &[T::AccountId],
        total_share_amount: Balance,
        reserved_amount: Balance,
    ) {
        let mut remaining_share: Balance = Zero::zero();
        if !trustees.is_empty() && total_share_amount != 0 {
            if let Some(share_amount) = total_share_amount.checked_div(
                (trustees.len() as u32)
                    .max(per_social_currency::MIN_TRUST_COUNT)
                    .into(),
            ) {
                trustees.iter().for_each(|trustee| {
                    <Accounts<T>>::mutate(trustee, |account| {
                        account.social = account.social.saturating_add(share_amount);
                    });
                });

                remaining_share = total_share_amount
                    .saturating_sub(share_amount.saturating_mul(trustees.len() as Balance));
            } else {
                remaining_share = total_share_amount;
            }
        }

        <Accounts<T>>::mutate(from, |account| {
            account.social = remaining_share;
            account.pending = reserved_amount;
        });
    }

    /// Set pending balance of `who` to a new value.
    ///
    /// Note this will not maintain total issuance, and the caller is
    /// expected to do it.
    pub fn set_pending_balance(who: &T::AccountId, amount: Balance) {
        <Accounts<T>>::mutate(who, |account| {
            account.pending = amount;
        });
    }

    pub(crate) fn do_staking(amount: &Balance) {
        <TotalStaking<T>>::mutate(|t| *t = t.saturating_add(*amount));
    }

    pub(crate) fn add_bonus(amount: &Balance) {
        <Bonus<T>>::mutate(|b| *b = b.saturating_add(*amount));
    }

    fn try_add_bonus(amount: &Balance) -> DispatchResult {
        <Bonus<T>>::try_mutate(|b| -> DispatchResult {
            let old_balance = *b;
            *b = old_balance
                .checked_add(*amount)
                .ok_or(Error::<T>::Overflow)?;
            Ok(())
        })
    }

    fn try_cut_bonus(amount: &Balance) -> DispatchResult {
        <Bonus<T>>::try_mutate(|b| -> DispatchResult {
            let old_balance = *b;
            *b = old_balance
                .checked_sub(*amount)
                .ok_or(Error::<T>::BonusTooLow)?;
            Ok(())
        })
    }
}

impl<T: Config> MultiBaseToken<T::AccountId, Balance> for Pallet<T> {
    fn get_bonus_amount() -> Balance {
        Self::get_bonus()
    }

    fn actual_balance(who: &T::AccountId) -> Balance {
        let free_balance = T::Currency::free_balance(T::BaceToken::get(), who);
        free_balance.saturating_add(Self::accounts(who).total())
    }

    fn pending_balance(who: &T::AccountId) -> Balance {
        Self::accounts(who).pending
    }

    fn social_balance(who: &T::AccountId) -> Balance {
        Self::accounts(who).social
    }

    #[transactional]
    fn transfer_social(from: &T::AccountId, to: &T::AccountId, amount: Balance) -> DispatchResult {
        let to_social_balance = Self::social_balance(to)
            .checked_add(amount)
            .ok_or(Error::<T>::Overflow)?;
        Self::pay_with_pending(from, amount)?;
        Self::set_social_balance(to, to_social_balance);
        Ok(())
    }

    #[transactional]
    fn staking(who: &T::AccountId, amount: &Balance) -> DispatchResult {
        Self::pay_with_pending(who, *amount)?;
        Self::do_staking(amount);
        Ok(())
    }

    #[transactional]
    fn pay_with_pending(from: &T::AccountId, amount: Balance) -> DispatchResult {
        let form_pending_balance = Self::pending_balance(from);
        match form_pending_balance >= amount {
            true => {
                Self::set_pending_balance(from, form_pending_balance - amount);
            }
            false => {
                T::Currency::transfer(
                    T::BaceToken::get(),
                    from,
                    &T::SocialPool::get(),
                    amount - form_pending_balance,
                )?;
                Self::set_pending_balance(from, Zero::zero());
            }
        }
        Ok(())
    }

    #[transactional]
    fn release(who: &T::AccountId, amount: &Balance) -> DispatchResult {
        let total_staking = Self::total_staking()
            .checked_sub(*amount)
            .ok_or(Error::<T>::StakingAmountTooLow)?;
        T::Currency::transfer(T::BaceToken::get(), &T::SocialPool::get(), who, *amount)?;
        <TotalStaking<T>>::put(total_staking);
        Ok(())
    }

    fn free_balance(who: &T::AccountId) -> Balance {
        T::Currency::free_balance(T::BaceToken::get(), who)
    }

    fn share(who: &T::AccountId, targets: &[T::AccountId]) -> Balance {
        let social_balance = Self::social_balance(who);

        let total_share_amount = per_social_currency::PRE_SHARE.mul_floor(social_balance);
        let reserved_amount = per_social_currency::PRE_RESERVED.mul_floor(social_balance);
        let burn_amount = per_social_currency::PRE_BURN.mul_floor(social_balance);
        let fee_amount = per_social_currency::PRE_FEE.mul_floor(social_balance);

        let pre_reward = social_balance
            .saturating_sub(total_share_amount)
            .saturating_sub(reserved_amount)
            .saturating_sub(burn_amount)
            .saturating_sub(fee_amount);

        let _ = T::Currency::slash(T::BaceToken::get(), &T::SocialPool::get(), burn_amount);

        Self::share_and_reserv(who, targets, total_share_amount, reserved_amount);
        Self::do_staking(&fee_amount);
        Self::add_bonus(&pre_reward);
        fee_amount
    }

    #[transactional]
    fn increase_bonus(who: &T::AccountId, amount: &Balance) -> DispatchResult {
        Self::staking(who, amount)?;
        Self::try_add_bonus(amount)
    }

    fn cut_bonus(amount: &Balance) -> DispatchResult {
        Self::try_cut_bonus(amount)
    }

    fn claim(who: &T::AccountId) -> DispatchResult {
        <Accounts<T>>::try_mutate(who, |account| -> DispatchResult {
            T::Currency::transfer(
                T::BaceToken::get(),
                &T::SocialPool::get(),
                who,
                account.pending,
            )?;
            account.pending = Zero::zero();
            Ok(())
        })
    }
}
