pub trait SeedsBase<AccountId> {

	/// Returns whether `seed` is the seed user.
	fn is_seed(seed: &AccountId) -> bool;

	/// Returns the number of seed users.
	fn get_seed_count() -> u32;

	/// Empty all seeds.
	fn remove_all();

	/// Add a seed.
	fn add_seed(new_seed: &AccountId);
}