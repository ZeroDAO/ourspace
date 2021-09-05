#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::pallet_prelude::*;
use frame_system::{ensure_signed, pallet_prelude::*};
use orml_traits::{
    arithmetic::{self, Signed},
    MultiCurrency, SocialCurrency, StakingCurrency,
};
use sp_runtime::{
    traits::{MaybeSerializeDeserialize, Member, StaticLookup},
    DispatchResult,
};
use zd_primitives::{per_social_currency, Balance};
use zd_traits::MultiBaseToken;

mod default_weight;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

use sp_std::convert::{TryFrom, TryInto};

pub use module::*;

#[frame_support::pallet]
pub mod module {

    use super::*;

    pub trait WeightInfo {
        fn transfer_social() -> Weight;
    }

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

        type Currency: MultiCurrency<Self::AccountId, CurrencyId = Self::CurrencyId, Balance = Balance>
            + SocialCurrency<Self::AccountId, Balance = Balance>
            + StakingCurrency<Self::AccountId>;

        /// Weight information for extrinsics in this module.
        type WeightInfo: WeightInfo;

        #[pallet::constant]
        type BaceToken: Get<Self::CurrencyId>;
    }

    #[pallet::error]
    pub enum Error<T> {
        Overflow,
        /// Bonus too low
        BonusTooLow,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Token transfer_social success. \[currency_id, from, to, amount\]
        TransferSocial(T::CurrencyId, T::AccountId, T::AccountId, Balance),
    }

    #[pallet::pallet]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

    #[pallet::storage]
    #[pallet::getter(fn get_bonus)]
    pub type Bonus<T: Config> = StorageValue<_, Balance, ValueQuery>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Transfer some balance to another social-currency account
        ///
        /// The dispatch origin for this call must be `Signed` by the
        /// transactor.
        #[pallet::weight(T::WeightInfo::transfer_social())]
        pub fn transfer_social(
            origin: OriginFor<T>,
            dest: <T::Lookup as StaticLookup>::Source,
            currency_id: T::CurrencyId,
            #[pallet::compact] amount: Balance,
        ) -> DispatchResultWithPostInfo {
            let from = ensure_signed(origin)?;
            let to = T::Lookup::lookup(dest)?;
            T::Currency::transfer_social(currency_id, &from, &to, amount)?;
            Self::deposit_event(Event::TransferSocial(currency_id, from, to, amount));
            Ok(().into())
        }
    }
}

impl<T: Config> Pallet<T> {

    fn try_add_bonus(amount: &Balance) -> DispatchResult {
        <Bonus<T>>::try_mutate(|b| -> DispatchResult {
            let old_balance = *b;
            *b = old_balance.checked_add(*amount).ok_or(Error::<T>::Overflow)?;
            Ok(())
        })
    }

    fn try_cut_bonus(amount: &Balance) -> DispatchResult {
        <Bonus<T>>::try_mutate(|b| -> DispatchResult {
            let old_balance = *b;
            *b = old_balance.checked_sub(*amount).ok_or(Error::<T>::BonusTooLow)?;
            Ok(())
        })
    }
}

impl<T: Config> MultiBaseToken<T::AccountId, Balance> for Pallet<T> {
    fn get_bonus_amount() -> Balance {
        Self::get_bonus()
    }

    fn staking(who: &T::AccountId, amount: &Balance) -> DispatchResult {
        T::Currency::staking(T::BaceToken::get(), who, *amount)
    }

    fn release(who: &T::AccountId, amount: &Balance) -> DispatchResult {
        T::Currency::release(T::BaceToken::get(), who, *amount)
    }

    fn free_balance(who: &T::AccountId) -> Balance {
        T::Currency::free_balance(T::BaceToken::get(), who)
    }

    fn social_balance(who: &T::AccountId) -> Balance {
        T::Currency::social_balance(T::BaceToken::get(), who)
    }

    fn share(who: &T::AccountId, targets: &[T::AccountId]) -> Result<Balance, DispatchError> {
        let total_share = T::Currency::social_balance(T::BaceToken::get(), &who);

        let total_share_amount = per_social_currency::PRE_SHARE.mul_floor(total_share);
        let reserved_amount = per_social_currency::PRE_RESERVED.mul_floor(total_share);
        let burn_amount = per_social_currency::PRE_BURN.mul_floor(total_share);
        let fee_amount = per_social_currency::PRE_FEE.mul_floor(total_share);

        let pre_reward = total_share
            .saturating_sub(total_share_amount)
            .saturating_sub(reserved_amount)
            .saturating_sub(burn_amount)
            .saturating_sub(fee_amount);

        let share_amount = total_share_amount
            .checked_div((targets.len() as u32).max(per_social_currency::MIN_TRUST_COUNT).into())
            .ok_or(Error::<T>::Overflow)?;
            
        T::Currency::bat_share(T::BaceToken::get(), &who, &targets.to_vec(), share_amount)?;
        T::Currency::thaw(T::BaceToken::get(), &who, reserved_amount)?;
        T::Currency::social_burn(T::BaceToken::get(), &who, burn_amount)?;
        Self::try_add_bonus(&pre_reward)?;
        T::Currency::social_staking(T::BaceToken::get(), &who, fee_amount)?;
        Ok(fee_amount)
    }

    fn increase_bonus(who: &T::AccountId, amount: &Balance) -> DispatchResult {
        T::Currency::staking(T::BaceToken::get(),who,*amount)?;
        Self::try_add_bonus(amount)
    }

    fn cut_bonus(amount: &Balance) -> DispatchResult {
        Self::try_cut_bonus(amount)
    }
}
