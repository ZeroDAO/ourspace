use sp_runtime::DispatchError;
use sp_std::vec::Vec;

pub trait TrustBase<AccountId> {
	/// 低级别操作，直接覆盖最新声誉
	fn get_trust_count(who: &AccountId) -> usize;
	fn get_trust_count_old(who: &AccountId) -> usize;
	fn is_trust(who: &AccountId, target: &AccountId) -> bool;
	fn is_trust_old(who: &AccountId, target: &AccountId) -> bool;
	fn get_trust_old(who: &AccountId) -> Vec<AccountId>;
	fn computed_path(users: &Vec<AccountId>) -> Result<(u32,u32), DispatchError>;
}