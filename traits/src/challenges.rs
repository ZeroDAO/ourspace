use sp_runtime::{DispatchResult,DispatchError};

pub trait ChallengeBase<AccountId, AppId, Balance,BlockNumber> {
    
    fn is_all_harvest(app_id: &AppId) -> bool;

    fn is_all_timeout(app_id: &AppId,now: &BlockNumber) -> bool;

    fn new(
        app_id: &AppId,
        who: &AccountId,
        path_finder: &AccountId,
        fee: Balance,
        staking: Balance,
        target: &AccountId,
        quantity: u32,
        score: u64,
    ) -> DispatchResult;

    fn next(
        app_id: &AppId,
        who: &AccountId,
        target: &AccountId,
        count: u32,
        up: impl FnOnce(Balance,u32,bool) -> Result<u32, DispatchError>,
    ) -> DispatchResult;

    fn question(
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
        up: impl Fn(bool,u32) -> DispatchResult,
    ) -> DispatchResult;

    fn new_evidence(
        app_id: &AppId,
        who: &AccountId,
        target: &AccountId,
        up: impl Fn(u32,u64) -> Result<bool, DispatchError>,
    ) -> Result<Option<u64>, DispatchError>;

    fn arbitral(
        app_id: &AppId,
        who: &AccountId,
        target: &AccountId,
        score: u64,
        up: impl Fn(u32) -> Result<(bool,bool), DispatchError>,
    ) -> DispatchResult;

    fn harvest(
        who: &AccountId,
        app_id: &AppId,
        target: &AccountId,
    ) -> Result<Option<u64>, DispatchError>;
}
