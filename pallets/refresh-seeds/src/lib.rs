// Copyright 2021 ZeroDAO
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # ZdRefreshSeeds Module
//!
//! ## Overview
//!
//! This module is used for seed selection and to ensure correctness through challenge games.
//!
//! ## Algorithms
//!
//! Users' trust relationships form a directed graph, and the user with the highest intermediary
//! count centrality will be the seed. To ensure consistency, it is necessary to calculate the
//! mediator centrality exactly. For example, we have the following path and we need to compute
//! the centrality score of D.
//!
//! A -> B -> D -> E
//! A -> C -> D -> E
//!
//! ### Calculating the score
//!
//! All shortest paths through `D` are
//!
//! 1 A -> B -> D -> E
//! 2 A -> C -> D -> E
//! 3 B -> D -> E
//! 4 C -> D -> E
//!
//! where there are two shortest paths of the same length from A to E, i.e. `num = 2`,
//! and the score for each path is `100 / 2`, and the score for the other two paths is
//! `100`. The total score for `D` is 300.
//!
//! |  path  | num | score |
//! |--------|-----|-------|
//! | ABDE   |  2  |  50   |
//! | ACDE   |  2  |  50   |
//! | BDE    |  1  |  100  |
//! | CDE    |  1  |  100  |
//!
//! ### Sorting
//!
//! When the network is large, the number of shortest paths through a node may be in the
//! hundreds of millions, and it is impractical and unnecessary to verify them all on
//! the chain. We therefore use interactive verification.
//!
//! #### Endpoint hash
//!
//! Hash the endpoints and take the last eight bits.
//!
//! |  path  | num | score |     sha1(start,stop)      |
//! |--------|-----|-------|---------------------------|
//! | ABDE   |  2  |  50   |     ...f9 90 6c f1        |
//! | ACDE   |  2  |  50   |     ...f9 90 6c f1        |
//! | BDE    |  1  |  100  |     ...7c fe 03 66        |
//! | CDE    |  1  |  100  |     ...65 ce 02 66        |
//!
//! #### Tree
//!
//! Starting from the tail of the hash, the centrality scores are summed separately.
//!
//! | order  | score |
//! |--------|-------|
//! |   f1   |  100  |
//! |   66   |  200  |
//!
//! Once the challenger has launched the challenge, `pathfinder` needs to upload all
//! the data from the first level via `PostResultHash`. This allows the challenger to
//! identify the location of the discrepancy and continue to question (examine) it. Suppose the
//! challenger questions(examine) `66`. Then `pathfinder` needs to upload the second level of data.
//!
//! | order  | score |
//! |--------|-------|
//! |   03   |  100  |
//! |   02   |  100  |
//!
//! Up to the maximum depth set by the system, `pathfinder` uploads all paths that meet the criteria
//! and the challenger can continue to `examine` the paths.
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! - `start` - Start refreshing the seeds.
//! - `add` - Add a seed candidate and centrality score.
//! - `challenge` - A challenge to the scoring of a seed.
//! - `examine` - Initiate a examine on the data at the specified location.
//! - `reply_hash` - Reply to examine upon receipt.
//! - `reply_hash_next` - Continue to upload reply data.
//! - `reply_path` - reply to the path after receiving the examine.
//! - `reply_path_next` - Continue to upload the path data for the reply.
//! - `reply_num` - Reply to the number of shortest paths between two users.
//! - `missed_in_hashs` - Indicate the missing path in the hash phase.
//! - `missed_in_paths` - Indicate missing paths in the path phase.
//! - `evidence_of_shorter` - Show evidence of shorter paths.
//! - `number_too_low` - Show evidence that the number of paths between two users is too small.
//! - `invalid_evidence` - The evidence was proved to be wrong.
//! - `harvest_challenge` - Receive the proceeds of the challenge.
//! - `harvest_seed` - Receive seed proceeds.
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::{
    codec::{Decode, Encode},
    ensure,
    traits::Get,
    transactional, RuntimeDebug,
};
use frame_system::{self as system};
use sp_runtime::{traits::Zero, DispatchError, DispatchResult};
use sp_std::{cmp::Ordering, vec::Vec};

pub use orml_utilities::OrderedSet;

use zd_primitives::{fee::SweeperFee, AppId, Balance, Metadata, Pool, TIRStep};
use zd_support::{ChallengeBase, MultiBaseToken, Reputation, SeedsBase, TrustBase};

pub use pallet::*;

#[macro_use]
pub mod mock;
mod tests;
pub mod types;
pub use self::types::*;
pub mod functions;

pub mod weights;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {

    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::{ensure_signed, pallet_prelude::*};

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type Reputation: Reputation<Self::AccountId, Self::BlockNumber, TIRStep>;
        type ChallengeBase: ChallengeBase<Self::AccountId, AppId, Balance, Self::BlockNumber>;
        type TrustBase: TrustBase<Self::AccountId>;
        type SeedsBase: SeedsBase<Self::AccountId>;
        type MultiBaseToken: MultiBaseToken<Self::AccountId, Balance>;

        /// Add the amount of staking for seed candidates.
        ///
        /// SeedStakingAmount = SeedChallengeAmount + SeedReservStaking
        #[pallet::constant]
        type SeedStakingAmount: Get<Balance>;

        /// Staking amount of seeding score for challenge seeding candidates.
        #[pallet::constant]
        type SeedChallengeAmount: Get<Balance>;

        /// This portion of the staking amount is not affected by the challenge and will be
        /// officially seeded when the user receives that portion.
        #[pallet::constant]
        type SeedReservStaking: Get<Balance>;

        ///  Maximum number of seeds.
        #[pallet::constant]
        type MaxSeedCount: Get<u32>;

        /// Confirmation period.
        #[pallet::constant]
        type ConfirmationPeriod: Get<Self::BlockNumber>;

        /// The weight information of this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    /// Data on the centrality scores of the challenged seed candidates.
    #[pallet::storage]
    #[pallet::getter(fn get_result_hashs)]
    pub type ResultHashsSets<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, Vec<OrderedSet<ResultHash>>, ValueQuery>;

    /// List of seeded candidates.
    #[pallet::storage]
    #[pallet::getter(fn get_candidate)]
    pub type Candidates<T: Config> = StorageMap<
        _,
        Twox64Concat,
        T::AccountId,
        Candidate<T::AccountId, T::BlockNumber>,
        ValueQuery,
    >;

    /// The set of valid centrality scores.
    #[pallet::storage]
    #[pallet::getter(fn get_score_list)]
    pub type ScoreList<T: Config> = StorageValue<_, Vec<u64>, ValueQuery>;

    /// Pathway for challenged seed candidates.
    #[pallet::storage]
    #[pallet::getter(fn get_path)]
    pub type Paths<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, Vec<Path<T::AccountId>>, ValueQuery>;

    /// The Missing Path
    #[pallet::storage]
    #[pallet::getter(fn get_missed_paths)]
    pub type MissedPaths<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, Vec<T::AccountId>, ValueQuery>;

    /// Have all seeds been confirmed
    #[pallet::storage]
    #[pallet::getter(fn seeds_confirmed)]
    pub type SeedsConfirmed<T: Config> = StorageValue<_, bool, ValueQuery>;

    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new round of seed refreshing was initiated. \[who\]
        RefershSeedStared(T::AccountId),
        /// New seeding candidates. \[pthfinder, candidate,score\]
        NewCandidate(T::AccountId, T::AccountId, u64),
        /// A new challenge. \[challenger, candidate\]
        NewChallenge(T::AccountId, T::AccountId),
        /// New examine. \[challenger, candidate\]
        NewExamine(T::AccountId, T::AccountId),
        /// The new hash was replied to. \[pthfinder, candidate, quantity, completed\]
        RepliedHash(T::AccountId, T::AccountId, u32, bool),
        /// Continued reply with a new hash \[pthfinder, candidate, completed\]
        ContinueRepliedHash(T::AccountId, T::AccountId, bool),
        /// The path was replied to. \[pthfinder, candidate, quantity, completed\]
        RepliedPath(T::AccountId, T::AccountId, u32, bool),
        /// The path continued to be replied to. \[pthfinder, candidate, completed\]
        ContinueRepliedPath(T::AccountId, T::AccountId, bool),
        /// Responded to the number of shortest paths between two users. \[pthfinder, candidate\]
        RepliedNum(T::AccountId, T::AccountId),
        /// Show the shortest path that was missed. \[challenger, candidate,index\]
        MissedPathPresented(T::AccountId, T::AccountId, u32),
        /// Shorter paths are shown. \[challenger, candidate,index\]
        ShorterPresented(T::AccountId, T::AccountId, u32),
        /// Evidence is presented that the total number of shortest paths is too small. \[challenger, candidate,index\]
        EvidenceOfNumTooLowPresented(T::AccountId, T::AccountId, u32),
        /// A certificate that the evidence is invalid was produced. \[challenger, candidate,score\]
        EvidenceOfInvalidPresented(T::AccountId, T::AccountId, u64),
        /// Received the proceeds of the challenge. \[who, candidate\]
        ChallengeHarvested(T::AccountId, T::AccountId),
        /// Received seed refresh proceeds. \[who, candidate\]
        SeedHarvested(T::AccountId, T::AccountId),
        /// A challenge was launched. \[candidate, score\]
        ChallengeRestarted(T::AccountId, u64),
        /// All seeds have been selected \[number \]
        SeedsSelected(u32),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Already exists
        AlreadyExist,
        /// No candidate exists
        NoCandidateExists,
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
        ScoreListEmpty,
        /// Step is not match
        StepNotMatch,
        /// Path does not exist
        PathDoesNotExist,
        /// The path is too short
        PathTooShort,
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
        /// ResultHash does not exist
        ResultHashNotExit,
        /// Unconfirmed data still available
        StillUnconfirmed,
        /// Time not yet reached or overflow
        SweeprtFail,
        /// Seed have been confirmed and are unchallengeable
        SeedAlreadyConfirmed,
        /// No target node found
        NoTargetNode,
        /// Maximum depth has been reached
        MaximumDepth,
        /// Missed Paths does not exist
        MissedPathsNotExist,
        /// Path uploaded, no hash challenge allowed
        PathUploaded,
        /// No path exists to challenge
        NoPathExists,
        /// Candidate does not exist or has been harvested
        CandidateNotExist,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Start seed refreshing.
        #[pallet::weight(T::WeightInfo::start())]
        #[transactional]
        pub fn start(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            T::Reputation::new_round()?;
            Self::deposit_event(Event::RefershSeedStared(who));
            Ok(().into())
        }

        /// Add `target` as a seed candidate with a centrality score of `score`.
        ///
        /// Funds from the caller's `SeedStakingAmount` will be deducted.
        #[pallet::weight(T::WeightInfo::add())]
        #[transactional]
        pub fn add(
            origin: OriginFor<T>,
            target: T::AccountId,
            score: u64,
        ) -> DispatchResultWithPostInfo {
            let pathfinder = ensure_signed(origin)?;
            Self::check_step()?;
            ensure!(
                !<Candidates<T>>::contains_key(target.clone()),
                Error::<T>::AlreadyExist
            );
            T::MultiBaseToken::staking(&pathfinder, &T::SeedStakingAmount::get())?;
            Self::candidate_insert(&target, &pathfinder, &score);
            T::Reputation::set_last_refresh_at();
            Self::deposit_event(Event::NewCandidate(pathfinder, target, score));
            Ok(().into())
        }

        /// Launch a challenge to the seed candidate `target` with a new score of `score`.
        ///
        /// Is a no-op if:
        ///
        /// - the candidate does not exist, or
        /// - The seeds have been challenged and are in `Free` status, but the confirmation period
        /// has expired.
        #[pallet::weight(T::WeightInfo::challenge())]
        #[transactional]
        pub fn challenge(
            origin: OriginFor<T>,
            target: T::AccountId,
            score: u64,
        ) -> DispatchResultWithPostInfo {
            let challenger = ensure_signed(origin)?;
            Self::check_step()?;
            let candidate = <Candidates<T>>::try_get(target.clone())
                .map_err(|_err| Error::<T>::NoCandidateExists)?;
            let staking = if candidate.has_challenge {
                Zero::zero()
            } else {
                T::SeedChallengeAmount::get()
            };
            if !candidate.has_challenge {
                ensure!(
                    candidate.add_at + T::ConfirmationPeriod::get() > Self::now(),
                    Error::<T>::SeedAlreadyConfirmed
                );
            }
            T::ChallengeBase::launch(
                &APP_ID,
                &target,
                &Metadata {
                    challenger: challenger.clone(),
                    pathfinder: candidate.pathfinder,
                    pool: Pool {
                        staking,
                        earnings: Zero::zero(),
                    },
                    score,
                    ..Metadata::default()
                },
            )?;
            <Candidates<T>>::mutate(&target, |c| c.has_challenge = true);
            T::Reputation::set_last_refresh_at();
            Self::deposit_event(Event::NewChallenge(challenger, target));
            Ok(().into())
        }

        /// Interrogate the data under `target`, `index` position of the seed candidate.
        ///
        /// - In the hash period, i.e. when the current depth is less than `DEEP`, the
        /// challenge data is the data in `ResultHashsSets`;
        /// - After the path has been uploaded, it points to `num` in the path data,
        /// which is the number of shortest paths between two points.
        #[pallet::weight(T::WeightInfo::examine())]
        #[transactional]
        pub fn examine(
            origin: OriginFor<T>,
            target: T::AccountId,
            index: u32,
        ) -> DispatchResultWithPostInfo {
            let challenger = ensure_signed(origin)?;
            Self::check_step()?;
            let result_hash_sets = Self::try_get_rhash(&target)?;
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
            T::ChallengeBase::examine(&APP_ID, &challenger, &target, index)?;
            T::Reputation::set_last_refresh_at();
            Self::deposit_event(Event::NewExamine(challenger, target));
            Ok(().into())
        }

        /// Reply to the hash data collection with a total of `quantity` entries.
        #[pallet::weight(T::WeightInfo::reply_hash(hashs.len().max(1) as u32))]
        #[transactional]
        pub fn reply_hash(
            origin: OriginFor<T>,
            target: T::AccountId,
            hashs: Vec<PostResultHash>,
            quantity: u32,
        ) -> DispatchResultWithPostInfo {
            let pathfinder = ensure_signed(origin)?;
            Self::check_step()?;
            let count = hashs.len();
            ensure!(quantity <= MAX_HASH_COUNT, Error::<T>::QuantityExceedsLimit);
            let _ = T::ChallengeBase::reply(
                &APP_ID,
                &pathfinder,
                &target,
                quantity,
                count as u32,
                |is_all_done, index, order| -> Result<u64, DispatchError> {
                    let new_order = Self::get_next_order(&target, &order, &(index as usize))?;
                    Self::update_result_hashs(&target, &hashs[..], is_all_done, index, false)?;
                    Self::deposit_event(Event::RepliedHash(
                        pathfinder.clone(),
                        target.clone(),
                        quantity,
                        is_all_done,
                    ));
                    Ok(new_order)
                },
            )?;
            T::Reputation::set_last_refresh_at();
            Ok(().into())
        }

        /// Continue to reply with hash data.
        ///
        /// The `pathfinder` should be uploaded within the allowed period, otherwise the system
        /// has the right to determine that it has failed.
        #[pallet::weight(T::WeightInfo::reply_hash_next(hashs.len().max(1) as u32))]
        #[transactional]
        pub fn reply_hash_next(
            origin: OriginFor<T>,
            target: T::AccountId,
            hashs: Vec<PostResultHash>,
        ) -> DispatchResultWithPostInfo {
            let challenger = ensure_signed(origin)?;
            Self::check_step()?;
            let count = hashs.len();
            T::ChallengeBase::next(
                &APP_ID,
                &challenger,
                &target,
                &(count as u32),
                |_, index, is_all_done| -> Result<(u64, u32), DispatchError> {
                    Self::update_result_hashs(&target, &hashs[..], is_all_done, index, true)?;
                    Self::deposit_event(Event::ContinueRepliedHash(
                        challenger.clone(),
                        target.clone(),
                        is_all_done,
                    ));
                    Ok((Zero::zero(), index))
                },
            )?;
            Ok(().into())
        }

        /// Respond to the path data for the challenge against `target` with a total of `quantity` entries.
        ///
        /// Is a no-op if:
        ///
        /// - the maximum depth is not reached (you should revert to hash at this point), or
        /// - Duplicate or incorrect paths.
        #[pallet::weight(T::WeightInfo::reply_path(paths.len().max(1) as u32))]
        #[transactional]
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
            let r_hashs_sets = Self::try_get_rhash(&target)?;
            let deep = r_hashs_sets.len();
            ensure!(deep == DEEP as usize, Error::<T>::DepthDoesNotMatch);
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
                    let index = index as usize;
                    let mut full_order = Self::get_full_order(&r_hashs_sets[..], &order, &index)?;
                    let new_order = full_order.try_to_u64().ok_or(Error::<T>::ConverError)?;
                    Self::checked_paths_vec(&paths[..], &target, &full_order.0[..], deep)?;
                    if is_all_done {
                        Self::verify_paths(
                            &paths[..],
                            &target,
                            &r_hashs_sets.last().unwrap().0[index].clone(),
                        )?;
                    }
                    <Paths<T>>::insert(&target, &paths);
                    Self::deposit_event(Event::RepliedPath(
                        pathfinder.clone(),
                        target.clone(),
                        quantity,
                        is_all_done,
                    ));
                    Ok(new_order)
                },
            )?;

            T::Reputation::set_last_refresh_at();
            Ok(().into())
        }

        /// Continue to reply to the path data.
        ///
        /// This is useful in case of too much data, or network congestion.
        #[pallet::weight(T::WeightInfo::reply_path_next(paths.len().max(1) as u32))]
        #[transactional]
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
                    let r_hashs_sets = Self::try_get_rhash(&target)?;
                    let deep = r_hashs_sets.len();
                    let r_hashs = r_hashs_sets.last().unwrap().0[index as usize].clone();
                    let mut full_paths = <Paths<T>>::get(&target);

                    full_paths.extend_from_slice(&paths);
                    let old_len = full_paths.len();
                    full_paths.sort();
                    full_paths.dedup();

                    ensure!(old_len == full_paths.len(), Error::<T>::NotMatch);

                    let full_order = FullOrder::from_u64(&order, deep + 1);

                    Self::checked_paths_vec(&paths[..], &target, &full_order.0[..], deep)?;

                    if is_all_done {
                        Self::verify_paths(&full_paths[..], &target, &r_hashs)?;
                    }
                    <Paths<T>>::mutate(&target, |p| *p = full_paths);
                    Self::deposit_event(Event::ContinueRepliedPath(
                        pathfinder.clone(),
                        target.clone(),
                        is_all_done,
                    ));
                    Ok((order, index))
                },
            )?;

            T::Reputation::set_last_refresh_at();
            Ok(().into())
        }

        /// Reply to the shortest path total.
        ///
        /// This occurs when the challenger believes that the total data in `pathfinder`'s
        /// path data is too large, requiring `pathfinder` to upload all shortest paths
        /// between the two endpoints to prove that the data is correct.
        ///
        /// `mid_paths` - Here the two endpoints are known, so only the middle `node` needs
        /// to be uploaded, or `[]` if there is no user in between.
        ///
        /// NOTE: When the total number of paths between two endpoints is greater than 100,
        /// the path is invalid. We are only accurate to two decimal places.
        #[pallet::weight(T::WeightInfo::reply_num(mid_paths.len().max(1) as u32))]
        #[transactional]
        pub fn reply_num(
            origin: OriginFor<T>,
            target: T::AccountId,
            mid_paths: Vec<Vec<T::AccountId>>,
        ) -> DispatchResultWithPostInfo {
            let challenger = ensure_signed(origin)?;
            Self::check_step()?;
            let old_len = mid_paths.len();
            let mut mid_paths = mid_paths;
            mid_paths.sort();
            mid_paths.dedup();

            ensure!(old_len == mid_paths.len(), Error::<T>::NotMatch);

            Self::do_reply_num(&challenger, &target, &mid_paths[..])?;
            T::Reputation::set_last_refresh_at();
            Self::deposit_event(Event::RepliedNum(challenger, target));
            Ok(().into())
        }

        /// Evidence of missing paths is produced at the hash stage.
        ///
        /// - `nodes`- The complete path vector.
        /// - `index` - The location where the missing path is located. For example, if
        /// the hash set is [5,8,10], if you have a path with a hash of `4`, then `index`
        /// will be `0`. If it is `11`, then `index` will be `2`.
        ///
        /// After a successful call we will enter the arbitration phase, as we cannot
        /// determine if `nodes` is the shortest path and will need to play the challenge
        /// game again to determine to determine this.
        #[pallet::weight(T::WeightInfo::missed_in_hashs())]
        #[transactional]
        pub fn missed_in_hashs(
            origin: OriginFor<T>,
            target: T::AccountId,
            nodes: Vec<T::AccountId>,
            index: u32,
        ) -> DispatchResultWithPostInfo {
            let challenger = ensure_signed(origin)?;

            ensure!(!<Paths<T>>::contains_key(&target), Error::<T>::PathUploaded);

            Self::evidence_of_missed(&challenger, &target, &nodes, index)?;
            Ok(().into())
        }

        /// Lost paths in the path set.
        ///
        /// Upload a valid `nodes`, if it is not in the path set, it is **possible** that `pathfinder`
        /// has lost the path. A successful call will lead to an arbitration phase, as we cannot
        /// determine if `nodes` is the shortest path and will need to determine this again through
        /// a challenge game.
        #[pallet::weight(T::WeightInfo::missed_in_paths())]
        #[transactional]
        pub fn missed_in_paths(
            origin: OriginFor<T>,
            target: T::AccountId,
            nodes: Vec<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            let challenger = ensure_signed(origin)?;

            ensure!(<Paths<T>>::contains_key(&target), Error::<T>::NoPathExists);

            Self::evidence_of_missed(&challenger, &target, &nodes, Zero::zero())?;
            Ok(().into())
        }

        /// Pass in a shorter valid path to prove that the path to the `index` position
        /// in the path set under `target` is invalid.
        ///
        /// `mid_paths` - Here the two endpoints are known, so only the middle `node` needs
        /// to be uploaded, or `[]` if there is no user in between.
        ///
        /// Since `shorter paths' are deterministic, a successful execution of `pathfinder`
        /// will result in failure and the current challenger will be challenged by other
        /// challengers as a new `pathfinder`.
        #[pallet::weight(T::WeightInfo::evidence_of_shorter())]
        #[transactional]
        pub fn evidence_of_shorter(
            origin: OriginFor<T>,
            target: T::AccountId,
            index: u32,
            mid_path: Vec<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            let challenger = ensure_signed(origin)?;
            Self::check_step()?;

            let p_path = Self::get_pathfinder_paths(&target, &index)?;
            let (start, stop) = Self::get_ends(&p_path);

            Self::check_mid_path(&mid_path[..], start, stop)?;

            let maybe_score = T::ChallengeBase::evidence(
                &APP_ID,
                &challenger,
                &target,
                |_, _| -> Result<bool, DispatchError> { Ok(false) },
            )?;
            Self::restart(&target, &challenger, &maybe_score.unwrap_or_default());
            Self::deposit_event(Event::ShorterPresented(challenger, target, index));
            Ok(().into())
        }

        /// Pass in all paths to prove that the total number of paths under `index` in path set
        /// of `target` is too small.
        ///
        /// `mid_paths` - Here the two endpoints are known, so only the middle `node` needs
        /// to be uploaded, or `[]` if there is no user in between.If the total number of paths
        /// exceeds `MAX_SHORTEST_PATH`, then only `MAX_SHORTEST_PATH` + 1 path needs to be
        /// uploaded to prove that the original path is invalid.
        ///
        /// NOTE: The length of the path must be the same as the original length, if you have a
        /// shorter path you should call `evidence_of_shorter`.
        #[pallet::weight(T::WeightInfo::number_too_low(mid_paths.len().max(2) as u32))]
        #[transactional]
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
                        ensure!(
                            mid_path.len() + 2 == p_path.nodes.len(),
                            Error::<T>::LengthNotEqual
                        );
                        let _ = Self::check_mid_path(&mid_path[..], start, stop)?;
                    }
                    ensure!(mid_paths.len() > p_path_total, Error::<T>::TooFewInNumber);
                    Ok(false)
                },
            )?;
            Self::restart(&target, &challenger, &maybe_score.unwrap_or_default());
            Self::deposit_event(Event::EvidenceOfNumTooLowPresented(
                challenger, target, index,
            ));
            Ok(().into())
        }

        /// Upload a shorter path `mid_path`, proving that the evidence presented by `missed_in_paths`
        /// or `missed_in_hashs` is wrong.
        ///
        /// If `mid_path` contains `target` , it means that `pathfinder` is wrong and the caller will
        /// be challenged as the new challenger using `score` as the new centrality score to be challenged.
        ///
        /// If `mid_path` does not contain `target`, then `pathfinder` cannot be proven wrong and the
        /// caller will act as a common `pathfinder` for beneficiary of `pathfinder` to share the pool
        /// equally.
        #[pallet::weight(T::WeightInfo::invalid_evidence())]
        #[transactional]
        pub fn invalid_evidence(
            origin: OriginFor<T>,
            target: T::AccountId,
            mid_path: Vec<T::AccountId>,
            score: u64,
        ) -> DispatchResultWithPostInfo {
            let challenger = ensure_signed(origin)?;
            Self::check_step()?;
            let missed_path =
                <MissedPaths<T>>::try_get(&target).map_err(|_| Error::<T>::MissedPathsNotExist)?;
            ensure!(
                mid_path.len() + 2 < missed_path.len(),
                Error::<T>::WrongPathLength
            );
            let (start, stop) = Self::get_nodes_ends(&missed_path[..]);
            Self::check_mid_path(&mid_path[..], start, stop)?;
            let through_target = mid_path.contains(&target);
            T::ChallengeBase::arbitral(
                &APP_ID,
                &challenger,
                &target,
                |_, _| -> Result<(bool, bool, u64), DispatchError> {
                    Ok((through_target, true, score))
                },
            )?;
            if through_target {
                Self::restart(&target, &challenger, &score);
            }
            Self::deposit_event(Event::EvidenceOfInvalidPresented(challenger, target, score));
            Ok(().into())
        }

        /// Receive the proceeds of the challenge.
        #[pallet::weight(T::WeightInfo::harvest_challenge())]
        #[transactional]
        pub fn harvest_challenge(
            origin: OriginFor<T>,
            target: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            Self::check_step()?;

            Self::do_harvest_challenge(&who, &target)?;
            Self::deposit_event(Event::ChallengeHarvested(who, target));
            Ok(().into())
        }

        /// Receive seed proceeds.
        ///
        /// Is a no-op if:
        /// - The challenge has not yet been fully collected, or
        /// - The seeds have not all passed the confirmation period.
        ///
        /// The first collection in each round will determine all the seeds first,
        /// e.g. if there are 100 seed candidates but the maximum number of seeds
        /// is 90, the 90 with the highest score will be taken. If the 91st and
        /// 90th scores are the same, the first to be claimed will be confirmed.
        #[pallet::weight(T::WeightInfo::harvest_seed())]
        #[transactional]
        pub fn harvest_seed(
            origin: OriginFor<T>,
            target: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            Self::check_step()?;
            let mut score_list = Self::get_score_list();
            let is_all_confirmed = Self::seeds_confirmed();
            if !is_all_confirmed {
                ensure!(
                    T::ChallengeBase::is_all_harvest(&APP_ID),
                    Error::<T>::StillUnharvestedChallenges
                );
                ensure!(Self::is_all_timeout(), Error::<T>::StillUnconfirmed);
                Self::hand_first_time(&mut score_list);
            }
            let len = score_list.len();

            ensure!(
                <Candidates<T>>::contains_key(&target),
                Error::<T>::CandidateNotExist
            );

            let candidate = <Candidates<T>>::take(&target);
            let staking_amount = if candidate.has_challenge {
                T::SeedReservStaking::get()
            } else {
                T::SeedStakingAmount::get()
            };
            let (bonus, maybe_index) =
                match !score_list.is_empty() && candidate.score >= score_list[0] {
                    true => {
                        if let Ok(index) = score_list.binary_search(&candidate.score) {
                            (
                                T::MultiBaseToken::get_bonus_amount() / (len as Balance),
                                Some(index),
                            )
                        } else {
                            (Zero::zero(), None)
                        }
                    }
                    false => (Zero::zero(), None),
                };
            let total_amount = bonus
                .checked_add(staking_amount)
                .ok_or(Error::<T>::Overflow)?;
            match who != candidate.pathfinder {
                true => {
                    let last = T::Reputation::get_last_refresh_at();
                    let (s_amount, p_amount) = total_amount
                        .checked_with_fee(last, Self::now())
                        .ok_or(Error::<T>::SweeprtFail)?;
                    T::MultiBaseToken::release(&who, &s_amount)?;
                    T::MultiBaseToken::release(&candidate.pathfinder, &p_amount)?;
                }
                false => {
                    T::MultiBaseToken::release(&candidate.pathfinder, &total_amount)?;
                }
            }
            T::MultiBaseToken::cut_bonus(&bonus)?;
            if let Some(index) = maybe_index {
                T::SeedsBase::add_seed(&target);
                score_list.remove(index);
            }
            if Self::is_all_harvest() {
                <SeedsConfirmed<T>>::put(false);
                T::Reputation::set_step(&TIRStep::Reputation);
            } else if !is_all_confirmed {
                <SeedsConfirmed<T>>::put(true);
            }
            Self::deposit_event(Event::SeedHarvested(who, target));
            <ScoreList<T>>::put(score_list);
            Ok(().into())
        }
    }
}
