use sp_runtime::DispatchResult;

pub trait MultiBaseToken<AccountId, Balance> {
    /// 获取当前 `bonus` 金额。
    fn get_bonus_amount() -> Balance;

    /// `who` staking `amount`。
    fn staking(who: &AccountId, amount: &Balance) -> DispatchResult;

    /// 释放 `amount` 的货币到 `who` 的账户。
    fn release(who: &AccountId, amount: &Balance) -> DispatchResult;

    /// 返回 `who` 的当前 `free_balance`。
    fn free_balance(who: &AccountId) -> Balance;

    /// 返回 `who` 的当前 `social_balance`。
    fn social_balance(who: &AccountId) -> Balance;

    /// 按比例将 `who` 的社交货币分割，并返回手续费金额。
    fn share(who: &AccountId, target: &[AccountId]) -> Balance;

    /// `who` 向池中注入 `amount` 金额的 `bonus`，优先扣除 `who` 的 `pending`。
    fn increase_bonus(who: &AccountId, amount: &Balance) -> DispatchResult;

    /// 直接减少 `amount` 金额的 `bonus`。
    ///
    /// 这是一个低级别操作，调用者应当自行维护金额平衡。
    fn cut_bonus(amount: &Balance) -> DispatchResult;

    /// 返回 `who` 的 `actual_balance`, 包括 `pending`,`social`和`free`。
    fn actual_balance(who: &AccountId) -> Balance;

    /// 返回 `who` 的 `pending_balance`。
    fn pending_balance(who: &AccountId) -> Balance;

    /// `from` 向 `to` 的社交账户中转入 `amount` 金额的货币。
    fn transfer_social(from: &AccountId, to: &AccountId, amount: Balance) -> DispatchResult;

    /// 优先使用 `from` 的 `pending` 向 `SocialPool` 账户转账 `amount` 金额的货币。
    fn pay_with_pending(from: &AccountId, amount: Balance) -> DispatchResult;

    /// 将 `who` 的 `pending` 取出。
    fn claim(who: &AccountId) -> DispatchResult;
}
