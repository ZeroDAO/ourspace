#![cfg(test)]

use super::*;
use crate::mock::{Event, *};
use frame_support::{assert_noop, assert_ok};

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
                    let from_old_free_balance = ZdToken::free_balance(&ALICE);
                    let to_old_free_balance = ZdToken::free_balance(&$value.1);
                    assert_ok!(ZdToken::transfer_social(
                        Origin::signed(ALICE),
                        $value.1,
                        $value.0
                    ));

                    assert_eq!(ZdToken::free_balance(&ALICE), from_old_free_balance - $value.0);
                    assert_eq!(ZdToken::actual_balance(&ALICE), from_old_free_balance - $value.0);
                    assert_eq!(ZdToken::free_balance(&$value.1), to_old_free_balance);
                    assert_eq!(ZdToken::actual_balance(&$value.1), to_old_free_balance + $value.0);
                    assert_eq!(ZdToken::social_balance(&$value.1), $value.0);

                    let social_transferred_event = Event::zd_tokens(crate::Event::TransferSocial(ALICE, $value.1, $value.0));
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

#[test]
fn staking_test() {
    new_test_ext().execute_with(|| {
        assert_ok!(ZdToken::staking(&ALICE, &100));
        assert_eq!(ZdToken::total_staking(), 100);

        assert_ok!(ZdToken::transfer_social(
            Origin::signed(ALICE),
            CHARLIE,
            600
        ));

        let old_balance = ZdToken::free_balance(&CHARLIE);

        <Accounts<Test>>::mutate(CHARLIE, |account| {
            account.pending = 599;
            account.social = 1;
        });

        assert!(ZdToken::staking(&CHARLIE, &600).is_err());

        assert_ok!(<Currencies as MultiCurrency<_>>::transfer(
            BaceToken::get(),
            &ALICE,
            &CHARLIE,
            1
        ));

        assert_ok!(ZdToken::staking(&CHARLIE, &600));

        assert_eq!(ZdToken::total_staking(), 100 + 600);
        assert_eq!(ZdToken::free_balance(&CHARLIE), old_balance);
        assert_eq!(ZdToken::pending_balance(&CHARLIE), 0);
        assert_eq!(ZdToken::social_balance(&CHARLIE), 1);
    });
}

#[test]
fn release_test() {
    new_test_ext().execute_with(|| {
        assert_ok!(ZdToken::staking(&ALICE, &100));

        assert_noop!(
            ZdToken::release(&BOB, &101),
            Error::<Test>::StakingAmountTooLow
        );
        assert_noop!(
            ZdToken::release(&BOB, &u128::MAX),
            Error::<Test>::StakingAmountTooLow
        );

        let old_balance = ZdToken::free_balance(&BOB);
        assert_ok!(ZdToken::release(&BOB, &100));

        assert_eq!(ZdToken::free_balance(&BOB), old_balance + 100);
        assert_eq!(ZdToken::total_staking(), 100 - 100);
    });
}

macro_rules! share_test {
    ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                new_test_ext().execute_with(|| {
                    let (total_social_balance, len) = $value;

                    assert_ok!(ZdToken::transfer_social(
                        Origin::signed(ALICE),
                        CHARLIE,
                        total_social_balance
                    ));

                    let targets = (100u64..(len + 100)).collect::<Vec<AccountId>>();

                    let total_share_amount = per_social_currency::PRE_SHARE.mul_floor(total_social_balance);
                    let reserved_amount = per_social_currency::PRE_RESERVED.mul_floor(total_social_balance);
                    let burn_amount = per_social_currency::PRE_BURN.mul_floor(total_social_balance);
                    let fee_amount = per_social_currency::PRE_FEE.mul_floor(total_social_balance);

                    let pre_reward =
                        total_social_balance - total_share_amount - reserved_amount - burn_amount - fee_amount;

                    let old_total_issuance = <Currencies as MultiCurrency<_>>::total_issuance(BaceToken::get());

                    assert_eq!(ZdToken::share(&CHARLIE, &targets[..]), fee_amount);

                    let count = targets.len() as u128;

                    let mut remaining_share: u128 = total_share_amount;
                    if !(count == 0 || total_share_amount < count) {
                        let share_amount =
                            total_share_amount / count.max(per_social_currency::MIN_TRUST_COUNT as u128);
                        for target in targets {
                            assert_eq!(ZdToken::social_balance(&target), share_amount);
                        }
                        remaining_share = total_share_amount - share_amount * count;
                    }
                    // println!("pre_reward: {:?}",pre_reward);
                    assert_eq!(ZdToken::get_bonus_amount(), pre_reward);
                    assert_eq!(ZdToken::social_balance(&CHARLIE), remaining_share);
                    assert_eq!(
                        <Currencies as MultiCurrency<_>>::total_issuance(BaceToken::get()),
                        old_total_issuance - burn_amount
                    );
                    assert_eq!(ZdToken::pending_balance(&CHARLIE), reserved_amount);
                });
            }
        )*
    }
}

share_test! {
    share_test_0: (10000, 12),
    share_test_1: (10000, 100),
    share_test_2: (10000, 10),
    share_test_3: (10, 1000),
    share_test_4: (0, 0),
}

#[test]
fn claim_test() {
    new_test_ext().execute_with(|| {
        assert_ok!(ZdToken::staking(&ALICE, &100));

        assert_ok!(ZdToken::claim(Origin::signed(CHARLIE)));

        <Accounts<Test>>::mutate(CHARLIE, |account| {
            account.pending = 91;
        });

        let old_balance = ZdToken::free_balance(&CHARLIE);
        assert_ok!(ZdToken::claim(Origin::signed(CHARLIE)));

        assert_eq!(ZdToken::free_balance(&CHARLIE), old_balance + 91);
        assert_eq!(ZdToken::pending_balance(&CHARLIE), 0);

        assert_ok!(ZdToken::claim(Origin::signed(CHARLIE)));

        assert_eq!(ZdToken::free_balance(&CHARLIE), old_balance + 91);
        assert_eq!(ZdToken::pending_balance(&CHARLIE), 0);

        <Accounts<Test>>::mutate(CHARLIE, |account| {
            account.pending = 91;
        });

        assert!(ZdToken::claim(Origin::signed(CHARLIE)).is_err());
    });
}
