use sp_runtime::DispatchResult;

pub trait RefreshPayrolls<AccountId, Balance> {
    fn add_payroll(pathfinder: &AccountId, total_fee: &Balance, count: u32) -> DispatchResult;
    fn add_record(pathfinder: &AccountId, who: &AccountId, fee: &Balance);
}
