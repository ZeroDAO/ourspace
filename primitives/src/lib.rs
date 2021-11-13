#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    codec::{Decode, Encode},
    RuntimeDebug,
};
use sp_runtime::{Perbill, traits::{AtLeast32Bit, AtLeast32BitUnsigned,Zero}};
use sp_std::convert::TryInto;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[derive(Encode, Debug, Decode, Eq, PartialEq, Copy, Clone, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum CurrencyId {
    ZDAO,
    SOCI,
}

pub type AppId = [u8; 8];

/// Balance of an account.
pub type Balance = u128;

pub const SWEEPER_PERIOD: u64 = 500;

/// When other users receive their earnings, they receive that percentage of the earnings.
pub const SWEEPER_PICKUP_RATIO: Perbill = Perbill::from_perthousand(20);

pub mod per_social_currency {
    use super::Perbill;

    pub const MIN_TRUST_COUNT: u32 = 150;
    /// Reserve the owner's free balance. The percentage can be adjusted by the community.
    pub const PRE_RESERVED: Perbill = Perbill::from_percent(10);

    /// Transfer to the social currency of users trusted by the owner.
    pub const PRE_SHARE: Perbill = Perbill::from_percent(10);

    /// Share to all users.
    pub const PRE_BURN: Perbill = Perbill::from_percent(10);

    /// Pathfinder's fee
    pub const PRE_FEE: Perbill = Perbill::from_percent(10);

    // Used to solve the verifier's Dilemma and refresh seed.
    // pub const PRE_REWARD: Perbill = Perbill::from_percent(10);
}

/// 系统处于算法的状态
#[derive(Encode, Decode, Copy, Clone, PartialEq, Eq, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum TIRStep {
    Free,
    Seed,
    Reputation,
}

impl Default for TIRStep {
    fn default() -> TIRStep {
        TIRStep::Free
    }
}

pub mod fee {
    use super::*;

    pub trait SweeperFee
    where
        Self: Sized,
    {
        /// 最后活跃时间为 `last` 且当前时间为 `now` 时，是否允许 `sweeper` 参与。
        fn is_allowed_sweeper<B: AtLeast32BitUnsigned>(last: B, now: B) -> bool;

        /// 返回经过检查的 `fee`。
        fn checked_with_fee<B: AtLeast32BitUnsigned>(
            &self,
            last: B,
            now: B,
        ) -> Option<(Self, Self)>;

        /// 返回未经检查的 `fee`。
        fn with_fee(&self) -> (Self, Self);
    }

    impl SweeperFee for Balance {
        fn is_allowed_sweeper<B: AtLeast32BitUnsigned>(last: B, now: B) -> bool {
            let now_into = TryInto::<u64>::try_into(now).ok().unwrap();
            let last_into = TryInto::<u64>::try_into(last).ok().unwrap();
            last_into + SWEEPER_PERIOD < now_into
        }

        fn checked_with_fee<B: AtLeast32BitUnsigned>(
            &self,
            last: B,
            now: B,
        ) -> Option<(Self, Self)> {
            match Balance::is_allowed_sweeper(last, now) {
                true => Some(self.with_fee()),
                false => None,
            }
        }

        fn with_fee(&self) -> (Self, Self) {
            let sweeper_fee = SWEEPER_PICKUP_RATIO.mul_floor(*self);
            (sweeper_fee, self.saturating_sub(sweeper_fee))
        }
    }
}

/// 返回近似到整数的自然对数，最大为 8 。
pub fn appro_ln(value: u32) -> u32 {
    if value < 3 {
        1
    }else if value < 8 {
        2
    }else if value < 21 {
        3
    }else if value < 55 {
        4
    }else if value < 149 {
        5
    }else if value < 404 {
        6
    }else if value < 1097 {
        7
    }else {
        8
    }
}

/// 挑战游戏的状态
#[derive(Encode, Decode, Copy, Clone, PartialEq, Eq, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum ChallengeStatus {
    Free,
    Examine,
    Reply,
    Evidence,
    Arbitral,
}

impl Default for ChallengeStatus {
    fn default() -> Self {
        ChallengeStatus::Examine
    }
}

/// 用于保存用户抵押和收益的资金池。
#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug)]
pub struct Pool {
    pub staking: Balance,
    pub earnings: Balance,
}

/// 断点续传的进度信息。
#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, Default, RuntimeDebug)]
pub struct Progress {
    pub total: u32,
    pub done: u32,
}

/// 挑战游戏的元数据。
#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug)]
pub struct Metadata<AccountId, BlockNumber> {
    /// 当前资金池。
    pub pool: Pool,

    /// `pathfinder` 和挑战者是否为共同受益人。
    pub joint_benefits: bool,

    /// 断点续传的进度信息。
    pub progress: Progress,

    /// 上一次更新时间。
    pub last_update: BlockNumber,

    /// 用于存储有用信息的备注数据。
    pub remark: u32,

    /// 当前得分。
    pub score: u64,

    /// `pathfinder` 的 `AccountId`。
    pub pathfinder: AccountId,

    /// 当前处于的挑战状态。
    pub status: ChallengeStatus,

    /// 挑战者的 `AccountId`。
    pub challenger: AccountId,
}

impl<AccountId, BlockNumber> Metadata<AccountId, BlockNumber>
where
    AccountId: Ord + Clone,
    BlockNumber: Copy + AtLeast32Bit,
{
    /// 资金池的总额。
    pub fn total_amount(&self) -> Option<Balance> {
        self.pool.staking.checked_add(self.pool.earnings)
    }

    /// 是否全部上传结束。
    pub fn is_all_done(&self) -> bool {
        self.progress.total == self.progress.done
    }

    /// 是否未上传结束。
    pub fn check_progress(&self) -> bool {
        self.progress.total >= self.progress.done
    }

    /// `who` 是否为挑战者。
    pub fn is_challenger(&self, who: &AccountId) -> bool {
        self.challenger == *who
    }

    /// `who` 是否为 `pathfinder` 。
    pub fn is_pathfinder(&self, who: &AccountId) -> bool {
        self.pathfinder == *who
    }

    /// 重置进度。
    pub fn new_progress(&mut self, total: u32) -> &mut Self {
        self.progress.total = total;
        self.progress.done = Zero::zero();
        self
    }

    /// 将进度向前推进 `count` 条。
    pub fn next(&mut self, count: u32) -> &mut Self {
        self.progress.done = self.progress.done.saturating_add(count);
        self
    }

    /// 设置挑战状态为 `status`。
    pub fn set_status(&mut self, status: &ChallengeStatus) {
        self.status = *status;
    }

    /// 再次开始。
    pub fn restart(&mut self, full_probative: bool) {
        self.status = ChallengeStatus::Free;
        self.joint_benefits = false;
        if full_probative {
            self.pathfinder = self.challenger.clone();
        }
    }
}

