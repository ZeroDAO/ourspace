#![cfg(test)]

use super::*;
// use crate::mock::{Event, *};
use frame_support::{assert_noop, assert_ok};

fn new_test_ext() -> sp_io::TestExternalities {
    let mut ext = ExtBuilder::default().build();
    ext.execute_with(|| System::set_block_number(1));
    ext
}

#[test]
fn new_round_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(ZdRefreshReputation::start(Origin::signed(ALICE)));
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