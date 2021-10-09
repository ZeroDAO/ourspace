use sp_runtime::{DispatchError, DispatchResult};
use zd_primitives::{ChallengeStatus, Metadata};

pub trait ChallengeBase<AccountId, AppId, Balance, BlockNumber> {
    fn set_metadata(
        app_id: &AppId,
        target: &AccountId,
        metadata: &Metadata<AccountId, BlockNumber>,
    );

    fn is_all_harvest(app_id: &AppId) -> bool;

    fn is_all_timeout(app_id: &AppId, now: &BlockNumber) -> bool;

    fn set_status(app_id: &AppId, target: &AccountId, status: &ChallengeStatus);

    fn launch(
        app_id: &AppId,
        target: &AccountId,
        metadata: &Metadata<AccountId, BlockNumber>,
    ) -> DispatchResult;

    fn next(
        app_id: &AppId,
        who: &AccountId,
        target: &AccountId,
        count: &u32,
        up: impl FnMut(u64, u32, bool) -> Result<(u64, u32), DispatchError>,
    ) -> DispatchResult;

    fn examine(app_id: &AppId, who: &AccountId, target: &AccountId, index: u32) -> DispatchResult;

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
        up: impl Fn(u32, u64) -> Result<bool, DispatchError>,
    ) -> Result<Option<u64>, DispatchError>;

    fn arbitral(
        app_id: &AppId,
        who: &AccountId,
        target: &AccountId,
        up: impl Fn(u64, u32) -> Result<(bool, bool, u64), DispatchError>,
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