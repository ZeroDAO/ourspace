#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    codec::{Decode, Encode},
    ensure,
    traits::Get,
};
use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
use frame_system::{ensure_signed, pallet_prelude::*};
use sp_runtime::{DispatchError, DispatchResult, Perbill};
use sp_std::vec::Vec;
use zd_traits::{Reputation, SeedsBase, TrustBase};
use zd_utilities::{UserSet, UserSetExt};
use zd_primitives::TIRStep;

pub use module::*;

pub const INIT_SEED_RANK: usize = 1000;
pub const MIN_TRUST_COUNT: u32 = 5;

#[derive(Encode, Decode, Clone, Eq, PartialEq, Default)]
pub struct TrustTemp<AccountId> {
    pub trust: UserSet<AccountId>,
    pub untrust: UserSet<AccountId>,
}

#[frame_support::pallet]
pub mod module {

    use super::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type Reputation: Reputation<Self::AccountId, Self::BlockNumber, TIRStep>;
        type SeedsBase: SeedsBase<Self::AccountId>;
        type DampingFactor: Get<Perbill>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn trust_list)]
    pub type TrustedList<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, UserSet<T::AccountId>, ValueQuery>;

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
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn trust(origin: OriginFor<T>, target: T::AccountId) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            Pallet::<T>::do_trust(&who, &target)?;
            Self::deposit_event(Event::Trusted(who, target));
            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn untrust(origin: OriginFor<T>, target: T::AccountId) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            Pallet::<T>::do_untrust(&who, &target)?;
            Self::deposit_event(Event::Untrusted(who, target));
            Ok(().into())
        }
    }
}

impl<T: Config> Pallet<T> {
    pub(crate) fn do_trust(who: &T::AccountId, target: &T::AccountId) -> DispatchResult {
        // TODO: Limit size
        ensure!(who != target, Error::<T>::UnableTrustYourself);

        <TrustedList<T>>::try_mutate(&who, |t| -> DispatchResult {
            ensure!(t.insert(target.clone()), Error::<T>::RepeatTrust);
            Ok(())
        })?;

        if T::Reputation::check_update_status(true).is_some() {
            let mut trust_temp_list = Self::trust_temp_list(&who);

            if !trust_temp_list.trust.remove(&target) {
                let _ = trust_temp_list.untrust.insert(target.clone());
            }

            <TrustTempList<T>>::insert(&who, trust_temp_list);
        }
        Ok(())
    }

    pub(crate) fn do_untrust(who: &T::AccountId, target: &T::AccountId) -> DispatchResult {
        ensure!(who != target, Error::<T>::UnableUntrustYourself);

        ensure!(
            <TrustTempList<T>>::contains_key(&who),
            Error::<T>::NonExistent
        );
        <TrustTempList<T>>::mutate(&who, |list| list.trust.remove(&target));

        if T::Reputation::check_update_status(true).is_some() {
            let mut trust_temp_list = Self::trust_temp_list(&who);

            if !trust_temp_list.untrust.remove(&target) {
                let _ = trust_temp_list.trust.insert(target.clone());
            }

            <TrustTempList<T>>::insert(&who, trust_temp_list);
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
        Self::get_trust_count(&who) + trust_temp.trust.len() - trust_temp.untrust.len()
    }

    fn is_trust(who: &T::AccountId, target: &T::AccountId) -> bool {
        <TrustedList<T>>::get(&who).contains(&target)
    }

    fn valid_nodes(nodes: &Vec<T::AccountId>) -> DispatchResult {
        for w in nodes.windows(2) {
            ensure!(
                Self::is_trust_old(&w[0], &w[1]),
                Error::<T>::WrongPath
            );
        }
        Ok(())
    }

    fn is_trust_old(who: &T::AccountId, target: &T::AccountId) -> bool {
        let temp_list = <TrustTempList<T>>::get(who);
        temp_list.trust.contains(&target)
            || Self::is_trust(&who, &target) && temp_list.untrust.contains(&target)
    }

    fn get_trust_old(who: &T::AccountId) -> Vec<T::AccountId> {
        let mut trusted_user = Self::trust_list(&who);
        let mut temp_list = Self::trust_temp_list(&who);
        trusted_user.sub_set(&temp_list.untrust.0);
        trusted_user.0.append(&mut temp_list.trust.0);
        trusted_user.0
    }

    fn computed_path(users: &Vec<T::AccountId>) -> Result<(u32, u32), DispatchError> {
        ensure!(T::SeedsBase::is_seed(&users[0]), Error::<T>::NotSeed);
        let mut start_ir = INIT_SEED_RANK as u32;
        let users_v = &users;
        let (dist, score) = users_v
            .windows(2)
            .map(|u| -> Result<(u32, u32), Error<T>> {
                if Self::is_trust(&u[0], &u[1]) {
                    let end_ir = T::Reputation::get_reputation_new(&u[1]).unwrap_or(0);
                    let item_dist =
                        f64::from(start_ir.saturating_sub(end_ir).max(3u32)).ln() as u32;
                    start_ir = end_ir;
                    let trust_count = Self::get_trust_count_old(&u[0]) as u32;
                    Ok((item_dist,trust_count))
                } else {
                    Err(Error::<T>::WrongPath)
                }
            })
            .try_fold::<_, _, Result<(u32, u32), Error<T>>>(
                (0u32, INIT_SEED_RANK as u32),
                |acc, d| {
                    let (dist,trust_count) = d?;
                    let item_score =
                        T::DampingFactor::get().mul_floor(acc.1) / trust_count.max(MIN_TRUST_COUNT) / dist;
                    Ok((acc.0.saturating_add(dist as u32), item_score))
                },
            )?;
        Ok((dist, score.into()))
    }
}
