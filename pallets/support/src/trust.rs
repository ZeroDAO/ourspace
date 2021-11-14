use sp_runtime::{DispatchError, DispatchResult};
use sp_std::vec::Vec;

pub trait TrustBase<AccountId> {
    /// Clear all data from the `TrustTempList`.
    fn remove_all_tmp();

    /// Get how many users `who` trusts.
    fn get_trust_count(who: &AccountId) -> usize;

    /// Gets the number of users trusted by `who` before the refresh started.
    fn get_trust_count_old(who: &AccountId) -> usize;

    /// Returns whether `who` trusts `target`.
    fn is_trust(who: &AccountId, target: &AccountId) -> bool;

    /// Returns whether `who` trusts `target` before the refresh started.
    fn is_trust_old(who: &AccountId, target: &AccountId) -> bool;

    /// Returns the user trusted by `who` before the refresh started.
    fn get_trust_old(who: &AccountId) -> Vec<AccountId>;

    /// Returns the total length of the `users` path as a tuple, which is passed 
    /// to the end user's reputation value, the first user is not the seed user, 
    /// or an error in the path will return `Error`.
    fn computed_path(users: &[AccountId]) -> Result<(u32, u32), DispatchError>;

    /// `Ok` will be returned if the path is correct.
    fn valid_nodes(nodes: &[AccountId]) -> DispatchResult;
}
