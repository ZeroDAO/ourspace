#[warn(unused_imports)]

use crate::{
	AccountId, Balance, Currencies, CurrencyId, Runtime,
};

use frame_support::assert_ok;
use sp_runtime::{
	traits::{SaturatedConversion, StaticLookup}
};
use orml_traits::MultiCurrencyExtended;

/* 
pub fn lookup_of_account(who: AccountId) -> <<Runtime as frame_system::Config>::Lookup as StaticLookup>::Source {
	<Runtime as frame_system::Config>::Lookup::unlookup(who)
}
*/

pub fn set_balance(currency_id: CurrencyId, who: &AccountId, balance: Balance) {
	assert_ok!(<Currencies as MultiCurrencyExtended<_>>::update_balance(
		currency_id,
		who,
		balance.saturated_into()
	));
}

#[cfg(test)]
pub mod tests {
	pub fn new_test_ext() -> sp_io::TestExternalities {
		frame_system::GenesisConfig::default()
			.build_storage::<crate::Runtime>()
			.unwrap()
			.into()
	}
}
