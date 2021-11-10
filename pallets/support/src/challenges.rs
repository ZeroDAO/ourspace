use sp_runtime::{DispatchError, DispatchResult};
use zd_primitives::{ChallengeStatus, Metadata};

pub trait ChallengeBase<AccountId, AppId, Balance, BlockNumber> {
    /// 直接修改挑战游戏的数据。
    fn set_metadata(
        app_id: &AppId,
        target: &AccountId,
        metadata: &Metadata<AccountId, BlockNumber>,
    );

    /// `app_id` 下的挑战是否全部结算完毕。
    fn is_all_harvest(app_id: &AppId) -> bool;

    /// `app_id` 下的挑战是否全部超过挑战时间。
    fn is_all_timeout(app_id: &AppId, now: &BlockNumber) -> bool;

    /// 设置 `app_id` 下针对 `target`挑战的状态。
    fn set_status(app_id: &AppId, target: &AccountId, status: &ChallengeStatus);

    /// 发起一个 `app_id` 下针对 `target` 的挑战， `metadata` 用来设置初始的挑战状态。
    fn launch(
        app_id: &AppId,
        target: &AccountId,
        metadata: &Metadata<AccountId, BlockNumber>,
    ) -> DispatchResult;

    /// 继续上传了 `app_id` 下针对 `target` 的挑战中 `count` 条数据，`who` 用来验证原始发起者。
    ///
    /// `up` 向调用者传递三个参数：
    ///
    /// - `score` 该挑战当前记录的分数。
    /// - `remark` 该挑战当前的备注信息，方便调用者记录挑战信息。
    /// - `is_all_done` 数据是否全部上传完成。
    ///
    /// 当 `up` 返回 `Error` 不执行挑战，否则更新返回的 `score` 和 `remark` 。
    fn next(
        app_id: &AppId,
        who: &AccountId,
        target: &AccountId,
        count: &u32,
        up: impl FnMut(u64, u32, bool) -> Result<(u64, u32), DispatchError>,
    ) -> DispatchResult;

    /// 向 `index` 位置的数据发起挑战
    fn examine(app_id: &AppId, who: &AccountId, target: &AccountId, index: u32) -> DispatchResult;

    /// 回复 `examine` 质询的数据，需要上传共 `total` 条数据，
    /// 本次上传 `count` 条。
    ///
    /// `up` 向调用者传递三个参数：
    ///
    /// - `is_all_done` 数据是否全部上传完成。
    /// - `score` 该挑战当前记录的分数。
    /// - `remark` 该挑战当前的备注信息，方便调用者记录挑战信息。
    ///
    /// 仅当 `up` 返回 `Ok(score)` 时更新挑战，并更新 `score`。
    fn reply(
        app_id: &AppId,
        who: &AccountId,
        target: &AccountId,
        total: u32,
        count: u32,
        up: impl Fn(bool, u32, u64) -> Result<u64, DispatchError>,
    ) -> DispatchResult;

    /// 提交证据
    ///
    /// `up` 向调用者传递两个参数：
    ///
    /// - `remark` 该挑战当前的备注信息，方便调用者记录挑战信息。
    /// - `score` 该挑战当前记录的分数。
    ///
    /// 仅当 `up` 返回 `Ok(needs_arbitration)` 时更新挑战，并根据其值进入相应状态：
    ///
    /// - `true` 挑战成功，将通过 `restart` 进行初始化。
    /// - `false` 证据力不足，挑战将进入仲裁状态。
    fn evidence(
        app_id: &AppId,
        who: &AccountId,
        target: &AccountId,
        up: impl Fn(u32, u64) -> Result<bool, DispatchError>,
    ) -> Result<Option<u64>, DispatchError>;

    /// 对提交的数据进行仲裁，这一般用于无法在链上直接计算但可验证的数据，例如
    /// 最短路径。
    ///
    /// `up` 向调用者传递两个参数：
    ///
    /// - `score` 该挑战当前记录的分数。
    /// - `remark` 该挑战当前的备注信息，方便调用者记录挑战信息。
    ///
    /// 仅当 `up` 返回 `Ok(joint_benefits, restart, score)` 时更新挑战，并根据其值进入相应状态：
    ///
    /// - `joint_benefits` 为 `true` 时，`pathfinder` 和 `challenger` 将作为共同受益人。
    /// - `restart` 是否需要初始化挑战，此时原 `challenger` 将接受挑战。
    /// - `score` 记录到挑战系统的分数。
    fn arbitral(
        app_id: &AppId,
        who: &AccountId,
        target: &AccountId,
        up: impl Fn(u64, u32) -> Result<(bool, bool, u64), DispatchError>,
    ) -> DispatchResult;

    /// 收取挑战收益。按照 `ChallengeStatus`, `is_all_done`, `joint_benefits` 分配：
    ///
    /// +-----------+------------+------------+------------+------------+
    /// |           |    Free    |    Reply   |   Examine  |  Evidence  |
    /// +-----------+------------+------------+------------+------------+
    /// |   完成    | pathfinder | pathfinder | challenger | challenger |
    /// +-----------+------------+------------+------------+------------+
    /// |   中断    | pathfinder | challenger | challenger | pathfinder |
    /// +-----------+------------+------------+------------+------------+
    ///
    /// 在 `Arbitral` 状态下，结算则按照 `joint_benefits` ，如果为 `true` ,则 `pathfinder`
    /// 和 `challenger` 平分奖励，否则全部归 `pathfinder` 所有。
    fn harvest(
        who: &AccountId,
        app_id: &AppId,
        target: &AccountId,
    ) -> Result<Option<u64>, DispatchError>;

    /// 结算当前挑战。这是一个低级别的操作。
    ///
	/// 当 `restart` 为 `true` 时，挑战将被设为 `Free` 状态，同时当 `joint_benefits` 为
	///  - `true` 将平分奖金池，并发送给 `challenger`。
	///  - `false` 直接修改挑战数据。
	///
	/// 当 `restart` 为 `false`，修改挑战系统中 `joint_benefits` 和 `score`。
    fn settle(
        app_id: &AppId,
        target: &AccountId,
        joint_benefits: bool,
        restart: bool,
        score: u64,
    ) -> DispatchResult;
}
