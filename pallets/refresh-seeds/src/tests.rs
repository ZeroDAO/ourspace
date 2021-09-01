#![cfg(test)]

extern crate time;
use time::*;

use super::*;
use crate::mock::{Event, *};
use frame_support::{assert_noop, assert_ok};

fn new_test_ext() -> sp_io::TestExternalities {
    let mut ext = ExtBuilder::default().build();
    ext.execute_with(|| System::set_block_number(1));
    ext
}

fn init_graph() {
    //            Construct a graph
    //
    //                     B
    //                 ↗  ↓  ↘
    //               A     E <-  D
    //                 ↘     ↗
    //                     C
    //
    // The shortest path through B
    // +-------+-------+-------+-------------------+
    // | Path  | total | score | sha1(start,stop)  |
    // +-------+-------+-------+-------------------+
    // |  ABD  |   2   | 100/2 |    ...f9906cf1    |
    // +-------+-------+-------+-------------------+
    // |  ABE  |   1   | 100/1 |    ...7cfe0266    |
    // +-------+-----------------------------------+
    // | total |         150                       |
    // +-------+-----------------------------------+
    //
    // hash: [AccountId,AccountId,total;...AccountId,AccountId,total;]
    // Path hash:
    // +-----------------------+------------------------------------------+
    // |  "0001,0002,0004,2;"  | a0e8df2a2f413bb7f3339c66130b770debb57796 |
    // +-----------------------+------------------------------------------+
    // |  "0001,0002,0005,1;"  | b339911bcb3a3080a2b6fcbd033facd968aecc4c |
    // +-----------------------+------------------------------------------+
    //
    // Deep 4:
    // +---------+--------------+--------+
    // |  order  | hash         | score  |
    // +---------+--------------+--------+
    // |   f1    | a0e8df2a     |  50    |
    // +---------+--------------+--------+
    // |   66    | b339911b     |  100   |
    // +---------+--------------+--------+
    //
    // Deep 3:
    // +---------+--------------+--------+
    // |  order  | hash         | score  |
    // +---------+--------------+--------+
    // |   6c    | 43eb70aa     |  50    |
    // +---------+--------------+--------+
    // |   02    | 3fd7de1d     |  100   |
    // +---------+--------------+--------+
    //
    // Deep 2:
    // +---------+--------------+--------+
    // |  order  | hash         | score  |
    // +---------+--------------+--------+
    // |   90    | c248b273     |  50    |
    // +---------+--------------+--------+
    // |   fe    | 5757fc60     |  100   |
    // +---------+--------------+--------+
    //
    // Deep 1:
    // +---------+--------------+--------+
    // |  order  | hash         | score  |
    // +---------+--------------+--------+
    // |   f9    | b3b4e091     |  50    |
    // +---------+--------------+--------+
    // |   7c    | 781bbaf6     |  100   |
    // +---------+--------------+--------+

    let paths = vec![[A, B], [A, C], [B, D], [B, E], [D, E], [C, D]];
    for path in paths {
        assert_ok!(ZdTrust::trust(Origin::signed(path[0]), path[1]));
    }
    assert_ok!(ZdRefreshSeeds::start(Origin::signed(PATHFINDER)));
    assert_ok!(ZdRefreshSeeds::add(Origin::signed(PATHFINDER), B, 150));
}

#[test]
fn start_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(ZdRefreshSeeds::start(Origin::signed(PATHFINDER),));
    });
}

#[test]
fn add_should_work() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            ZdRefreshSeeds::add(Origin::signed(PATHFINDER), A, 60),
            Error::<Test>::StepNotMatch
        );
        let free_balance = ZdToken::free_balance(&PATHFINDER);
        assert_ok!(ZdRefreshSeeds::add(Origin::signed(PATHFINDER), A, 60));

        let new_free_balance = ZdToken::free_balance(&PATHFINDER);
        assert_eq!(free_balance - new_free_balance, SeedStakingAmount::get());

        assert_eq!(
            Candidate {
                score: 60,
                pathfinder: PATHFINDER,
                has_challenge: false,
                add_at: 1,
            },
            <Candidates<Test>>::get(A)
        );

        assert_noop!(
            ZdRefreshSeeds::add(Origin::signed(PATHFINDER), A, 60),
            Error::<Test>::AlreadyExist
        );
    });
}

#[test]
fn challenge_should_work() {
    new_test_ext().execute_with(|| {
        init_graph();
        assert_ok!(ZdRefreshSeeds::challenge(Origin::signed(CHALLENGER), B, 50,));
        assert!(<Candidates<Test>>::get(A).has_challenge);
        assert_noop!(
            ZdRefreshSeeds::challenge(Origin::signed(CHALLENGER), B, 50,),
            Error::<Test>::AlreadyChallenged
        );
        assert_noop!(
            ZdRefreshSeeds::challenge(Origin::signed(CHALLENGER), A, 50,),
            Error::<Test>::NoCandidateExists
        );
    });
}

#[test]
fn reply_hash_should_work() {
    new_test_ext().execute_with(|| {
        init_graph();
        assert_ok!(ZdRefreshSeeds::challenge(
            Origin::signed(CHALLENGER),
            B,
            150,
        ));
        // Deep 1:
        // +---------+--------------+--------+
        // |  order  | hash         | score  |
        // +---------+--------------+--------+
        // |   f9    | b3b4e091     |  50    |
        // +---------+--------------+--------+
        // |   7c    | 781bbaf6     |  100   |
        // +---------+--------------+--------+
        assert_ok!(ZdRefreshSeeds::reply_hash(
            Origin::signed(PATHFINDER),
            B,
            vec![PostResultHash("f9".to_string(), 50, "b3b4e091".to_string())],
            2,
        ));
        assert_ok!(ZdRefreshSeeds::reply_hash_next(
            Origin::signed(PATHFINDER),
            B,
            vec![PostResultHash(
                "7c".to_string(),
                100,
                "781bbaf6".to_string()
            )],
        ));
        assert_ok!(ZdRefreshSeeds::examine(Origin::signed(CHALLENGER), B, 1,));
        // Deep 2:
        // +---------+--------------+--------+
        // |  order  | hash         | score  |
        // +---------+--------------+--------+
        // |   90    | c248b273     |  50    |
        // +---------+--------------+--------+
        assert_ok!(ZdRefreshSeeds::reply_hash(
            Origin::signed(PATHFINDER),
            B,
            vec![PostResultHash("90".to_string(), 50, "c248b273".to_string())],
            1,
        ));
        assert_ok!(ZdRefreshSeeds::examine(Origin::signed(CHALLENGER), B, 0,));
        // Deep 3:
        // +---------+--------------+--------+
        // |  order  | hash         | score  |
        // +---------+--------------+--------+
        // |   6c    | 43eb70aa     |  50    |
        // +---------+--------------+--------+
        assert_ok!(ZdRefreshSeeds::reply_hash(
            Origin::signed(PATHFINDER),
            B,
            vec![PostResultHash("6c".to_string(), 50, "43eb70aa".to_string())],
            1,
        ));
        assert_ok!(ZdRefreshSeeds::examine(Origin::signed(CHALLENGER), B, 0,));
        // Deep 4:
        // +---------+--------------+--------+
        // |  order  | hash         | score  |
        // +---------+--------------+--------+
        // |   f1    | a0e8df2a     |  50    |
        // +---------+--------------+--------+
        assert_ok!(ZdRefreshSeeds::reply_hash(
            Origin::signed(PATHFINDER),
            B,
            vec![PostResultHash("f1".to_string(), 50, "a0e8df2a".to_string())],
            1,
        ));
        assert_ok!(ZdRefreshSeeds::examine(Origin::signed(CHALLENGER), B, 0,));
        // Path hash:
        // +-----------------------+------------------------------------------+
        // |  "0001,0002,0004,2;"  | a0e8df2a2f413bb7f3339c66130b770debb57796 |
        // +-----------------------+------------------------------------------+
        // |  "0001,0002,0005,1;"  | b339911bcb3a3080a2b6fcbd033facd968aecc4c |
        // +-----------------------+------------------------------------------+
        assert_ok!(ZdRefreshSeeds::reply_path(
            Origin::signed(PATHFINDER),
            B,
            vec![Path {
                nodes: vec![A, B, D],
                total: 2
            }],
            1,
        ));


    });
}