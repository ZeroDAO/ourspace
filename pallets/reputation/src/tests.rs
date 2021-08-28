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
                nonce: 1,
                last: 1,
                next: INIT_PERIOD + 1,
                period: INIT_PERIOD,
                step: TIRStep::FREE,
            }
        );
    });
}

#[test]
fn new_round_should_fail() {
    new_test_ext().execute_with(|| {
        ZdReputation::set_step(&TIRStep::REPUTATION);
        assert_noop!(ZdReputation::new_round(), Error::<Test>::AlreadyInUpdating);
        ZdReputation::set_step(&TIRStep::FREE);
        assert_ok!(ZdReputation::new_round());
        ZdReputation::set_step(&TIRStep::FREE);
        System::set_block_number(INIT_PERIOD - 1);
        assert_noop!(ZdReputation::new_round(), Error::<Test>::IntervalIsTooShort);
    });
}

#[test]
fn mutate_reputation_should_work() {
    new_test_ext().execute_with(|| {
        ZdReputation::mutate_reputation(&ALICE, &21);
        assert_eq!(ZdReputation::get_reputation_new(&ALICE), Some(21));
    });
}

macro_rules! refresh_reputation_should_work {
    ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                new_test_ext().execute_with(|| {
                    assert_ok!(ZdReputation::new_round());
                    let init_reputation = ReputationScore {
                        score: 671u32,
                        nonce: 1,
                    };
                    <ReputationScores<Test>>::mutate(ALICE,|s| s[0] = init_reputation.clone());
                    <SystemInfo<Test>>::mutate(|s| s.nonce = 2);
                    assert_ok!(ZdReputation::refresh_reputation(&(ALICE, $value)));

                    assert_eq!(ZdReputation::get_reputation_new(&ALICE), Some($value));

                    assert_eq!(
                        <ReputationScores<Test>>::get(ALICE)[0],
                        ReputationScore {
                            score: $value,
                            nonce: 2,
                        }
                    );

                    assert_eq!(
                        <ReputationScores<Test>>::get(ALICE)[1],
                        init_reputation
                    );
                });
            }
        )*
    }
}

refresh_reputation_should_work! {
    refresh_reputation_should_work_0: 18,
    refresh_reputation_should_work_1: 1345,
    refresh_reputation_should_work_2: 0,
    refresh_reputation_should_work_3: u32::MAX,
}

#[test]
fn refresh_reputation_should_fail() {
    new_test_ext().execute_with(|| {
        assert_ok!(ZdReputation::new_round());
        assert_ok!(ZdReputation::refresh_reputation(&(ALICE, 18)));

        assert_noop!(
            ZdReputation::refresh_reputation(&(ALICE, 18)),
            Error::<Test>::ReputationAlreadyUpdated
        );
    });
}

#[test]
fn last_refresh_at_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(ZdReputation::new_round());
        ZdReputation::set_last_refresh_at();
        assert_eq!(<SystemInfo<Test>>::get().last, 1);
        assert_eq!(ZdReputation::get_last_refresh_at(), 1);
        System::set_block_number(12000);
        ZdReputation::set_last_refresh_at();
        assert_eq!(ZdReputation::get_last_refresh_at(), 12000);
        System::set_block_number(11);
        ZdReputation::set_last_refresh_at();
        assert_eq!(ZdReputation::get_last_refresh_at(), 11);
        System::set_block_number(1666600);
        ZdReputation::set_last_refresh_at();
        assert_eq!(ZdReputation::get_last_refresh_at(), 1666600);
        System::set_block_number(235783);
        ZdReputation::set_last_refresh_at();
        assert_eq!(ZdReputation::get_last_refresh_at(), 235783);
    });
}


#[test]
fn set_free_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(ZdReputation::new_round());
        System::set_block_number(150);

        ZdReputation::set_step(&TIRStep::REPUTATION);
        ZdReputation::set_free();

        assert_eq!(<SystemInfo<Test>>::get().last, 150);
        assert_eq!(<SystemInfo<Test>>::get().step, TIRStep::FREE);
    });
}

