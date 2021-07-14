#![cfg(test)]

use super::*;
use crate::mock::*;
use frame_support::{assert_noop, assert_ok, dispatch};

fn new_test_ext() -> sp_io::TestExternalities {
    let mut ext = ExtBuilder::default().build();
    ext.execute_with(|| System::set_block_number(1));
    ext
}

#[test]
fn set_period_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(ZdReputation::set_period(Origin::root(), 18));

        assert_eq!(<SystemInfo<Test>>::get().period, 18);
    });
}

#[test]
fn set_period_should_fail() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            ZdReputation::set_period(Origin::signed(ALICE), 18),
            dispatch::DispatchError::BadOrigin
        );
    });
}

#[test]
fn new_round_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(ZdReputation::new_round());
        assert_eq!(
            <SystemInfo<Test>>::get(),
            OperationStatus {
                nonce: 0,
                last: 1,
                updating: true,
                next: INIT_PERIOD + 1,
                period: INIT_PERIOD
            }
        );
    });
}

#[test]
fn new_round_should_fail() {
    new_test_ext().execute_with(|| {
        assert_ok!(ZdReputation::new_round());
        assert_noop!(
            ZdReputation::new_round(),
            Error::<Test>::AlreadyInUpdating
        );
    });
}

#[test]
fn mutate_reputation_should_work() {
    new_test_ext().execute_with(|| {
        assert_eq!(ZdReputation::mutate_reputation(&ALICE,21),());

        assert_eq!(ZdReputation::get_reputation_new(&ALICE),Some(21));
    });
}

#[test]
fn refresh_reputation_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(ZdReputation::new_round());
        assert_ok!(ZdReputation::refresh_reputation(&(ALICE,18)));

        assert_eq!(ZdReputation::get_reputation_new(&ALICE),Some(18));
    });
}