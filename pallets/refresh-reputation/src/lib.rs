//! # ZdRefreshReputation Module
//!
//! ## Overview
//!
//! This module is used to refresh the reputation and ensure the correct reputation value by challenging 
//! the game. The process for refreshing reputation is as follows.
//!
//! 1 `start` - Bringing the system into a refreshable state.
//! 2 `refresh` - `pathfinder` staking the corresponding amount and refreshes the user's reputation value.
//! 3 `challenge` - `challenger` staking the corresponding amount to challenge the incorrect reputation value.
//! 4 `challenge_update` - `challenger` uploads the correct path, the system does not validate the value and 
//! enters arbitration when the path upload is complete.
//! 5 `arbitral` - Anyone can staking a certain amount of money to upload a shorter path or a different score 
//! to prove that the original data is wrong
//!
//! ## Implementations
//!
//! ### Dispatchable Functions
//!
//! - `start` - Turn on reputation refreshing.
//! - `refresh` - Accepts an array of users and a tuple of reputation values, and refreshes the reputation values 
//! of all users within the array.
//! - `harvest_ref_all` - Callers receive all their refresh proceeds.
//! - `harvest_ref_all_sweeper` - `sweeper` collects `pathfinder` timeout for unclaimed refresh proceeds.
//! - `harvest_challenge` - Callers receive the proceeds of the challenge.
//! - `challenge` - Challenge the reputation value of the incoming user.
//! - `arbitral` - Upload a shorter path to arbitrate on an already existing path.
//! - `challenge_update` - Upload the challenge path.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::{
    codec::{Decode, Encode},
    ensure, pallet,
    traits::Get,
    transactional, RuntimeDebug,
};
use frame_system::{self as system, ensure_signed};
use sp_runtime::{traits::Zero, DispatchError, DispatchResult};
use sp_std::vec::Vec;
use zd_primitives::{
    fee::SweeperFee, AppId, Balance, ChallengeStatus, Metadata, Pool, Progress, TIRStep,
};
use zd_support::{ChallengeBase, MultiBaseToken, Reputation, SeedsBase, TrustBase};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod weights;
pub use weights::WeightInfo;

pub use pallet::*;

const APP_ID: AppId = *b"repu    ";

/// Maximum number of active paths
const MAX_NODE_COUNT: usize = 5;
/// Maximum number of refreshes for the same address
const MAX_REFRESH: u32 = 500;

/// Update records of the target user's reputation value.
#[derive(Encode, Decode, Clone, Default, RuntimeDebug)]
pub struct Record<BlockNumber, Balance> {
    /// The time at which the refresh occurs.
    pub update_at: BlockNumber,

    /// The handling fee obtained for this refresh.
    pub fee: Balance,
}

/// The earnings record for `pathfinder`.
#[derive(Encode, Decode, Clone, Default, PartialEq, RuntimeDebug)]
pub struct Payroll<Balance, BlockNumber> {
    /// Number of refreshes.
    pub count: u32,

    /// The total amount of fees received.
    pub total_fee: Balance,

    /// Last refreshed time.
    pub update_at: BlockNumber,
}

/// Returns the total amount due to `pathfinder`, including the amount of the staking and the earnings.
impl<BlockNumber> Payroll<Balance, BlockNumber> {
    fn total_amount<T: Config>(&self) -> Balance {
        T::UpdateStakingAmount::get()
            .saturating_mul(self.count.into())
            .saturating_add(self.total_fee)
    }
}

/// The path to a trusting relationship from seed to user.
#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug)]
pub struct Path<AccountId> {
    /// The path between the seed and the target user, excluding the seed and the target user.
    pub nodes: Vec<AccountId>,

    /// The reputation that users gain from this seed is worth points.
    pub score: u32,
}

impl<AccountId> Path<AccountId> {
    // Returns whether or not the longest path is exceeded, as it does not include the 
    // seed and target user, so 2 needs to be added
    fn check_nodes_leng(&self) -> bool {
        self.nodes.len() + 2 <= MAX_NODE_COUNT
    }

    fn exclude_zero(&self) -> bool {
        self.check_nodes_leng() && self.score != 0
    }
}

#[pallet]
pub mod pallet {
    use super::*;

    use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
    use frame_system::pallet_prelude::*;
    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type MultiBaseToken: MultiBaseToken<Self::AccountId, Balance>;
        type Reputation: Reputation<Self::AccountId, Self::BlockNumber, TIRStep>;
        type TrustBase: TrustBase<Self::AccountId>;
        type SeedsBase: SeedsBase<Self::AccountId>;
        type ChallengeBase: ChallengeBase<Self::AccountId, AppId, Balance, Self::BlockNumber>;

        /// Maximum number of uploads.
        #[pallet::constant]
        type MaxUpdateCount: Get<u32>;

        /// The amount to be staking.
        #[pallet::constant]
        type UpdateStakingAmount: Get<Balance>;

        /// After this number of blocks has been exceeded, the data will be validated.
        #[pallet::constant]
        type ConfirmationPeriod: Get<Self::BlockNumber>;

        /// After this period, no new refreshes can be added, this is to prevent malicious delays 
        /// leading to long update periods.
        #[pallet::constant]
        type RefRepuTiomeOut: Get<Self::BlockNumber>;

        /// The weight information of this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    /// Start time for this round of refreshments.
    #[pallet::storage]
    #[pallet::getter(fn started_at)]
    pub type StartedAt<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;

    /// The bill payable for `AccountId`.
    #[pallet::storage]
    #[pallet::getter(fn get_payroll)]
    pub type Payrolls<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, Payroll<Balance, T::BlockNumber>, ValueQuery>;

    /// `pathfinder` updated records for the `target` user.
    #[pallet::storage]
    #[pallet::getter(fn update_record)]
    pub type Records<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        T::AccountId,
        Twox64Concat,
        T::AccountId,
        Record<T::BlockNumber, Balance>,
        ValueQuery,
    >;

    /// The path of the trust relationship from `seed` to `target`.
    #[pallet::storage]
    #[pallet::getter(fn get_path)]
    pub type Paths<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        T::AccountId,
        Twox64Concat,
        T::AccountId,
        Path<T::AccountId>,
        ValueQuery,
    >;

    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Some reputations have been updated. \[pathfinder, count, fee\]
        ReputationRefreshed(T::AccountId, u32, Balance),
        /// Reputation renewal has begun \[who\]
        Started(T::AccountId),
        /// Refreshed earnings are harvested \[pathfinder, amount\]
        RefreshedHarvested(T::AccountId, Balance),
        /// Refreshed earnings are harvested \[pathfinder, sweeper, pathfinder_amount, sweeper_amount\]
        RefreshedHarvestedBySweeper(T::AccountId, T::AccountId, Balance, Balance),
        /// Refreshed earnings are harvested \[pathfinder, target\]
        ChallengeHarvested(T::AccountId, T::AccountId),
        /// A new challenge has been launched \[challenger, target\]
        Challenge(T::AccountId, T::AccountId),
        /// A new arbitral has been launched \[challenger, target\]
        Arbitral(T::AccountId, T::AccountId),
        /// The new path is uploaded \[challenger, target\]
        PathUpdated(T::AccountId, T::AccountId),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Quantity reaches limit.
        QuantityLimitReached,
        /// Error getting fee.
        ErrorFee,
        /// Challenge timeout.
        ChallengeTimeout,
        /// Calculation overflow.
        Overflow,
        /// Calculation overflow.
        FailedSweeper,
        /// The presence of unharvested challenges.
        ChallengeNotClaimed,
        /// Excessive number of seeds
        ExcessiveBumberOfSeeds,
        /// Error getting user reputation
        ReputationError,
        /// The path already exists
        PathAlreadyExist,
        /// Wrong path
        WrongPath,
        /// Error calculating dist
        DistErr,
        /// The dist is too long or score is too low.
        DistTooLong,
        /// Paths and seeds do not match
        NotMatch,
        /// Status mismatch
        StatusErr,
        /// Not yet started
        NotYetStarted,
        /// Already started
        AlreadyStarted,
        /// The challenged reputation is the same as the original reputation
        SameReputation,
        /// Exceeds the allowed refresh time
        RefreshTiomeOut,
        /// Same path length, but score too low
        ScoreTooLow,
        /// Exceed the refresh limit
        ExceedMaxRefresh,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Start a new round.
        /// 
        /// Anyone can call on it without collateral. The user does so for two purposes:
        /// 
        /// - The existence of `Payrolls` that have expired for collection, for which the caller 
        /// will receive a percentage of the amount.
        /// - `pathfinder` gets a first-mover advantage, preempting users with higher renewal fees.
        /// 
        /// Is a no-op if:
        /// 
        /// 1 Challenges that remain uncollected, or
        /// 2 Already started, or
        /// 3 Minimum interval not exceeded.
        #[pallet::weight(T::WeightInfo::start())]
        #[transactional]
        pub fn start(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            Self::check_step_and_not_stared()?;

            ensure!(
                T::ChallengeBase::is_all_harvest(&APP_ID),
                Error::<T>::ChallengeNotClaimed
            );

            let total_fee = Payrolls::<T>::drain()
                .try_fold::<_, _, Result<Balance, DispatchError>>(
                    0u128,
                    |acc: Balance, (pathfinder, payroll)| {
                        let (sweeper_fee, without_fee) = payroll.total_amount::<T>().with_fee();

                        T::MultiBaseToken::release(&pathfinder, &without_fee)?;

                        acc.checked_add(sweeper_fee)
                            .ok_or_else(|| Error::<T>::Overflow.into())
                    },
                )?;
            T::MultiBaseToken::release(&who, &total_fee)?;
            <StartedAt<T>>::put(Self::now());
            Self::deposit_event(Event::Started(who));
            Ok(().into())
        }

        /// Refresh the reputation value of a group of users.
        #[pallet::weight(T::WeightInfo::refresh((user_scores.len() as u32).max(1u32)))]
        #[transactional]
        pub fn refresh(
            origin: OriginFor<T>,
            user_scores: Vec<(T::AccountId, u32)>,
        ) -> DispatchResultWithPostInfo {
            let pathfinder = ensure_signed(origin)?;
            let user_count = user_scores.len();
            ensure!(
                user_count as u32 <= T::MaxUpdateCount::get(),
                Error::<T>::QuantityLimitReached
            );
            Self::check_step_and_stared()?;
            let now_block_number = Self::now();
            Self::check_timeout(&now_block_number)?;

            let old_count = Self::get_payroll(&pathfinder).count;
            ensure!(
                old_count.saturating_add(user_count as u32) < MAX_REFRESH,
                Error::<T>::ExceedMaxRefresh
            );

            let amount = T::UpdateStakingAmount::get()
                .checked_mul(user_count as Balance)
                .ok_or(Error::<T>::Overflow)?;
            T::MultiBaseToken::staking(&pathfinder, &amount)?;
            let total_fee = user_scores
                .iter()
                .try_fold::<_, _, Result<Balance, DispatchError>>(
                    Zero::zero(),
                    |acc_amount, user_score| {
                        let fee = Self::do_refresh(&pathfinder, user_score, &now_block_number)?;
                        acc_amount
                            .checked_add(fee)
                            .ok_or_else(|| Error::<T>::Overflow.into())
                    },
                )?;
            Self::mutate_payroll(
                &pathfinder,
                &total_fee,
                &(user_count as u32),
                &now_block_number,
            )?;

            T::Reputation::set_last_refresh_at();

            Self::deposit_event(Event::ReputationRefreshed(
                pathfinder,
                user_count as u32,
                total_fee,
            ));
            Ok(().into())
        }

        /// The caller receives all their earnings and clears all update records.
        /// 
        /// An `Err` will be returned if the user's last update has not passed the confirmation period.
        /// 
        /// NOTE: This is more economical and efficient than collecting each item in turn.
        #[pallet::weight(T::WeightInfo::harvest_ref_all())]
        #[transactional]
        pub fn harvest_ref_all(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            let pathfinder = ensure_signed(origin)?;
            Self::next_step();
            let now_block_number = Self::now();
            let payroll = Payrolls::<T>::take(&pathfinder);
            Self::can_harvest(&payroll, &now_block_number)?;
            let total_amount = payroll.total_amount::<T>();
            T::MultiBaseToken::release(&pathfinder, &total_amount)?;
            <Records<T>>::remove_prefix(&pathfinder);
            Self::deposit_event(Event::RefreshedHarvested(pathfinder, total_amount));
            Ok(().into())
        }

        /// `sweeper` collects `pathfinder` overdue proceeds.
        /// 
        /// `sweeper` receives a percentage of the proceeds from it.
        /// 
        /// NOTE: It is the responsibility of `pathfinder` to secure the cleanliness of the data on the chain 
        /// by collecting the proceeds and clearing the data in a timely manner. The `sweeper` policy ensures 
        /// that the system runs smoothly.
        #[pallet::weight(T::WeightInfo::harvest_ref_all_sweeper())]
        #[transactional]
        pub fn harvest_ref_all_sweeper(
            origin: OriginFor<T>,
            pathfinder: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            let sweeper = ensure_signed(origin)?;
            Self::next_step();
            let payroll = Payrolls::<T>::take(&pathfinder);
            let now_block_number = Self::now();
            Self::can_harvest(&payroll, &now_block_number)?;
            let (sweeper_fee, without_fee) = payroll
                .total_amount::<T>()
                .checked_with_fee(payroll.update_at, Self::now())
                .ok_or(Error::<T>::FailedSweeper)?;
            <Records<T>>::remove_prefix(&pathfinder);
            T::MultiBaseToken::release(&sweeper, &sweeper_fee)?;
            T::MultiBaseToken::release(&pathfinder, &without_fee)?;
            Self::deposit_event(Event::RefreshedHarvestedBySweeper(
                pathfinder,
                sweeper,
                without_fee,
                sweeper_fee,
            ));
            Ok(().into())
        }

        /// Receive the benefits of a challenge against `target`.
        /// 
        /// The caller must be the winner of the challenge.
        #[pallet::weight(T::WeightInfo::harvest_challenge())]
        #[transactional]
        pub fn harvest_challenge(
            origin: OriginFor<T>,
            target: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            Self::next_step();
            T::ChallengeBase::harvest(&who, &APP_ID, &target)?;
            Self::deposit_event(Event::ChallengeHarvested(who, target));
            Ok(().into())
        }

        /// Launching the challenge.
        /// 
        /// A challenge to the `target` reputation updated by `pathfinder` requires a total 
        /// of `quantity` of paths to be uploaded, the correct reputation score is `score`.
        /// 
        /// Is a no-op if:
        /// 
        /// - `score` is the same as the original reputation value, or
        /// - `quantity` is greater than the number of seeds, or
        /// - Reputation value has not been updated, or
        /// - Reputation value has been challenged, or
        /// - Reputation value has exceeded the confirmation period.
        /// 
        /// NOTE: If you need to challenge the reputation of an existing challenge, you should 
        /// call `arbitral`.
        #[pallet::weight(T::WeightInfo::challenge())]
        #[transactional]
        pub fn challenge(
            origin: OriginFor<T>,
            target: T::AccountId,
            pathfinder: T::AccountId,
            quantity: u32,
            score: u32,
        ) -> DispatchResultWithPostInfo {
            let challenger = ensure_signed(origin)?;
            Self::check_step()?;
            ensure!(
                quantity <= T::SeedsBase::get_seed_count(),
                Error::<T>::ExcessiveBumberOfSeeds
            );
            let reputation =
                T::Reputation::get_reputation_new(&target).ok_or(Error::<T>::ReputationError)?;
            ensure!(score != reputation, Error::<T>::SameReputation);
            let record = <Records<T>>::take(&pathfinder, &target);
            ensure!(
                record.update_at + T::ConfirmationPeriod::get() > Self::now(),
                Error::<T>::ChallengeTimeout
            );
            Payrolls::<T>::mutate(&pathfinder, |f| {
                f.total_fee = f.total_fee.saturating_sub(record.fee);
                f.count = f.count.saturating_sub(1);
            });

            T::ChallengeBase::launch(
                &APP_ID,
                &target,
                &Metadata {
                    pool: Pool {
                        staking: Zero::zero(),
                        earnings: record.fee,
                    },
                    remark: reputation,
                    pathfinder,
                    challenger: challenger.clone(),
                    progress: Progress {
                        total: quantity,
                        done: Zero::zero(),
                    },
                    ..Metadata::default()
                },
            )?;

            T::ChallengeBase::set_status(&APP_ID, &target, &ChallengeStatus::Arbitral);
            Self::deposit_event(Event::Challenge(challenger, target));
            Ok(().into())
        }

        /// Arbitration of paths in challenge.
        /// 
        /// accepts the correct path `paths` for `seeds` under `target` , `seeds` and `paths` are sets 
        /// that must be kept one to one correspondence.
        /// 
        /// NOTE: 
        /// 
        /// - The caller must correct all errors, otherwise the other challenger can launch `arbitral` 
        /// again, thus causing this challenge to fail.
        /// - During the protection period, the same caller may initiate `arbitral` several times 
        /// without paying a single staking.
        #[pallet::weight(T::WeightInfo::arbitral(seeds.len().max(paths.len()) as u32))]
        #[transactional]
        pub fn arbitral(
            origin: OriginFor<T>,
            target: T::AccountId,
            seeds: Vec<T::AccountId>,
            paths: Vec<Path<T::AccountId>>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            Self::check_step()?;
            let count = seeds.len();
            ensure!(count == paths.len(), Error::<T>::NotMatch);
            T::ChallengeBase::arbitral(
                &APP_ID,
                &who,
                &target,
                |score, remark| -> Result<(bool, bool, u64), _> {
                    let score = score as u32;
                    let new_score =
                        Self::do_update_path_verify(&target, &seeds[..], &paths[..], score)?;
                    T::Reputation::mutate_reputation(&target, &new_score);
                    Ok((new_score == remark, false, new_score.into()))
                },
            )?;
            Self::deposit_event(Event::Arbitral(who, target));
            Ok(().into())
        }

        /// Challenger upload path.
        /// 
        /// Accepts the correct path `paths` for `seeds` under `target` , `seeds` and `paths` 
        /// are sets and must correspond one-to-one.
        /// 
        /// During the upload protection period, the challenger can call this interface several 
        /// times in order to finish uploading all the paths. This "intermittent upload" is useful 
        /// when the number of seeds is too large, or when the network is congested.
        #[pallet::weight(T::WeightInfo::challenge_update(seeds.len().max(paths.len()) as u32))]
        #[transactional]
        pub fn challenge_update(
            origin: OriginFor<T>,
            target: T::AccountId,
            seeds: Vec<T::AccountId>,
            paths: Vec<Path<T::AccountId>>,
        ) -> DispatchResultWithPostInfo {
            let challenger = ensure_signed(origin)?;
            Self::check_step()?;
            let count = seeds.len();
            ensure!(count == paths.len(), Error::<T>::NotMatch);

            T::ChallengeBase::next(
                &APP_ID,
                &challenger,
                &target,
                &(count as u32),
                |score, remark, is_all_done| -> Result<(u64, u32), DispatchError> {
                    let new_score =
                        Self::do_update_path(&target, &seeds[..], &paths[..], score as u32)?;
                    if is_all_done {
                        T::Reputation::mutate_reputation(&target, &new_score);
                    }
                    Ok((new_score as u64, remark))
                },
            )?;
            Self::deposit_event(Event::PathUpdated(challenger, target));
            Ok(().into())
        }
    }
}

impl<T: Config> Pallet<T> {
    // pub

    /// Add `pathfinder` `amount` of accounts payable, and `count` updates to the original, 
    /// and set last active time to `now`.
    pub fn mutate_payroll(
        pathfinder: &T::AccountId,
        amount: &Balance,
        count: &u32,
        now: &T::BlockNumber,
    ) -> DispatchResult {
        <Payrolls<T>>::try_mutate(&pathfinder, |f| -> DispatchResult {
            let total_fee = f
                .total_fee
                .checked_add(*amount)
                .ok_or(Error::<T>::Overflow)?;

            let count = f.count.checked_add(*count).ok_or(Error::<T>::Overflow)?;
            *f = Payroll {
                count,
                total_fee,
                update_at: *now,
            };
            Ok(())
        })
    }

    /// Add or modify a challenge record for `who` under `pathfinder`, where the processing fee 
    /// obtained is `fee`, and set the update time to `now`.
    pub fn mutate_record(
        pathfinder: &T::AccountId,
        who: &T::AccountId,
        fee: &Balance,
        now: &T::BlockNumber,
    ) {
        <Records<T>>::mutate(&pathfinder, &who, |r| {
            *r = Record {
                update_at: *now,
                fee: *fee,
            }
        });
    }

    // pub(crate)

    pub(crate) fn check_step() -> DispatchResult {
        ensure!(
            T::Reputation::is_step(&TIRStep::Reputation),
            Error::<T>::StatusErr
        );
        Ok(())
    }

    pub(crate) fn next_step() {
        if <StartedAt<T>>::exists() {
            let now = Self::now();
            let is_last_ref_timeout =
                T::Reputation::get_last_refresh_at() + T::ConfirmationPeriod::get() < now;
            let is_cha_all_timeout = T::ChallengeBase::is_all_timeout(&APP_ID, &now);
            if is_last_ref_timeout && is_cha_all_timeout {
                T::TrustBase::remove_all_tmp();
                T::Reputation::set_free();
                <StartedAt<T>>::kill();
            }
        }
    }

    pub(crate) fn do_refresh(
        pathfinder: &T::AccountId,
        user_score: &(T::AccountId, u32),
        update_at: &T::BlockNumber,
    ) -> Result<Balance, DispatchError> {
        T::Reputation::refresh_reputation(user_score)?;
        let who = &user_score.0;
        let fee = Self::share(who);
        Self::mutate_record(&pathfinder, &who, &fee, update_at);
        Ok(fee)
    }

    pub(crate) fn share(user: &T::AccountId) -> Balance {
        let targets = T::TrustBase::get_trust_old(user);
        T::MultiBaseToken::share(user, &targets[..])
    }

    pub(crate) fn get_dist(
        paths: &Path<T::AccountId>,
        seed: &T::AccountId,
        target: &T::AccountId,
    ) -> Option<u32> {
        if paths.check_nodes_leng() {
            let mut nodes = paths.nodes.clone();
            nodes.insert(0, seed.clone());
            nodes.push(target.clone());
            if let Ok((dist, score)) = T::TrustBase::computed_path(&nodes[..]) {
                if score == paths.score {
                    return Some(dist);
                }
            }
        }
        None
    }

    pub(crate) fn do_update_path(
        target: &T::AccountId,
        seeds: &[T::AccountId],
        paths: &[Path<T::AccountId>],
        score: u32,
    ) -> Result<u32, DispatchError> {
        let new_score = seeds
            .iter()
            .zip(paths.iter())
            .try_fold(score, |acc, (seed, path)| {
                ensure!(
                    !Paths::<T>::contains_key(seed, target),
                    Error::<T>::PathAlreadyExist
                );
                ensure!(path.exclude_zero(), Error::<T>::WrongPath);
                acc.checked_add(path.score).ok_or(Error::<T>::Overflow)
            })?;
        for (seed, path) in seeds.iter().zip(paths.iter()) {
            Paths::<T>::insert(seed, target, path);
        }
        Ok(new_score)
    }

    pub(crate) fn do_update_path_verify(
        target: &T::AccountId,
        seeds: &[T::AccountId],
        paths: &[Path<T::AccountId>],
        score: u32,
    ) -> Result<u32, DispatchError> {
        let new_score = seeds.iter().zip(paths.iter()).try_fold(
            score,
            |acc, (seed, path)| -> Result<u32, DispatchError> {
                let dist_new = Self::get_dist(path, seed, target).ok_or(Error::<T>::DistErr)?;
                let old_path = Self::get_path(&seed, &target);
                if let Some(old_dist) = Self::get_dist(&old_path, seed, target) {
                    ensure!(old_dist >= dist_new, Error::<T>::DistTooLong);
                    if old_dist == dist_new {
                        ensure!(old_path.score > path.score, Error::<T>::ScoreTooLow);
                    }
                }
                let acc = acc
                    .checked_sub(old_path.score)
                    .and_then(|s| s.checked_add(path.score))
                    .ok_or(Error::<T>::Overflow)?;

                Ok(acc)
            },
        )?;
        for (seed, path) in seeds.iter().zip(paths.iter()) {
            Paths::<T>::mutate_exists(&seed, &target, |p| {
                *p = if path.score == 0 {
                    None
                } else {
                    Some(path.clone())
                };
            })
        }
        Ok(new_score)
    }

    // private

    fn check_step_and_stared() -> DispatchResult {
        Self::check_step()?;
        ensure!(<StartedAt<T>>::exists(), Error::<T>::NotYetStarted);
        Ok(())
    }

    fn now() -> T::BlockNumber {
        system::Module::<T>::block_number()
    }

    fn check_step_and_not_stared() -> DispatchResult {
        Self::check_step()?;
        ensure!(!<StartedAt<T>>::exists(), Error::<T>::AlreadyStarted);
        Ok(())
    }

    fn can_harvest(
        payroll: &Payroll<Balance, T::BlockNumber>,
        now: &T::BlockNumber,
    ) -> DispatchResult {
        ensure!(
            payroll.update_at + T::ConfirmationPeriod::get() < *now,
            Error::<T>::ExcessiveBumberOfSeeds
        );
        Ok(())
    }

    fn check_timeout(now: &T::BlockNumber) -> DispatchResult {
        ensure!(
            *now < <StartedAt<T>>::get() + T::RefRepuTiomeOut::get(),
            Error::<T>::RefreshTiomeOut
        );
        Ok(())
    }
}
