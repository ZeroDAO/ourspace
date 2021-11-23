#[warn(unused_imports)]
use crate::{
    AccountId, Currencies, CurrencyId, GetNativeCurrencyId, MaxSeedCount, Runtime, System,
    ZdRefreshSeeds, ZdTrust,
};
use frame_benchmarking::{account, whitelisted_caller};
use frame_system::RawOrigin;
use sp_std::prelude::*;
use zd_refresh_seeds::{OrderedSet, Path, PostResultHash, ResultHash, DEEP, MAX_HASH_COUNT, RANGE};

use orml_benchmarking::runtime_benchmarks;
use orml_traits::MultiCurrency;
use sp_runtime::DispatchError;

const NATIVE: CurrencyId = GetNativeCurrencyId::get();
const MAX_SEED_COUNT: u32 = MaxSeedCount::get();

fn init_challenge(score: u64) {
    System::set_block_number(2000);
    let pathfinder: AccountId = account("pathfinder", 0, 0);
    let target: AccountId = account("target", 0, 0);
    System::set_block_number(2000);
    let _ = Currencies::deposit(NATIVE, &pathfinder, 1_000_000_000_000u128);
    let _ = ZdRefreshSeeds::start(RawOrigin::Signed(pathfinder.clone()).into());
    let _ = ZdRefreshSeeds::add(RawOrigin::Signed(pathfinder.clone()).into(), target, score);
}

fn do_challenge(pathfinder_score: u64, challenger_score: u64) {
    init_challenge(pathfinder_score);
    let challenger: AccountId = account("challenger", 0, 0);
    let target: AccountId = account("target", 0, 0);
    let _ = Currencies::deposit(NATIVE, &challenger, 1_000_000_000_000u128);
    let _ = ZdRefreshSeeds::challenge(
        RawOrigin::Signed(challenger.clone()).into(),
        target,
        challenger_score,
    );
}

fn do_reply_hash(a: u32, does_examine: bool) -> Result<Vec<PostResultHash>, DispatchError> {
    let pathfinder: AccountId = account("pathfinder", 0, 0);
    let target: AccountId = account("target", 0, 0);
    let challenger: AccountId = account("challenger", 0, 0);

    let mut post_hash_vec: Vec<PostResultHash> = vec![];
    let index = (a / 2) + 1;
    let score: u64 = 10u64;
    for post_id_u32 in 0..a {
        let post_id = post_id_u32 as u8;
        post_hash_vec.push(PostResultHash([post_id; RANGE], score));
    }
    let mut r_hash_vec: Vec<ResultHash> = vec![];
    for id in 0..MAX_HASH_COUNT {
        r_hash_vec.push(ResultHash {
            order: [id as u8; RANGE],
            score: score,
        });
    }
    do_challenge(score * (post_hash_vec.len() as u64), 100u64);
    let _ = ZdRefreshSeeds::reply_hash(
        RawOrigin::Signed(pathfinder.clone()).into(),
        target.clone(),
        post_hash_vec.clone(),
        a,
    );
    for i in 2..DEEP {
        if i == DEEP - 1 {
            r_hash_vec[index as usize].score = score * (post_hash_vec.len() as u64);
        }

        let r_hash_set = OrderedSet::from(r_hash_vec.clone());
        ZdRefreshSeeds::insert_hash(&target, r_hash_set)?;
    }
    if does_examine {
        let _ = ZdRefreshSeeds::examine(
            RawOrigin::Signed(challenger.clone()).into(),
            target.clone(),
            index,
        );
    }
    Ok(post_hash_vec)
}

fn do_reply_path(a: u32, does_examine: bool) -> Result<Vec<Path<AccountId>>, DispatchError> {
    let pathfinder: AccountId = account("pathfinder", 0, 0);
    let challenger: AccountId = account("challenger", 0, 0);

    let mut paths: Vec<Path<_>> = vec![];

    let start_node: AccountId = account("start", 0, 0);
    let end_node: AccountId = account("end", 0, 0);
    let target: AccountId = account("target", 0, 0);

    for i in 1..(a + 1) {
        let crossed: AccountId = account("node", 0, i);
        let _ = ZdTrust::trust(
            RawOrigin::Signed(start_node.clone()).into(),
            crossed.clone().into(),
        );
        let _ = ZdTrust::trust(
            RawOrigin::Signed(crossed.clone()).into(),
            target.clone().into(),
        );
        paths.push(Path {
            nodes: vec![
                start_node.clone(),
                crossed.clone(),
                target.clone(),
                end_node.clone(),
            ],
            total: a,
        })
    }

    let _ = ZdTrust::trust(
        RawOrigin::Signed(target.clone()).into(),
        end_node.clone().into(),
    );

    paths.sort();

    let full_order =
        ZdRefreshSeeds::make_full_order(&start_node.clone(), &end_node.clone(), DEEP as usize);

    // let mut posts: Vec<PostResultHash> = vec![];
    let score = (100 / a) as u64;
    let total_score = score * (a as u64);

    let posts = (1..(DEEP + 1))
        .map(|d| {
            let mut order = [0u8; RANGE];
            order.copy_from_slice(&full_order[((d - 1) as usize * RANGE)..(d as usize * RANGE)]);
            PostResultHash(order, total_score)
        })
        .collect::<Vec<PostResultHash>>();

    do_challenge(total_score, 88u64);

    for (index, post_r_hash) in posts.iter().enumerate() {
        if index != 0 {
            let _ = ZdRefreshSeeds::examine(
                RawOrigin::Signed(challenger.clone()).into(),
                target.clone(),
                0,
            );
        }

        let _ = ZdRefreshSeeds::reply_hash(
            RawOrigin::Signed(pathfinder.clone()).into(),
            target.clone(),
            vec![post_r_hash.clone()],
            1,
        );
    }

    if does_examine {
        let _ = ZdRefreshSeeds::examine(
            RawOrigin::Signed(challenger.clone()).into(),
            target.clone(),
            0,
        );
    }

    Ok(paths)
}

runtime_benchmarks! {
    { Runtime, zd_refresh_seeds }

    _ {}

    start {
        System::set_block_number(2000);
        let caller: AccountId = whitelisted_caller();
        Currencies::deposit(NATIVE, &caller, 1_000_000_000_000u128)?;
    }: _(RawOrigin::Signed(caller.clone()))

    add {
        System::set_block_number(2000);
        let pathfinder: AccountId = account("pathfinder", 0, 0);
        let target: AccountId = account("target", 0, 0);
        System::set_block_number(2000);
        Currencies::deposit(NATIVE, &pathfinder, 1_000_000_000_000u128)?;
        ZdRefreshSeeds::start(RawOrigin::Signed(pathfinder.clone()).into())?;
    }: _(RawOrigin::Signed(pathfinder.clone()),target,100)

    challenge {
        init_challenge(100u64);
        let challenger: AccountId = account("challenger", 0, 0);
        let target: AccountId = account("target", 0, 0);
        Currencies::deposit(NATIVE, &challenger, 1_000_000_000_000u128)?;
    }: _(RawOrigin::Signed(challenger.clone()),target,22)

    examine {
        let pathfinder: AccountId = account("pathfinder", 0, 0);
        let target: AccountId = account("target", 0, 0);
        let challenger: AccountId = account("challenger", 0, 0);

        let a = MAX_HASH_COUNT - 1;
        let post_hash_vec = do_reply_hash(a,true)?;
        ZdRefreshSeeds::reply_hash(RawOrigin::Signed(pathfinder.clone()).into(),target.clone(),post_hash_vec.clone(),a)?;
    }: _(RawOrigin::Signed(challenger.clone()),target.clone(),10)

    // Case where path verification is required
    reply_hash {
        let pathfinder: AccountId = account("pathfinder", 0, 0);
        let target: AccountId = account("target", 0, 0);
        let challenger: AccountId = account("challenger", 0, 0);

        let a in 1 .. MAX_HASH_COUNT;
        let post_hash_vec = do_reply_hash(a,true)?;
    }: _(RawOrigin::Signed(pathfinder.clone()),target,post_hash_vec,a)

    reply_hash_next {
        let pathfinder: AccountId = account("pathfinder", 0, 0);
        let target: AccountId = account("target", 0, 0);
        let challenger: AccountId = account("challenger", 0, 0);

        let a in 2 .. MAX_HASH_COUNT;
        let post_hash_vec = do_reply_hash(a,true)?;
        ZdRefreshSeeds::reply_hash(RawOrigin::Signed(pathfinder.clone()).into(),target.clone(),post_hash_vec[..1].to_vec(),a)?;
    }: _(RawOrigin::Signed(pathfinder.clone()),target,post_hash_vec[1..].to_vec())

    reply_path {
        let a in 1 .. 99;

        let pathfinder: AccountId = account("pathfinder", 0, 0);
        let target: AccountId = account("target", 0, 0);
        let paths = do_reply_path(a,true)?;

    }: _(RawOrigin::Signed(pathfinder.clone()),target,paths,a)

    reply_path_next {
        let a in 2 .. 99;

        let pathfinder: AccountId = account("pathfinder", 0, 0);
        let target: AccountId = account("target", 0, 0);
        let paths = do_reply_path(a,true)?;

        ZdRefreshSeeds::reply_path(RawOrigin::Signed(pathfinder.clone()).into(),target.clone(),paths[..1].to_vec(),a)?;

    }: _(RawOrigin::Signed(pathfinder.clone()),target,paths[1..].to_vec())

    reply_num {
        let a in 2 .. 99;

        let pathfinder: AccountId = account("pathfinder", 0, 0);
        let target: AccountId = account("target", 0, 0);
        let challenger: AccountId = account("challenger", 0, 0);

        let paths = do_reply_path(a,true)?;

        let mid_paths = paths.iter()
            .map(|path| path.nodes[1..(path.nodes.len() - 1)].to_vec())
            .collect::<Vec<Vec<AccountId>>>();

        ZdRefreshSeeds::reply_path(RawOrigin::Signed(pathfinder.clone()).into(),target.clone(),paths,a)?;

        ZdRefreshSeeds::examine(
            RawOrigin::Signed(challenger.clone()).into(),
            target.clone(),
            0,
        )?;

    }: _(RawOrigin::Signed(pathfinder.clone()),target,mid_paths)

    evidence_of_shorter {
        let count = 10u32;

        let pathfinder: AccountId = account("pathfinder", 0, 0);
        let target: AccountId = account("target", 0, 0);
        let start_node: AccountId = account("start", 0, 0);
        let end_node: AccountId = account("end", 0, 0);
        let mid_node: AccountId = account("mid_node", 0, 0);
        let challenger: AccountId = account("challenger", 0, 0);

        ZdTrust::trust(
            RawOrigin::Signed(start_node.clone()).into(),
            mid_node.clone().into(),
        )?;

        ZdTrust::trust(
            RawOrigin::Signed(mid_node.clone()).into(),
            end_node.clone().into(),
        )?;

        let paths = do_reply_path(count,true)?;

        ZdRefreshSeeds::reply_path(RawOrigin::Signed(pathfinder.clone()).into(),target.clone(),paths,count)?;

    }: _(RawOrigin::Signed(challenger.clone()),target,0,vec![
        mid_node
    ])

    number_too_low {
        let a in 2 .. 99;

        let pathfinder: AccountId = account("pathfinder", 0, 0);
        let target: AccountId = account("target", 0, 0);
        let start_node: AccountId = account("start", 0, 0);
        let mid_node: AccountId = account("mid_node", 0, 0);
        let challenger: AccountId = account("challenger", 0, 0);

        ZdTrust::trust(
            RawOrigin::Signed(start_node.clone()).into(),
            mid_node.clone().into(),
        )?;

        ZdTrust::trust(
            RawOrigin::Signed(mid_node.clone()).into(),
            target.clone().into(),
        )?;

        let paths = do_reply_path(a - 1,true)?;

        let mut mid_paths = paths.iter()
            .map(|path| path.nodes[1..(path.nodes.len() - 1)].to_vec())
            .collect::<Vec<Vec<AccountId>>>();

        mid_paths.push(vec![
            mid_node,
            target.clone()
        ]);

        ZdRefreshSeeds::reply_path(RawOrigin::Signed(pathfinder.clone()).into(),target.clone(),paths,a - 1)?;
    }: _(RawOrigin::Signed(challenger.clone()),target,0,mid_paths)

    missed_in_hashs {
        let pathfinder: AccountId = account("pathfinder", 0, 0);
        let target: AccountId = account("target", 0, 0);
        let challenger: AccountId = account("challenger", 0, 0);

        let mock_start: AccountId = account("mock_start", 0, 0);
        let mock_end: AccountId = account("mock_end", 0, 0);

        ZdTrust::trust(
            RawOrigin::Signed(mock_start.clone()).into(),
            target.clone().into(),
        )?;

        ZdTrust::trust(
            RawOrigin::Signed(target.clone()).into(),
            mock_end.clone().into(),
        )?;

        let full_order = ZdRefreshSeeds::make_full_order(
            &mock_start,
            &mock_end,
            1
        );

        let index = full_order[0];
        let mut post_hash_vec: Vec<PostResultHash> = vec![];
        let score: u64 = 10u64;
        let count = MAX_HASH_COUNT;

        for post_id_u32 in 0..count {
            if post_id_u32 != (index as u32) {
                let post_id = post_id_u32 as u8;
                post_hash_vec.push(PostResultHash([post_id,0u8], score));
            }
        }

        do_challenge(score * (post_hash_vec.len() as u64), 100u64);
        ZdRefreshSeeds::reply_hash(
            RawOrigin::Signed(pathfinder.clone()).into(),
            target.clone(),
            post_hash_vec.clone(),
            count - 1,
        )?;

    }: _(RawOrigin::Signed(challenger.clone()),target.clone(),vec![
        mock_start,
        target,
        mock_end
    ], index as u32)

    missed_in_paths {
        let pathfinder: AccountId = account("pathfinder", 0, 0);
        let target: AccountId = account("target", 0, 0);
        let challenger: AccountId = account("challenger", 0, 0);

        let start_node: AccountId = account("start", 0, 0);
        let end_node: AccountId = account("end", 0, 0);
        let mid_node: AccountId = account("mid_node", 0, 0);

        ZdTrust::trust(
            RawOrigin::Signed(start_node.clone()).into(),
            mid_node.clone().into(),
        )?;

        ZdTrust::trust(
            RawOrigin::Signed(mid_node.clone()).into(),
            target.clone().into(),
        )?;
        let count = 99u32;
        let paths = do_reply_path(count,true)?;
        ZdRefreshSeeds::reply_path(RawOrigin::Signed(pathfinder.clone()).into(),target.clone(),paths,count)?;
    }: _(RawOrigin::Signed(challenger.clone()),target.clone(),vec![
        start_node,
        mid_node,
        target,
        end_node
    ])

    invalid_evidence {
        let pathfinder: AccountId = account("pathfinder", 0, 0);
        let target: AccountId = account("target", 0, 0);
        let challenger: AccountId = account("challenger", 0, 0);

        let mock_start: AccountId = account("mock_start", 0, 0);
        let mock_end: AccountId = account("mock_end", 0, 0);

        ZdTrust::trust(
            RawOrigin::Signed(mock_start.clone()).into(),
            target.clone().into(),
        )?;

        ZdTrust::trust(
            RawOrigin::Signed(target.clone()).into(),
            mock_end.clone().into(),
        )?;

        ZdTrust::trust(
            RawOrigin::Signed(mock_start.clone()).into(),
            mock_end.clone().into(),
        )?;

        let score = 10u64;

        do_challenge(score, 100u64);

        ZdRefreshSeeds::reply_hash(
            RawOrigin::Signed(pathfinder.clone()).into(),
            target.clone(),
            vec![
                PostResultHash([0u8,0u8], score)
            ],
            1,
        )?;

        ZdRefreshSeeds::missed_in_hashs(RawOrigin::Signed(challenger.clone()).into(),target.clone(),vec![
            mock_start,
            target.clone(),
            mock_end
        ],1)?;

    }: _(RawOrigin::Signed(challenger.clone()),target.clone(),vec![],6000u64)

    harvest_challenge {
        let target: AccountId = account("target", 0, 0);
        do_challenge(200u64, 100u64);
        System::set_block_number(4000);
        let sweeper: AccountId = account("sweeper", 0, 0);
    }: _(RawOrigin::Signed(sweeper.clone()),target.clone())

    harvest_seed {
        System::set_block_number(2000);

        let pathfinder: AccountId = account("pathfinder", 0, 0);
        let target: AccountId = account("target", 0, 0);

        Currencies::deposit(NATIVE, &pathfinder, 1_000_000_000_000u128)?;
        ZdRefreshSeeds::start(RawOrigin::Signed(pathfinder.clone()).into())?;
        ZdRefreshSeeds::add(RawOrigin::Signed(pathfinder.clone()).into(), target.clone(), 1000u64)?;

        for i in 0..(MAX_SEED_COUNT + 10) {
            let other_target: AccountId = account("other_target", 0, i);
            ZdRefreshSeeds::add(RawOrigin::Signed(pathfinder.clone()).into(), other_target, (100u32 + i) as u64)?;
        }

        System::set_block_number(4000);
        let sweeper: AccountId = account("sweeper", 0, 0);
    }: _(RawOrigin::Signed(sweeper.clone()),target.clone())

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::benchmarking::utils::tests::new_test_ext;
    use orml_benchmarking::impl_benchmark_test_suite;

    impl_benchmark_test_suite!(new_test_ext(),);
}
