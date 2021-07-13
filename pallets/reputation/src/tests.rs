#![cfg(test)]

use super::*;
use crate::mock::*;
use frame_support::{assert_ok,assert_noop};

fn new_test_ext() -> sp_io::TestExternalities {
    let mut ext = ExtBuilder::default().build();
    ext.execute_with(|| System::set_block_number(1));
    ext
}

#[test]
fn new_round_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(ZdReputation::new_round());
        assert_eq!(<SystemInfo<Test>>::get(), OperationStatus {
            nonce: 0,
            last: 1,
            updating: true,
            next: INIT_PERIOD + 1,
            period: INIT_PERIOD
        });
    });
}