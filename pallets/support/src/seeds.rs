pub trait SeedsBase<AccountId> {

	/// 返回 `seed` 是否为种子用户。
	fn is_seed(seed: &AccountId) -> bool;

	/// 返回种子用户数量。
	fn get_seed_count() -> u32;

	/// 清空所有种子。
	fn remove_all();

	/// 增加种子。
	fn add_seed(new_seed: &AccountId);
}