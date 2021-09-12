pub trait SeedsBase<AccountId> {
	fn is_seed(seed: &AccountId) -> bool;
	fn get_seed_count() -> u32;
	fn remove_all();
	fn add_seed(new_seed: &AccountId);
}