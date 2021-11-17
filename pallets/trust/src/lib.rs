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

//! # ZdTrust Module
//!
//! ## Overview
//!
//! This module manages the user's trust relationship, calculates path length 
//! and reputation value transfer.
//!
//! ### Implementations
//!
//! This module implements the following trait :
//!
//! - `TrustBase` - Some interfaces for interacting with trust relationships.
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! - `trust` - The caller trusts the incoming user.
//! - `do_untrust` - The caller untrusts the incoming user.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::{
    codec::{Decode, Encode},
    ensure,
    traits::Get,
    transactional,
};
use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
use frame_system::{ensure_signed, pallet_prelude::*};
use orml_utilities::OrderedSet;
use sp_runtime::{DispatchError, DispatchResult, Perbill};
use sp_std::vec::Vec;
use zd_primitives::{appro_ln, TIRStep};
use zd_support::{Reputation, SeedsBase, TrustBase};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod weights;
pub use module::*;
pub use weights::WeightInfo;

pub const INIT_SEED_RANK: u32 = 1000;
pub const MIN_TRUST_COUNT: u32 = 5;

/// Trusted relationship cache.
///
/// To maintain a consistent set of trust relationships throughout the seeding 
/// and reputation updates, we cache the added trust relationships.
#[derive(Encode, Decode, Clone, Eq, PartialEq, Default)]
pub struct TrustTemp<AccountId> {
    /// Users trusted after the start of the refresh.
    pub trust: OrderedSet<AccountId>,

    /// Untrusted users after the refresh has started.
    pub untrust: OrderedSet<AccountId>,
}

#[frame_support::pallet]
pub mod module {

    use super::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type Reputation: Reputation<Self::AccountId, Self::BlockNumber, TIRStep>;
        type SeedsBase: SeedsBase<Self::AccountId>;

        /// Configuration of the relevant parameters necessary for the system.
        type DampingFactor: Get<Perbill>;

        /// The maximum number of users a user can trust.
        #[pallet::constant]
        type MaxTrustCount: Get<u32>;

        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    /// Store a list of user trusts.
    #[pallet::storage]
    #[pallet::getter(fn trust_list)]
    pub type TrustedList<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, OrderedSet<T::AccountId>, ValueQuery>;

    /// The trust cache added during the refresh period is emptied at the end of the refresh.
    #[pallet::storage]
    #[pallet::getter(fn trust_temp_list)]
    pub type TrustTempList<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, TrustTemp<T::AccountId>, ValueQuery>;

    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A user trusted another user. \[who, target\]
        Trusted(T::AccountId, T::AccountId),
        /// A user untrusted another user. \[who, target\]
        Untrusted(T::AccountId, T::AccountId),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Unable to trust yourself
        UnableTrustYourself,
        /// Already trusted this user
        RepeatTrust,
        /// Unable to untrust yourself
        UnableUntrustYourself,
        /// No target user exists
        NonExistent,
        /// Wrong path
        WrongPath,
        /// This is not a seeded user
        NotSeed,
        /// Exceeding the maximum number of trust limits
        TooMuchTrust,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// The caller trust `target`.
        ///
        /// If it has already been trusted, an `Err` is returned.
        #[pallet::weight(T::WeightInfo::trust())]
        #[transactional]
        pub fn trust(origin: OriginFor<T>, target: T::AccountId) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            Self::do_trust(&who, &target)?;
            Self::deposit_event(Event::Trusted(who, target));
            Ok(().into())
        }

        /// The caller untrust `target`.
        ///
        /// If the trust relationship does not exist, an `Err` will be returned.
        #[pallet::weight(T::WeightInfo::untrust())]
        #[transactional]
        pub fn untrust(origin: OriginFor<T>, target: T::AccountId) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            Self::do_untrust(&who, &target)?;
            Self::deposit_event(Event::Untrusted(who, target));
            Ok(().into())
        }
    }
}

impl<T: Config> Pallet<T> {
    pub(crate) fn do_trust(who: &T::AccountId, target: &T::AccountId) -> DispatchResult {
        ensure!(who != target, Error::<T>::UnableTrustYourself);

        <TrustedList<T>>::try_mutate(&who, |t| -> DispatchResult {
            ensure!(
                (t.len() as u32) < T::MaxTrustCount::get(),
                Error::<T>::TooMuchTrust
            );
            ensure!(t.insert(target.clone()), Error::<T>::RepeatTrust);
            Ok(())
        })?;

        if !T::Reputation::is_step(&TIRStep::Free) {
            let mut trust_temp_list = Self::trust_temp_list(&who);

            if !trust_temp_list.trust.remove(target) {
                let _ = trust_temp_list.untrust.insert(target.clone());
            }

            <TrustTempList<T>>::mutate(&who, |t| *t = trust_temp_list);
        }
        Ok(())
    }

    pub(crate) fn do_untrust(who: &T::AccountId, target: &T::AccountId) -> DispatchResult {
        ensure!(who != target, Error::<T>::UnableUntrustYourself);

        <TrustedList<T>>::try_mutate(&who, |t| -> DispatchResult {
            ensure!(t.remove(target), Error::<T>::NonExistent);
            Ok(())
        })?;

        if !T::Reputation::is_step(&TIRStep::Free) {
            let mut trust_temp_list = Self::trust_temp_list(&who);

            if !trust_temp_list.untrust.remove(target) {
                let _ = trust_temp_list.trust.insert(target.clone());
            }

            <TrustTempList<T>>::mutate(&who, |t| *t = trust_temp_list);
        }
        Ok(())
    }
}

impl<T: Config> TrustBase<T::AccountId> for Pallet<T> {
    fn remove_all_tmp() {
        <TrustTempList<T>>::remove_all();
    }

    fn get_trust_count(who: &T::AccountId) -> usize {
        Pallet::<T>::trust_list(&who).len()
    }

    fn get_trust_count_old(who: &T::AccountId) -> usize {
        let trust_temp = Self::trust_temp_list(&who);
        // must be greater than zero and cannot overflow
        Self::get_trust_count(who) + trust_temp.trust.len() - trust_temp.untrust.len()
    }

    fn is_trust(who: &T::AccountId, target: &T::AccountId) -> bool {
        <TrustedList<T>>::get(&who).contains(target)
    }

    fn valid_nodes(nodes: &[T::AccountId]) -> DispatchResult {
        for w in nodes.windows(2) {
            ensure!(Self::is_trust_old(&w[0], &w[1]), Error::<T>::WrongPath);
        }
        Ok(())
    }

    fn is_trust_old(who: &T::AccountId, target: &T::AccountId) -> bool {
        let temp_list = <TrustTempList<T>>::get(who);
        temp_list.trust.contains(target)
            || (Self::is_trust(who, target) && !temp_list.untrust.contains(target))
    }

    fn get_trust_old(who: &T::AccountId) -> Vec<T::AccountId> {
        let mut trusted_user = Self::trust_list(&who);
        let mut temp_list = Self::trust_temp_list(&who);
        trusted_user.sub_set(&temp_list.untrust.0);
        trusted_user.0.append(&mut temp_list.trust.0);
        trusted_user.0
    }

    fn computed_path(users: &[T::AccountId]) -> Result<(u32, u32), DispatchError> {
        ensure!(T::SeedsBase::is_seed(&users[0]), Error::<T>::NotSeed);
        let mut start_ir = INIT_SEED_RANK;
        let (dist, score) = users
            .windows(2)
            .map(|u| -> Result<(u32, u32), Error<T>> {
                if Self::is_trust(&u[0], &u[1]) {
                    let end_ir = T::Reputation::get_reputation(&u[1]).unwrap_or(0);
                    let item_dist = appro_ln(start_ir.saturating_sub(end_ir));
                    start_ir = end_ir;
                    let trust_count = Self::get_trust_count_old(&u[0]) as u32;
                    Ok((item_dist, trust_count))
                } else {
                    Err(Error::<T>::WrongPath)
                }
            })
            .try_fold::<_, _, Result<(u32, u32), Error<T>>>((0u32, INIT_SEED_RANK), |acc, d| {
                let (dist, trust_count) = d?;
                let item_score = T::DampingFactor::get().mul_floor(acc.1)
                    / trust_count.max(MIN_TRUST_COUNT)
                    / dist;
                Ok((acc.0.saturating_add(dist as u32), item_score))
            })?;
        Ok((dist, score))
    }
}
