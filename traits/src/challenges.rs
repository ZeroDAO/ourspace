use sp_runtime::{DispatchResult,DispatchError};
use frame_support::{
    codec::{Decode, Encode},
    RuntimeDebug,
};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[derive(Encode, Decode, Copy, Clone, PartialEq, Eq, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum ChallengeStatus {
    FREE,
    EXAMINE,
    REPLY,
    EVIDENCE,
    ARBITRATION,
}

impl Default for ChallengeStatus {
    fn default() -> Self {
        ChallengeStatus::EXAMINE
    }
}

pub trait ChallengeBase<AccountId, AppId, Balance, BlockNumber> {

    fn is_all_harvest(app_id: &AppId) -> bool;

    fn is_all_timeout(app_id: &AppId,now: &BlockNumber) -> bool;

    fn set_status(app_id: &AppId, target: &AccountId,status: &ChallengeStatus);

    fn new(
        app_id: &AppId,
        who: &AccountId,
        path_finder: &AccountId,
        fee: Balance,
        staking: Balance,
        target: &AccountId,
        quantity: u32,
        score: u64,
        remark: u32,
    ) -> DispatchResult;

    fn next(
        app_id: &AppId,
        who: &AccountId,
        target: &AccountId,
        count: &u32,
        up: impl FnMut(u64,u32,bool) -> Result<(u64, u32), DispatchError>,
    ) -> DispatchResult;

    fn examine(
        app_id: &AppId,
        who: AccountId,
        target: &AccountId,
        index: u32,
    ) -> DispatchResult;

    fn reply(
        app_id: &AppId,
        who: &AccountId,
        target: &AccountId,
        total: u32,
        count: u32,
        up: impl Fn(bool, u32, u64) -> Result<u64, DispatchError>,
    ) -> DispatchResult;

    fn evidence(
        app_id: &AppId,
        who: &AccountId,
        target: &AccountId,
        up: impl Fn(u32,u64) -> Result<bool, DispatchError>,
    ) -> Result<Option<u64>, DispatchError>;

    fn arbitral(
        app_id: &AppId,
        who: &AccountId,
        target: &AccountId,
        up: impl Fn(u64,u32) -> Result<(bool, bool, u64), DispatchError>,
    ) -> DispatchResult;

    fn harvest(
        who: &AccountId,
        app_id: &AppId,
        target: &AccountId,
    ) -> Result<Option<u64>, DispatchError>;

    fn settle(
        app_id: &AppId,
        target: &AccountId,
        joint_benefits: bool,
        restart: bool,
        score: u64,
    ) -> DispatchResult;
}
