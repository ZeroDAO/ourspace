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
    traits::Zero,
    DispatchError, DispatchResult,
};
use sp_std::{cmp::Ordering, vec::Vec};

pub use pallet::*;

pub mod types;
pub use self::types::*;
pub mod functions;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

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
        /// Already exists
        AlreadyExist,
        /// No permission or wrong time
        NoUpdatesAllowed,
        // No corresponding data exists
        NotExist,
        /// Depth limit exceeded
        DepthLimitExceeded,
        /// Overflow
        Overflow,
        /// Index Exceeds maximum
        IndexExceedsMaximum,
        /// Quantity exceeds limit
        QuantityExceedsLimit,
        /// Already exists
        AlreadyExists,
        /// NonExistent
        NonExistent,
        /// Path vector is empty
        NoPath,
        /// Certain data do not match
        NotMatch,
        /// The path is too long
        PathTooLong,
        /// Path length limit exceeded
        ExceededLengthLimit,
        /// Depth not yet reached
        DepthDoesNotMatch,
        /// Path lengths are not equal
        LengthNotEqual,
        /// The data at this index position does not match
        PathIndexError,
        /// Too Few In Number
        TooFewInNumber,
        /// Path length too long or not short enough
        WrongPathLength,
        /// There are still unearned challenges
        StillUnharvestedChallenges,
        /// score list is not empty
        ScoreListNotEmpty,
        /// Step is not match 
        StepNotMatch,
        /// Path does not exist
        PathDoesNotExist,
        /// The path is too short
        PathTooTooShort,
        /// Order does not match
        OrderNotMatch,
        /// An error occurred converting the data
        ConverError,
        /// Data is empty, cannot call next
        DataEmpty,
        /// No duplicate data allowed
        DataDuplication,
        /// Excessive number of paths
        LengthTooLong,
        /// Hash mismatch
        HashMismatch,
        /// Score mismatch
        ScoreMismatch,
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
                Zero::zero(),
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
            match <Paths<T>>::try_get(&target) {
                Ok(paths) => {
                    ensure!(
                        (index as usize) < paths.len(),
                        Error::<T>::IndexExceedsMaximum
                    );
                }
                Err(_) => {
                    let result_hash_set = result_hash_sets.last().unwrap();
                    ensure!(
                        (index as usize) < result_hash_set.len(),
                        Error::<T>::IndexExceedsMaximum
                    );
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
            ensure!(quantity <= MAX_HASH_COUNT, Error::<T>::QuantityExceedsLimit);
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
                Error::<T>::AlreadyExists
            );
            let hash_len = <ResultHashsSets<T>>::decode_len(&target)
                .ok_or_else(|| Error::<T>::NonExistent)?;
            ensure!(hash_len == DEEP as usize, Error::<T>::DepthDoesNotMatch);
            ensure!(!paths.is_empty(), Error::<T>::NoPath);
            let mut paths = paths;
            paths.sort();
            paths.dedup();
            ensure!(count == paths.len(), Error::<T>::NotMatch);
            let _ = T::ChallengeBase::reply(
                &APP_ID,
                &pathfinder,
                &target,
                quantity,
                count as u32,
                |is_all_done, index, order| -> Result<u64, DispatchError> {
                    let deep = <ResultHashsSets<T>>::decode_len(&target).ok_or(Error::<T>::NonExistent)?;
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

                    ensure!(old_len == full_paths.len(), Error::<T>::NotMatch);

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
                        Error::<T>::NotMatch
                    );
                    match <Paths<T>>::try_get(&target) {
                        Ok(path_vec) => {
                            let mut same_ends = false;
                            ensure!(
                                !<MissedPaths<T>>::contains_key(&target),
                                Error::<T>::AlreadyExist
                            );
                            for p in path_vec {
                                if *start == p.nodes[0] && stop == p.nodes.last().unwrap() {
                                    ensure!(
                                        p.nodes.len() == nodes.len(),
                                        Error::<T>::LengthNotEqual
                                    );
                                    ensure!(p.nodes != nodes, Error::<T>::AlreadyExist);
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
                                Error::<T>::IndexExceedsMaximum
                            );
                            let segment_order = full_order[deep..].to_vec();
                            if index > 0 {
                                ensure!(
                                    last_r_hash[index - 1].order[..].to_vec() < segment_order,
                                    Error::<T>::PathIndexError
                                );
                            }
                            if index < last_r_hash.len() - 1 {
                                ensure!(
                                    last_r_hash[index].order[..].to_vec() > segment_order,
                                    Error::<T>::PathIndexError
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
                Error::<T>::NotMatch
            );

            ensure!(
                p_path.nodes.len() + 2 > nodes.len(),
                Error::<T>::PathTooLong
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

            let mut mid_paths = mid_paths;
            mid_paths.sort();
            mid_paths.dedup();

            ensure!(
                mid_paths.len() <= (MAX_SHORTEST_PATH + 1) as usize,
                Error::<T>::ExceededLengthLimit
            );

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
                        ensure!(path.len() == p_path_len, Error::<T>::LengthNotEqual);
                    }
                    ensure!(
                        mid_paths.len() > p_path_total,
                        Error::<T>::TooFewInNumber
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
                Error::<T>::WrongPathLength
            );
            T::TrustBase::valid_nodes(&paths)?;
            let afer_target = paths.contains(&target);
            T::ChallengeBase::arbitral(
                &APP_ID,
                &challenger,
                &target,
                |_,_| -> Result<(bool, bool, u64), DispatchError> { Ok((afer_target, true, score)) },
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
                Error::<T>::StillUnharvestedChallenges
            );
            let is_sweeper = who == target;
            let mut score_list = Self::get_score_list();
            ensure!(score_list.is_empty(), Error::<T>::ScoreListNotEmpty);
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

            if score_list.is_empty() || Self::is_all_harvest() {
                T::Reputation::set_step(&TIRStep::REPUTATION);
            }
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
