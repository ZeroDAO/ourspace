#![cfg(test)]

use super::*;
use crate::mock::{Event, *};
use frame_support::{assert_noop, assert_ok, dispatch};

fn new_test_ext() -> sp_io::TestExternalities {
    let mut ext = ExtBuilder::default().build();
    ext.execute_with(|| System::set_block_number(1));
    ext
}

#[test]
fn refresh_reputation_should_work() {
    new_test_ext().execute_with(|| {
        let user_scores = vec![(BOB, 12), (CHARLIE, 18), (DAVE, 1200)];
        let social_token_amount = 100;

        assert_ok!(ZdRefreshReputation::new_round(Origin::root()));
        assert_ok!(Tokens::transfer_social(
            ZDAO,
            &ALICE,
            &BOB,
            social_token_amount
        ));

        assert_eq!(Currencies::social_balance(ZDAO, &BOB), social_token_amount);
        assert_ok!(ZdRefreshReputation::refresh_reputation(
            Origin::signed(ALICE),
            user_scores
        ));

        let fee = <FeeRation>::get().mul_floor(social_token_amount);

        let reputation_refreshed_event = Event::zd_refresh_reputation(crate::Event::ReputationRefreshed(ALICE, 3, fee));
        assert!(System::events()
            .iter()
            .any(|record| record.event == reputation_refreshed_event));

        assert_eq!(ZdRefreshReputation::get_fees(ALICE), fee);
    });
}