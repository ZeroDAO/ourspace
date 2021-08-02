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
use zd_primitives::{Amount, AppId, Balance};
use zd_traits::{ChallengeBase, Reputation, SeedsBase, TrustBase};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use sp_runtime::{
    traits::{AtLeast32BitUnsigned, Zero},
    DispatchError, DispatchResult,
};
use sp_std::{convert::TryInto, vec::Vec};

pub use pallet::*;

const APP_ID: AppId = *b"seed    ";

// Candidate
#[derive(Encode, Decode, Clone, Default, RuntimeDebug)]
pub struct Candidate<AccountId> {
    pub score: u64,
    pub pathfinder: AccountId,
}

#[derive(Encode, Decode, Clone, Default, RuntimeDebug)]
pub struct ResultSet<BlockNumber> {
    pub order: u32,
    pub score: u64,
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
        type Reputation: Reputation<Self::AccountId, Self::BlockNumber>;
        type ChallengeBase: ChallengeBase<Self::AccountId, AppId, Balance>;
        #[pallet::constant]
        type StakingAmount: Get<Balance>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn get_result_sets)]
    pub type ResultSets<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, ResultSet<T::BlockNumber>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn get_candidate)]
    pub type Candidates<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, Candidate<T::AccountId>, ValueQuery>;

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
        // 已存在
        AlreadyExist,
        //
        NoUpdatesAllowed,
        // 不存在对应数据
        NotExist,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        // 增加新候选种子
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn add(
            origin: OriginFor<T>,
            target: T::AccountId,
            score: u64,
        ) -> DispatchResultWithPostInfo {
            let pathfinder = ensure_signed(origin)?;
            let _ = T::Reputation::check_update_status(true).ok_or(Error::<T>::NoUpdatesAllowed)?;

            ensure!(
                <Candidates<T>>::contains_key(target.clone()),
                Error::<T>::AlreadyExist
            );

            T::Currency::staking(T::BaceToken::get(), &pathfinder, T::StakingAmount::get())?;

            <Candidates<T>>::insert(target, Candidate { score, pathfinder });

            T::Reputation::set_last_refresh_at();

            Ok(().into())
        }

        // 新的挑战
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn new_challenge(
            origin: OriginFor<T>,
            target: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            let challenger = ensure_signed(origin)?;

            let candidate = <Candidates<T>>::try_get(target.clone())
                .map_err(|_err| Error::<T>::NoUpdatesAllowed)?;

            T::ChallengeBase::new(
                &APP_ID,
                &challenger,
                &candidate.pathfinder,
                Zero::zero(),
                T::StakingAmount::get(),
                &target,
                Zero::zero(),
                Zero::zero(),
            )?;

            T::Reputation::set_last_refresh_at();

            Ok(().into())
        }

        // 质询
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn question(
            origin: OriginFor<T>,
            target: T::AccountId,
            index: u32,
        ) -> DispatchResultWithPostInfo {
            let challenger = ensure_signed(origin)?;

            T::ChallengeBase::question(&APP_ID, challenger, &target, index)?;

            T::Reputation::set_last_refresh_at();

            Ok(().into())
        }
    }
}
