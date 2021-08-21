#![cfg(test)]

use super::*;
use crate::mock::*;
use frame_support::{assert_noop, assert_ok, dispatch};
use system::Account;

fn initialize_trust() {
    let relationships: Vec<(AccountId, AccountId)> =
        vec![(FERDIE, BOB), (ALICE, CHARLIE), (ALICE, BOB), (BOB, CHARLIE), (CHARLIE, DAVE), (DAVE, EVE)];

    for (a, b) in relationships.iter() {
        assert_eq!(ZdTrust::do_trust(a, b), Ok(()));
    }
}

fn new_test_ext() -> sp_io::TestExternalities {
    let mut ext = ExtBuilder::default().build();
    ext.execute_with(|| System::set_block_number(1));
    ext
}

#[test]
fn get_trust_count_should_work() {
    new_test_ext().execute_with(|| {
        initialize_trust();
        assert_eq!(ZdTrust::get_trust_count(&BOB), 1);
        assert_eq!(ZdTrust::get_trust_count(&ALICE), 2);
        assert_eq!(ZdTrust::get_trust_count(&BOB), 1);
        assert_eq!(ZdTrust::get_trust_count(&CHARLIE), 1);
    });
}

#[test]
fn is_trust_should_work() {
    new_test_ext().execute_with(|| {
        initialize_trust();
        assert_eq!(ZdTrust::is_trust(&FERDIE, &BOB), true);
        assert_eq!(ZdTrust::is_trust(&ALICE, &BOB), true);
        assert_eq!(ZdTrust::is_trust(&ALICE,&EVE), false);
        assert_eq!(ZdTrust::is_trust(&20,&ALICE), false);
        assert_eq!(ZdTrust::is_trust(&20,&21), false);
        assert_eq!(ZdTrust::is_trust(&ALICE,&21), false);
    });
}

#[test]
fn valid_nodes_should_work() {
    new_test_ext().execute_with(|| {
        initialize_trust();
        assert_ok!(ZdTrust::valid_nodes(&vec![ALICE,BOB]));
        assert_ok!(ZdTrust::valid_nodes(&vec![ALICE,BOB,CHARLIE]));
        assert_ok!(ZdTrust::valid_nodes(&vec![ALICE,BOB,CHARLIE,DAVE,EVE]));
    });
}

#[test]
fn valid_nodes_should_fail() {
    new_test_ext().execute_with(|| {
        initialize_trust();
        assert_noop!(ZdTrust::valid_nodes(&vec![ALICE,BOB,DAVE]), Error::<Test>::WrongPath);
        assert_noop!(ZdTrust::valid_nodes(&vec![ALICE,BOB,CHARLIE, 21]), Error::<Test>::WrongPath);
        assert_noop!(ZdTrust::valid_nodes(&vec![21,BOB,CHARLIE]), Error::<Test>::WrongPath);
        assert_noop!(ZdTrust::valid_nodes(&vec![21,22,23]), Error::<Test>::WrongPath);
        assert_noop!(ZdTrust::valid_nodes(&vec![21,21,21]), Error::<Test>::WrongPath);
        assert_noop!(ZdTrust::valid_nodes(&vec![ALICE,ALICE]), Error::<Test>::WrongPath);
    });
}

#[test]
fn computed_path_should_work() {
    new_test_ext().execute_with(|| {
        initialize_trust();
        assert_ok!(ZdSeeds::new_seed(Origin::root(), ALICE));
        // vec![(FERDIE, BOB), (ALICE, CHARLIE), (ALICE, BOB), (BOB, CHARLIE), (CHARLIE, DAVE), (DAVE, EVE)];
        assert_ok!(ZdTrust::computed_path(&vec![ALICE,BOB]), (2,250));
        assert_ok!(ZdTrust::computed_path(&vec![ALICE,BOB,DAVE]), (3,50));
    });
}
