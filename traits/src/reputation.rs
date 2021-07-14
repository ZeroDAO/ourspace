use sp_runtime::DispatchResult;

pub trait Reputation<AccountId, BlockNumber> {
	/// 低级别操作，直接覆盖最新声誉
	fn mutate_reputation(target: &AccountId, ir: u32);

	/// 获取最新声誉，更新过程中会返回未经挑战的声誉值
	fn get_reputation_new(target: &AccountId) -> Option<u32>;

	/// 更新声誉值
	fn refresh_reputation(
		user_score: &(AccountId,u32)
	) -> DispatchResult;

	/// 设置最后更新时间
	fn last_refresh_at();

	/// 检查更新状态与预期是否相符，并返回序号,
	/// 与预期不同则返回 None
	fn check_update_status(update_mode: bool) -> Option<u32>;

	/// 设置最后挑战的时间，包括子挑战
	fn last_challenge_at();

	/// 检查更新整体是否结束，未结束并符合结束条件则结束
	fn end_refresh() -> DispatchResult;

	/// 开始新的一轮
	fn new_round() -> DispatchResult;

}