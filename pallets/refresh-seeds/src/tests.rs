#![cfg(test)]

// extern crate time;

use super::*;
use crate::mock::{Event, *};
use frame_support::{assert_noop, assert_ok};

fn new_test_ext() -> sp_io::TestExternalities {
    let mut ext = ExtBuilder::default().build();
    ext.execute_with(|| System::set_block_number(1));
    ext
}

fn init_graph(score: u64) {
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
    assert_ok!(ZdRefreshSeeds::add(Origin::signed(PATHFINDER), B, score));
}

#[test]
fn start_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(ZdRefreshSeeds::start(Origin::signed(PATHFINDER),));
        let seeds_event = Event::zd_refresh_seeds(crate::Event::RefershSeedStared(PATHFINDER));
        assert!(System::events()
            .iter()
            .any(|record| record.event == seeds_event));
    });
}

#[test]
fn add_should_work() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            ZdRefreshSeeds::add(Origin::signed(PATHFINDER), A, 60),
            Error::<Test>::StepNotMatch
        );
        assert_ok!(ZdRefreshSeeds::start(Origin::signed(PATHFINDER),));
        let free_balance = ZdToken::free_balance(&PATHFINDER);

        assert_ok!(ZdRefreshSeeds::add(Origin::signed(PATHFINDER), A, 60));

        let add_event = Event::zd_refresh_seeds(crate::Event::NewCandidate(PATHFINDER, A, 60));
        assert!(System::events()
            .iter()
            .any(|record| record.event == add_event));

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
        init_graph(150);
        assert_ok!(ZdRefreshSeeds::challenge(Origin::signed(CHALLENGER), B, 50,));

        let event = Event::zd_refresh_seeds(crate::Event::NewChallenge(CHALLENGER, B));
        assert!(System::events().iter().any(|record| record.event == event));

        assert!(<Candidates<Test>>::get(B).has_challenge);
        assert!(ZdRefreshSeeds::challenge(Origin::signed(CHALLENGER), B, 50,).is_err());
        assert_noop!(
            ZdRefreshSeeds::challenge(Origin::signed(CHALLENGER), A, 50,),
            Error::<Test>::NoCandidateExists
        );
        assert_ok!(ZdRefreshSeeds::add(Origin::signed(PATHFINDER), C, 12));
        System::set_block_number(ConfirmationPeriod::get() + 1);
        assert_noop!(
            ZdRefreshSeeds::challenge(Origin::signed(CHALLENGER), C, 50,),
            Error::<Test>::SeedAlreadyConfirmed
        );
    });
}

fn init_challenge(total_bonus: &Balance) {
    assert_ok!(ZdRefreshSeeds::add(Origin::signed(PATHFINDER), D, 50));
    assert_ok!(ZdRefreshSeeds::add(Origin::signed(PATHFINDER), C, 100));
    assert_ok!(ZdToken::increase_bonus(&TREASURY, total_bonus));

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
    let event = Event::zd_refresh_seeds(crate::Event::RepliedHash(PATHFINDER, B, 2, false));
    assert!(System::events().iter().any(|record| record.event == event));

    assert_ok!(ZdRefreshSeeds::reply_hash_next(
        Origin::signed(PATHFINDER),
        B,
        vec![PostResultHash(
            "7c".to_string(),
            100,
            "781bbaf6".to_string()
        )],
    ));
    let event = Event::zd_refresh_seeds(crate::Event::ContinueRepliedHash(PATHFINDER, B, true));
    assert!(System::events().iter().any(|record| record.event == event));

    assert_ok!(ZdRefreshSeeds::examine(Origin::signed(CHALLENGER), B, 1,));
    let event = Event::zd_refresh_seeds(crate::Event::NewExamine(CHALLENGER, B));
    assert!(System::events().iter().any(|record| record.event == event));

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

    let event = Event::zd_refresh_seeds(crate::Event::RepliedHash(PATHFINDER, B, 1, true));
    assert!(System::events().iter().any(|record| record.event == event));

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
}

#[test]
fn pathfinder_win() {
    new_test_ext().execute_with(|| {
        init_graph(150);
        let total_bonus: Balance = 991;
        init_challenge(&total_bonus);

        assert_ok!(ZdRefreshSeeds::examine(Origin::signed(CHALLENGER), B, 0,));

        assert!(ZdRefreshSeeds::challenge(Origin::signed(CHALLENGER), B, 50,).is_err());
        // Path hash:
        // +-----------------------+------------------------------------------+
        // |  "0001,0002,0004,2;"  | a0e8df2a2f413bb7f3339c66130b770debb57796 |
        // +-----------------------+------------------------------------------+
        // |  "0001,0002,0005,1;"  | b339911bcb3a3080a2b6fcbd033facd968aecc4c |
        // +-----------------------+------------------------------------------+
        assert_noop!(
            ZdRefreshSeeds::reply_path(Origin::signed(PATHFINDER), B, vec![], 1,),
            Error::<Test>::NoPath
        );

        assert_noop!(
            ZdRefreshSeeds::reply_path(
                Origin::signed(PATHFINDER),
                B,
                vec![
                    Path {
                        nodes: vec![A, B, D],
                        total: 2
                    },
                    Path {
                        nodes: vec![A, B, D],
                        total: 2
                    }
                ],
                2,
            ),
            Error::<Test>::NotMatch
        );

        assert_ok!(ZdRefreshSeeds::reply_path(
            Origin::signed(PATHFINDER),
            B,
            vec![Path {
                nodes: vec![A, B, D],
                total: 2
            }],
            1,
        ));
        let event = Event::zd_refresh_seeds(crate::Event::RepliedPath(PATHFINDER, B, 1, true));
        assert!(System::events().iter().any(|record| record.event == event));

        assert!(ZdRefreshSeeds::challenge(Origin::signed(CHALLENGER), B, 50,).is_err());

        assert_noop!(
            ZdRefreshSeeds::examine(Origin::signed(CHALLENGER), B, 5,),
            Error::<Test>::IndexExceedsMaximum
        );

        assert_ok!(ZdRefreshSeeds::examine(Origin::signed(CHALLENGER), B, 0,));

        assert!(ZdRefreshSeeds::challenge(Origin::signed(CHALLENGER), B, 50,).is_err());

        assert_noop!(
            ZdRefreshSeeds::reply_hash(
                Origin::signed(PATHFINDER),
                B,
                vec![
                    PostResultHash("f9".to_string(), 50, "b3b4e091".to_string()),
                    PostResultHash("7c".to_string(), 100, "781bbaf6".to_string())
                ],
                2,
            ),
            Error::<Test>::MaximumDepth
        );

        assert_noop!(
            ZdRefreshSeeds::reply_num(
                Origin::signed(PATHFINDER),
                B,
                vec![
                    vec![B], // ABD
                    vec![C],
                    vec![E], // ACD
                ],
            ),
            Error::<Test>::LengthNotEqual
        );

        assert_noop!(
            ZdRefreshSeeds::reply_num(
                Origin::signed(PATHFINDER),
                B,
                vec![
                    vec![B], // ABD
                    vec![B],
                ],
            ),
            Error::<Test>::NotMatch
        );

        assert_ok!(ZdRefreshSeeds::reply_num(
            Origin::signed(PATHFINDER),
            B,
            vec![
                vec![B], // ABD
                vec![C], // ACD
            ],
        ));

        assert_noop!(
            ZdRefreshSeeds::examine(Origin::signed(CHALLENGER), B, 5,),
            Error::<Test>::IndexExceedsMaximum
        );

        assert!(ZdRefreshSeeds::challenge(Origin::signed(CHALLENGER), B, 50,).is_err());

        assert!(ZdRefreshSeeds::harvest_challenge(Origin::signed(PATHFINDER), B,).is_err());

        assert_noop!(
            ZdRefreshSeeds::harvest_seed(Origin::signed(CHALLENGER), B,),
            Error::<Test>::StillUnharvestedChallenges
        );

        let mut pathfinder_balance = ZdToken::free_balance(&PATHFINDER);
        System::set_block_number(ChallengeTimeout::get() + 2);
        assert_ok!(ZdRefreshSeeds::harvest_challenge(
            Origin::signed(PATHFINDER),
            B,
        ));
        let event = Event::zd_refresh_seeds(crate::Event::ChallengeHarvested(PATHFINDER, B));
        assert!(System::events().iter().any(|record| record.event == event));
        pathfinder_balance += ChallengeStakingAmount::get() + SeedChallengeAmount::get();
        assert_eq!(ZdToken::free_balance(&PATHFINDER), pathfinder_balance);

        ZdReputation::set_last_refresh_at();
        assert_noop!(
            ZdRefreshSeeds::harvest_seed(Origin::signed(CHALLENGER), B,),
            Error::<Test>::StillUnconfirmed
        );

        System::set_block_number(ChallengeTimeout::get() + ConfirmationPeriod::get() + 3);
        assert_ok!(ZdRefreshSeeds::harvest_seed(Origin::signed(PATHFINDER), B,));
        let bonus_1 = total_bonus / (MaxSeedCount::get() as u128);
        pathfinder_balance += bonus_1 + SeedReservStaking::get();
        assert_eq!(ZdToken::free_balance(&PATHFINDER), pathfinder_balance);
        assert_ok!(ZdRefreshSeeds::harvest_seed(Origin::signed(PATHFINDER), C,));
        let event = Event::zd_refresh_seeds(crate::Event::SeedHarvested(PATHFINDER, C));
        assert!(System::events().iter().any(|record| record.event == event));
        let rel_total_amount = total_bonus - bonus_1 + SeedStakingAmount::get();
        pathfinder_balance += rel_total_amount;

        assert_eq!(ZdToken::free_balance(&PATHFINDER), pathfinder_balance);

        assert_ok!(ZdRefreshSeeds::harvest_seed(Origin::signed(PATHFINDER), D,));
        pathfinder_balance += SeedStakingAmount::get();

        assert_eq!(ZdToken::free_balance(&PATHFINDER), pathfinder_balance);
    });
}

#[test]
fn missed_path_at_rhashs() {
    new_test_ext().execute_with(|| {
        //
        //                     F
        //                     ↑
        //                     B
        //                 ↗  ↓  ↘
        //               A     E <-  D
        //                 ↘     ↗
        //                     C
        //
        // A -> B -> F order 4ed0601f

        assert_ok!(ZdTrust::trust(Origin::signed(B), F));

        // A -> F for test invalid_evidence
        assert_ok!(ZdTrust::trust(Origin::signed(A), F));

        init_graph(150);

        assert_ok!(ZdRefreshSeeds::challenge(
            Origin::signed(CHALLENGER),
            B,
            150,
        ));

        assert_noop!(
            ZdRefreshSeeds::reply_hash(
                Origin::signed(PATHFINDER),
                B,
                vec![
                    PostResultHash("f9".to_string(), 50, "b3b4e091".to_string()),
                    PostResultHash("7c".to_string(), 100, "781bbaf6".to_string())
                ],
                MAX_HASH_COUNT + 1,
            ),
            Error::<Test>::QuantityExceedsLimit
        );

        assert_ok!(ZdRefreshSeeds::reply_hash(
            Origin::signed(PATHFINDER),
            B,
            vec![
                PostResultHash("f9".to_string(), 50, "b3b4e091".to_string()),
                PostResultHash("7c".to_string(), 100, "781bbaf6".to_string())
            ],
            2,
        ));

        assert!(ZdRefreshSeeds::harvest_challenge(Origin::signed(PATHFINDER), B,).is_err());

        assert_noop!(
            ZdRefreshSeeds::evidence_of_missed(Origin::signed(CHALLENGER), B, vec![A, B, F], 1),
            Error::<Test>::PathIndexError
        );

        assert_noop!(
            ZdRefreshSeeds::evidence_of_missed(Origin::signed(CHALLENGER), B, vec![A, B, F], 3),
            Error::<Test>::IndexExceedsMaximum
        );

        assert_noop!(
            ZdRefreshSeeds::invalid_evidence(Origin::signed(CHALLENGER), B, vec![], 60),
            Error::<Test>::MissedPathsNotExist
        );

        assert_ok!(ZdRefreshSeeds::evidence_of_missed(
            Origin::signed(CHALLENGER),
            B,
            vec![A, B, F],
            0
        ));

        assert!(ZdRefreshSeeds::challenge(Origin::signed(CHALLENGER), B, 50,).is_err());

        assert_noop!(
            ZdRefreshSeeds::invalid_evidence(Origin::signed(CHALLENGER), B, vec![B], 60),
            Error::<Test>::WrongPathLength
        );

        assert_ok!(ZdRefreshSeeds::invalid_evidence(
            Origin::signed(CHALLENGER),
            B,
            vec![],
            60
        ));
        let event = Event::zd_refresh_seeds(crate::Event::EvidenceOfInvalidPresented(CHALLENGER, B, 60));
        assert!(System::events().iter().any(|record| record.event == event));

        System::set_block_number(ConfirmationPeriod::get() + 1);

        assert_noop!(
            ZdRefreshSeeds::harvest_seed(Origin::signed(CHALLENGER), B,),
            Error::<Test>::StillUnharvestedChallenges
        );
    });
}

#[test]
fn evidence_of_shorter_test() {
    new_test_ext().execute_with(|| {
        //
        //                     B
        //                 ↗  ↓  ↘
        //               A     E <-  D
        //                 ↘     ↗
        //                     C
        //
        // A -> D

        assert_ok!(ZdTrust::trust(Origin::signed(A), D));

        init_graph(150);

        let total_bonus: Balance = 991;
        init_challenge(&total_bonus);

        assert_ok!(ZdRefreshSeeds::examine(Origin::signed(CHALLENGER), B, 0,));
        assert_ok!(ZdRefreshSeeds::reply_path(
            Origin::signed(PATHFINDER),
            B,
            vec![Path {
                nodes: vec![A, B, D],
                total: 2
            }],
            1,
        ));

        assert_noop!(
            ZdRefreshSeeds::reply_path(
                Origin::signed(PATHFINDER),
                B,
                vec![Path {
                    nodes: vec![A, B, D],
                    total: 2
                }],
                1,
            ),
            Error::<Test>::AlreadyExists
        );

        assert_ok!(ZdRefreshSeeds::evidence_of_shorter(
            Origin::signed(CHALLENGER),
            B,
            0,
            vec![]
        ));

        let event = Event::zd_refresh_seeds(crate::Event::ShorterPresented(CHALLENGER, B, 0));
        assert!(System::events().iter().any(|record| record.event == event));
    });
}

#[test]
fn number_too_low_test() {
    new_test_ext().execute_with(|| {
        //
        //                     B
        //                 ↗  ↓  ↘
        //               A     E <-  D
        //                 ↘     ↗
        //               ↓     C     ^
        //               F __________|
        //
        //

        assert_ok!(ZdTrust::trust(Origin::signed(A), F));
        assert_ok!(ZdTrust::trust(Origin::signed(F), D));

        // for test Err LengthNotEqual
        assert_ok!(ZdTrust::trust(Origin::signed(E), D));
        assert_ok!(ZdTrust::trust(Origin::signed(A), D));

        init_graph(150);

        let total_bonus: Balance = 991;
        init_challenge(&total_bonus);

        assert_ok!(ZdRefreshSeeds::examine(Origin::signed(CHALLENGER), B, 0,));
        assert_ok!(ZdRefreshSeeds::reply_path(
            Origin::signed(PATHFINDER),
            B,
            vec![Path {
                nodes: vec![A, B, D],
                total: 2
            }],
            1,
        ));

        assert_noop!(
            ZdRefreshSeeds::number_too_low(
                Origin::signed(CHALLENGER),
                B,
                0,
                vec![vec![B,E], vec![C], vec![F],],
            ),
            Error::<Test>::LengthNotEqual
        );

        assert_noop!(
            ZdRefreshSeeds::number_too_low(
                Origin::signed(CHALLENGER),
                B,
                0,
                vec![vec![], vec![C]],
            ),
            Error::<Test>::LengthNotEqual
        );

        assert_noop!(
            ZdRefreshSeeds::number_too_low(
                Origin::signed(CHALLENGER),
                B,
                0,
                vec![vec![C], vec![F],],
            ),
            Error::<Test>::TooFewInNumber
        );

        assert_ok!(ZdRefreshSeeds::number_too_low(
            Origin::signed(CHALLENGER),
            B,
            0,
            vec![vec![B], vec![C], vec![F],],
        ));
        let event = Event::zd_refresh_seeds(crate::Event::EvidenceOfNumTooLowPresented(
            CHALLENGER, B, 0,
        ));
        assert!(System::events().iter().any(|record| record.event == event));

        assert!(ZdRefreshSeeds::harvest_challenge(
            Origin::signed(PATHFINDER),
            B,
        ).is_err());
    });
}

#[test]
fn reply_path_next_test() {
    new_test_ext().execute_with(|| {
        assert_ok!(ZdTrust::trust(Origin::signed(D), F));
        assert_ok!(ZdTrust::trust(Origin::signed(B), G));
        assert_ok!(ZdTrust::trust(Origin::signed(G), F));
        //
        //                     B     ->   G
        //                 ↗  ↓  ↘      ↓
        //               A     E <-  D -> F
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
        // |  ABDF |   3   | 100/3 |    ...4ed0601f    |
        // +-------+-------+-------+-------------------+
        // |  ABGF |   3   | 100/3 |    ...4ed0601f    |
        // +-------+-----------------------------------+
        // | total |         216                       |
        // +-------+-----------------------------------+
        //
        // hash: [AccountId,AccountId,total;...AccountId,AccountId,total;]
        // Path hash:
        // +-------------------------------------------------+------------------------------------------+
        // |  "0001,0002,0004,2;"                            | a0e8df2a2f413bb7f3339c66130b770debb57796 |
        // +-------------------------------------------------+------------------------------------------+
        // |  "0001,0002,0005,1;"                            | b339911bcb3a3080a2b6fcbd033facd968aecc4c |
        // +-------------------------------------------------+------------------------------------------+
        // |  "0001,0002,0004,0006,3;0001,0002,0007,0006,3;" | 252ca0457a02555ccd3e37513d5989328ad9a476 |
        // +-------------------------------------------------+------------------------------------------+
        //
        // Deep 4:
        // +---------+--------------+--------+
        // |  order  | hash         | score  |
        // +---------+--------------+--------+
        // |   f1    | a0e8df2a     |  50    |
        // +---------+--------------+--------+
        // |   66    | b339911b     |  100   |
        // +---------+--------------+--------+
        // |   1f    | 252ca045     |  66    |
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
        // |   60    | 0898dcbe     |  66    |
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
        // |   d0    | f20e66b6     |  66    |
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
        // |   4e    | 7bdb0996     |  66    |
        // +---------+--------------+--------+

        init_graph(216);
        assert_ok!(ZdRefreshSeeds::challenge(
            Origin::signed(CHALLENGER),
            B,
            250,
        ));

        assert_noop!(
            ZdRefreshSeeds::reply_hash(
                Origin::signed(PATHFINDER),
                B,
                vec![
                    PostResultHash("f91".to_string(), 50, "b3b4e091".to_string()),
                    PostResultHash("7c".to_string(), 100, "781bbaf6".to_string()),
                    PostResultHash("4e".to_string(), 66, "7bdb0996".to_string()),
                ],
                3,
            ),
            Error::<Test>::PostConverFail
        );

        assert_noop!(
            ZdRefreshSeeds::reply_hash(
                Origin::signed(PATHFINDER),
                B,
                vec![
                    PostResultHash("f9".to_string(), 50, "b3b4e09大".to_string()),
                    PostResultHash("7c".to_string(), 100, "781bbaf6".to_string()),
                    PostResultHash("4e".to_string(), 66, "7bdb0996".to_string()),
                ],
                3,
            ),
            Error::<Test>::PostConverFail
        );

        assert_noop!(
            ZdRefreshSeeds::reply_hash(
                Origin::signed(PATHFINDER),
                B,
                vec![
                    PostResultHash("f9".to_string(), 50, "b3b4e091".to_string()),
                    PostResultHash("4e".to_string(), 66, "7bdb0996".to_string()),
                    PostResultHash("4e".to_string(), 66, "11111111".to_string()),
                ],
                3,
            ),
            Error::<Test>::DataDuplication
        );

        // Deep 1:
        assert_ok!(ZdRefreshSeeds::reply_hash(
            Origin::signed(PATHFINDER),
            B,
            vec![
                PostResultHash("f9".to_string(), 50, "b3b4e091".to_string()),
                PostResultHash("7c".to_string(), 100, "781bbaf6".to_string()),
                PostResultHash("4e".to_string(), 66, "7bdb0996".to_string()),
            ],
            3,
        ));

        assert_ok!(ZdRefreshSeeds::examine(Origin::signed(CHALLENGER), B, 0,));
        // Deep 2:
        // +---------+--------------+--------+
        // |   d0    | f20e66b6     |  66    |
        // +---------+--------------+--------+
        assert_ok!(ZdRefreshSeeds::reply_hash(
            Origin::signed(PATHFINDER),
            B,
            vec![PostResultHash("d0".to_string(), 66, "f20e66b6".to_string())],
            1,
        ));
        assert_noop!(
            ZdRefreshSeeds::reply_path(
                Origin::signed(PATHFINDER),
                B,
                vec![Path {
                    nodes: vec![A, B, D],
                    total: 2
                }],
                1,
            ),
            Error::<Test>::DepthDoesNotMatch
        );
        assert_ok!(ZdRefreshSeeds::examine(Origin::signed(CHALLENGER), B, 0,));
        // Deep 3:
        // +---------+--------------+--------+
        // |   60    | 0898dcbe     |  66    |
        // +---------+--------------+--------+
        assert_ok!(ZdRefreshSeeds::reply_hash(
            Origin::signed(PATHFINDER),
            B,
            vec![PostResultHash("60".to_string(), 66, "0898dcbe".to_string())],
            1,
        ));
        assert_ok!(ZdRefreshSeeds::examine(Origin::signed(CHALLENGER), B, 0,));
        // Deep 4:
        // +---------+--------------+--------+
        // |   1f    | 252ca045     |  66    |
        // +---------+--------------+--------+
        assert_ok!(ZdRefreshSeeds::reply_hash(
            Origin::signed(PATHFINDER),
            B,
            vec![PostResultHash("1f".to_string(), 66, "252ca045".to_string())],
            1,
        ));

        assert_ok!(ZdRefreshSeeds::examine(Origin::signed(CHALLENGER), B, 0,));
        // +-------+-----------------------------------+
        // |  ABDF |   3   | 100/3 |    ...4ed0601f    |
        // +-------+-------+-------+-------------------+
        // |  ABGF |   3   | 100/3 |    ...4ed0601f    |
        // +-------+-----------------------------------+
        // +-------------------------------------------------+------------------------------------------+
        // |  "0001,0002,0004,0006,3;0001,0002,0007,0006,3;" | 252ca0457a02555ccd3e37513d5989328ad9a476 |
        // +-------------------------------------------------+------------------------------------------+
        assert_ok!(ZdRefreshSeeds::reply_path(
            Origin::signed(PATHFINDER),
            B,
            vec![Path {
                nodes: vec![A, B, D, F],
                total: 3
            }],
            2,
        ));

        assert_ok!(ZdRefreshSeeds::reply_path_next(
            Origin::signed(PATHFINDER),
            B,
            vec![Path {
                nodes: vec![A, B, G, F],
                total: 3
            }],
        ));
        let event = Event::zd_refresh_seeds(crate::Event::ContinueRepliedPath(PATHFINDER, B, true));
        assert!(System::events().iter().any(|record| record.event == event));
    });
}

#[test]
fn missed_at_paths_test() {
    new_test_ext().execute_with(|| {
        assert_ok!(ZdTrust::trust(Origin::signed(D), F));
        assert_ok!(ZdTrust::trust(Origin::signed(B), G));
        assert_ok!(ZdTrust::trust(Origin::signed(G), F));
        //
        //                     B     ->   G
        //                 ↗  ↓  ↘      ↓
        //               A     E <-  D -> F
        //                 ↘     ↗
        //                     C
        //
        // for test Err LengthNotEqual, This should call `evidence_of_shorter`
        assert_ok!(ZdTrust::trust(Origin::signed(B), F));
        //
        // The shortest path through B
        // +-------+-------+-------+-------------------+
        // | Path  | total | score | sha1(start,stop)  |
        // +-------+-------+-------+-------------------+
        // |  ABD  |   2   | 100/2 |    ...f9906cf1    |
        // +-------+-------+-------+-------------------+
        // |  ABE  |   1   | 100/1 |    ...7cfe0266    |
        // +-------+-----------------------------------+
        // |  ABDF |   3   | 100/3 |    ...4ed0601f    |
        // +-------+-------+-------+-------------------+
        // |  ABGF |   3   | 100/3 |    ...4ed0601f    |
        // +-------+-----------------------------------+
        // | total |         216                       |
        // +-------+-----------------------------------+
        //
        // hash: [AccountId,AccountId,total;...AccountId,AccountId,total;]
        // Path hash:
        // +---------------------------+------------------------------------------+
        // |  "0001,0002,0004,2;"      | a0e8df2a2f413bb7f3339c66130b770debb57796 |
        // +---------------------------+------------------------------------------+
        // |  "0001,0002,0005,1;"      | b339911bcb3a3080a2b6fcbd033facd968aecc4c |
        // +-------------------------------------------------+--------------------+
        // |  "0001,0002,0004,0006,3;" | e0c4ada0da592ca29f92d1e6056a8ae6849b301e |
        // +-----------↑-------------------------------------+--------------------+
        //             |_____________ Missing a path
        //
        // Deep 4:
        // +---------+--------------+--------+
        // |  order  | hash         | score  |
        // +---------+--------------+--------+
        // |   f1    | a0e8df2a     |  50    |
        // +---------+--------------+--------+
        // |   66    | b339911b     |  100   |
        // +---------+--------------+--------+
        // |   1f    | e0c4ada0     |  33    |
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
        // |   60    | 5dbb6cab     |  33    |
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
        // |   d0    | b00dbe72     |  33    |
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
        // |   4e    | 993a00e0     |  33    |
        // +---------+--------------+--------+

        init_graph(183);
        assert_ok!(ZdRefreshSeeds::challenge(
            Origin::signed(CHALLENGER),
            B,
            250,
        ));
        // Deep 1:
        assert_ok!(ZdRefreshSeeds::reply_hash(
            Origin::signed(PATHFINDER),
            B,
            vec![
                PostResultHash("f9".to_string(), 50, "b3b4e091".to_string()),
                PostResultHash("7c".to_string(), 100, "781bbaf6".to_string()),
                PostResultHash("4e".to_string(), 33, "993a00e0".to_string()),
            ],
            3,
        ));
        assert_ok!(ZdRefreshSeeds::examine(Origin::signed(CHALLENGER), B, 0,));
        // Deep 2:
        // +---------+--------------+--------+
        // |   d0    | b00dbe72     |  33    |
        // +---------+--------------+--------+
        assert_ok!(ZdRefreshSeeds::reply_hash(
            Origin::signed(PATHFINDER),
            B,
            vec![PostResultHash("d0".to_string(), 33, "b00dbe72".to_string())],
            1,
        ));
        assert_ok!(ZdRefreshSeeds::examine(Origin::signed(CHALLENGER), B, 0,));
        // Deep 3:
        // +---------+--------------+--------+
        // |   60    | 5dbb6cab     |  33    |
        // +---------+--------------+--------+
        assert_ok!(ZdRefreshSeeds::reply_hash(
            Origin::signed(PATHFINDER),
            B,
            vec![PostResultHash("60".to_string(), 33, "5dbb6cab".to_string())],
            1,
        ));
        assert_ok!(ZdRefreshSeeds::examine(Origin::signed(CHALLENGER), B, 0,));
        // Deep 4:
        // +---------+--------------+--------+
        // |   1f    | e0c4ada0     |  33    |
        // +---------+--------------+--------+
        assert_ok!(ZdRefreshSeeds::reply_hash(
            Origin::signed(PATHFINDER),
            B,
            vec![PostResultHash("1f".to_string(), 33, "e0c4ada0".to_string())],
            1,
        ));

        assert_ok!(ZdRefreshSeeds::examine(Origin::signed(CHALLENGER), B, 0,));
        // +-------+-----------------------------------+
        // |  ABDF |   3   | 100/3 |    ...4ed0601f    |
        // +-------+-------+-------+-------------------+
        // +-------------------------------------------------+--------------------+
        // |  "0001,0002,0004,0006,3;" | e0c4ada0da592ca29f92d1e6056a8ae6849b301e |
        // +-------------------------------------------------+--------------------+
        assert_ok!(ZdRefreshSeeds::reply_path(
            Origin::signed(PATHFINDER),
            B,
            vec![Path {
                nodes: vec![A, B, D, F],
                total: 3
            }],
            1,
        ));

        assert_noop!(
            ZdRefreshSeeds::evidence_of_missed(Origin::signed(CHALLENGER), B, vec![A, C, D, E], 1),
            Error::<Test>::NoTargetNode
        );

        assert_noop!(
            ZdRefreshSeeds::evidence_of_missed(Origin::signed(CHALLENGER), B, vec![A, B, D, F], 1),
            Error::<Test>::AlreadyExist
        );

        assert_noop!(
            ZdRefreshSeeds::evidence_of_missed(Origin::signed(CHALLENGER), B, vec![A, B, E], 1),
            Error::<Test>::NotMatch
        );

        assert_noop!(
            ZdRefreshSeeds::evidence_of_missed(Origin::signed(CHALLENGER), B, vec![A, B, F], 1),
            Error::<Test>::LengthNotEqual
        );

        assert_ok!(ZdRefreshSeeds::evidence_of_missed(
            Origin::signed(CHALLENGER),
            B,
            vec![A, B, G, F],
            1
        ));
    });
}
