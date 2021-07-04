#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::{
	codec::{Decode, Encode}, ensure, traits::Get
};
use sp_runtime::{DispatchResult, DispatchError, Perbill};
use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
use frame_system::{ensure_signed, ensure_root, pallet_prelude::*};
use zd_traits::{Reputation, TrustBase, SeedsBase};
use zd_utilities::{UserSet, UserSetExt};
use sp_std::vec::Vec;

pub use module::*;

/// 种子用户初始化声誉值
pub const INIT_SEED_RANK: usize = 1000;

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
        type Reputation: Reputation<Self::AccountId, Self::BlockNumber>;
        type SeedsBase: SeedsBase<Self::AccountId>;
        type DampingFactor: Get<Perbill>;
	}

    #[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

    #[pallet::storage]
	#[pallet::getter(fn trust_list)]
    pub type TrustedList<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, UserSet<T::AccountId>, ValueQuery>;

    #[pallet::storage]
	#[pallet::getter(fn trust_temp_list)]
    pub type TrustTempList<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, TrustTemp<T::AccountId>, ValueQuery>;

    #[pallet::event]
	#[pallet::metadata(T::AccountId = "AccountId")]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		SomethingStored(u32, T::AccountId),
	}

	#[pallet::error]
	pub enum Error<T> {
		NoneValue,
		StorageOverflow,
        NoValueStored,
	}

    #[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn trust(origin: OriginFor<T>, target: T::AccountId) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            Pallet::<T>::do_trust(&who, &target)?;
            Ok(().into())
		}

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn untrust(origin: OriginFor<T>, who: T::AccountId, target: T::AccountId) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            Pallet::<T>::do_untrust(&who, &target)?;
            Ok(().into())
		}

        // 测试期间管理员添加信任
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn trust_by_admin(origin: OriginFor<T>, who: T::AccountId, target: T::AccountId) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            Pallet::<T>::do_trust(&who, &target)?;
            Ok(().into())
		}

        // 测试期间管理员添加信任
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn untrust_by_admin(origin: OriginFor<T>, who: T::AccountId, target: T::AccountId) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            Pallet::<T>::do_untrust(&who, &target)?;
            Ok(().into())
        }
    }
}

impl<T: Config> Pallet<T> {

    pub(crate) fn do_trust(who: &T::AccountId, target: &T::AccountId) -> DispatchResult {
        // TODO: 限制大小
        ensure!(who != target, Error::<T>::NoValueStored);

        <TrustedList<T>>::try_mutate(&who, |t| -> DispatchResult {      
            ensure!(t.insert(target.clone()), Error::<T>::NoValueStored);
            Ok(())
        })?;

        if T::Reputation::check_update_status(true).is_some() {
            let mut trust_temp_list = Self::trust_temp_list(&who);

            if !trust_temp_list.trust.remove(&target) {
                let _ = trust_temp_list.untrust.insert(target.clone());
            }

            <TrustTempList<T>>::insert(&who,trust_temp_list);
        }
        Ok(())
    }

    pub(crate) fn do_untrust(who: &T::AccountId, target: &T::AccountId) -> DispatchResult {
        ensure!(who != target, Error::<T>::NoValueStored);

        ensure!(<TrustTempList<T>>::contains_key(&who), Error::<T>::NoValueStored);
        <TrustTempList<T>>::mutate(&who, |list| list.trust.remove(&target));

        if T::Reputation::check_update_status(true).is_some() {
            let mut trust_temp_list = Self::trust_temp_list(&who);

            if !trust_temp_list.untrust.remove(&target) {
                let _ = trust_temp_list.trust.insert(target.clone());
            }

            <TrustTempList<T>>::insert(&who,trust_temp_list);
        }
        Ok(())
    }
}

impl<T: Config> TrustBase<T::AccountId> for Pallet<T> {
    fn get_trust_count(who: &T::AccountId) -> usize {
        Pallet::<T>::trust_list(&who).len()
    }

    fn get_trust_count_old(who: &T::AccountId) -> usize {
        let trust_temp = Self::trust_temp_list(&who);
        Self::get_trust_count(&who) + trust_temp.trust.len() - trust_temp.untrust.len()
    }

    fn is_trust(who: &T::AccountId, target: &T::AccountId) -> bool {
        <TrustedList<T>>::get(&who).contains(&target)
    }

    fn is_trust_old(who: &T::AccountId, target: &T::AccountId) -> bool {
        let temp_list = <TrustTempList<T>>::get(who);
        temp_list.trust.contains(&target) || Self::is_trust(&who,&target) && temp_list.untrust.contains(&target)
    }

    fn get_trust_old(who: &T::AccountId) -> Vec<T::AccountId> {
        let mut trusted_user = Self::trust_list(&who);
        let mut temp_list = Self::trust_temp_list(&who);
        trusted_user.sub_set(&temp_list.untrust.0);
        trusted_user.0.append(&mut temp_list.trust.0);
        trusted_user.0
    }

    fn computed_path(users: &Vec<T::AccountId>) -> Result<(u32,u32), DispatchError> {
        // TODO: 最小 trust count
        ensure!(T::SeedsBase::is_seed(&users[0]), Error::<T>::NoValueStored);
        let mut start_ir = INIT_SEED_RANK as u32;
        let users_v = &users;
        let (dist, score) = users_v
            .windows(2)
            .map(|u|{
                if Self::is_trust(&u[0], &u[1]) {
                    let end_ir = T::Reputation::get_reputation_new(&u[1]).unwrap_or(0);
                    // let item_dist = f64::from(start_ir.saturating_sub(end_ir).max(3u32)) as u32;
                    let item_dist = f64::from(start_ir.saturating_sub(end_ir).max(3u32)).ln() as u32;
                    start_ir = end_ir;
                    Some(item_dist)
                } else {
                    None
                }
            })
            .try_fold::<_, _, Result<(u32,u32), Error<T>>>((0u32, INIT_SEED_RANK as u32), |acc, d| {
                ensure!(d.is_some(), Error::<T>::NoValueStored);
                let dist = d.unwrap();
                // TODO 获取 trust_count
                let trust_count = 10u32;
                let item_score = T::DampingFactor::get()
                    .mul_floor(acc.1) / trust_count.max(100) / dist;
                Ok((acc.0.saturating_add(dist as u32), item_score))
            })?;
        Ok((dist, score.into()))
    }
}