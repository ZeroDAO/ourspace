#![cfg_attr(not(feature = "std"), no_std)]

// use frame_support::{ensure, dispatch::DispatchResultWithPostInfo, pallet, pallet_prelude::*};
use frame_support::{
    codec::{Decode, Encode},
    ensure, pallet,
    traits::Get,
    RuntimeDebug,
};
use orml_traits::{MultiCurrencyExtended, StakingCurrency};
use sha1::{Digest, Sha1};
use zd_primitives::{Amount, AppId, Balance};
use zd_traits::{ChallengeBase, Reputation, SeedsBase, TrustBase};
use zd_utilities::{UserSet, UserSetExt};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use sp_runtime::{
    traits::{AtLeast32Bit, Zero},
    DispatchError, DispatchResult,
};
use sp_std::{cmp::Ordering, convert::TryInto, vec::Vec};

pub use pallet::*;

const APP_ID: AppId = *b"seed    ";
const DEEP: u32 = 4;
const RANGE: u32 = 100;

// Candidate
#[derive(Encode, Decode, Clone, Default, RuntimeDebug)]
pub struct Candidate<AccountId> {
    pub score: u64,
    pub pathfinder: AccountId,
}

#[derive(Encode, Decode, Clone, Default, RuntimeDebug)]
pub struct ResultHash {
    pub order: u32,
    pub score: u64,
    pub hash: String,
}

// TODO binary_search_by_key & sort_by_key

impl Eq for ResultHash {}

impl Ord for ResultHash {
    fn cmp(&self, other: &Self) -> Ordering {
        self.order.cmp(&other.order)
    }
}

impl PartialOrd for ResultHash {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for ResultHash {
    fn eq(&self, other: &Self) -> bool {
        self.order == other.order
    }
}

type PathId = u128;

pub trait Convert<A>
where
    A: AtLeast32Bit + Copy,
    Self: Sized,
{
    fn from_ids(start: &A, stop: &A) -> Self;
    fn to_ids(&self) -> (A, A);
}

/// |<------PathId------>|
/// |***...***||***...***|
///      |          |
///    start       end
impl<A: AtLeast32Bit + Copy> Convert<A> for PathId {
    fn from_ids(start: &A, end: &A) -> Self {
        // .saturated_into()
        let start_into = TryInto::<u128>::try_into(*start).ok().unwrap();
        let end_into = TryInto::<u128>::try_into(*end).ok().unwrap();
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

pub trait OrderHelper {
    fn to_order(&self) -> u32;
    fn does_math(&self, order: &u32) -> bool;
}

impl OrderHelper for PathId {
    fn to_order(&self) -> u32 {
        (self % (RANGE as u128)) as u32
    }

    fn does_math(&self, order: &u32) -> bool {
        *self >= RANGE.saturating_mul(*order).into()
            && *self < (RANGE + 1).saturating_mul(*order).into()
    }
}

#[derive(Encode, Decode, Clone, Default, PartialEq, RuntimeDebug)]
pub struct Path<AccountId> {
    pub id: PathId,
    pub nodes: Vec<AccountId>,
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
        type TrustBase: TrustBase<Self::AccountId>;
        #[pallet::constant]
        type StakingAmount: Get<Balance>;
        type AccountIdForPathId: Member
            + Parameter
            + AtLeast32Bit
            + Copy
            + From<Self::AccountId>
            + Into<Self::AccountId>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn get_result_hash)]
    pub type ResultHashs<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, Vec<UserSet<ResultHash>>, ValueQuery>;

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
        // Depth limit exceeded
        DepthLimitExceeded,
        // Overflow
        Overflow,
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
            let result_hash_sets =
                <ResultHashs<T>>::try_get(&target).map_err(|_| Error::<T>::DepthLimitExceeded)?;

            let result_hash_set = result_hash_sets.last().unwrap().clone();
            ensure!(
                (index + 1) as usize <= result_hash_set.len(),
                Error::<T>::DepthLimitExceeded
            );
            T::ChallengeBase::question(
                &APP_ID,
                challenger,
                &target,
                result_hash_set.0[index as usize].order,
            )?;
            T::Reputation::set_last_refresh_at();
            Ok(().into())
        }

        // reply_hash
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn reply_hash(
            origin: OriginFor<T>,
            target: T::AccountId,
            result_hashs: Vec<ResultHash>,
            quantity: u32,
        ) -> DispatchResultWithPostInfo {
            let challenger = ensure_signed(origin)?;
            let count = result_hashs.len();
            ensure!(quantity <= RANGE, Error::<T>::DepthLimitExceeded);
            let _ = T::ChallengeBase::reply(
                &APP_ID,
                challenger,
                &target,
                quantity,
                count as u32,
                |is_all_done, index| -> DispatchResult {
                    Self::update_result_hashs(&target, &result_hashs, is_all_done, index)
                },
            )?;
            T::Reputation::set_last_refresh_at();
            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn reply_hash_next(
            origin: OriginFor<T>,
            target: T::AccountId,
            result_hashs: Vec<ResultHash>,
        ) -> DispatchResultWithPostInfo {
            let challenger = ensure_signed(origin)?;
            let count = result_hashs.len();
            T::ChallengeBase::next(
                &APP_ID,
                &challenger,
                &target,
                count as u32,
                |_, index, is_all_done| -> Result<u32, DispatchError> {
                    Self::update_result_hashs(&target, &result_hashs, is_all_done, index)?;
                    Ok(index)
                },
            )?;
            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn evidence_of_missing(
            origin: OriginFor<T>,
            target: T::AccountId,
            nodes: Vec<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            let challenger = ensure_signed(origin)?;
            Self::checked_path(&nodes, &target)?;

            let mut mid_nodes = nodes.clone();
            mid_nodes.remove(0);
            mid_nodes.pop();

            // path_id of nodes of challenger
            let c_path_id = Self::to_path_id(&nodes[0], &nodes.last().unwrap());

            let c_order = c_path_id.to_order();

            T::ChallengeBase::new_evidence(
                &APP_ID,
                challenger,
                &target,
                |order| -> Result<bool, DispatchError> {
                    match <Paths<T>>::try_get(&target) {
                        Ok(path_vec) => {
                            ensure!(c_path_id.does_math(&order), Error::<T>::DepthLimitExceeded);
                            for p in path_vec {
                                if p.id == c_path_id {
                                    ensure!(
                                        p.nodes.len() == mid_nodes.len(),
                                        Error::<T>::DepthLimitExceeded
                                    );
                                    ensure!(p.nodes != mid_nodes, Error::<T>::DepthLimitExceeded);
                                }
                            }
                            // TODO 需要存储路径，接受二次挑战
                            Ok(true)
                        }
                        Err(_) => {
                            let result_hash_set = <ResultHashs<T>>::try_get(&target)
                                .map_err(|_| Error::<T>::DepthLimitExceeded)?;
                            for result_hash in result_hash_set.last().unwrap().0.clone() {
                                if result_hash.order == c_order {
                                    break;
                                }
                            }
                            // TODO 不需要存储路径，直接成功
                            Ok(false)
                        }
                    }
                },
            )?;

            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn evidence_of_shorter(
            origin: OriginFor<T>,
            target: T::AccountId,
            index: u32,
            path: Vec<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            let challenger = ensure_signed(origin)?;
            Self::checked_path(&path, &target)?;
            let p_path = Self::get_pathfinder_paths(&target, &index)?;
            let path_id = Self::to_path_id(&path[0], &path.last().unwrap());
            ensure!(path_id == p_path.id, Error::<T>::DepthLimitExceeded);
            ensure!(
                p_path.nodes.len() + 2 > path.len(),
                Error::<T>::DepthLimitExceeded
            );

            T::ChallengeBase::new_evidence(
                &APP_ID,
                challenger,
                &target,
                |_| -> Result<bool, DispatchError> { Ok(true) },
            )?;

            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn evidence_of_wrong_number(
            origin: OriginFor<T>,
            target: T::AccountId,
            index: u32,
            mid_paths: Vec<Vec<T::AccountId>>,
        ) -> DispatchResultWithPostInfo {
            let challenger = ensure_signed(origin)?;
            let p_path = Self::get_pathfinder_paths(&target, &index)?;
            let p_path_total = p_path.total as usize;
            let p_path_len = p_path.nodes.len() + 2;

            // TODO 去除重复

            let (start, stop) = Self::get_ids(&p_path.id);

            T::ChallengeBase::new_evidence(
                &APP_ID,
                challenger,
                &target,
                |_| -> Result<bool, DispatchError> { 
                    for mid_path in mid_paths.clone() {
                        let mut path = mid_path;
                        path.insert(0, start.clone());
                        path.insert(path.len(), stop.clone());
                        Self::checked_path(&path, &target)?;
                        if path.len() < p_path_len {
                            return Ok(true);
                        }
                        ensure!(path.len() == p_path_len, Error::<T>::DepthLimitExceeded);
                    };
                    ensure!(mid_paths.len() > p_path_total, Error::<T>::DepthLimitExceeded);
                    Ok(true)
                 },
            )?;
            
            Ok(().into())
        }
    }
}

impl<T: Config> Pallet<T> {
    pub(crate) fn get_pathfinder_paths(target: &T::AccountId,index: &u32) -> Result<Path<T::AccountId>, DispatchError> {
        let paths =<Paths<T>>::try_get(&target).map_err(|_| Error::<T>::DepthLimitExceeded)?;
        let index = *index as usize;
        ensure!(paths.len() > index, Error::<T>::DepthLimitExceeded);
        Ok(paths[index].clone())
    }

    pub(crate) fn get_ids(path_id: &PathId) -> (T::AccountId,T::AccountId) {
        let (start, stop):(T::AccountIdForPathId,T::AccountIdForPathId) = path_id.to_ids();
        (start.into(), stop.into())
    }

    pub(crate) fn hash(data: &[u8]) -> Digest {
        let mut hasher = Sha1::new();
        hasher.update(data);
        hasher.digest()
    }

    pub(crate) fn to_path_id(start: &T::AccountId, stop: &T::AccountId) -> PathId {
        PathId::from_ids(
            &T::AccountIdForPathId::from(start.clone()),
            &T::AccountIdForPathId::from(stop.clone()),
        )
    }

    pub(crate) fn checked_path(path: &Vec<T::AccountId>, target: &T::AccountId) -> DispatchResult {
        ensure!(path.len() >= 2, Error::<T>::DepthLimitExceeded);
        ensure!(path.contains(&target), Error::<T>::DepthLimitExceeded);
        T::TrustBase::valid_nodes(&path)?;
        Ok(())
    }

    pub(crate) fn update_result_hashs(
        target: &T::AccountId,
        result_hashs: &Vec<ResultHash>,
        do_validate: bool,
        index: u32,
    ) -> DispatchResult {
        let mut res_hash_set = Self::get_result_hash(&target);
        let current_deep = res_hash_set.len();
        ensure!((current_deep as u32) < DEEP, Error::<T>::DepthLimitExceeded);
        let result_vec = UserSet::from(result_hashs.clone());
        // TODO 判断是增加还是新建！或者放到上面去
        // TODO is_ascii
        res_hash_set.push(result_vec);
        match do_validate {
            true => Self::validate_result_hashs(&res_hash_set, index, &target),
            false => Ok(()),
        }

        // TODO 上传
    }

    pub(crate) fn validate_path_hash() {

    }

    pub(crate) fn validate_result_hashs(
        result_hashs: &Vec<UserSet<ResultHash>>,
        index: u32,
        target: &T::AccountId,
    ) -> DispatchResult {
        let deep = result_hashs.len();
        if deep == 0 {
            return Ok(());
        }
        let mut data = "".to_string();
        let fold_score = result_hashs[deep - 1]
            .0
            .iter()
            .try_fold::<_, _, Result<u64, DispatchError>>(0u64, |acc, r| {
                if deep < 2 {
                    data += &r.hash;
                }
                ensure!(r.order < RANGE, Error::<T>::DepthLimitExceeded);
                acc.checked_add(r.score)
                    .ok_or_else(|| Error::<T>::Overflow.into())
            })?;
        let total_score = match deep {
            1 => Self::get_candidate(&target).score,
            _ => {
                ensure!(
                    Self::hash(data.as_bytes()).to_string()
                        == result_hashs[deep - 2].0[index as usize].hash,
                    Error::<T>::DepthLimitExceeded
                );
                result_hashs[deep - 2].0[index as usize].score
            }
        };
        ensure!(fold_score == total_score, Error::<T>::DepthLimitExceeded);
        Ok(())
    }
}
