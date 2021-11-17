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

//! # ZdReputation Module
//!
//! ## Overview
//!
//! The Reputation module is the core module of the reputation system and
//! provides status management of the entire system.
//!
//! ### Implementations
//!
//! The Reputation module implements the following trait :
//!
//!  - `Reputation` -  Provides the ability to obtain and modify user
//! reputation values and to obtain and record system status.
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! - `set_period` - Setting the system update interval to a given number
//! of blocks requires administrator privileges.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::{
    codec::{Decode, Encode},
    ensure, pallet,
    traits::Get,
    RuntimeDebug,
    transactional,
};
use frame_system::{self as system};
use sp_runtime::{traits::Zero, DispatchResult};
use zd_primitives::TIRStep;
use zd_support::Reputation;

pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

/// Maximum quantity for seeds
pub const MAX_SEED: usize = 500;
/// Seed user initializes reputation values
pub const INIT_SEED_RANK: usize = 1000;

/// The state of the entire reputation system.
#[derive(Encode, Decode, Clone, PartialEq, Default, Eq, RuntimeDebug)]
pub struct OperationStatus<BlockNumber> {
    /// For each update round, add 1 to `nonce`.
    pub nonce: u32,

    /// It is used to facilitate other modules to control time-related when
    /// the latest active block of relational actions for the whole system
    /// of the state.
    pub last: BlockNumber,

    /// The next round starts at least after this block.
    pub next: BlockNumber,

    /// Minimum interval between rounds.
    pub period: BlockNumber,

    /// Whether the Reputation System is being updated and which step it
    /// is currently at.
    pub step: TIRStep,
}

/// User reputation value.
#[derive(Encode, Decode, Clone, PartialEq, Default, Eq, RuntimeDebug)]
pub struct ReputationScore {
    /// Reputation value.
    pub score: u32,

    /// The reputation value is updated in the `nonce` round.
    pub nonce: u32,
}

#[pallet]
pub mod pallet {
    use super::*;

    use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn system_info)]
    pub type SystemInfo<T: Config> = StorageValue<_, OperationStatus<T::BlockNumber>, ValueQuery>;

    /// Stores the last two updates of the reputation value of user `AccountId`.
    #[pallet::storage]
    #[pallet::getter(fn get_ir)]
    pub type ReputationScores<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, [ReputationScore; 2], ValueQuery>;

    /// Initializes a `period` to the given value.
    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub period: T::BlockNumber,
    }

    #[cfg(feature = "std")]
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

    /// Set the system update interval to the given number of blocks.
    ///
    /// The dispatch origin for this call must be `Signed` by the root.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        #[transactional]
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

    pub(crate) fn set_last_refresh(now: T::BlockNumber) {
        SystemInfo::<T>::mutate(|operation_status| {
            operation_status.last = now;
        });
    }

    pub(crate) fn do_set_period(period: T::BlockNumber) -> DispatchResult {
        SystemInfo::<T>::try_mutate(|operation_status| {
            ensure!(
                operation_status.step == TIRStep::Free,
                Error::<T>::UnableToSetPeriod
            );
            operation_status.period = period;
            Ok(())
        })
    }
}

impl<T: Config> Reputation<T::AccountId, T::BlockNumber, TIRStep> for Pallet<T> {
    // Low-level operation. Make changes directly to the latest nonce's REPUTATION
    fn mutate_reputation(target: &T::AccountId, ir: &u32) {
        ReputationScores::<T>::mutate(&target, |x| x[0].score = *ir);
    }

    fn set_step(step: &TIRStep) {
        <SystemInfo<T>>::mutate(|operation_status| operation_status.step = *step);
    }

    fn is_step(step: &TIRStep) -> bool {
        *step == <SystemInfo<T>>::get().step
    }

    #[transactional]
    fn new_round() -> DispatchResult {
        let now_block_number = Self::now();
        <SystemInfo<T>>::try_mutate(|operation_status| {
            ensure!(
                operation_status.step == TIRStep::Free,
                Error::<T>::AlreadyInUpdating
            );
            ensure!(
                now_block_number >= operation_status.next,
                Error::<T>::IntervalIsTooShort
            );
            let next = now_block_number + operation_status.period;
            operation_status.nonce += 1;
            operation_status.next = next;
            operation_status.last = now_block_number;
            operation_status.step = TIRStep::Seed;
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

    fn get_reputation(target: &T::AccountId) -> Option<u32> {
        let system_info = Self::system_info();
        let nonce = system_info.nonce;
        let irs = Self::get_ir(target);
        match system_info.step == TIRStep::Free {
            true => {
                if irs[0].nonce == nonce {
                    return Some(irs[0].score);
                }
            }
            false => {
                // nonce cannot be smaller than 1
                if irs[0].nonce == nonce - 1 {
                    return Some(irs[0].score);
                } else if irs[1].nonce == nonce - 1 {
                    return Some(irs[1].score);
                }
            }
        }
        None
    }

    #[transactional]
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

    fn get_last_refresh_at() -> T::BlockNumber {
        Self::system_info().last
    }

    fn set_last_refresh_at() {
        Self::set_last_refresh(Self::now());
    }

    fn set_free() {
        let now = Self::now();
        let operation_status = Self::system_info();
        if operation_status.step == TIRStep::Free {
            return;
        }

        SystemInfo::<T>::mutate(|operation_status| {
            operation_status.last = now;
            operation_status.step = TIRStep::Free;
        });
    }
}
