#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    codec::{Decode, Encode},
    ensure,
    traits::Get,
    RuntimeDebug,
};
use frame_system::{self as system};
use sha1::{Digest, Sha1};
use orml_utilities::OrderedSet;
use zd_primitives::{fee::ProxyFee, AppId, Balance, TIRStep};
use zd_traits::{ChallengeBase, MultiBaseToken, Reputation, SeedsBase, TrustBase};

use sp_runtime::{
    traits::{AtLeast32Bit, Zero},
    DispatchError, DispatchResult,
};
use sp_std::{cmp::Ordering, fmt::Display, vec::Vec};

const APP_ID: AppId = *b"seed    ";
const DEEP: u8 = 4;
const RANGE: usize = 2;
/// Number of valid shortest paths.
const MAX_SHORTEST_PATH: u32 = 100;

const MAX_HASH_COUNT: u32 = 16u32.pow(RANGE as u32);

#[derive(Encode, Decode, Clone, Default, Ord, PartialOrd, PartialEq, Eq, RuntimeDebug)]
pub struct Candidate<AccountId> {
    pub score: u64,
    pub pathfinder: AccountId,
}

#[derive(Encode, Decode, Clone, Default, Ord, PartialOrd, PartialEq, Eq, RuntimeDebug)]pub struct FullOrder(pub Vec<u8>);
impl FullOrder {
    fn to_u64(&mut self) -> Option<u64> {
        let len = self.0.len();
        if len > 8 {
            return None;
        }
        let mut arr = [0u8; 8];
        self.0.extend_from_slice(&arr[len..]);
        arr.copy_from_slice(self.0.as_slice());
        Some(u64::from_le_bytes(arr))
    }

    fn from_u64(from: &u64, deep: usize) -> Self {
        let mut full_order = FullOrder::default();
        if deep > 8 {
            full_order.0 = u64::to_le_bytes(*from).to_vec();
        } else {
            full_order.0 = u64::to_le_bytes(*from)[..deep].to_vec();
        }
        full_order
    }

    fn connect(&mut self, order: &Vec<u8>) {
        self.0.extend_from_slice(&order[..RANGE]);
    }

    fn connect_to_u64(&mut self, order: &Vec<u8>) -> Option<u64> {
        self.connect(order);
        self.to_u64()
    }
}

#[derive(Encode, Decode, Clone, Default, RuntimeDebug)]
pub struct ResultHash {
    pub order: [u8; RANGE],
    pub score: u64,
    pub hash: [u8; 8],
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

#[derive(Encode, Decode, Ord, PartialOrd, Eq, Clone, Default, PartialEq, RuntimeDebug)]
pub struct Path<AccountId> {
    pub nodes: Vec<AccountId>,
    pub total: u32,
}

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {

    use super::*;

    use frame_system::{ensure_signed, pallet_prelude::*};

    use frame_support::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type Reputation: Reputation<Self::AccountId, Self::BlockNumber, TIRStep>;
        type ChallengeBase: ChallengeBase<Self::AccountId, AppId, Balance, Self::BlockNumber>;
        type TrustBase: TrustBase<Self::AccountId>;
        type SeedsBase: SeedsBase<Self::AccountId>;
        type MultiBaseToken: MultiBaseToken<Self::AccountId, Balance>;
        #[pallet::constant]
        type SeedStakingAmount: Get<Balance>;
        #[pallet::constant]
        type MaxSeedCount: Get<u32>;
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
    pub type ResultHashsSets<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, Vec<OrderedSet<ResultHash>>, ValueQuery>;

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
        RefershSeedStared(T::AccountId),
    }

    #[pallet::error]
    pub enum Error<T> {
        // Already exists
        AlreadyExist,
        NoUpdatesAllowed,
        // No corresponding data exists
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
            let who = ensure_signed(origin)?;
            T::Reputation::new_round()?;
            Self::deposit_event(Event::RefershSeedStared(who));
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
            T::MultiBaseToken::staking(&pathfinder, &T::SeedStakingAmount::get())?;
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
                T::SeedStakingAmount::get(),
                &target,
                Zero::zero(),
                score,
            )?;
            T::Reputation::set_last_refresh_at();
            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn examine(
            origin: OriginFor<T>,
            target: T::AccountId,
            index: u32,
        ) -> DispatchResultWithPostInfo {
            let challenger = ensure_signed(origin)?;
            Self::check_step()?;
            let result_hash_sets =
                <ResultHashsSets<T>>::try_get(&target).map_err(|_| Error::<T>::DepthLimitExceeded)?;
            // let remark: u32;
            match <Paths<T>>::try_get(&target) {
                Ok(paths) => {
                    ensure!(
                        (index as usize) < paths.len(),
                        Error::<T>::DepthLimitExceeded
                    );
                    // remark = index;
                }
                Err(_) => {
                    let result_hash_set = result_hash_sets.last().unwrap();
                    ensure!(
                        (index as usize) < result_hash_set.len(),
                        Error::<T>::DepthLimitExceeded
                    );
                    // remark = result_hash_set.0[index as usize].order;
                }
            }
            T::ChallengeBase::examine(&APP_ID, challenger, &target, index)?;
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
            ensure!(quantity <= MAX_HASH_COUNT, Error::<T>::DepthLimitExceeded);
            let _ = T::ChallengeBase::reply(
                &APP_ID,
                &challenger,
                &target,
                quantity,
                count as u32,
                |is_all_done, index,order| -> Result<u64, DispatchError> {
                    let new_order = Self::get_next_order(&target, &order, &(index as usize))?;
                    Self::update_result_hashs(&target, &result_hashs, is_all_done, index,false)?;
                    Ok(new_order)
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
                &(count as u32),
                |_, index, is_all_done| -> Result<(u64,u32), DispatchError> {
                    Self::update_result_hashs(&target, &result_hashs, is_all_done, index,true)?;
                    Ok((Zero::zero(), index))
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
            let hash_len = <ResultHashsSets<T>>::decode_len(&target)
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
                |is_all_done, index, order| -> Result<u64, DispatchError> {
                    let deep = <ResultHashsSets<T>>::decode_len(&target).ok_or(Error::<T>::DepthLimitExceeded)?;
                    let r_hashs =
                        <ResultHashsSets<T>>::get(&target).last().unwrap().0[index as usize].clone();
                    Self::checked_paths_vec(&paths, &target, &FullOrder::from_u64(&order, deep).0, deep)?;
                    if is_all_done {
                        Self::verify_paths(&paths, &target, &r_hashs)?;
                    }
                    <Paths<T>>::insert(&target, &paths);
                    Ok(order)
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
                &(count as u32),
                |order, index, is_all_done| -> Result<(u64, u32), DispatchError> {
                    let deep = <ResultHashsSets<T>>::decode_len(&target).ok_or(Error::<T>::DepthLimitExceeded)?;
                    let r_hashs =
                        <ResultHashsSets<T>>::get(&target).last().unwrap().0[index as usize].clone();
                    let mut full_paths = <Paths<T>>::get(&target);

                    full_paths.extend_from_slice(&paths);
                    let old_len = full_paths.len();
                    full_paths.sort();
                    full_paths.dedup();

                    ensure!(old_len == full_paths.len(), Error::<T>::DepthLimitExceeded);

                    Self::checked_paths_vec(&paths, &target, &FullOrder::from_u64(&order, deep).0, deep)?;

                    if is_all_done {
                        Self::verify_paths(&full_paths, &target, &r_hashs)?;
                    }
                    <Paths<T>>::mutate(&target, |p| *p = full_paths);
                    Ok((order, index))
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
            let mut mid_paths = mid_paths;
            mid_paths.sort();
            mid_paths.dedup();

            Self::do_reply_num(&challenger, &target, &mid_paths)?;
            T::Reputation::set_last_refresh_at();
            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn evidence_of_missing(
            origin: OriginFor<T>,
            target: T::AccountId,
            nodes: Vec<T::AccountId>,
            index: u32,
        ) -> DispatchResultWithPostInfo {
            let challenger = ensure_signed(origin)?;
            Self::check_step()?;
            Self::checked_nodes(&nodes, &target)?;

            let (start,stop) = Self::get_nodes_ends(&nodes);

            let deep = <ResultHashsSets<T>>::decode_len(&target).ok_or(Error::<T>::DepthLimitExceeded)?;
            let full_order = Self::to_full_order(&start,&stop,deep + 1);

            let maybe_score = T::ChallengeBase::evidence(
                &APP_ID,
                &challenger,
                &target,
                |_, order| -> Result<bool, DispatchError> {
                    let index = index as usize;
                    let r_order = FullOrder::from_u64(&order, deep);
                    ensure!(
                        r_order.0 == full_order[..deep].to_vec(),
                        Error::<T>::DepthLimitExceeded
                    );
                    match <Paths<T>>::try_get(&target) {
                        Ok(path_vec) => {
                            let mut same_ends = false;
                            ensure!(
                                !<MissedPaths<T>>::contains_key(&target),
                                Error::<T>::DepthLimitExceeded
                            );
                            for p in path_vec {
                                if *start == p.nodes[0] && stop == p.nodes.last().unwrap() {
                                    ensure!(
                                        p.nodes.len() == nodes.len(),
                                        Error::<T>::DepthLimitExceeded
                                    );
                                    ensure!(p.nodes != nodes, Error::<T>::DepthLimitExceeded);
                                    same_ends = true;
                                }
                            }
                            Ok(!same_ends)
                        }
                        Err(_) => {
                            let result_hash_sets = <ResultHashsSets<T>>::try_get(&target)
                                .map_err(|_| Error::<T>::DepthLimitExceeded)?;
                            let last_r_hash = &result_hash_sets.last().unwrap().0;
                            ensure!(
                                index < last_r_hash.len(),
                                Error::<T>::DepthLimitExceeded
                            );
                            let segment_order = full_order[deep..].to_vec();
                            if index > 0 {
                                ensure!(
                                    last_r_hash[index - 1].order[..].to_vec() < segment_order,
                                    Error::<T>::DepthLimitExceeded
                                );
                            }
                            if index < last_r_hash.len() - 1 {
                                ensure!(
                                    last_r_hash[index].order[..].to_vec() > segment_order,
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
            nodes: Vec<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            let challenger = ensure_signed(origin)?;
            Self::check_step()?;
            Self::checked_nodes(&nodes, &target)?;

            let p_path = Self::get_pathfinder_paths(&target, &index)?;
            let (start,stop) = Self::get_ends(&p_path);

            ensure!(
                *start == nodes[0] && stop == nodes.last().unwrap(),
                Error::<T>::DepthLimitExceeded
            );

            ensure!(
                p_path.nodes.len() + 2 > nodes.len(),
                Error::<T>::DepthLimitExceeded
            );

            let maybe_score = T::ChallengeBase::evidence(
                &APP_ID,
                &challenger,
                &target,
                |_, _| -> Result<bool, DispatchError> { Ok(false) },
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

            // TODO Removal of duplicates

            let (start, stop) = Self::get_ends(&p_path);
            let maybe_score = T::ChallengeBase::evidence(
                &APP_ID,
                &challenger,
                &target,
                |_, _| -> Result<bool, DispatchError> {
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
                |_| -> Result<(bool, bool, u64), DispatchError> { Ok((afer_target, true, score)) },
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
            // TODO Throwing error if less than zero
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

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn skim(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            let _ = ensure_signed(origin)?;
            match Self::is_all_harvest() {
                true => {
                    <ScoreList<T>>::kill();
                }
                false => {
                    if <ScoreList<T>>::get().is_empty() {
                        <Candidates<T>>::remove_all();
                    }
                }
            }
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

    pub(crate) fn to_full_order(start: &T::AccountId, stop: &T::AccountId, deep: usize) -> Vec<u8> {
        let mut points = T::AccountId::encode(start);
        points.extend(T::AccountId::encode(stop).iter().cloned());
        let points_hash = Self::sha1_hasher(&points);
        let index = points_hash.len() - deep;
        points_hash[index..].to_vec()
    }

    // pub(crate) fn full_order(start: &T::AccountId, stop: &T::AccountId, deep: u8) -> [u8] {}

    pub(crate) fn check_hash(data: &[u8], hash: &[u8; 8]) -> bool {
        Self::sha1_hasher(data)[..8] == hash[..]
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

    pub(crate) fn get_ends(path: &Path<T::AccountId>) -> (&T::AccountId, &T::AccountId) {
        Self::get_nodes_ends(&path.nodes)
    }

    pub(crate) fn get_nodes_ends(nodes: &Vec<T::AccountId>) -> (&T::AccountId, &T::AccountId) {
        let stop = nodes.last().unwrap();
        (&nodes[0], stop)
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

    pub(crate) fn sha1_hasher(data: &[u8]) -> [u8; 20] {
        let mut hasher = Sha1::new();
        hasher.update(data);
        let result = hasher.finalize();
        let mut r = [0u8; 20];
        r.clone_from_slice(&result[..]);
        r
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
        <ResultHashsSets<T>>::remove(&target);
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
        order: &Vec<u8>,
        deep: usize,
    ) -> DispatchResult {
        for p in paths {
            ensure!(
                p.total > 0 && p.total < MAX_SHORTEST_PATH,
                Error::<T>::DepthLimitExceeded
            );
            let (start,stop) = Self::get_ends(&p);
            ensure!(
                Self::to_full_order(start,stop,deep) == *order,
                Error::<T>::DepthLimitExceeded
            );
            Self::checked_nodes(&p.nodes, target)?;
        }
        Ok(())
    }

    pub(crate) fn get_next_order(target: &T::AccountId, old_order: &u64,index: &usize) -> Result<u64, Error<T>> {
        let r_hashs_sets = <ResultHashsSets<T>>::try_get(target).map_err(|_| Error::<T>::DepthLimitExceeded)?;
        let next_level_order = r_hashs_sets.last().unwrap().0[*index].order.to_vec();
        let deep = r_hashs_sets.len();
        let mut full_order = FullOrder::from_u64(old_order, deep);
        full_order.connect_to_u64(&next_level_order).ok_or(Error::<T>::DepthLimitExceeded)
    }

    pub(crate) fn update_result_hashs(
        target: &T::AccountId,
        new_r_hashs: &Vec<ResultHash>,
        do_verify: bool,
        index: u32,
        next: bool,
    ) -> DispatchResult {
        let mut r_hashs_sets = <ResultHashsSets<T>>::get(target);
        let current_deep = r_hashs_sets.len();
        ensure!((current_deep as u8) < DEEP, Error::<T>::DepthLimitExceeded);

        match next {
            true => {
                ensure!(!r_hashs_sets.is_empty(), Error::<T>::DepthLimitExceeded);
                let mut r_hashs_vec = r_hashs_sets[current_deep - 1].0.clone();
                r_hashs_vec.extend_from_slice(&new_r_hashs[..]);
                let full_hashs_set = OrderedSet::from(r_hashs_vec.clone());
                ensure!(r_hashs_vec.len() == full_hashs_set.len(), Error::<T>::DepthLimitExceeded);
                r_hashs_sets[current_deep -1] = full_hashs_set;
            },
            false => {
                let r_hashs_set = OrderedSet::from(new_r_hashs.clone());
                ensure!(new_r_hashs.len() == r_hashs_set.len(), Error::<T>::DepthLimitExceeded);
                r_hashs_sets.push(r_hashs_set);
            },
        }

        if do_verify {
            Self::verify_result_hashs(&r_hashs_sets, index, &target)?;
        }

        <ResultHashsSets<T>>::mutate(target,|rs| *rs = r_hashs_sets);
        Ok(())
    }

    pub(crate) fn verify_paths(
        paths: &Vec<Path<T::AccountId>>,
        target: &T::AccountId,
        result_hash: &ResultHash,
    ) -> DispatchResult {
        let enlarged_total_score =
            paths
                .iter()
                .try_fold::<_, _, Result<u32, DispatchError>>(0u32, |acc, p| {
                    Self::checked_nodes(&p.nodes, &target)?;
                    ensure!(
                        p.total < 100,
                        Error::<T>::DepthLimitExceeded
                    );
                    // Two-digit accuracy
                    let score = 100 / p.total;
                    Ok(acc.saturating_add(score))
                })?;
        let total_score = enlarged_total_score
            .checked_div(100)
            .ok_or(Error::<T>::DepthLimitExceeded)?;

        // [AccountId,AccountId,total-...AccountId,AccountId,total-]
        let list_v = paths
            .iter()
            .flat_map(|path| {
                let mut nodes_v = path
                    .nodes
                    .iter()
                    .flat_map(|node| {
                        // push `,`
                        let mut node = node.encode();
                        node.push(44u8);
                        node
                    })
                    .collect::<Vec<u8>>();
                // path.total < 100
                nodes_v.push(path.total as u8);
                // push `-`
                nodes_v.push(45u8);
                nodes_v
            })
            .collect::<Vec<u8>>();

        ensure!(
            Self::check_hash(&list_v[..], &result_hash.hash),
            Error::<T>::DepthLimitExceeded
        );

        ensure!(
            total_score as u64 == result_hash.score,
            Error::<T>::DepthLimitExceeded
        );
        Ok(())
    }

    pub(crate) fn verify_result_hashs(
        result_hashs: &Vec<OrderedSet<ResultHash>>,
        index: u32,
        target: &T::AccountId,
    ) -> DispatchResult {
        let deep = result_hashs.len();
        if deep == 0 {
            return Ok(());
        }
        let mut data: Vec<u8> = Vec::default();

        let fold_score = result_hashs[deep - 1]
            .0
            .iter()
            .try_fold::<_, _, Result<u64, Error<T>>>(0u64, |acc, r| {
                if deep < 2 {
                    data.extend_from_slice(&r.hash);
                }
                ensure!(
                    r.order.len() == RANGE as usize,
                    Error::<T>::DepthLimitExceeded
                );
                acc.checked_add(r.score)
                    .ok_or_else(|| Error::<T>::Overflow.into())
            })?;
        let total_score = match deep {
            1 => Self::get_candidate(&target).score,
            _ => {
                ensure!(
                    Self::check_hash(
                        data.as_slice(),
                        &result_hashs[deep - 2].0[index as usize].hash
                    ),
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
            |_, index,_| -> Result<u64, DispatchError> {
                let p_path = Self::get_pathfinder_paths(&target, &index)?;
                ensure!(
                    (count as u32) == p_path.total,
                    Error::<T>::DepthLimitExceeded
                );
                let (start, stop) = Self::get_ends(&p_path);
                for mid_path in mid_paths {
                    let _ = Self::check_mid_path(&mid_path, &start, &stop, &target)?;
                }
                Ok(Zero::zero())
            },
        )?;
        Ok(())
    }
}
