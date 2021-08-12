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
use sha1::{Digest, Sha1};
use zd_primitives::{fee::ProxyFee, Amount, AppId, Balance, TIRStep};
use zd_traits::{ChallengeBase, MultiBaseToken, Reputation, SeedsBase, TrustBase};
use zd_utilities::{UserSet, UserSetExt};

use sp_runtime::{
    traits::{AtLeast32Bit, Zero},
    DispatchError, DispatchResult,
};
use sp_std::{cmp::Ordering, convert::TryInto, fmt::Display, vec::Vec};

pub use pallet::*;

const APP_ID: AppId = *b"seed    ";
const DEEP: u32 = 4;
const RANGE: u32 = 100;
// Don't exceed 100
const MAX_SHORTEST_PATH: u32 = 100;

// Candidate
#[derive(Encode, Decode, Clone, Default, Ord, PartialOrd, PartialEq, Eq, RuntimeDebug)]
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

impl ResultHash {
    fn limit(&self) -> (u32, u32) {
        // No overflow possible
        (self.order * RANGE, (self.order + 1) * RANGE)
    }
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

///  |<--- -PathId---->|
///  +--------+--------+
///  |  start |  stop  |
///  +--------+--------+

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
    fn does_math_up_order(&self, order: &u32) -> bool;
    fn does_math_limit_order(&self, uper_mimit: &u32, lower_limit: &u32) -> bool;
}

impl OrderHelper for PathId {
    fn to_order(&self) -> u32 {
        (self % (RANGE as u128)) as u32
    }

    fn does_math_up_order(&self, order: &u32) -> bool {
        *self >= RANGE.saturating_mul(*order).into()
            && *self < (RANGE + 1).saturating_mul(*order).into()
    }

    fn does_math_limit_order(&self, uper_mimit: &u32, lower_limit: &u32) -> bool {
        let order = self.to_order();
        order >= *lower_limit && order < *uper_mimit
    }
}

#[derive(Encode, Decode, Ord, PartialOrd, Eq, Clone, Default, PartialEq, RuntimeDebug)]
pub struct Path<AccountId> {
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
        type Reputation: Reputation<Self::AccountId, Self::BlockNumber, TIRStep>;
        type ChallengeBase: ChallengeBase<Self::AccountId, AppId, Balance, Self::BlockNumber>;
        type TrustBase: TrustBase<Self::AccountId>;
        type SeedsBase: SeedsBase<Self::AccountId>;
        type MultiBaseToken: MultiBaseToken<Self::AccountId, Balance>;
        #[pallet::constant]
        type StakingAmount: Get<Balance>;
        #[pallet::constant]
        type MaxSeedCount: Get<Balance>;
        #[pallet::constant]
        type HarvestPeriod: Get<Balance>;
        type AccountIdForPathId: Member
            + Parameter
            + AtLeast32Bit
            + Copy
            + Display
            + From<Self::AccountId>
            + Into<Self::AccountId>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn get_result_hashs)]
    pub type ResultHashs<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, Vec<UserSet<ResultHash>>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn get_candidate)]
    pub type Candidates<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, Candidate<T::AccountId>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn get_score_list)]
    pub type ScoreList<T: Config> = StorageValue<_, Vec<u64>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn get_path)]
    pub type Paths<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, Vec<Path<T::AccountId>>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn get_missed_paths)]
    pub type MissedPaths<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, Vec<T::AccountId>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn get_last_refresh_at)]
    pub type LastRefreshAt<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;

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
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn start(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            let _ = ensure_signed(origin)?;
            T::Reputation::new_round()?;
            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn add(
            origin: OriginFor<T>,
            target: T::AccountId,
            score: u64,
        ) -> DispatchResultWithPostInfo {
            let pathfinder = ensure_signed(origin)?;
            Self::check_step()?;
            ensure!(
                <Candidates<T>>::contains_key(target.clone()),
                Error::<T>::AlreadyExist
            );
            T::Currency::staking(T::BaceToken::get(), &pathfinder, T::StakingAmount::get())?;
            Self::candidate_insert(&target, &pathfinder, &score);
            T::Reputation::set_last_refresh_at();
            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn challenge(
            origin: OriginFor<T>,
            target: T::AccountId,
            score: u64,
        ) -> DispatchResultWithPostInfo {
            let challenger = ensure_signed(origin)?;
            Self::check_step()?;
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
                score,
            )?;
            T::Reputation::set_last_refresh_at();
            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn question(
            origin: OriginFor<T>,
            target: T::AccountId,
            index: u32,
        ) -> DispatchResultWithPostInfo {
            let challenger = ensure_signed(origin)?;
            Self::check_step()?;
            let result_hash_sets =
                <ResultHashs<T>>::try_get(&target).map_err(|_| Error::<T>::DepthLimitExceeded)?;
            let remark: u32;
            match <Paths<T>>::try_get(&target) {
                Ok(paths) => {
                    ensure!(
                        (index as usize) < paths.len(),
                        Error::<T>::DepthLimitExceeded
                    );
                    remark = index;
                }
                Err(_) => {
                    let result_hash_set = result_hash_sets.last().unwrap();
                    ensure!(
                        (index as usize) < result_hash_set.len(),
                        Error::<T>::DepthLimitExceeded
                    );
                    remark = result_hash_set.0[index as usize].order;
                }
            }
            T::ChallengeBase::question(&APP_ID, challenger, &target, remark)?;
            T::Reputation::set_last_refresh_at();
            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn reply_hash(
            origin: OriginFor<T>,
            target: T::AccountId,
            result_hashs: Vec<ResultHash>,
            quantity: u32,
        ) -> DispatchResultWithPostInfo {
            let challenger = ensure_signed(origin)?;
            Self::check_step()?;
            let count = result_hashs.len();
            ensure!(quantity <= RANGE, Error::<T>::DepthLimitExceeded);
            let _ = T::ChallengeBase::reply(
                &APP_ID,
                &challenger,
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
            Self::check_step()?;
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
        pub fn reply_path(
            origin: OriginFor<T>,
            target: T::AccountId,
            paths: Vec<Path<T::AccountId>>,
            quantity: u32,
        ) -> DispatchResultWithPostInfo {
            let pathfinder = ensure_signed(origin)?;
            Self::check_step()?;
            let count = paths.len();
            ensure!(
                <Paths<T>>::try_get(&target).is_err(),
                Error::<T>::DepthLimitExceeded
            );
            let hash_len = <ResultHashs<T>>::decode_len(&target)
                .ok_or_else(|| Error::<T>::DepthLimitExceeded)?;
            ensure!(hash_len == DEEP as usize, Error::<T>::DepthLimitExceeded);
            ensure!(!paths.is_empty(), Error::<T>::DepthLimitExceeded);
            let mut paths = paths;
            paths.sort();
            paths.dedup();
            ensure!(count == paths.len(), Error::<T>::DepthLimitExceeded);
            let _ = T::ChallengeBase::reply(
                &APP_ID,
                &pathfinder,
                &target,
                quantity,
                count as u32,
                |is_all_done, index| -> DispatchResult {
                    let r_hashs =
                        <ResultHashs<T>>::get(&target).last().unwrap().0[index as usize].clone();
                    let (uper_limit, lower_limit) = r_hashs.limit();
                    Self::checked_paths_vec(&paths, &target, uper_limit, lower_limit)?;
                    if is_all_done {
                        Self::verify_paths(&paths, &target)?;
                    }
                    <Paths<T>>::insert(&target, &paths);
                    Ok(().into())
                },
            )?;

            T::Reputation::set_last_refresh_at();
            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn reply_path_next(
            origin: OriginFor<T>,
            target: T::AccountId,
            paths: Vec<Path<T::AccountId>>,
        ) -> DispatchResultWithPostInfo {
            let pathfinder = ensure_signed(origin)?;
            Self::check_step()?;
            let count = paths.len();
            T::ChallengeBase::next(
                &APP_ID,
                &pathfinder,
                &target,
                count as u32,
                |_, index, is_all_done| -> Result<u32, DispatchError> {
                    let r_hashs =
                        <ResultHashs<T>>::get(&target).last().unwrap().0[index as usize].clone();
                    let (uper_limit, lower_limit) = r_hashs.limit();
                    let mut full_paths = <Paths<T>>::get(&target);
                    full_paths.extend_from_slice(&paths);
                    let old_len = full_paths.len();
                    full_paths.sort();
                    full_paths.dedup();
                    ensure!(old_len == full_paths.len(), Error::<T>::DepthLimitExceeded);
                    Self::checked_paths_vec(&paths, &target, uper_limit, lower_limit)?;
                    if is_all_done {
                        Self::verify_paths(&full_paths, &target)?;
                    }
                    <Paths<T>>::mutate(&target, |p| *p = full_paths);
                    // TODO 如果全部结束，则修改返回值为 uper limit
                    Ok(index)
                },
            )?;

            T::Reputation::set_last_refresh_at();
            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn reply_num(
            origin: OriginFor<T>,
            target: T::AccountId,
            mid_paths: Vec<Vec<T::AccountId>>,
        ) -> DispatchResultWithPostInfo {
            let challenger = ensure_signed(origin)?;
            Self::check_step()?;
            // TODO 检测重复
            Self::do_reply_num(&challenger, &target, &mid_paths)?;
            // TODO restart - 或者等待超时
            T::Reputation::set_last_refresh_at();
            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn evidence_of_missing(
            origin: OriginFor<T>,
            target: T::AccountId,
            nodes: Vec<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            let challenger = ensure_signed(origin)?;
            Self::check_step()?;
            Self::checked_nodes(&nodes, &target)?;

            // path_id of nodes of challenger
            let c_path_id = Self::to_path_id(&nodes[0], &nodes.last().unwrap());
            let c_order = c_path_id.to_order();

            let maybe_score = T::ChallengeBase::new_evidence(
                &APP_ID,
                &challenger,
                &target,
                |order, c_score| -> Result<bool, DispatchError> {
                    match <Paths<T>>::try_get(&target) {
                        Ok(path_vec) => {
                            let mut have_path_id = false;
                            ensure!(
                                !<MissedPaths<T>>::contains_key(&target),
                                Error::<T>::DepthLimitExceeded
                            );
                            ensure!(
                                c_path_id.does_math_up_order(&order),
                                Error::<T>::DepthLimitExceeded
                            );
                            for p in path_vec {
                                if Self::get_path_id(&p) == c_path_id {
                                    ensure!(
                                        p.nodes.len() == nodes.len(),
                                        Error::<T>::DepthLimitExceeded
                                    );
                                    ensure!(p.nodes != nodes, Error::<T>::DepthLimitExceeded);
                                    have_path_id = true;
                                }
                            }
                            Ok(!have_path_id)
                        }
                        Err(_) => {
                            let result_hash_set = <ResultHashs<T>>::try_get(&target)
                                .map_err(|_| Error::<T>::DepthLimitExceeded)?;
                            for result_hash in result_hash_set.last().unwrap().0.clone() {
                                ensure!(
                                    !(result_hash.order <= c_order
                                        && result_hash.order + RANGE > c_order),
                                    Error::<T>::DepthLimitExceeded
                                );
                            }
                            // arbitration : Unable to determine the shortest path
                            Ok(true)
                        }
                    }
                },
            )?;

            match maybe_score {
                Some(score) => Self::restart(&target, &challenger, &score),
                None => <MissedPaths<T>>::insert(&target, nodes),
            }

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
            Self::check_step()?;
            Self::checked_nodes(&path, &target)?;
            let p_path = Self::get_pathfinder_paths(&target, &index)?;
            let path_id = Self::to_path_id(&path[0], &path.last().unwrap());
            ensure!(
                path_id == Self::get_path_id(&p_path),
                Error::<T>::DepthLimitExceeded
            );
            ensure!(
                p_path.nodes.len() + 2 > path.len(),
                Error::<T>::DepthLimitExceeded
            );
            let mut score: u64;
            let maybe_score = T::ChallengeBase::new_evidence(
                &APP_ID,
                &challenger,
                &target,
                |_, c_score| -> Result<bool, DispatchError> { Ok(false) },
            )?;
            Self::restart(&target, &challenger, &maybe_score.unwrap_or_default());
            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn number_too_low(
            origin: OriginFor<T>,
            target: T::AccountId,
            index: u32,
            mid_paths: Vec<Vec<T::AccountId>>,
        ) -> DispatchResultWithPostInfo {
            let challenger = ensure_signed(origin)?;
            Self::check_step()?;
            let p_path = Self::get_pathfinder_paths(&target, &index)?;
            let p_path_total = p_path.total as usize;
            let p_path_len = p_path.nodes.len() + 2;

            ensure!(
                mid_paths.len() <= (MAX_SHORTEST_PATH + 1) as usize,
                Error::<T>::DepthLimitExceeded
            );

            // TODO 去除重复

            let (start, stop) = Self::get_ids(&p_path);
            let maybe_score = T::ChallengeBase::new_evidence(
                &APP_ID,
                &challenger,
                &target,
                |_, c_score| -> Result<bool, DispatchError> {
                    for mid_path in mid_paths.clone() {
                        let path = Self::check_mid_path(&mid_path, &start, &stop, &target)?;
                        if path.len() < p_path_len {
                            return Ok(true);
                        }
                        ensure!(path.len() == p_path_len, Error::<T>::DepthLimitExceeded);
                    }
                    ensure!(
                        mid_paths.len() > p_path_total,
                        Error::<T>::DepthLimitExceeded
                    );
                    Ok(false)
                },
            )?;
            Self::restart(&target, &challenger, &maybe_score.unwrap_or_default());
            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn invalid_evidence(
            origin: OriginFor<T>,
            target: T::AccountId,
            paths: Vec<T::AccountId>,
            score: u64,
        ) -> DispatchResultWithPostInfo {
            let challenger = ensure_signed(origin)?;
            Self::check_step()?;
            let missed_path =
                <MissedPaths<T>>::try_get(&target).map_err(|_| Error::<T>::DepthLimitExceeded)?;
            ensure!(
                paths.len() < missed_path.len() && paths.len() >= 2,
                Error::<T>::DepthLimitExceeded
            );
            T::TrustBase::valid_nodes(&paths)?;
            let afer_target = paths.contains(&target);
            T::ChallengeBase::arbitral(
                &APP_ID,
                &challenger,
                &target,
                score,
                |_| -> Result<(bool, bool), DispatchError> { Ok((afer_target, true)) },
            )?;
            if afer_target {
                Self::restart(&target, &challenger, &score);
            }
            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn harvest_challenge(
            origin: OriginFor<T>,
            target: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            Self::check_step()?;

            Self::do_harvest_challenge(&who, &target)?;

            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn harvest_seed(
            origin: OriginFor<T>,
            target: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            Self::check_step()?;
            ensure!(
                T::ChallengeBase::is_all_harvest(&APP_ID),
                Error::<T>::DepthLimitExceeded
            );
            let is_sweeper = who == target;
            let mut score_list = Self::get_score_list();
            // TODO 小于零则抛错
            let len = Self::hand_first_time(&mut score_list);

            let candidate = <Candidates<T>>::take(&target);
            if !score_list.is_empty() && candidate.score >= score_list[0] {
                if let Ok(index) = score_list.binary_search(&candidate.score) {
                    T::SeedsBase::add_seed(&target);
                    score_list.remove(index);
                    let bonus = T::MultiBaseToken::get_bonus_amount();
                    let amount = bonus / (len as Balance);
                    match is_sweeper {
                        true => {
                            let now_block_number = system::Module::<T>::block_number();
                            let last = Self::get_last_refresh_at();
                            if let Some((s_amount, p_amount)) =
                                amount.checked_with_fee(last, now_block_number)
                            {
                                T::MultiBaseToken::release(&who, &s_amount)?;
                                T::MultiBaseToken::release(&candidate.pathfinder, &p_amount)?;
                            }
                        }
                        false => {
                            T::MultiBaseToken::release(&candidate.pathfinder, &amount)?;
                        }
                    }
                }
            }

            if score_list.is_empty() || Self::is_all_harvest() {}
            <ScoreList<T>>::put(score_list);
            Ok(().into())
        }
    }
}

impl<T: Config> Pallet<T> {
    fn is_all_harvest() -> bool {
        <Candidates<T>>::iter_values().next().is_none()
    }

    fn check_step() -> DispatchResult {
        ensure!(
            T::Reputation::is_step(&TIRStep::SEED),
            Error::<T>::DepthLimitExceeded
        );
        Ok(())
    }

    fn hand_first_time(score_list: &mut Vec<u64>) -> usize {
        let max_seed_count = T::MaxSeedCount::get() as usize;

        let mut len = score_list.len();
        if len > max_seed_count {
            *score_list = score_list[(len - max_seed_count)..].to_vec();
            len = max_seed_count;
        }
        T::SeedsBase::remove_all();
        len
    }

    pub(crate) fn get_pathfinder_paths(
        target: &T::AccountId,
        index: &u32,
    ) -> Result<Path<T::AccountId>, DispatchError> {
        let paths = <Paths<T>>::try_get(&target).map_err(|_| Error::<T>::DepthLimitExceeded)?;
        let index = *index as usize;
        ensure!(paths.len() > index, Error::<T>::DepthLimitExceeded);
        Ok(paths[index].clone())
    }

    pub(crate) fn do_harvest_challenge<'a>(
        who: &T::AccountId,
        target: &'a T::AccountId,
    ) -> DispatchResult {
        match T::ChallengeBase::harvest(&who, &APP_ID, &target)? {
            Some(score) => {
                <Candidates<T>>::mutate(target, |c| {
                    Self::mutate_score(&c.score, &score);
                    c.score = score
                });
            }
            None => (),
        }
        Self::remove_challenge(&target);
        Ok(())
    }

    pub(crate) fn get_ids<'a>(path: &'a Path<T::AccountId>) -> (&T::AccountId, &T::AccountId) {
        let stop = path.nodes.last().unwrap();
        (&path.nodes[0], stop)
    }

    pub(crate) fn get_path_id(path: &Path<T::AccountId>) -> PathId {
        let stop = path.nodes.last().unwrap();
        Self::to_path_id(&path.nodes[0], stop)
    }

    pub(crate) fn candidate_insert(targer: &T::AccountId, pathfinder: &T::AccountId, score: &u64) {
        <Candidates<T>>::insert(
            targer,
            Candidate {
                score: *score,
                pathfinder: pathfinder.clone(),
            },
        );
        let mut score_list = Self::get_score_list();
        Self::score_list_insert(&mut score_list, score);
    }

    pub(crate) fn mutate_score(old_score: &u64, new_score: &u64) {
        let mut score_list = Self::get_score_list();
        if let Ok(index) = score_list.binary_search(old_score) {
            score_list.remove(index);
        }
        Self::score_list_insert(&mut score_list, new_score);
    }

    pub fn score_list_insert(score_list: &mut Vec<u64>, score: &u64) {
        let index = score_list
            .binary_search(score)
            .unwrap_or_else(|index| index);
        score_list.insert(index, *score);
        <ScoreList<T>>::put(score_list);
    }

    pub(crate) fn check_mid_path(
        mid_path: &Vec<T::AccountId>,
        start: &T::AccountId,
        stop: &T::AccountId,
        target: &T::AccountId,
    ) -> Result<Vec<T::AccountId>, DispatchError> {
        let mut path = mid_path.clone();
        path.insert(0, start.clone());
        path.insert(path.len(), stop.clone());
        Self::checked_nodes(&path, &target)?;
        Ok(path.to_vec())
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

    pub(crate) fn restart(target: &T::AccountId, pathfinder: &T::AccountId, score: &u64) {
        <Candidates<T>>::mutate(&target, |c| {
            Self::mutate_score(&c.score, score);
            c.score = *score;
            c.pathfinder = pathfinder.clone();
        });
        Self::remove_challenge(&target)
    }

    pub(crate) fn remove_challenge(target: &T::AccountId) {
        <Paths<T>>::remove(&target);
        <ResultHashs<T>>::remove(&target);
        <MissedPaths<T>>::remove(&target);
    }

    pub(crate) fn checked_nodes(
        nodes: &Vec<T::AccountId>,
        target: &T::AccountId,
    ) -> DispatchResult {
        ensure!(nodes.len() >= 2, Error::<T>::DepthLimitExceeded);
        ensure!(nodes.contains(&target), Error::<T>::DepthLimitExceeded);
        T::TrustBase::valid_nodes(&nodes)?;
        Ok(())
    }

    pub(crate) fn checked_paths_vec(
        paths: &Vec<Path<T::AccountId>>,
        target: &T::AccountId,
        uper_limit: u32,
        lower_limit: u32,
    ) -> DispatchResult {
        for p in paths {
            ensure!(
                Self::get_path_id(p).does_math_limit_order(&uper_limit, &lower_limit),
                Error::<T>::DepthLimitExceeded
            );
            ensure!(
                p.total > 0 && p.total < MAX_SHORTEST_PATH,
                Error::<T>::DepthLimitExceeded
            );
            Self::checked_nodes(&p.nodes, target)?;
        }
        Ok(())
    }

    pub(crate) fn update_result_hashs(
        target: &T::AccountId,
        result_hashs: &Vec<ResultHash>,
        do_verify: bool,
        index: u32,
    ) -> DispatchResult {
        let mut res_hash_set = Self::get_result_hashs(&target);
        let current_deep = res_hash_set.len();
        ensure!((current_deep as u32) < DEEP, Error::<T>::DepthLimitExceeded);
        let result_vec = UserSet::from(result_hashs.clone());
        // TODO 判断是增加还是新建！或者放到上面去
        // TODO is_ascii
        res_hash_set.push(result_vec);

        match do_verify {
            true => Self::verify_result_hashs(&res_hash_set, index, &target),
            false => Ok(()),
        }

        // TODO 上传
    }

    pub(crate) fn verify_paths(
        paths: &Vec<Path<T::AccountId>>,
        target: &T::AccountId,
    ) -> DispatchResult {
        let enlarged_total_score =
            paths
                .iter()
                .try_fold::<_, _, Result<u32, DispatchError>>(0u32, |acc, p| {
                    Self::checked_nodes(&p.nodes, &target)?;
                    // Two-digit accuracy
                    let score = 100 / p.total;
                    Ok(acc.saturating_add(score))
                })?;

        // String: "AccountId,AccountId,total-AccountId,AccountId,total..."
        let list_str = paths
            .iter()
            .map(|p| {
                p.nodes
                    .iter()
                    .map(|a| T::AccountIdForPathId::from(a.clone()).to_string())
                    .chain([p.total.to_string()])
                    .collect::<Vec<String>>()
                    .join(",")
            })
            .collect::<Vec<String>>()
            .join("-");
        let hash = Self::hash(list_str.as_bytes()).to_string();
        // TODO 验证分数和hash
        Ok(())
    }

    pub(crate) fn verify_result_hashs(
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

    fn do_reply_num(
        challenger: &T::AccountId,
        target: &T::AccountId,
        mid_paths: &Vec<Vec<T::AccountId>>,
    ) -> DispatchResult {
        let count = mid_paths.len();
        let _ = T::ChallengeBase::reply(
            &APP_ID,
            challenger,
            target,
            Zero::zero(),
            Zero::zero(),
            |_, index| -> DispatchResult {
                let p_path = Self::get_pathfinder_paths(&target, &index)?;
                ensure!(
                    (count as u32) == p_path.total,
                    Error::<T>::DepthLimitExceeded
                );
                // TODO 修改端点获取方式
                let (start, stop) = Self::get_ids(&p_path);
                for mid_path in mid_paths {
                    let _ = Self::check_mid_path(&mid_path, &start, &stop, &target)?;
                }
                Ok(())
            },
        )?;
        Ok(())
    }
}
