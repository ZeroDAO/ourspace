use sp_runtime::DispatchError;

pub trait StartChallenge<AccountId,Balance> {
	/// 低级别操作，直接覆盖最新声誉
	fn start(target: &AccountId, analyst: &AccountId) -> Result<Balance, DispatchError>;

}