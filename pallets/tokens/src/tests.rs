#![cfg(test)]

use super::*;
use crate::mock::{Event, *};
use frame_support::assert_ok;

fn new_test_ext() -> sp_io::TestExternalities {
    let mut ext = ExtBuilder::default().build();
    ext.execute_with(|| System::set_block_number(1));
    ext
}

macro_rules! transfer_social {
    ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                new_test_ext().execute_with(|| {
                    let from_old_free_balance = Currencies::free_balance(DOT, &ALICE);
                    let to_old_free_balance = Currencies::free_balance(DOT, &$value.1);
                    assert_ok!(ZdToken::transfer_social(
                        Origin::signed(ALICE),
                        $value.1,
                        DOT,
                        $value.0
                    ));
            
                    assert_eq!(Currencies::free_balance(DOT, &ALICE), from_old_free_balance - $value.0);
                    assert_eq!(Currencies::free_balance(DOT, &$value.1), to_old_free_balance);
                    assert_eq!(Currencies::total_balance(DOT, &$value.1), to_old_free_balance + $value.0);
                    assert_eq!(Currencies::social_balance(DOT, &$value.1), $value.0);
            
                    let social_transferred_event = Event::zd_tokens(crate::Event::TransferSocial(DOT, ALICE, $value.1, $value.0));
                    assert!(System::events().iter().any(|record| record.event == social_transferred_event));
                });
            }
        )*
    }
}

transfer_social! {
    transfer_social_0: (10, BOB),
    transfer_social_1: (0, BOB),
    transfer_social_2: (9, BOB),
    transfer_social_3: (10293711u128, BOB),
    transfer_social_4: (100, CHARLIE),
    transfer_social_5: (1, CHARLIE),
}