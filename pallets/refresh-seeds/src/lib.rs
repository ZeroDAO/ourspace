//! # ZdRefreshSeeds Module
//! 
//! ## 介绍
//! 
//! 该模块用于种子选取并通过挑战游戏确保正确。
//! 
//! ## 算法
//! 
//! 用户的信任关系构成有向图，其中介数中心度最高的用户将成为种子。为了保证一致性，要
//! 求精确地计算介数中心度。例入有如下路径，需要我们计算 D 的中心度得分。
//! 
//! A -> B -> D -> E
//! A -> C -> D -> E
//! 
//! ### 计算得分
//! 
//! 经过 `D` 的所有最短路径为：
//! 
//! 1 A -> B -> D -> E
//! 2 A -> C -> D -> E
//! 3 B -> D -> E
//! 4 C -> D -> E
//! 
//! 其中 A 到 E 有两条相同长度的最短路径，即 `num = 2`，每条路径的得分为 `100 / 2`, 其他两条
//! 路径的得分为 100 。`D` 的总得分为 300 。
//! 
//! |  path  | num | score |
//! |--------|-----|-------|
//! | ABDE   |  2  |  50   |
//! | ACDE   |  2  |  50   |
//! | BDE    |  1  |  100  |
//! | CDE    |  1  |  100  |
//! 
//! ### 排序
//! 
//! 当网络较大时，经过某个节点的最短路径数量可能有几亿条，将其在链上一一验证是不现实，而且也是
//! 没有必要的。因此我们采用交互式验证。
//! 
//! #### 端点hash
//! 
//! 对端点进行哈希，取最后八位。
//! 
//! |  path  | num | score |     sha1(start,stop)      |
//! |--------|-----|-------|---------------------------|
//! | ABDE   |  2  |  50   |     ...f9 90 6c f1        |
//! | ACDE   |  2  |  50   |     ...f9 90 6c f1        |
//! | BDE    |  1  |  100  |     ...7c fe 03 66        |
//! | CDE    |  1  |  100  |     ...65 ce 02 66        |
//! 
//! #### 树
//! 
//! 从哈希值尾部开始，对该位置值相同的中心度得分进行求和。
//! 
//! | order  | score |
//! |--------|-------|
//! |   f1   |  100  |
//! |   66   |  200  |
//! 
//! 当挑战者发起挑战后，`pathfinder` 需要通过 `PostResultHash` 上传第一层的所有数据。这样
//! 挑战者可识别出差异位置，并继续质询。假设挑战者质询 `66` 。则 `pathfinder` 需要上传第二
//! 层数据：
//! 
//! | order  | score |
//! |--------|-------|
//! |   03   |  100  |
//! |   02   |  100  |
//! 
//! 直至系统设定的最大深度， `pathfinder` 上传符合条件的所有路径，挑战者可以继续对路径进行质询。
//! 
//! ## 接口
//!
//! ### 可调用函数
//!
//! - `start` - 开启种子更新。
//! - `add` - 增加一个种子候选人和中心度得分。
//! - `challenge` - 对一个种子的得分发起挑战。
//! - `examine` - 对指定位置的数据发起质询。
//! - `reply_hash` - 在收到质询后回复。
//! - `reply_hash_next` - 继续上传回复数据。
//! - `reply_path` - 在收到质询后回复路径。
//! - `reply_path_next` - 继续上传回复的路径数据。
//! - `reply_num` - 回复两个用户间最短路径数量。
//! - `missed_in_hashs` - 在hash阶段指出缺失的路径。
//! - `missed_in_paths` - 在 path 阶段指出缺失的路径
//! - `evidence_of_shorter` - 出示更短路径的证据。
//! - `number_too_low` - 出示两点之间路径数量过小的证据。
//! - `invalid_evidence` - 证明证据是错误的。
//! - `harvest_challenge` - 领取挑战收益。
//! - `harvest_seed` - 领取种子收益。
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::{
    codec::{Decode, Encode},
    ensure,
    traits::Get,
    RuntimeDebug,
    transactional,
};
use frame_system::{self as system};
use sp_runtime::{traits::Zero, DispatchError, DispatchResult};
use sp_std::{cmp::Ordering,vec::Vec};

pub use orml_utilities::OrderedSet;

use zd_primitives::{fee::SweeperFee, AppId, Balance, TIRStep, Metadata, Pool};
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

        /// 添加种子候选人的抵押金额。
        /// 
        /// SeedStakingAmount = SeedChallengeAmount + SeedReservStaking
        #[pallet::constant]
        type SeedStakingAmount: Get<Balance>;

        /// 挑战种子候选人种子度得分的抵押金额。
        #[pallet::constant]
        type SeedChallengeAmount: Get<Balance>;

        /// 这部分抵押金额不受到挑战影响，用户领取该部分金额的同时将会正式添加种子。
        #[pallet::constant]
        type SeedReservStaking: Get<Balance>;

        /// 种子的最大数量。
        #[pallet::constant]
        type MaxSeedCount: Get<u32>;

        /// 确认周期。
        #[pallet::constant]
        type ConfirmationPeriod: Get<Self::BlockNumber>;

        /// The weight information of this pallet.
		type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    /// 被挑战种子候选人的中心度得分数据。
    #[pallet::storage]
    #[pallet::getter(fn get_result_hashs)]
    pub type ResultHashsSets<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, Vec<OrderedSet<ResultHash>>, ValueQuery>;

    /// 种子候选人列表。
    #[pallet::storage]
    #[pallet::getter(fn get_candidate)]
    pub type Candidates<T: Config> = StorageMap<
        _,
        Twox64Concat,
        T::AccountId,
        Candidate<T::AccountId, T::BlockNumber>,
        ValueQuery,
    >;

    /// 有效中心度得分集合。
    #[pallet::storage]
    #[pallet::getter(fn get_score_list)]
    pub type ScoreList<T: Config> = StorageValue<_, Vec<u64>, ValueQuery>;

    /// 被挑战种子候选人的路径
    #[pallet::storage]
    #[pallet::getter(fn get_path)]
    pub type Paths<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, Vec<Path<T::AccountId>>, ValueQuery>;

    /// 缺失的路径
    #[pallet::storage]
    #[pallet::getter(fn get_missed_paths)]
    pub type MissedPaths<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, Vec<T::AccountId>, ValueQuery>;

    /// 是否已确认全部种子
    #[pallet::storage]
    #[pallet::getter(fn seeds_confirmed)]
    pub type SeedsConfirmed<T: Config> = StorageValue<_, bool, ValueQuery>;

    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// 发起了新一轮的种子更新。 \[who\]
        RefershSeedStared(T::AccountId),
        /// 新的种子候选人。 \[pthfinder, candidate,score\]
        NewCandidate(T::AccountId, T::AccountId, u64),
        /// 新的挑战。 \[challenger, candidate\]
        NewChallenge(T::AccountId, T::AccountId),
        /// 新的质询。 \[challenger, candidate\]
        NewExamine(T::AccountId, T::AccountId),
        /// 新的hash被回复了。 \[pthfinder, candidate, quantity, completed\]
        RepliedHash(T::AccountId, T::AccountId, u32, bool),
        /// 继续回复了新的hash \[pthfinder, candidate, completed\]
        ContinueRepliedHash(T::AccountId, T::AccountId, bool),
        /// 回复了路径。 \[pthfinder, candidate, quantity, completed\]
        RepliedPath(T::AccountId, T::AccountId, u32, bool),
        /// 继续回复了路径。 \[pthfinder, candidate, completed\]
        ContinueRepliedPath(T::AccountId, T::AccountId, bool),
        /// 回复了两用户间最短路径数量。 \[pthfinder, candidate\]
        RepliedNum(T::AccountId, T::AccountId),
        /// 出示了漏掉的最短路径。 \[challenger, candidate,index\]
        MissedPathPresented(T::AccountId, T::AccountId, u32),
        /// 出示了更短的路径。 \[challenger, candidate,index\]
        ShorterPresented(T::AccountId, T::AccountId, u32),
        /// 出示了两用户间最短路径总量过小的证据。 \[challenger, candidate,index\]
        EvidenceOfNumTooLowPresented(T::AccountId, T::AccountId, u32),
        /// 出示了证据为无效的证明。 \[challenger, candidate,score\]
        EvidenceOfInvalidPresented(T::AccountId, T::AccountId, u64),
        /// 领取了挑战收益。 \[who, candidate\]
        ChallengeHarvested(T::AccountId, T::AccountId),
        /// 领取了种子更新收益。 \[who, candidate\]
        SeedHarvested(T::AccountId, T::AccountId),
        /// 发起了一个挑战。 \[candidate, score\]
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
        /// 开始种子更新。
        #[pallet::weight(T::WeightInfo::start())]
        #[transactional]
        pub fn start(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            T::Reputation::new_round()?;
            Self::deposit_event(Event::RefershSeedStared(who));
            Ok(().into())
        }

        /// 添加 `target` 为种子候选人，中心度得分为 `score` 。
        /// 
        /// 将扣除调用者 `SeedStakingAmount` 的资金。
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

        /// 对种子候选人 `target` 发起挑战，新的得分为 `score` 。
        /// 
        /// 以下情况下将失败：
        /// 
        /// - 候选人不存在，或
        /// - 种子已经过挑战，并且处于 `Free` 状态，但已经超过确认期。
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
                }

            )?;
            <Candidates<T>>::mutate(&target, |c| c.has_challenge = true);
            T::Reputation::set_last_refresh_at();
            Self::deposit_event(Event::NewChallenge(challenger, target));
            Ok(().into())
        }

        /// 质询种子候选人 `target` 下，`index` 位置的数据。
        /// 
        /// - 在 hash 期，即当前深度小于 `DEEP` , 挑战数据指是 `ResultHashsSets` 中的数据;
        /// - 在路径上传后，指向的是路径数据中的 `num`，即两点间最短路径的数量。
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

        /// 回复hash数据集合，一共有 `quantity` 条。
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

        /// 继续回复 hash 数据。
        /// 
        /// `pathfinder` 应当在允许期间内上传，否则系统有权判定为失败。
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

        /// 回复针对 `target` 的挑战的路径数据，一共为 `quantity` 条。
        /// 
        /// 以下情况将失败：
        /// 
        /// - 未达到最大深度（此时应该回复 hash）,或
        /// - 路径重复或错误。
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

        /// 继续回复路径数据。
        /// 
        /// 这在数据过多，或网络拥堵的情况下很有用。
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

        /// 回复最短路径总量。
        /// 
        /// 这发生在挑战者认为 `pathfinder` 的路径数据中总量数据过大的情况下，需要 `pathfinder`
        /// 上传两端点间所有最短路径，以证明数据正确性。
        /// 
        /// `mid_paths` - 此处两个端点已经确定，所以只需要上传中间的 `node`,如果中间没有用户，则
        /// 为 `[]` 。
        /// 
        /// NOTE: 当两端点间路径总数大于 100 时，该路径是无效的。我们只精确到小数点后两位。
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

        /// 在 hash 阶段出具丢失路径的证据。
        /// 
        /// - `nodes`- 完整的路径向量。
        /// - `index` - 丢失路径处于的位置。例如，hash 集为 [5,8,10], 如果你的拥有hash为 4 的路径，则
        /// `index` 为 `0` 。如果为 `11`,则 `index` 为 `2` 。
        /// 
        /// 调用成功后将进入仲裁阶段，因为我们无法确定 `nodes` 是否为最短路径，需要再次通过挑战游戏确
        /// 定。
        #[pallet::weight(T::WeightInfo::missed_in_hashs())]
        #[transactional]
        pub fn missed_in_hashs(
            origin: OriginFor<T>,
            target: T::AccountId,
            nodes: Vec<T::AccountId>,
            index: u32,
        ) -> DispatchResultWithPostInfo {
            let challenger = ensure_signed(origin)?;

            ensure!(
                !<Paths<T>>::contains_key(&target),
                Error::<T>::PathUploaded
            );

            Self::evidence_of_missed(
                &challenger,
                &target,
                &nodes,
                index,
            )?;
            Ok(().into())
        }

        /// 路径集中丢失的路径。
        /// 
        /// 上传有效的 `nodes` ，如果它不在路径集中，则证明 `pathfinder` ** 有可能 ** 丢失了该路径。调用成功
        /// 后将进入仲裁阶段，因为我们无法确定 `nodes` 是否为最短路径，需要再次通过挑战游戏确定。
        #[pallet::weight(T::WeightInfo::missed_in_paths())]
        #[transactional]
        pub fn missed_in_paths(
            origin: OriginFor<T>,
            target: T::AccountId,
            nodes: Vec<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            let challenger = ensure_signed(origin)?;

            ensure!(
                <Paths<T>>::contains_key(&target),
                Error::<T>::NoPathExists
            );

            Self::evidence_of_missed(
                &challenger,
                &target,
                &nodes,
                Zero::zero(),
            )?;
            Ok(().into())
        }

        /// 传入更短的有效路径，来证明针对 `target` 中的路径集中 `index` 位置的路径是无效的。
        /// 
        /// 路径两个端点已经确定，因此仅仅需要传入不包含端点的中间节点 : `mid_path`。
        /// 
        /// 因为“更短的路径”是确定性的，执行成功后 `pathfinder` 将失败，而当前挑战者将作为
        /// 新的 `pathfinder` 接受其他挑战者的挑战。
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

        /// 传入全部路径来证明 `target` 的路径集中`index`下的路径总量过小。
        /// 
        /// 路径两个端点已经确定，因此仅仅需要传入不包含端点的中间节点 : `mid_path`。如果路径总数
        /// 超过 `MAX_SHORTEST_PATH` ,那么只需要上传 `MAX_SHORTEST_PATH` + 1 条路径以证明原路径
        /// 是无效的。
        /// 
        /// NOTE: 路径的长度必须和原有长度一致，如果你有更短的路径，应该调用 `evidence_of_shorter`。
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

        /// 上传更短的路径 `mid_path`, 证明 `missed_in_paths` 或 `missed_in_hashs` 出示的证据是错误的。
        /// 
        /// 如果 `mid_path` 包含 `target` ,则表示 `pathfinder` 是错误的，调用者将作为新的挑战者，使用 `score`
        /// 作为新的中心度得分接受挑战。
        /// 
        /// 如果`mid_path` 不包含 `target`，则无法证明 `pathfinder` 是错误的，调用者将作为 `pathfinder` 的共同
        /// 受益人平分池中的资金。
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

        /// 领取挑战收益。
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

        /// 领取种子收益。
        /// 
        /// 以下情况将失败：
        /// - 挑战尚未全部领取完毕，或
        /// - 种子尚未全部超过确认期。
        /// 
        /// 每一轮中首次领取将首先确定全部种子，例如种子候选人为 100 个，但种子最大数量
        /// 为 90 个，则会取得分最高的 90 个。如果第 91 个和第 90 个得分相同，则首先领取
        /// 的会被确认。
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
