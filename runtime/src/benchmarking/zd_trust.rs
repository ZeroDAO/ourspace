use crate::{AccountId, Runtime, ZdReputation, ZdTrust, MaxTrustCount};
use frame_benchmarking::account;
use frame_system::RawOrigin;
use sp_std::prelude::*;
use zd_support::Reputation;
use zd_primitives::TIRStep;

use orml_benchmarking::runtime_benchmarks;

const MAX_TRUST_COUNT: u32 = MaxTrustCount::get();
const SEED: u32 = 0;

runtime_benchmarks! {
    { Runtime, zd_trust }

    _ {}

    // trust in worst case
    trust {
        let target: AccountId = account("target", 0, SEED);
        for i in 2..MAX_TRUST_COUNT {
            let from_i: AccountId = account("from", 0, i);
            let _ = ZdTrust::trust(RawOrigin::Signed(from_i.clone()).into(), target.clone().into());
        }

        ZdReputation::set_step(&TIRStep::Reputation);

        for ii in 2..MAX_TRUST_COUNT {
            let from_i: AccountId = account("from", 0, ii);
            let _ = ZdTrust::untrust(RawOrigin::Signed(from_i.clone()).into(), target.clone().into());
        }

        for iii in (MAX_TRUST_COUNT * 2 + 1)..(MAX_TRUST_COUNT * 3) {
            let from_i: AccountId = account("from", 0, iii);
            let _ = ZdTrust::trust(RawOrigin::Signed(from_i.clone()).into(), target.clone().into());
        }

        let who: AccountId = account("who", 0, SEED);
    }: _(RawOrigin::Signed(who.clone()), target.into())

    // untrust in worst case
    untrust {
        let target: AccountId = account("target", 0, SEED);

        for i in 2..MAX_TRUST_COUNT {
            let from_i: AccountId = account("from", 0, i);
            let _ = ZdTrust::trust(RawOrigin::Signed(from_i.clone()).into(), target.clone().into());
        }

        ZdReputation::set_step(&TIRStep::Reputation);

        for ii in 2..MAX_TRUST_COUNT {
            let from_i: AccountId = account("from", 0, ii);
            let _ = ZdTrust::untrust(RawOrigin::Signed(from_i.clone()).into(), target.clone().into());
        }

        for iii in (MAX_TRUST_COUNT * 2 + 1)..(MAX_TRUST_COUNT * 3) {
            let from_i: AccountId = account("from", 0, iii);
            let _ = ZdTrust::trust(RawOrigin::Signed(from_i.clone()).into(), target.clone().into());
        }

		let who: AccountId = account("who", 0, SEED);
		let _ = ZdTrust::trust(RawOrigin::Signed(who.clone()).into(), target.clone().into());
    }: _(RawOrigin::Signed(who.clone()), target.into())

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::benchmarking::utils::tests::new_test_ext;
    use orml_benchmarking::impl_benchmark_test_suite;

    impl_benchmark_test_suite!(new_test_ext(),);
}
