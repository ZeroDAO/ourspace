//! # ZdSeeds Module
//!
//! ## 介绍
//!
//! 存储种子数据并提供交互接口的模块。
//!
//! ### 实现
//!
//! 挑战模块实现了以下 trait :
//!
//!  - `SeedsBase` - 与种子数据交互的接口。
//!
//! ## 接口
//!
//! ### 可调用函数
//!
//! - `new_seed` - 增加新的种子，需要管理员权限。
//! - `remove_seed` - 删除种子，需要管理员权限。

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
        /// 增加种子，如果种子已存在则调用失败。
        ///
        /// 调用需要管理员权限。
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

        /// 删除提供的种子，如果种子不存在则调用失败。
        ///
        /// 调用需要管理员权限。
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
