use sp_runtime::DispatchResult;

pub trait MultiBaseToken<AccountId, Balance> {
    fn get_bonus_amount() -> Balance;
    fn staking(who: &AccountId, amount: &Balance) -> DispatchResult;
    fn release(who: &AccountId, amount: &Balance) -> DispatchResult;
    fn free_balance(who: &AccountId) -> Balance;
    fn social_balance(who: &AccountId) -> Balance;
    fn share(who: &AccountId, target: &[AccountId]) -> Balance;
    fn increase_bonus(who: &AccountId, amount: &Balance) -> DispatchResult;
    fn cut_bonus(amount: &Balance) -> DispatchResult;
    fn actual_balance(who: &AccountId) -> Balance;
    fn pending_balance(who: &AccountId) -> Balance;
    fn transfer_social(from: &AccountId, to: &AccountId, amount: Balance) -> DispatchResult;
    fn pay_with_pending(from: &AccountId, amount: Balance) -> DispatchResult;
	fn claim(who: &AccountId) -> DispatchResult;
}
