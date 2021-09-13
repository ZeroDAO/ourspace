#![cfg(test)]

use super::*;
use crate::mock::{Event, *};
use frame_support::{assert_err_ignore_postinfo, assert_noop, assert_ok};
use zd_primitives::per_social_currency;

fn new_test_ext() -> sp_io::TestExternalities {
    let mut ext = ExtBuilder::default().build();
    ext.execute_with(|| System::set_block_number(1));
    ext
}

#[test]
fn start_should_work() {
    new_test_ext().execute_with(|| {
        ZdReputation::set_step(&TIRStep::Reputation);
        System::set_block_number(2000);
        assert_ok!(ZdRefreshReputation::start(Origin::signed(ALICE)));
        let new_event = Event::zd_refresh_reputation(crate::Event::Started(ALICE));
        assert!(System::events().iter().any(|record| record.event == new_event));
    });
}

const INIT_PAYROLLS: [Payroll<Balance, BlockNumber>; 6] = [
    Payroll {
        count: 11,
        total_fee: 1001,
        update_at: 1,
    },
    Payroll {
        count: 112,
        total_fee: 1021,
        update_at: 1,
    },
    Payroll {
        count: 100,
        total_fee: 10011233,
        update_at: 1,
    },
    Payroll {
        count: 2,
        total_fee: 1,
        update_at: 1,
    },
    Payroll {
        count: 1,
        total_fee: 0,
        update_at: 1,
    },
    Payroll {
        count: 0,
        total_fee: 13,
        update_at: 1,
    },
];

pub struct InitAccount {
    account: AccountId,
    soc_amount: Balance,
    score: u32,
}

const INIT_ACCOUNT: [InitAccount; 6] = [
    InitAccount {
        account: ALICE,
        soc_amount: 100111,
        score: 199,
    },
    InitAccount {
        account: 99,
        soc_amount: 101,
        score: 0,
    },
    InitAccount {
        account: BOB,
        soc_amount: 2000,
        score: 1,
    },
    InitAccount {
        account: DAVE,
        soc_amount: 212333,
        score: 322,
    },
    InitAccount {
        account: EVE,
        soc_amount: 122199,
        score: 1998,
    },
    InitAccount {
        account: FERDIE,
        soc_amount: 10,
        score: 0,
    },
];

#[test]
fn start_with_payrolls() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            ZdRefreshReputation::start(Origin::signed(SWEEPRT)),
            Error::<Test>::StatusErr
        );

        ZdReputation::set_step(&TIRStep::Reputation);

        assert_noop!(
            ZdRefreshReputation::start(Origin::signed(SWEEPRT)),
            Error::<Test>::NotInTime
        );

        System::set_block_number(2000);

        // init staking pool
        assert_ok!(ZdToken::staking(&FERDIE, &1_000_000_000_000u128));

        for (i, payroll) in INIT_PAYROLLS.iter().enumerate() {
            <Payrolls<Test>>::insert(&INIT_ACCOUNT[i].account, payroll);
        }

        let who_balance = ZdToken::free_balance(&SWEEPRT);

        let old_balances = INIT_ACCOUNT
            .iter()
            .map(|a| ZdToken::free_balance(&a.account))
            .collect::<Vec<Balance>>();

        assert_ok!(ZdRefreshReputation::start(Origin::signed(SWEEPRT)));

        let total_fee = INIT_PAYROLLS.iter().enumerate().fold(0, |acc, (i, p)| {
            let staking_amount = <mock::Test as Config>::UpdateStakingAmount::get();
            let total_amount = staking_amount * (p.count as u128) + p.total_fee;
            let (sweeper_fee, awards) = total_amount.with_fee();

            assert_eq!(
                ZdToken::free_balance(&INIT_ACCOUNT[i].account),
                awards + old_balances[i]
            );
            acc + sweeper_fee
        });

        assert_eq!(ZdToken::free_balance(&SWEEPRT), who_balance + total_fee);
    });
}

#[test]
fn refresh_should_work() {
    new_test_ext().execute_with(|| {
        let user_scores = INIT_ACCOUNT[..4]
            .iter()
            .map(|a| (a.account, a.score))
            .collect::<Vec<(AccountId, u32)>>();
        let user_scores_too_long = vec![(BOB, 0), (CHARLIE, 0), (DAVE, 0), (EVE, 0), (FERDIE, 0)];
        for a in INIT_ACCOUNT.iter() {
            assert_ok!(ZdToken::transfer_social(
                Origin::signed(SWEEPRT),
                a.account,
                a.soc_amount
            ));
        }

        assert_noop!(
            ZdRefreshReputation::refresh(Origin::signed(PATHFINDER), user_scores.clone()),
            Error::<Test>::StatusErr
        );

        assert_ok!(ZdReputation::new_round());
        ZdReputation::set_step(&TIRStep::Reputation);
        System::set_block_number(2000);
        assert_noop!(
            ZdRefreshReputation::refresh(Origin::signed(PATHFINDER), user_scores.clone()),
            Error::<Test>::NotYetStarted
        );
        assert_ok!(ZdRefreshReputation::start(Origin::signed(PATHFINDER)));
        assert_noop!(
            ZdRefreshReputation::refresh(Origin::signed(PATHFINDER), user_scores_too_long),
            Error::<Test>::QuantityLimitReached
        );
        assert!(
            ZdRefreshReputation::refresh(Origin::signed(CHARLIE), user_scores.clone()).is_err()
        );
        assert_ok!(ZdRefreshReputation::refresh(
            Origin::signed(PATHFINDER),
            user_scores.clone()
        ));

        for a in INIT_ACCOUNT[..4].iter() {
            assert_eq!(
                <Records<Test>>::get(&PATHFINDER, a.account).fee,
                per_social_currency::PRE_FEE.mul_floor(a.soc_amount)
            );
        }

        let total_fee = INIT_ACCOUNT[..4]
            .iter()
            .map(|f| per_social_currency::PRE_FEE.mul_floor(f.soc_amount))
            .sum();

        assert_eq!(
            <Payrolls<Test>>::get(&PATHFINDER).count,
            user_scores.len() as u32
        );
        assert_eq!(<Payrolls<Test>>::get(&PATHFINDER).total_fee, total_fee);

        let new_event = Event::zd_refresh_reputation(crate::Event::ReputationRefreshed(PATHFINDER,user_scores.len() as u32,total_fee));
        assert!(System::events().iter().any(|record| record.event == new_event));
    });
}

macro_rules! next_step_should_work {
    ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                new_test_ext().execute_with(|| {
                    ZdReputation::set_step(&TIRStep::Reputation);
                    assert_ok!(ZdTrust::trust(Origin::signed(ALICE),BOB));
                    <StartedAt<Test>>::put(1);
                    ZdReputation::set_last_refresh_at();

                    System::set_block_number($value.0);

                    ZdRefreshReputation::next_step();

                    assert_eq!(
                        !ZdTrust::is_trust_old(&ALICE,&BOB),
                        $value.1
                    );
                    assert_eq!(
                        !ZdReputation::is_step(&TIRStep::Free),
                        $value.1
                    );

                    assert_eq!(
                        <StartedAt<Test>>::exists(),
                        $value.1
                    );
                });
            }
        )*
    }
}

next_step_should_work! {
    next_step_should_work_0: (10,true),
    next_step_should_work_1: (5000, false),
    next_step_should_work_2: (<mock::Test as Config>::ConfirmationPeriod::get() + 2, false),
    next_step_should_work_3: (20,true),
    next_step_should_work_4: (62,true),
}

macro_rules! harvest_ref_all_should_work {
    ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                new_test_ext().execute_with(|| {
                    <Payrolls<Test>>::insert(&PATHFINDER, Payroll {
                        count: $value.0,
                        total_fee: $value.1,
                        update_at: 1,
                    });
                    let total_amount = UpdateStakingAmount::get() * $value.0 + $value.1;
                    assert_ok!(ZdToken::staking(&ALICE, &1_000_000_000_000u128));
                    for a in INIT_ACCOUNT.iter() {
                        <Records<Test>>::insert(&PATHFINDER,&a.account,Record {
                            update_at: 11,
                            fee: 111,
                        });
                    }
                    System::set_block_number(500);
                    let old_balances = ZdToken::free_balance(&PATHFINDER);
                    assert_ok!(ZdRefreshReputation::harvest_ref_all(Origin::signed(PATHFINDER)));
                    let new_balances = ZdToken::free_balance(&PATHFINDER);
                    assert_eq!(new_balances - old_balances, total_amount);
                    for a in INIT_ACCOUNT.iter() {
                        assert!(<Records<Test>>::try_get(&PATHFINDER,&a.account).is_err());
                    }
                    let new_event = Event::zd_refresh_reputation(crate::Event::RefreshedHarvested(PATHFINDER, total_amount));
                    assert!(System::events().iter().any(|record| record.event == new_event));
                });
            }
        )*
    }
}

harvest_ref_all_should_work! {
    harvest_ref_all_should_work_0: (2,1000),
    harvest_ref_all_should_work_1: (0,11),
    harvest_ref_all_should_work_2: (0,0),
    harvest_ref_all_should_work_3: (12,0),
    harvest_ref_all_should_work_4: (212,1000),
}

fn init_sys(score: u32) {
    let init_seeds = vec![SEED1, SEED2, SEED3, SEED4];
    for seed in init_seeds {
        ZdSeeds::add_seed(&seed);
    }
    let init_paths = vec![
        vec![SEED1, ALICE, TARGET],
        vec![SEED2, ALICE, BOB, TARGET],
        vec![SEED3, TARGET],
        vec![SEED3, ALICE, TARGET],
        vec![SEED3, ALICE, BOB, TARGET],
        vec![SEED4, TARGET],
    ];
    for path in init_paths {
        // println!("path: {:?}",path);
        for nodes in path.windows(2) {
            // println!("{:?} -> {:?}",nodes[0],nodes[1]);
            if !ZdTrust::is_trust(&nodes[0], &nodes[1]) {
                assert_ok!(ZdTrust::trust(Origin::signed(nodes[0]), nodes[1]));
            }
        }
    }
    assert_ok!(ZdToken::transfer_social(
        Origin::signed(ALICE),
        TARGET,
        1000
    ));
    assert_ok!(ZdReputation::new_round());

    ZdReputation::set_step(&TIRStep::Reputation);
    <StartedAt<Test>>::put(1);

    assert_ok!(ZdRefreshReputation::refresh(
        Origin::signed(PATHFINDER),
        vec![(TARGET, score)]
    ));
}

#[test]
fn challenge_should_work() {
    new_test_ext().execute_with(|| {
        init_sys(100);
        assert_ok!(ZdRefreshReputation::challenge(
            Origin::signed(CHALLENGER),
            TARGET,
            PATHFINDER,
            3,
            20
        ));
        let payroll = ZdRefreshReputation::get_payroll(PATHFINDER);
        assert_eq!(payroll.total_fee, 0);
        assert_eq!(payroll.count, 0);
        assert_eq!(payroll.update_at, 1);
        let new_event = Event::zd_refresh_reputation(crate::Event::Challenge(CHALLENGER, TARGET));
        assert!(System::events().iter().any(|record| record.event == new_event));
    });
}

#[test]
fn challenge_should_fail() {
    new_test_ext().execute_with(|| {
        init_sys(100);
        assert_noop!(
            ZdRefreshReputation::challenge(Origin::signed(CHALLENGER), TARGET, PATHFINDER, 3, 100),
            Error::<Test>::SameReputation
        );
        assert_noop!(
            ZdRefreshReputation::challenge(Origin::signed(CHALLENGER), TARGET, PATHFINDER, 10, 55),
            Error::<Test>::ExcessiveBumberOfSeeds
        );
        assert_noop!(
            ZdRefreshReputation::challenge(
                Origin::signed(CHALLENGER),
                TARGET,
                PATHFINDER,
                u32::MAX,
                55
            ),
            Error::<Test>::ExcessiveBumberOfSeeds
        );
        System::set_block_number(<mock::Test as Config>::ConfirmationPeriod::get() + 100);
        assert_err_ignore_postinfo!(
            ZdRefreshReputation::challenge(Origin::signed(CHALLENGER), TARGET, PATHFINDER, 3, 55),
            Error::<Test>::ChallengeTimeout
        );
    });
}

#[test]
fn challenge_update_should_work() {
    new_test_ext().execute_with(|| {
        init_sys(100);
        assert_ok!(ZdRefreshReputation::challenge(
            Origin::signed(CHALLENGER),
            TARGET,
            PATHFINDER,
            3,
            20
        ));
        /*
        vec![SEED1, ALICE, TARGET],
        vec![SEED2, ALICE, BOB, TARGET],
        vec![SEED3, TARGET],
        vec![SEED3, ALICE, TARGET],
        vec![SEED3, ALICE, BOB, TARGET],
         */
        let (score1, score2, score3) = (18u32, 12u32, 99u32);
        let seeds = vec![SEED1, SEED2];
        let paths = vec![
            Path {
                nodes: vec![ALICE],
                score: score1,
            },
            Path {
                nodes: vec![ALICE, BOB],
                score: score2,
            },
        ];
        assert_ok!(ZdRefreshReputation::challenge_update(
            Origin::signed(CHALLENGER),
            TARGET,
            seeds.clone(),
            paths.clone()
        ));
        let new_event = Event::zd_refresh_reputation(crate::Event::PathUpdated(CHALLENGER, TARGET));
        assert!(System::events().iter().any(|record| record.event == new_event));
        for seed in seeds.iter() {
            assert!(Paths::<Test>::contains_key(seed, TARGET));
        }
        assert_ok!(ZdRefreshReputation::challenge_update(
            Origin::signed(CHALLENGER),
            TARGET,
            vec![SEED3],
            vec![Path {
                nodes: vec![],
                score: score3
            }]
        ));
        assert_eq!(
            ZdReputation::get_reputation_new(&TARGET),
            Some(score1 + score2 + score3)
        );
    });
}

#[test]
fn challenge_update_should_fail() {
    new_test_ext().execute_with(|| {
        init_sys(100);
        assert_ok!(ZdRefreshReputation::challenge(
            Origin::signed(CHALLENGER),
            TARGET,
            PATHFINDER,
            3,
            20
        ));

        assert_err_ignore_postinfo!(
            ZdRefreshReputation::challenge_update(
                Origin::signed(CHALLENGER),
                TARGET,
                vec![SEED1, SEED3],
                vec![Path {
                    nodes: vec![ALICE],
                    score: 12
                }]
            ),
            Error::<Test>::NotMatch
        );

        assert_ok!(ZdRefreshReputation::challenge_update(
            Origin::signed(CHALLENGER),
            TARGET,
            vec![SEED3],
            vec![Path {
                nodes: vec![ALICE, TARGET],
                score: 11
            }]
        ));

        assert_err_ignore_postinfo!(
            ZdRefreshReputation::challenge_update(
                Origin::signed(CHALLENGER),
                TARGET,
                vec![SEED3],
                vec![Path {
                    nodes: vec![ALICE, TARGET],
                    score: 12
                }]
            ),
            Error::<Test>::PathAlreadyExist
        );

        assert_err_ignore_postinfo!(
            ZdRefreshReputation::challenge_update(
                Origin::signed(CHALLENGER),
                TARGET,
                vec![SEED2],
                vec![Path {
                    nodes: vec![ALICE, BOB, TARGET, TARGET, TARGET, BOB, TARGET],
                    score: 12
                }]
            ),
            Error::<Test>::WrongPath
        );

        assert_err_ignore_postinfo!(
            ZdRefreshReputation::challenge_update(
                Origin::signed(CHALLENGER),
                TARGET,
                vec![SEED2],
                vec![Path {
                    nodes: vec![ALICE, BOB, TARGET],
                    score: u32::MAX
                }]
            ),
            Error::<Test>::Overflow
        );
    });
}

#[test]
fn arbitral_should_work() {
    new_test_ext().execute_with(|| {
        init_sys(100);
        assert_ok!(ZdRefreshReputation::challenge(
            Origin::signed(CHALLENGER),
            TARGET,
            PATHFINDER,
            3,
            20
        ));
        /*
        vec![SEED1, ALICE, TARGET],
        vec![SEED2, ALICE, BOB, TARGET],
        vec![SEED3, TARGET],
        vec![SEED3, ALICE, TARGET],
        vec![SEED3, ALICE, BOB, TARGET],
        score1 : 1000 / 1.max(5) / (1000 - 0).ln() = 28.5714
                28 / 2.max(5) / 1 = 5.6
        score2 : 1000 / 1.max(5) / (1000 - 0).ln() = 28.5714
                28 / 2.max(5) / (0 - 0).ln() = 5.6
                5 / 1.max(5) / (0 - 0).ln() = 1
        score3 : 1000 / 2.max(5) / (1000 - 0).ln() = 28.5714
         */
        let paths = vec![
            Path {
                nodes: vec![],
                score: 11,
            },
            Path {
                nodes: vec![ALICE, BOB],
                score: 21,
            },
            Path {
                nodes: vec![ALICE, BOB],
                score: 29,
            },
        ];
        assert_ok!(ZdRefreshReputation::challenge_update(
            Origin::signed(CHALLENGER),
            TARGET,
            vec![SEED1, SEED2, SEED3],
            paths.clone()
        ));

        assert_eq!(ZdReputation::get_reputation(&TARGET), Some(0));
        assert_eq!(ZdReputation::get_reputation(&ALICE), Some(0));

        assert_ok!(ZdRefreshReputation::arbitral(
            Origin::signed(CHALLENGER),
            TARGET,
            vec![SEED3, SEED2, SEED1],
            vec![
                Path {
                    nodes: vec![ALICE, BOB],
                    score: 1,
                },
                Path {
                    nodes: vec![ALICE],
                    score: 5,
                },
                Path {
                    nodes: vec![ALICE, BOB],
                    score: 1,
                },
            ]
        ));

        // Allow challengers to fix errors within ChallengeTimeout
        assert_ok!(ZdRefreshReputation::arbitral(
            Origin::signed(CHALLENGER),
            TARGET,
            vec![SEED3],
            vec![Path {
                nodes: vec![ALICE],
                score: 5,
            }]
        ));

        System::set_block_number(ChallengeTimeout::get() + 10);

        assert_ok!(ZdRefreshReputation::arbitral(
            Origin::signed(SUB_CHALLENGER),
            TARGET,
            vec![SEED3],
            vec![Path {
                nodes: vec![],
                score: 28,
            }]
        ));
        assert_eq!(ZdTrust::is_trust(&SEED4, &TARGET),true);
        assert_ok!(ZdRefreshReputation::arbitral(
            Origin::signed(SUB_CHALLENGER),
            TARGET,
            vec![SEED4],
            vec![Path {
                nodes: vec![],
                score: 28,
            }]
        ));
    });
}

#[test]
fn arbitral_should_fail() {
    new_test_ext().execute_with(|| {
        init_sys(100);
        let paths = vec![
            Path {
                nodes: vec![],
                score: 11,
            },
            Path {
                nodes: vec![ALICE, BOB],
                score: 21,
            },
            Path {
                nodes: vec![ALICE],
                score: 5,
            },
        ];
        assert_ok!(ZdRefreshReputation::challenge(
            Origin::signed(CHALLENGER),
            TARGET,
            PATHFINDER,
            3,
            20
        ));
        assert_ok!(ZdRefreshReputation::challenge_update(
            Origin::signed(CHALLENGER),
            TARGET,
            vec![SEED1, SEED2, SEED3],
            paths.clone()
        ));
        /*
        vec![SEED1, ALICE, TARGET],
        vec![SEED2, ALICE, BOB, TARGET],
        vec![SEED3, TARGET],
        vec![SEED3, ALICE, TARGET],
        vec![SEED3, ALICE, BOB, TARGET],
        score1 : 1000 / 1.max(5) / (1000 - 0).ln() = 28.5714
                28 / 2.max(5) / 1 = 5.6
        score2 : 1000 / 1.max(5) / (1000 - 0).ln() = 28.5714
                28 / 2.max(5) / (0 - 0).ln() = 5.6
                5 / 1.max(5) / (0 - 0).ln() = 1
        score3 : 1000 / 2.max(5) / (1000 - 0).ln() = 28.5714
         */

        assert_err_ignore_postinfo!(
            ZdRefreshReputation::arbitral(
                Origin::signed(CHALLENGER),
                TARGET,
                vec![SEED3],
                vec![
                    Path {
                        nodes: vec![ALICE],
                        score: 5,
                    },
                    Path {
                        nodes: vec![ALICE],
                        score: 5,
                    }
                ]
            ),
            Error::<Test>::NotMatch
        );

        assert_err_ignore_postinfo!(
            ZdRefreshReputation::arbitral(
                Origin::signed(CHALLENGER),
                TARGET,
                vec![SEED3],
                vec![
                    Path {
                        nodes: vec![BOB],
                        score: 5,
                    },
                ]
            ),
            Error::<Test>::DistErr
        );

        assert_err_ignore_postinfo!(
            ZdRefreshReputation::arbitral(
                Origin::signed(CHALLENGER),
                TARGET,
                vec![SEED3],
                vec![Path {
                    nodes: vec![],
                    score: 25,
                }]
            ),
            Error::<Test>::DistErr
        );

        assert_err_ignore_postinfo!(
            ZdRefreshReputation::arbitral(
                Origin::signed(CHALLENGER),
                TARGET,
                vec![SEED3],
                vec![Path {
                    nodes: vec![ALICE, BOB],
                    score: 1,
                }]
            ),
            Error::<Test>::DistTooLong
        );
    });
}
