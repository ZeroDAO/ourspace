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

//! # ZdSeeds Module
//!
//! ## Overview
//!
//! A module that stores seed data and provides an interactive interface.
//!
//! ### Implementations
//!
//! This pallet provides implementations for the following traits.
//!
//!  - `SeedsBase` - Interface for interaction with seed data.
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! - `new_seed` - Add new seeds, root access required.
//! - `remove_seed` - Remove seeds, root access required.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use orml_utilities::OrderedSet;
pub use pallet::*;
use zd_primitives::TIRStep;
use zd_support::{Reputation, SeedsBase};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
    use frame_system::ensure_root;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type Reputation: Reputation<Self::AccountId, Self::BlockNumber, TIRStep>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn get_seeds)]
    pub type Seeds<T: Config> = StorageValue<_, OrderedSet<T::AccountId>, ValueQuery>;

    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Seed added. \[seed\]
        SeedAdded(T::AccountId),
        /// Seed removed. \[seed\]
        SeedRemoved(T::AccountId),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Seed users already exist
        AlreadySeedUser,
        /// Not a seed user
        NotSeedUser,
        /// Status error
        StatusErr,
        /// Calculation overflow.
        Overflow,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Add seed, or return `Err` if seeds already exist.
        ///
        /// The dispatch origin for this call must be `Signed` by the transactor.
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn new_seed(origin: OriginFor<T>, seed: T::AccountId) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            ensure!(
                T::Reputation::is_step(&TIRStep::Free),
                Error::<T>::StatusErr
            );
            ensure!(!Self::is_seed(&seed), Error::<T>::AlreadySeedUser);
            Self::add_seed(&seed);
            Ok(().into())
        }

        /// Remove seed, or return `Err` if seeds not exist.
        ///
        /// The dispatch origin for this call must be `Signed` by the transactor.
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn remove_seed(origin: OriginFor<T>, seed: T::AccountId) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            ensure!(
                T::Reputation::is_step(&TIRStep::Free),
                Error::<T>::StatusErr
            );
            ensure!(Self::is_seed(&seed), Error::<T>::NotSeedUser);
            <Seeds<T>>::get().remove(&seed);
            Seeds::<T>::mutate(|seeds| {
                seeds.remove(&seed);
            });
            Self::deposit_event(Event::SeedRemoved(seed));
            Ok(().into())
        }
    }
}

impl<T: Config> SeedsBase<T::AccountId> for Pallet<T> {
    fn get_seed_count() -> u32 {
        Seeds::<T>::get().len() as u32
    }

    fn is_seed(seed: &T::AccountId) -> bool {
        Seeds::<T>::get().contains(seed)
    }

    fn remove_all() {
        Seeds::<T>::kill();
    }

    fn add_seed(new_seed: &T::AccountId) {
        Seeds::<T>::mutate(|seeds| seeds.insert(new_seed.clone()));
        Self::deposit_event(Event::SeedAdded(new_seed.clone()));
    }
}
