#![cfg_attr(not(feature = "std"), no_std)]

// use frame_support::{ensure, dispatch::DispatchResultWithPostInfo, pallet, pallet_prelude::*};
use frame_support::{
    codec::{Decode, Encode},
    ensure, pallet,
    traits::Get,
    RuntimeDebug,
};
use frame_system::{self as system};
use orml_traits::{MultiCurrencyExtended, StakingCurrency};
use zd_primitives::{factor, Amount, Balance};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use sp_runtime::{
    traits::{AtLeast32BitUnsigned, Zero},
    DispatchError, DispatchResult,
};
use sp_std::{convert::TryInto, vec::Vec};

pub use pallet::*;

#[derive(Encode, Decode, Clone, Default, RuntimeDebug)]
pub struct ResultSet<BlockNumber> {
    pub order: u32,
    pub score: u32,
    pub hash: BlockNumber,
}

type PathId = u128;

pub trait Convert<A>
where
    A: AtLeast32BitUnsigned,
    Self: Sized,
{
    fn from_ids(start: A, stop: A) -> Self;
    fn to_ids(&self) -> (A, A);
}

// 将两个 AccountId 转换为一个id
impl<A: AtLeast32BitUnsigned> Convert<A> for PathId {
    fn from_ids(start: A, end: A) -> Self {
        let start_into = TryInto::<u128>::try_into(start).ok().unwrap();
        let end_into = TryInto::<u128>::try_into(end).ok().unwrap();
        (start_into << 64) | end_into
    }

    fn to_ids(&self) -> (A, A) {
        let start = self >> 64;
        let end = self & 0xfffffffffffffff;
        (
            A::try_from(start).ok().unwrap(),
            A::try_from(end).ok().unwrap(),
        )
    }
}

#[derive(Hash, Encode, Decode, Clone, Default, RuntimeDebug)]
pub struct Path<AccountId> {
    pub id: PathId,
    pub nodes: Option<Vec<AccountId>>,
    pub total: u32,
}

#[pallet]
pub mod pallet {
    use super::*;

    use frame_system::{ensure_signed, pallet_prelude::*};

    use frame_support::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize + Ord;
        type BaceToken: Get<Self::CurrencyId>;
        type Currency: MultiCurrencyExtended<
                Self::AccountId,
                CurrencyId = Self::CurrencyId,
                Balance = Balance,
                Amount = Amount,
            > + StakingCurrency<Self::AccountId>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn get_result_sets)]
    pub type ResultSets<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, ResultSet<T::BlockNumber>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn get_path)]
    pub type Paths<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, Vec<Path<T::AccountId>>, ValueQuery>;

    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        NewExamine,
    }

    #[pallet::error]
    pub enum Error<T> {
        // 已存在质询
        AlreadyExist,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        // 发起新的质询
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn new_examine(
            origin: OriginFor<T>,
            target: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            let challenger = ensure_signed(origin)?;

            Ok(().into())
        }
    }
}
