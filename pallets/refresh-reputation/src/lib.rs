#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::{
    codec::{Decode, Encode},
    ensure, pallet,
    traits::Get,
    RuntimeDebug,
};
use frame_system::{self as system, ensure_signed};
use orml_traits::{MultiCurrency, SocialCurrency, StakingCurrency};
use sp_runtime::{traits::Zero, DispatchResult, DispatchError, Perbill};
use sp_std::vec::Vec;
use zd_primitives::Balance;
use zd_traits::{Reputation, StartChallenge, TrustBase};

mod mock;
mod tests;

pub use pallet::*;

#[derive(Encode, Decode, Clone, Default, RuntimeDebug)]
pub struct Record<BlockNumber, Balance> {
    pub update_at: BlockNumber,
    pub fee: Balance,
}

#[pallet]
pub mod pallet {
    use super::*;

    use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
    use frame_system::pallet_prelude::*;
    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize + Ord;
        type BaceToken: Get<Self::CurrencyId>;
        type Currency: MultiCurrency<Self::AccountId, CurrencyId = Self::CurrencyId, Balance = Balance>
            + StakingCurrency<Self::AccountId>
            + SocialCurrency<Self::AccountId>;
        type ShareRatio: Get<Perbill>;
        type FeeRation: Get<Perbill>;
        type SelfRation: Get<Perbill>;
        type MaxUpdateCount: Get<u32>;
        type UpdateStakingAmount: Get<Balance>;
        type ConfirmationPeriod: Get<Self::BlockNumber>;
        type Reputation: Reputation<Self::AccountId, Self::BlockNumber>;
        type TrustBase: TrustBase<Self::AccountId>;
    }
    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn get_fees)]
    pub type Fees<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, Balance, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn update_record)]
    pub type Records<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        T::AccountId,
        Twox64Concat,
        T::AccountId,
        Record<T::BlockNumber, Balance>,
        ValueQuery,
    >;

    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Some reputations have been updated. \[analyst, count, fee\]
        ReputationRefreshed(T::AccountId, u32, Balance),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Quantity reaches limit
        QuantityLimitReached,
        /// Not in the update period
        NoUpdatesAllowed,
        /// Error getting fee
        ErrorFee,
        /// Challenge timeout
        ChallengeTimeout,
        /// Calculation overflow.
        Overflow,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn new_round(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            T::Reputation::new_round()?;
            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn refresh_reputation(
            origin: OriginFor<T>,
            user_scores: Vec<(T::AccountId, u32)>,
        ) -> DispatchResultWithPostInfo {
            let analyst = ensure_signed(origin)?;
            let user_count = user_scores.len();
            ensure!(
                user_count as u32 <= T::MaxUpdateCount::get(),
                Error::<T>::QuantityLimitReached
            );

            let _ = T::Reputation::check_update_status(true).ok_or(Error::<T>::NoUpdatesAllowed)?;

            let amount = T::UpdateStakingAmount::get()
                .checked_mul(user_count as Balance)
                .ok_or(Error::<T>::Overflow)?;
            T::Currency::staking(T::BaceToken::get(), &analyst, amount)?;

            let now_block_number = system::Module::<T>::block_number();

            let total_fee = user_scores
                .iter()
                .try_fold::<_, _, Result<Balance, DispatchError>>(
                    Zero::zero(),
                    |acc_amount, user_score| {
                        let fee = Self::do_refresh(&analyst, &user_score, &now_block_number)?;
                        acc_amount
                            .checked_add(fee)
                            .ok_or_else(|| Error::<T>::Overflow.into())
                    },
                )?;

            Self::mutate_fee(&analyst, &total_fee)?;
            T::Reputation::last_refresh_at();

            Self::deposit_event(Event::ReputationRefreshed(
                analyst,
                user_count as u32,
                total_fee,
            ));

            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn receiver_all(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            T::Reputation::end_refresh()?;

            let analyst = ensure_signed(origin)?;

            let fee = Fees::<T>::take(&analyst);
            T::Currency::release(T::BaceToken::get(), &analyst, fee)?;
            <Records<T>>::remove_prefix(&analyst);
            Ok(().into())
        }
    }
}

impl<T: Config> Pallet<T> {
    pub(crate) fn do_refresh(
        analyst: &T::AccountId,
        user_score: &(T::AccountId, u32),
        update_at: &T::BlockNumber,
    ) -> Result<Balance, DispatchError> {
        T::Reputation::refresh_reputation(&user_score)?;
        let who = &user_score.0;

        let fee = Self::share(who.clone())?;
        <Records<T>>::mutate(&analyst, &who, |_| Record { update_at, fee });
        Ok(fee)
    }

    pub(crate) fn mutate_fee(
        analyst: &T::AccountId,
        amount: &Balance,
    ) -> DispatchResult {
        <Fees<T>>::try_mutate(&analyst, |f| -> DispatchResult {
            *f = f.checked_add(*amount).ok_or(Error::<T>::Overflow)?;
            Ok(())
        })
    }

    pub(crate) fn share(user: T::AccountId) -> Result<Balance, DispatchError> {
        let targets = T::TrustBase::get_trust_old(&user);
        let total_share = T::Currency::social_balance(T::BaceToken::get(), &user);

        T::Currency::bat_share(
            T::BaceToken::get(),
            &user,
            &targets,
            T::ShareRatio::get().mul_floor(total_share),
        )?;
        T::Currency::thaw(
            T::BaceToken::get(),
            &user,
            T::SelfRation::get().mul_floor(total_share),
        )?;
        let actor_amount = T::FeeRation::get().mul_floor(total_share);
        T::Currency::social_staking(T::BaceToken::get(), &user, actor_amount.clone())?;

        Ok(actor_amount)
    }
}

impl<T: Config> StartChallenge<T::AccountId, Balance> for Pallet<T> {
    fn start(target: &T::AccountId, analyst: &T::AccountId) -> Result<Balance, DispatchError> {
        let _ = T::Reputation::check_update_status(true).ok_or(Error::<T>::NoUpdatesAllowed)?;

        let record = <Records<T>>::take(&target, &analyst);

        ensure!(
            record.update_at + T::ConfirmationPeriod::get() > system::Module::<T>::block_number(),
            Error::<T>::ChallengeTimeout
        );

        Fees::<T>::mutate(&analyst, |f| f.saturating_sub(record.fee));

        Ok(record.fee)
    }
}
