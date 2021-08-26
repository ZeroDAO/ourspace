#![cfg(test)]

use super::*;
use crate::mock::*;
use frame_support::{assert_noop, assert_ok};

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

const INIT_PAYROLLS: [Payroll<Balance>; 6] = [
    Payroll {
        count: 11,
        total_fee: 1001,
    },
    Payroll {
        count: 112,
        total_fee: 1021,
    },
    Payroll {
        count: 100,
        total_fee: 10011233,
    },
    Payroll {
        count: 2,
        total_fee: 1,
    },
    Payroll {
        count: 1,
        total_fee: 0,
    },
    Payroll {
        count: 0,
        total_fee: 13,
    },
];

const INIT_ACCOUNT: [AccountId; 6] = [ALICE, BOB, CHARLIE, DAVE, EVE, FERDIE];

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
            <Payrolls<Test>>::insert(&INIT_ACCOUNT[i], payroll);
        }

        let who_balance = ZdToken::free_balance(&SWEEPRT);

        let old_balances = INIT_ACCOUNT
            .iter()
            .map(|a| ZdToken::free_balance(a) )
            .collect::<Vec<Balance>>();

        assert_ok!(ZdRefreshReputation::start(Origin::signed(SWEEPRT)));

        let total_fee = INIT_PAYROLLS.iter().enumerate().fold(0,|acc,(i,p)|{

            let staking_amount = <mock::Test as Config>::UpdateStakingAmount::get();
            let total_amount = staking_amount * (p.count as u128) + p.total_fee;
            let (sweeper_fee, awards) = total_amount.with_fee();

            assert_eq!(ZdToken::free_balance(&INIT_ACCOUNT[i]), awards + old_balances[i]);
            acc + sweeper_fee
        });

        assert_eq!(ZdToken::free_balance(&SWEEPRT), who_balance + total_fee);
        
    });
}



#[test]
fn refresh_should_work() {
    new_test_ext().execute_with(|| {

        let user_scores = vec![(BOB, 12), (CHARLIE, 18)];
        // let user_scores_too_long = vec![(BOB, 12), (CHARLIE, 18), (DAVE, 1200),(EVE, 1223),(FERDIE, 322)];

        assert_noop!(
            ZdRefreshReputation::refresh(Origin::signed(PATHFINDER),user_scores.clone()),
            Error::<Test>::StatusErr
        );

        ZdReputation::new_round();
        ZdReputation::set_step(&TIRStep::REPUTATION);

        System::set_block_number(2000);

        // ZdRefreshReputation::start(Origin::signed(SWEEPRT));

        assert_ok!(ZdRefreshReputation::refresh(Origin::signed(PATHFINDER),user_scores.clone()));
        
    });
}

/*
#[test]
fn refresh_should_fail() {
    new_test_ext().execute_with(|| {
        let user_scores = vec![(BOB, 12), (CHARLIE, 18)];
        let user_scores_too_long = vec![(BOB, 12), (CHARLIE, 18), (DAVE, 1200),(EVE, 1223),(FERDIE, 322)];

        assert_noop!(
            (ZdRefreshReputation::refresh(
                Origin::signed(ALICE),
                user_scores
            )),
            Error::<Test>::NoUpdatesAllowed
        );

        assert_ok!(ZdRefreshReputation::start(Origin::signed(ALICE)));

        assert_noop!(
            (ZdRefreshReputation::refresh(
                Origin::signed(ALICE),
                user_scores_too_long
            )),
            Error::<Test>::QuantityLimitReached
        );
    });
}
*/
