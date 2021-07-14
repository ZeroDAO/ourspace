#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::{
    codec::{Decode, Encode},
    ensure, pallet,
    traits::Get,
    RuntimeDebug,
};
use frame_system::{self as system};
use sp_runtime::{traits::Zero, DispatchResult};
use zd_traits::Reputation;

pub use pallet::*;

mod mock;
mod tests;

/// Maximum quantity for seeds
pub const MAX_SEED: usize = 500;
/// Seed user initializes reputation values
pub const INIT_SEED_RANK: usize = 1000;

#[derive(Encode, Decode, Clone, PartialEq, Default, Eq, RuntimeDebug)]
pub struct OperationStatus<BlockNumber> {
    pub nonce: u32,
    pub last: BlockNumber,
    pub updating: bool,
    pub next: BlockNumber,
    pub period: BlockNumber,
}

#[derive(Encode, Decode, Clone, Default, RuntimeDebug)]
pub struct ReputationScore {
    pub score: u32,
    pub nonce: u32,
}

impl<BlockNumber> OperationStatus<BlockNumber> {
    fn check_update_status(&self, update_mode: bool) -> Option<u32> {
        if self.updating == update_mode {
            Some(self.nonce)
        } else {
            None
        }
    }
}

#[pallet]
pub mod pallet {
    use super::*;

    use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        #[pallet::constant]
        type ChallengePerior: Get<Self::BlockNumber>;
        type ConfirmationPeriod: Get<Self::BlockNumber>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn system_info)]
    pub type SystemInfo<T: Config> = StorageValue<_, OperationStatus<T::BlockNumber>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn last_challenge)]
    pub type LastChallenge<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn get_ir)]
    pub type ReputationScores<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, [ReputationScore; 2], ValueQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub period: T::BlockNumber,
    }

    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            GenesisConfig {
                period: Zero::zero(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            Pallet::<T>::do_set_period(self.period)
                .expect("Create PERIOD for OperationStatus cannot fail while building genesis");
        }
    }

    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId", T::BlockNumber = "BlockNumber")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Updated period. \[period\]
        UpdatedPeriod(T::BlockNumber),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Too short an interval.
        IntervalIsTooShort,
        /// Already in the process of being updated.
        AlreadyInUpdating,
        /// Setting disabled during reputation update.
        UnableToSetPeriod,
        /// Reputation already updated.
        ReputationAlreadyUpdated,
        /// The challenge is not over yet.
        ChallengeNotOverYet,
        /// Too short an interval between renewal periods.
        TooShortAnInterval,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn set_period(
            origin: OriginFor<T>,
            period: T::BlockNumber,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            Self::do_set_period(period)?;
            Self::deposit_event(Event::UpdatedPeriod(period));
            Ok(().into())
        }
    }
}

impl<T: Config> Pallet<T> {
    pub(crate) fn now() -> T::BlockNumber {
        system::Module::<T>::block_number()
    }

    pub(crate) fn is_challenge_end(now: T::BlockNumber) -> bool {
        now > Self::last_challenge() + T::ChallengePerior::get()
    }

    pub(crate) fn set_last_challenge(now: &T::BlockNumber) {
        LastChallenge::<T>::put(now);
    }

    pub(crate) fn set_last_refresh(now: T::BlockNumber) {
        SystemInfo::<T>::mutate(|operation_status| {
            operation_status.last = now;
        });
    }

    pub(crate) fn do_set_period(period: T::BlockNumber) -> DispatchResult {
        SystemInfo::<T>::try_mutate(|operation_status| {
            ensure!(!operation_status.updating, Error::<T>::UnableToSetPeriod);
            operation_status.period = period;
            Ok(())
        })
    }
}

impl<T: Config> Reputation<T::AccountId, T::BlockNumber> for Pallet<T> {
    // Low-level operation. Make changes directly to the latest nonce's REPUTATION
    fn mutate_reputation(target: &T::AccountId, ir: u32) {
        ReputationScores::<T>::mutate(&target, |x| x[0].score = ir);
    }

    fn new_round() -> DispatchResult {
        // TODO：检查数据是否清空完毕
        // TODO: 设置代领机制
        // TODO：检查是否初始化
        let now_block_number = Self::now();
        <SystemInfo<T>>::try_mutate(|operation_status| {
            ensure!(!operation_status.updating, Error::<T>::AlreadyInUpdating);
            ensure!(
                now_block_number >= operation_status.next,
                Error::<T>::IntervalIsTooShort
            );
            let next = now_block_number + operation_status.period;
            let old_nonce = operation_status.nonce;
            operation_status.nonce = old_nonce + 1;
            operation_status.updating = true;
            operation_status.next = next;
            operation_status.last = now_block_number;
            Ok(())
        })
    }

    fn get_reputation_new(target: &T::AccountId) -> Option<u32> {
        let new_nonce = Self::system_info().nonce;
        let irs = Self::get_ir(target);
        if irs[0].nonce == new_nonce {
            Some(irs[0].score)
        } else if irs[1].nonce == new_nonce {
            Some(irs[1].score)
        } else {
            None
        }
    }

    fn refresh_reputation(user_score: &(T::AccountId, u32)) -> DispatchResult {
        let who = &user_score.0;
        let nonce = Self::system_info().nonce;
        ReputationScores::<T>::try_mutate(&who, |reputation| -> DispatchResult {
            ensure!(
                reputation[0].nonce < nonce,
                Error::<T>::ReputationAlreadyUpdated
            );
            let old = reputation[0].clone();
            *reputation = [
                ReputationScore {
                    nonce,
                    score: user_score.1,
                },
                old,
            ];
            Ok(())
        })
    }

    fn last_refresh_at() {
        Self::set_last_refresh(Self::now());
    }

    fn check_update_status(update_mode: bool) -> Option<u32> {
        Self::system_info().check_update_status(update_mode)
    }

    fn last_challenge_at() {
        Self::set_last_challenge(&Self::now());
    }

    fn end_refresh() -> DispatchResult {
        let now = Self::now();

        let operation_status = Self::system_info();

        if !operation_status.updating {
            return Ok(());
        }

        ensure!(
            Self::is_challenge_end(now.clone()),
            Error::<T>::ChallengeNotOverYet
        );

        ensure!(
            now > operation_status.last + T::ConfirmationPeriod::get(),
            Error::<T>::TooShortAnInterval
        );

        SystemInfo::<T>::mutate(|operation_status| {
            operation_status.last = now.clone();
            operation_status.updating = false;
        });
        
        Ok(())
    }
}
