use sp_runtime::{DispatchError, DispatchResult};
use sp_std::vec::Vec;

pub trait TrustBase<AccountId> {
    /// 清除 `TrustTempList` 中的所有数据。
    fn remove_all_tmp();

    /// 获取 `who` 信任了多少用户。
    fn get_trust_count(who: &AccountId) -> usize;

    /// 获取 `who` 在更新开始前信任了多少用户。
    fn get_trust_count_old(who: &AccountId) -> usize;

    /// 返回 `who` 是否信任 `target`。
    fn is_trust(who: &AccountId, target: &AccountId) -> bool;

    /// 返回 `who` 在更新开始前是否信任 `target`。
    fn is_trust_old(who: &AccountId, target: &AccountId) -> bool;

    /// 返回 `who` 在更新开始前信任的用户。
    fn get_trust_old(who: &AccountId) -> Vec<AccountId>;

    /// 以元组返回 `users` 路径的总长度，这传递到最终用户的声誉
    /// 值，首个用户并非种子用户，或者路径错误将返回 `Error`。
    fn computed_path(users: &[AccountId]) -> Result<(u32, u32), DispatchError>;

    /// 路径正确时将返回 `Ok`。
    fn valid_nodes(nodes: &[AccountId]) -> DispatchResult;
}
