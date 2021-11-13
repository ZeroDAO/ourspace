use sp_runtime::DispatchResult;

pub trait Reputation<AccountId, BlockNumber, TIRStep> {
    /// 直接修改 `target` 中第一个 nonce 下的声誉值。
    fn mutate_reputation(target: &AccountId, ir: &u32);

    /// 设置 `TIRStep` 为 `step`。
    fn set_step(step: &TIRStep);

    /// 当前 `TIRStep` 是否为 `step`。
    fn is_step(step: &TIRStep) -> bool;

    /// 获取 `target` 系统最新 `nonce` 下的声誉值，如果系统正在更新中，则
    /// 有可能返回未经验证的声誉值。
    fn get_reputation_new(target: &AccountId) -> Option<u32>;

    /// 返回 `target` 最新经过验证的声誉值。
    fn get_reputation(target: &AccountId) -> Option<u32>;

    /// 接受一个 `AccountId`, `u32` 的元组，仅当该用户未刷新时执行刷新。
    fn refresh_reputation(user_score: &(AccountId, u32)) -> DispatchResult;

    /// 返回种子和声誉刷新的系统级最后更新区块。
    fn get_last_refresh_at() -> BlockNumber;

    /// 修改最新刷新时间为当前区块。
    fn set_last_refresh_at();

    /// 设置系统状态为 `TIRStep::Free`。
    fn set_free();

    /// 开启新的一轮。
    fn new_round() -> DispatchResult;
}
