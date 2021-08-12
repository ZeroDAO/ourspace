#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
use zd_traits::{Reputation, SeedsBase};

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
	use frame_system::pallet_prelude::*;
	use frame_system::ensure_root;

	/// 种子用户初始化声誉值
	pub const INIT_SEED_SCORE: u32 = 1000;
	
    #[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Reputation: Reputation<Self::AccountId, Self::BlockNumber>;
	}

    #[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn get_seeds)]
	pub type Seeds<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn seeds_count)]
	pub type SeedsCount<T: Config> = StorageValue<_, u32, ValueQuery>;

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
		/// Exceeding the maximum number of seed users
		SeedsLimitReached,
		/// Seed users already exist
		AlreadySeedUser,
		/// Not a seed user
		NotSeedUser,
	}

    #[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn add_seed(origin: OriginFor<T>,new_seed: T::AccountId) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
			T::Reputation::check_update_status(false);

			ensure!(Seeds::<T>::contains_key(&new_seed), Error::<T>::SeedsLimitReached);

			Seeds::<T>::insert(&new_seed,INIT_SEED_SCORE);

			SeedsCount::<T>::mutate(|c| *c += 1);

			Self::deposit_event(Event::SeedAdded(new_seed));

			Ok(().into())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn remove_seed(origin: OriginFor<T>, seed: T::AccountId) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
			let _ = T::Reputation::check_update_status(false);

			ensure!(!<Seeds<T>>::contains_key(&seed), Error::<T>::NotSeedUser);

			<Seeds<T>>::remove(&seed);
			SeedsCount::<T>::mutate(|c| *c -= 1);

			Self::deposit_event(Event::SeedRemoved(seed));

			Ok(().into())
		}
	}
}

impl<T: Config> SeedsBase<T::AccountId> for Pallet<T> {

	fn get_seed_count() -> u32 {
		Self::seeds_count()
	}

	fn is_seed(seed: &T::AccountId) -> bool {
		Seeds::<T>::contains_key(seed)
	}

	fn remove_all() {
		Seeds::<T>::remove_all();
		SeedsCount::<T>::put(0u32);
	}

	fn add_seed(new_seed: &T::AccountId) {
		Seeds::<T>::mutate(&new_seed,|s|*s = INIT_SEED_SCORE);
		SeedsCount::<T>::mutate(|c| *c += 1);
		Self::deposit_event(Event::SeedAdded(new_seed.clone()));
	}
}