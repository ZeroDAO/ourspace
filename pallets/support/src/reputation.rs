use sp_runtime::DispatchResult;

pub trait Reputation<AccountId, BlockNumber, TIRStep> {
    fn mutate_reputation(target: &AccountId, ir: &u32);

    fn set_step(step: &TIRStep);

    fn is_step(step: &TIRStep) -> bool;

    fn get_reputation_new(target: &AccountId) -> Option<u32>;

    fn get_reputation(target: &AccountId) -> Option<u32>;

    fn refresh_reputation(user_score: &(AccountId, u32)) -> DispatchResult;

    fn get_last_refresh_at() -> BlockNumber;

    fn set_last_refresh_at();

    fn set_free();

    fn new_round() -> DispatchResult;
}
