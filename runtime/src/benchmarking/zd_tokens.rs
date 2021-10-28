use crate::{AccountId, CurrencyId, Currencies, GetNativeCurrencyId, Runtime, ZdToken};
use sp_std::prelude::*;
use frame_system::RawOrigin;
use frame_benchmarking::{account};
use frame_support::assert_ok;

use zd_support::MultiBaseToken;

use orml_benchmarking::runtime_benchmarks;
use orml_traits::MultiCurrency;

const NATIVE: CurrencyId = GetNativeCurrencyId::get();
const SEED: u32 = 0;

runtime_benchmarks! {
	{ Runtime, zd_tokens }

    _ {}

    transfer_social {
        let from: AccountId = account("from", 0, SEED);
        let to: AccountId = account("to", 0, SEED);
		Currencies::deposit(NATIVE, &from, 10_000)?;
	}: _(RawOrigin::Signed(from.clone()), to.into(), 10_000)

	claim {
		let vault = account("vault", 0, 0);
        assert_ok!(Currencies::deposit(NATIVE, &vault, 1_000_000_000_000u128));
		let _ = <ZdToken as MultiBaseToken<_,_>>::staking(&vault.clone(), &10_000u128);
		let who: AccountId = account("who", 0, SEED);
		ZdToken::set_pending_balance(&who.clone(), 10_000u128);
	}: _(RawOrigin::Signed(who.clone()))

}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::benchmarking::utils::tests::new_test_ext;
	use orml_benchmarking::impl_benchmark_test_suite;

	impl_benchmark_test_suite!(new_test_ext(),);
}
