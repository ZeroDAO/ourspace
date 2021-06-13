#![cfg(test)]

use super::*;
use crate::mock::{Event, *};
use frame_support::assert_ok;

const ENDOWED_AMOUNT: u128 = 1_000_000_000_000_000;

fn new_test_ext() -> sp_io::TestExternalities {
    let mut ext = ExtBuilder::default().build();
    ext.execute_with(|| System::set_block_number(1));
    ext
}

#[test]
fn test_transfer_social() {
    new_test_ext().execute_with(|| {
        assert_ok!(SocoinToken::transfer_social(
            Origin::signed(ALICE),
            BOB,
            DOT,
            10
        ));

        assert_eq!(Currencies::free_balance(DOT, &ALICE), ENDOWED_AMOUNT - 10);
        assert_eq!(Currencies::free_balance(DOT, &BOB), ENDOWED_AMOUNT);
        assert_eq!(Currencies::actual_balance(DOT, &BOB), ENDOWED_AMOUNT + 10);
        assert_eq!(Currencies::social_balance(DOT, &BOB), 10);

        let social_transferred_event = Event::socoin_tokens(crate::Event::TransferSocial(DOT, ALICE, BOB, 10));
        assert!(System::events().iter().any(|record| record.event == social_transferred_event));
    });
}
