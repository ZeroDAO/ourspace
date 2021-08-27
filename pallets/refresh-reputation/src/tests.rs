#![cfg(test)]

use super::*;
use crate::mock::*;
use frame_support::{assert_noop, assert_ok};
use zd_primitives::per_social_currency;

fn new_test_ext() -> sp_io::TestExternalities {
    let mut ext = ExtBuilder::default().build();
    ext.execute_with(|| System::set_block_number(1));
    ext
}

#[test]
fn start_should_work() {
    new_test_ext().execute_with(|| {
        ZdReputation::set_step(&TIRStep::REPUTATION);
        System::set_block_number(2000);
        assert_ok!(ZdRefreshReputation::start(Origin::signed(ALICE)));
    });
}

const INIT_PAYROLLS: [Payroll<Balance,BlockNumber>; 6] = [
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

        ZdReputation::set_step(&TIRStep::REPUTATION);

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
        let user_scores_too_long = vec![
            (BOB, 0),
            (CHARLIE, 0),
            (DAVE, 0),
            (EVE, 0),
            (FERDIE, 0),
        ];
        for a in INIT_ACCOUNT.iter() {
            assert_ok!(ZdToken::transfer_social(
                Origin::signed(SWEEPRT),
                a.account,
                ZDAO,
                a.soc_amount
            ));
        }

        assert_noop!(
            ZdRefreshReputation::refresh(
                Origin::signed(PATHFINDER),
                user_scores.clone()
            ),
            Error::<Test>::StatusErr
        );

        assert_ok!(ZdReputation::new_round());
        ZdReputation::set_step(&TIRStep::REPUTATION);
        System::set_block_number(2000);
        assert_noop!(
            ZdRefreshReputation::refresh(
                Origin::signed(PATHFINDER),
                user_scores.clone()
            ),
            Error::<Test>::NotYetStarted
        );
        assert_ok!(ZdRefreshReputation::start(Origin::signed(PATHFINDER)));
        assert_noop!(
            ZdRefreshReputation::refresh(
                Origin::signed(PATHFINDER),
                user_scores_too_long
            ),
            Error::<Test>::QuantityLimitReached
        );
        assert!(
            ZdRefreshReputation::refresh(
                Origin::signed(CHARLIE),
             user_scores.clone()
            ).is_err()
        );
        assert_ok!(ZdRefreshReputation::refresh(
            Origin::signed(PATHFINDER),
            user_scores.clone()
        ));

        for a in INIT_ACCOUNT[..4].iter() {
            assert_eq!(<Records<Test>>::get(&PATHFINDER,a.account).fee, per_social_currency::PRE_FEE.mul_floor(a.soc_amount));
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
    });
}

macro_rules! next_step_should_work {
    ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                new_test_ext().execute_with(|| {
                    ZdReputation::set_step(&TIRStep::REPUTATION);
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
                        !ZdReputation::is_step(&TIRStep::FREE),
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
    next_step_should_work_2: (199,true),
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