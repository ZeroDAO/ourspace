#[warn(unused_must_use)]
use crate::{
    AccountId, Currencies, CurrencyId, GetNativeCurrencyId, MaxSeedCount, MaxTrustCount,
    MaxUpdateCount, Runtime, System, ZdRefreshReputation, ZdReputation, ZdSeeds, ZdToken, ZdTrust,
};
use frame_benchmarking::{account, whitelisted_caller};
use frame_system::RawOrigin;
use sp_std::prelude::*;
use zd_primitives::TIRStep;
use zd_refresh_reputation::Path;

use orml_benchmarking::runtime_benchmarks;
use orml_traits::MultiCurrency;
use zd_support::{MultiBaseToken, Reputation, SeedsBase, TrustBase};

use frame_support::assert_ok;

const NATIVE: CurrencyId = GetNativeCurrencyId::get();
const MAX_TRUST_COUNT: u32 = MaxTrustCount::get();
const MAX_UPDATE_COUNT: u32 = MaxUpdateCount::get();
const MAX_REFRESH: u32 = 500;
const MAX_SEED_COUNT: u32 = MaxSeedCount::get();
const MAX_NODE_COUNT: u32 = 5;

fn init_harvest(pathfinder: &AccountId) {
    let vault = account("vault", 0, 0);
    let _ = Currencies::deposit(NATIVE, &vault, 1_000_000_000_000u128);
    assert_ok!(ZdToken::staking(&vault, &1_000_000_000_000u128));

    let _ = ZdRefreshReputation::start(RawOrigin::Signed(pathfinder.clone()).into());

    let now = System::block_number();

    for t in 1..MAX_REFRESH {
        let targer: AccountId = account("targer", 0, t);
        ZdRefreshReputation::mutate_record(pathfinder, &targer.clone(), &200u128, &now);
    }
    let _ = ZdRefreshReputation::mutate_payroll(pathfinder, &2000u128, &MAX_REFRESH, &now);
    ZdReputation::set_step(&TIRStep::Reputation);
}

fn init_challenge(challenger: &AccountId, targer: &AccountId, score: u32) {
    let vault = account("vault", 0, 0);
    let _ = Currencies::deposit(NATIVE, &vault, 1_000_000_000_000u128);
    let accounts = vec![(targer.clone(), score)];
    ZdReputation::set_step(&TIRStep::Reputation);
    let pathfinder: AccountId = account("pathfinder", 0, 0);
    let _ = Currencies::deposit(NATIVE, &pathfinder, 1_000_000_000_000u128);
    let _ = ZdRefreshReputation::start(RawOrigin::Signed(pathfinder.clone()).into());

    let _ = ZdRefreshReputation::refresh(RawOrigin::Signed(pathfinder.clone()).into(), accounts);

    let _ = Currencies::deposit(NATIVE, challenger, 1_000_000_000_000u128);
}

fn checked_trust(source: &AccountId, targer: &AccountId) {
    if !<ZdTrust as TrustBase<_>>::is_trust(source, targer) {
        let _ = ZdTrust::trust(
            RawOrigin::Signed(source.clone()).into(),
            targer.clone().into(),
        );
    }
}

runtime_benchmarks! {
    { Runtime, zd_refresh_reputation }

    _ {}

    // Construct 10 unfilled orders
    start {
        ZdReputation::set_step(&TIRStep::Reputation);
        System::set_block_number(1);

        let vault = account("vault", 0, 0);

        for i in 2..12 {
            let finder: AccountId = account("finder", 0, i);
            let total_fee = 1_000;
            ZdRefreshReputation::mutate_payroll(
               &finder,
                &total_fee.clone(),
                &20,
                &1
            )?;
        }
        Currencies::deposit(NATIVE, &vault, 1_000_000_000_000u128)?;
        assert_ok!(ZdToken::staking(&vault, &1_000_000_000_000u128));

        System::set_block_number(2000);
        let starter: AccountId = account("pathfinder", 0, 0);

    }: _(RawOrigin::Signed(starter.clone()))

    refresh {
        let a in 0 .. MAX_UPDATE_COUNT;

        let vault = account("vault", 0, 0);
        assert_ok!(Currencies::deposit(NATIVE, &vault, 1_000_000_000_000u128));
        let mut accounts: Vec<(AccountId,u32)> = vec![];
        for targer in 0..a {
            let targer_account: AccountId = account("targer", 0, targer);
            accounts.push((targer_account.clone(),100));
            let _ = <ZdToken as MultiBaseToken<_,_>>::transfer_social(&vault.clone(), &targer_account.clone(), 10_000);
            for trustee in 1..MAX_TRUST_COUNT {
                let trustee_account: AccountId = account("trustee", targer, trustee);
                checked_trust(&targer_account,&trustee_account);
            }
        }
        assert_ok!(ZdReputation::new_round());
        ZdReputation::set_step(&TIRStep::Reputation);
        let caller: AccountId = whitelisted_caller();
        let _ = Currencies::deposit(NATIVE, &caller, 1_000_000_000_000u128)?;
        System::set_block_number(2000);
        assert_ok!(ZdRefreshReputation::start(RawOrigin::Signed(vault.clone()).into()));
    }: _(RawOrigin::Signed(caller.clone()),accounts)

    harvest_ref_all {
        let pathfinder: AccountId = account("pathfinder", 0, 0);
        init_harvest(&pathfinder);
        System::set_block_number(2000);
    }: _(RawOrigin::Signed(pathfinder.clone()))

    harvest_ref_all_sweeper {
        let pathfinder: AccountId = account("pathfinder", 0, 0);
        init_harvest(&pathfinder);
        System::set_block_number(2000);
        let sweeper: AccountId = account("sweeper", 0, 0);
    }: _(RawOrigin::Signed(sweeper.clone()),pathfinder)

    challenge {
        let challenger = account("challenger", 0, 0);
        let targer: AccountId = account("targer", 0, 0);
        init_challenge(&challenger,&targer,2);
        for s in 1..2 {
            let new_seed: AccountId = account("seed", 0, s);
            <ZdSeeds as SeedsBase<_>>::add_seed(&new_seed);
        }
        let pathfinder: AccountId = account("pathfinder", 0, 0);
    }: _(RawOrigin::Signed(challenger.clone()),targer.clone(),pathfinder,1,2)

    challenge_update {
        let a in 1 .. MAX_SEED_COUNT;
        let challenger = account("challenger", 0, 0);
        let total_score = a * 2;

        let targer: AccountId = account("targer", 0, 0);
        init_challenge(&challenger,&targer,total_score);

        let mut seeds: Vec<AccountId> = vec![];
        let mut paths: Vec<Path<AccountId>> = vec![];

        for b in 1..a {
            let new_seed: AccountId = account("seed", 0, b);
            seeds.push(new_seed.clone());
            <ZdSeeds as SeedsBase<_>>::add_seed(&new_seed);
            let nodes = (3..MAX_NODE_COUNT)
                .map(|c|account("challenger", b, c))
                .collect::<Vec<AccountId>>();
            paths.push(Path {
                nodes,
                score: a * 2,
            })
        }

        let vault = account("vault", 0, 0);
        Currencies::deposit(NATIVE, &vault, 1_000_000_000_000u128)?;
        assert_ok!(ZdToken::staking(&vault, &1_000_000_000u128));

        let pathfinder: AccountId = account("pathfinder", 0, 0);
        let _ = ZdRefreshReputation::challenge(RawOrigin::Signed(challenger.clone()).into(),targer.clone(),pathfinder,a - 1,2)?;
    }: _(RawOrigin::Signed(challenger.clone()),targer.clone(),seeds,paths)

    harvest_challenge {
        let challenger = account("challenger", 0, 0);
        let targer: AccountId = account("targer", 0, 0);
        let pathfinder: AccountId = account("pathfinder", 0, 0);

        init_challenge(&challenger,&targer,2);
        for s in 1..2 {
            let new_seed: AccountId = account("seed", 0, s);
            <ZdSeeds as SeedsBase<_>>::add_seed(&new_seed);
        }
        let vault = account("vault", 0, 0);
        Currencies::deposit(NATIVE, &vault, 1_000_000_000_000u128)?;
        assert_ok!(ZdToken::staking(&vault, &1_000_000_000u128));
        let _ = ZdRefreshReputation::challenge(RawOrigin::Signed(challenger.clone()).into(),targer.clone(),pathfinder,1,2)?;
        System::set_block_number(2000);
        let sweeper: AccountId = account("sweeper", 0, 0);
    }: _(RawOrigin::Signed(sweeper.clone()),targer.clone())

    arbitral {
        let a in 0 .. MAX_SEED_COUNT;

        let challenger = account("challenger", 0, 0);
        let targer: AccountId = account("targer", 0, 0);
        let pathfinder: AccountId = account("pathfinder", 0, 0);

        let mut seeds: Vec<AccountId> = vec![];
        let mut paths: Vec<Path<AccountId>> = vec![];

        for b in 1..(a + 2) {
            let seed: AccountId = account("seed", 0, b);
            <ZdSeeds as SeedsBase<_>>::add_seed(&seed);
            seeds.push(seed.clone());
            let first_node: AccountId = account("node", 0, 1);
            let mut nodes: Vec<AccountId> = vec![first_node.clone()];
            checked_trust(&seed,&first_node);
            for c in 2..(MAX_NODE_COUNT - 1) {
                let source_account: AccountId = account("node", 0, c - 1);
                let target_account: AccountId = account("node", 0, c);
                nodes.push(target_account.clone());
                checked_trust(&source_account,&target_account);
            }
            let last_node: AccountId = account("node", 0, MAX_NODE_COUNT - 2);
            checked_trust(&last_node,&targer);

            let mut path = nodes.clone();
            path.insert(0, seed.clone());
            path.push(targer.clone());

            let (_,score) = <ZdTrust as TrustBase<_>>::computed_path(&path)?;
            paths.push(Path {
                nodes,
                score
            });
        }

        init_challenge(&challenger,&targer,2);
        let _ = ZdRefreshReputation::challenge(RawOrigin::Signed(challenger.clone()).into(),targer.clone(),pathfinder,1,2)?;
        // RawOrigin::Signed(challenger.clone()),targer.clone(),seeds,paths
        let seed_1: AccountId = account("seed", 0, 1);
        let nodes_1: Vec<AccountId> = vec![
            account("wrong", 0, 1),
            account("wrong", 0, 2),
        ];
        let path_1 = Path {
            nodes: nodes_1,
            score: 1,
        };
        let _ = ZdRefreshReputation::challenge_update(RawOrigin::Signed(challenger.clone()).into(),targer.clone(),vec![seed_1],vec![path_1])?;
        let who: AccountId = account("who", 0, 1);
        let _ = Currencies::deposit(NATIVE, &who, 1_000_000_000_000u128);
        System::set_block_number(2000);
    }: _(RawOrigin::Signed(who.clone()),targer.clone(),seeds,paths)

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::benchmarking::utils::tests::new_test_ext;
    use orml_benchmarking::impl_benchmark_test_suite;

    impl_benchmark_test_suite!(new_test_ext(),);
}
