use sp_runtime::{DispatchResult,DispatchError};

pub trait MultiBaseToken<AccountId, Balance> {
	fn get_bonus_amount() -> Balance;
	fn staking(who: &AccountId, amount: &Balance) -> DispatchResult;
	fn release(who: &AccountId, amount: &Balance) -> DispatchResult;
	fn share(who: &AccountId, target: &AccountId) -> Result<Balance, DispatchError>;
}