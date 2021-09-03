use sp_runtime::{DispatchResult,DispatchError};
use sp_std::vec::Vec;

pub trait MultiBaseToken<AccountId, Balance> {
	fn get_bonus_amount() -> Balance;
	fn staking(who: &AccountId, amount: &Balance) -> DispatchResult;
	fn release(who: &AccountId, amount: &Balance) -> DispatchResult;
	fn free_balance(who: &AccountId) -> Balance;
	fn social_balance(who: &AccountId) -> Balance;
	fn share(who: &AccountId, target: &Vec<AccountId>) -> Result<Balance, DispatchError>;
	fn increase_bonus(who: &AccountId, amount: &Balance) -> DispatchResult;
	fn cut_bonus(amount: &Balance) -> DispatchResult;
}