//! # ZdRefreshReputation Module
//!
//! ## 介绍
//!
//! 该模块用于刷新声誉并通过挑战游戏确保正确的声誉值。刷新声誉的流程如下：
//!
//! 1 `start` - 使系统进入可刷新状态；
//! 2 `refresh` - `pathfinder` 抵押相应金额，刷新用户声誉值；
//! 3 `challenge` - `challenger` 抵押相应金额，对错误的声誉值发起挑战；
//! 4 `challenge_update` - `challenger` 上传正确的路径，系统不进行数值验证，路径上传完成后进入仲裁状态；
//! 5 `arbitral` - 任何人都可以抵押一定的金额，上传更短的路径或不一样的得分来证明原数据是错误的；
//!
//! ## 接口
//!
//! ### 可调用函数
//!
//! - `start` - 开启声誉刷新。
//! - `refresh` - 接受一个用户和声誉值元组的数组，并刷新数组内所有用户的声誉值。
//! - `harvest_ref_all` - 调用者领取其所有刷新收益。
//! - `harvest_ref_all_sweeper` - `sweeper` 领取 `pathfinder` 超时未领取的刷新收益。
//! - `harvest_challenge` - 调用者领取自己的挑战收益。
//! - `challenge` - 向传入用户的声誉值发起挑战。
//! - `arbitral` - 上传更短的路径对已存在的路径进行仲裁。
//! - `challenge_update` - 上传挑战路径。

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

/// 目标用户的声誉值更新记录。
#[derive(Encode, Decode, Clone, Default, RuntimeDebug)]
pub struct Record<BlockNumber, Balance> {
    /// 刷新发送的时间。
    pub update_at: BlockNumber,

    /// 本次刷新获得的手续费。
    pub fee: Balance,
}

/// `pathfinder` 的收益记录。
#[derive(Encode, Decode, Clone, Default, PartialEq, RuntimeDebug)]
pub struct Payroll<Balance, BlockNumber> {
    /// 共刷新了多少个用户的声誉值。
    pub count: u32,

    /// 获得的手续费总额。
    pub total_fee: Balance,

    /// 最后刷新时间。
    pub update_at: BlockNumber,
}

/// 返回应支付给 `pathfinder` 的全部金额，包括抵押金额和收益。
impl<BlockNumber> Payroll<Balance, BlockNumber> {
    fn total_amount<T: Config>(&self) -> Balance {
        T::UpdateStakingAmount::get()
            .saturating_mul(self.count.into())
            .saturating_add(self.total_fee)
    }
}

/// 从种子到用户的信任传递路径。
#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug)]
pub struct Path<AccountId> {
    /// 从种子到目标用户之间的路径，不包括种子和目标用户。
    pub nodes: Vec<AccountId>,

    /// 用户从该种子获得的声誉值得分。
    pub score: u32,
}

impl<AccountId> Path<AccountId> {
    // 返回是否超过了最长路径，因为不包括种子和目标用户，所以需要加上2
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

        /// 最多上传数量。
        #[pallet::constant]
        type MaxUpdateCount: Get<u32>;

        /// 需要抵押的金额。
        #[pallet::constant]
        type UpdateStakingAmount: Get<Balance>;

        /// 超过此区块数的时间后，数据将被确认。
        #[pallet::constant]
        type ConfirmationPeriod: Get<Self::BlockNumber>;

        /// 超过此期间后，将不可新增刷新，这是为了防止恶意的拖延导致更新期过长。
        #[pallet::constant]
        type RefRepuTiomeOut: Get<Self::BlockNumber>;

        /// The weight information of this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    /// 本轮刷新的开始时间。
    #[pallet::storage]
    #[pallet::getter(fn started_at)]
    pub type StartedAt<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;

    /// `AccountId` 的应付账单。
    #[pallet::storage]
    #[pallet::getter(fn get_payroll)]
    pub type Payrolls<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, Payroll<Balance, T::BlockNumber>, ValueQuery>;

    /// `pathfinder` 更新的 `target` 用户的记录。
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

    /// `seed` 到 `target` 的信任关系路径。
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
        /// 开始新的一轮。
        /// 
        /// 任何人都可以调用，无需抵押。用户处于两种目的：
        /// 
        /// - 存在过期领取的 `Payrolls` ，调用者将会获得一定比例的金额；
        /// - `pathfinder` 获得先发优势，抢先更新手续费较高的用户。
        /// 
        /// 以下情况无法开始：
        /// 
        /// 1 尚存在未被领取的挑战，或者
        /// 2 已经开始，或
        /// 3 未超过最小间隔时间。
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

        /// 刷新一组用户的声誉值。
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

        /// 调用者领取其所有收益，并清空所有更新记录。
        /// 
        /// 用户的最后一条更新未过确认期将返回错误。
        /// 
        /// NOTE: 相比于每条领取依次，这样更节省高效。
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

        /// `sweeper` 领取 `pathfinder` 过期未领的收益。
        /// 
        /// `sweeper` 从中获得一定比例的收益。
        /// 
        /// NOTE: `pathfinder` 有责任及时领取收益并清除数据，以保障链上数据的清洁。`sweeper` 策略保障
        /// 系统顺畅运行。
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

        /// 领取针对 `target` 的挑战收益。
        /// 
        /// 调用者必须为该挑战的获胜者。
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

        /// 发起挑战。
        /// 
        /// 对 `pathfinder` 更新的 `target` 声誉发起挑战，需要上传路径一共为 `quantity` 条，正确的
        /// 声誉得分是 `score` 。
        /// 
        /// 以下情况不执行
        /// 
        /// - `score` 与原声誉值相同，或
        /// - `quantity` 大于种子数量，或
        /// - 声誉值未被更新，或
        /// - 声誉值已被挑战，或
        /// - 声誉值已超过确认期。
        /// 
        /// NOTE: 如果需要挑战已存在的挑战的声誉，应当调用 `arbitral` 。
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

        /// 对挑战中的路径仲裁。
        /// 
        /// 接受 `target` 下的 `seeds` 的正确路径 `paths` , `seeds` 和 `paths` 是集合，必须保持
        /// 一一对应的关系。
        /// 
        /// NOTE: 
        /// - 调用者必须纠正所有错误，否则其他挑战者可再次发起 `arbitral` ,从而导致本次挑战失败。
        /// - 在保护期限内，同一个调用者可多次发起 `arbitral` 而只需支付一次抵押。
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

        /// 挑战者上传路径。
        /// 
        /// 接受 `target` 下的 `seeds` 的正确路径 `paths` , `seeds` 和 `paths` 是集合，必须保持
        /// 一一对应的关系。
        /// 
        /// 在上传保护期内，挑战者可多次调用本接口，以便将所有路径上传完毕。这种“断点续传”，在种子数量过大，或网络
        /// 拥堵的情况下很有用。
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

    /// 在原有的基础上，增加 `pathfinder` `amount` 的应付账款 , 以及 `count` 个更新，并将
    /// 最后活动时间设置为 `now` 。
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

    /// 增加或修改 `pathfinder` 下针对`who`的挑战记录，其中获得的手续费为 `fee`,并将更新时间设置为 `now`。
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
