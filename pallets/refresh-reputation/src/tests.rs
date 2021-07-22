#![cfg(test)]

use super::*;
use crate::mock::{Event, *};
use frame_support::{assert_noop, assert_ok};

fn new_test_ext() -> sp_io::TestExternalities {
    let mut ext = ExtBuilder::default().build();
    ext.execute_with(|| System::set_block_number(1));
    ext
}

#[test]
fn new_round_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(ZdRefreshReputation::new_round(Origin::signed(ALICE)));
    });
}

#[test]
fn refresh_should_work() {
    new_test_ext().execute_with(|| {
        let user_scores = vec![(BOB, 12), (CHARLIE, 18), (DAVE, 1200)];
        let social_token_amount = 100;

        assert_ok!(ZdRefreshReputation::new_round(Origin::signed(BOB)));
        assert_ok!(Tokens::transfer_social(
            ZDAO,
            &ALICE,
            &BOB,
            social_token_amount
        ));

        assert_eq!(Currencies::social_balance(ZDAO, &BOB), social_token_amount);
        assert_ok!(ZdRefreshReputation::refresh(
            Origin::signed(ALICE),
            user_scores
        ));

        let pathfinder_fee = <FeeRation>::get().mul_floor(social_token_amount);
        let leave_self = <SelfRation>::get().mul_floor(social_token_amount);

        let reputation_refreshed_event = Event::zd_refresh_reputation(crate::Event::ReputationRefreshed(ALICE, 3, pathfinder_fee));
        assert!(System::events()
            .iter()
            .any(|record| record.event == reputation_refreshed_event));

        assert_eq!(ZdRefreshReputation::get_payroll(ALICE), Payroll {
            total_fee: pathfinder_fee,
            count: 3,
        });

        assert_eq!(Currencies::social_balance(ZDAO, &BOB), social_token_amount - pathfinder_fee - leave_self);
    });
}

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

        assert_ok!(ZdRefreshReputation::new_round(Origin::signed(ALICE)));

        assert_noop!(
            (ZdRefreshReputation::refresh(
                Origin::signed(ALICE),
                user_scores_too_long
            )),
            Error::<Test>::QuantityLimitReached
        );
    });
}