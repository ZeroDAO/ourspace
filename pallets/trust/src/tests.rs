#![cfg(test)]

use super::*;
use crate::mock::{Event, *};
use frame_support::{assert_noop, assert_ok};

fn initialize_trust() {
    let relationships: Vec<(AccountId, AccountId)> = vec![
        (FERDIE, BOB),
        (ALICE, CHARLIE),
        (ALICE, BOB),
        (BOB, CHARLIE),
        (CHARLIE, DAVE),
        (DAVE, EVE),
    ];

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
        assert_eq!(ZdTrust::is_trust(&ALICE, &EVE), false);
        assert_eq!(ZdTrust::is_trust(&20, &ALICE), false);
        assert_eq!(ZdTrust::is_trust(&20, &21), false);
        assert_eq!(ZdTrust::is_trust(&ALICE, &21), false);
    });
}

#[test]
fn valid_nodes_should_work() {
    new_test_ext().execute_with(|| {
        initialize_trust();
        assert_ok!(ZdTrust::valid_nodes(&vec![ALICE, BOB]));
        assert_ok!(ZdTrust::valid_nodes(&vec![ALICE, BOB, CHARLIE]));
        assert_ok!(ZdTrust::valid_nodes(&vec![ALICE, BOB, CHARLIE, DAVE, EVE]));
    });
}

#[test]
fn valid_nodes_should_fail() {
    new_test_ext().execute_with(|| {
        initialize_trust();
        assert_noop!(
            ZdTrust::valid_nodes(&vec![ALICE, BOB, DAVE]),
            Error::<Test>::WrongPath
        );
        assert_noop!(
            ZdTrust::valid_nodes(&vec![ALICE, BOB, CHARLIE, 21]),
            Error::<Test>::WrongPath
        );
        assert_noop!(
            ZdTrust::valid_nodes(&vec![21, BOB, CHARLIE]),
            Error::<Test>::WrongPath
        );
        assert_noop!(
            ZdTrust::valid_nodes(&vec![21, 22, 23]),
            Error::<Test>::WrongPath
        );
        assert_noop!(
            ZdTrust::valid_nodes(&vec![21, 21, 21]),
            Error::<Test>::WrongPath
        );
        assert_noop!(
            ZdTrust::valid_nodes(&vec![ALICE, ALICE]),
            Error::<Test>::WrongPath
        );
    });
}

#[test]
fn computed_path_should_work() {
    new_test_ext().execute_with(|| {
        initialize_trust();
        assert_ok!(ZdSeeds::new_seed(Origin::root(), ALICE));
        // vec![(FERDIE, BOB), (ALICE, CHARLIE), (ALICE, BOB), (BOB, CHARLIE), (CHARLIE, DAVE), (DAVE, EVE)];
        // 1000 / 2.max(5) / (1000 - 0).ln() = 28.5714
        assert_ok!(ZdTrust::computed_path(&vec![ALICE, BOB]), (7, 28));
        // 28 / 0.max(5) / 1 = 5.6
        assert_ok!(
            ZdTrust::computed_path(&vec![ALICE, BOB, CHARLIE]),
            (1 + 7, 5)
        );
    });
}

#[test]
fn computed_path_should_fail() {
    new_test_ext().execute_with(|| {
        initialize_trust();
        assert_ok!(ZdSeeds::new_seed(Origin::root(), ALICE));
        assert_noop!(
            ZdTrust::computed_path(&vec![BOB, CHARLIE]),
            Error::<Test>::NotSeed
        );
        assert_noop!(
            ZdTrust::computed_path(&vec![ALICE, BOB, 22]),
            Error::<Test>::WrongPath
        );
    });
}

#[test]
fn get_trust_count_old_should_work() {
    new_test_ext().execute_with(|| {
        initialize_trust();
        ZdReputation::set_step(&TIRStep::SEED);

        // (ALICE, CHARLIE), (ALICE, BOB)
        assert_ok!(ZdTrust::do_trust(&ALICE, &DAVE));
        assert_eq!(ZdTrust::get_trust_count_old(&ALICE), 2);
        assert_eq!(ZdTrust::get_trust_count(&ALICE), 3);

        assert_ok!(ZdTrust::do_untrust(&ALICE, &DAVE));
        assert_eq!(ZdTrust::get_trust_count_old(&ALICE), 2);
        assert_eq!(ZdTrust::get_trust_count(&ALICE), 2);

        assert_ok!(ZdTrust::do_untrust(&ALICE, &CHARLIE));
        assert_eq!(ZdTrust::get_trust_count_old(&ALICE), 2);
        assert_eq!(ZdTrust::get_trust_count(&ALICE), 1);
    });
}

#[test]
fn is_trust_old_should_work() {
    new_test_ext().execute_with(|| {
        ZdReputation::set_step(&TIRStep::SEED);

        assert_eq!(ZdTrust::is_trust_old(&ALICE, &DAVE), false);
        assert_ok!(ZdTrust::do_trust(&ALICE, &DAVE));
        assert_eq!(ZdTrust::is_trust_old(&ALICE, &DAVE), false);
    });
}

#[test]
fn get_trust_old_should_work() {
    new_test_ext().execute_with(|| {
        initialize_trust();

        // (ALICE, CHARLIE), (ALICE, BOB)
        assert_eq!(ZdTrust::get_trust_old(&ALICE), vec![BOB, CHARLIE]);

        ZdReputation::set_step(&TIRStep::SEED);

        assert_ok!(ZdTrust::do_trust(&ALICE, &DAVE));

        assert_eq!(ZdTrust::trust_temp_list(&ALICE).trust.0.is_empty(), true);
        assert_eq!(ZdTrust::trust_temp_list(&ALICE).untrust.0, vec![DAVE]);

        assert_eq!(ZdTrust::get_trust_old(&ALICE), vec![BOB, CHARLIE]);

        assert_ok!(ZdTrust::do_untrust(&ALICE, &CHARLIE));
        assert_eq!(ZdTrust::get_trust_old(&ALICE), vec![BOB, CHARLIE]);
    });
}

#[test]
fn trust_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(ZdTrust::trust(Origin::signed(ALICE), BOB));
        assert_eq!(ZdTrust::is_trust(&ALICE, &BOB), true);

        ZdReputation::set_step(&TIRStep::SEED);
        assert_ok!(ZdTrust::trust(Origin::signed(ALICE), CHARLIE));
        assert_eq!(ZdTrust::is_trust(&ALICE, &CHARLIE), true);
        assert_eq!(ZdTrust::is_trust_old(&ALICE, &CHARLIE), false);

        assert_eq!(ZdTrust::trust_temp_list(&ALICE).trust.0.is_empty(), true);
        assert_eq!(ZdTrust::trust_temp_list(&ALICE).untrust.0, vec![CHARLIE]);

        let trust_event = Event::zd_trust(crate::Event::Trusted(ALICE, CHARLIE));
        assert!(System::events()
            .iter()
            .any(|record| record.event == trust_event));
    });
}

#[test]
fn untrust_should_work() {
    new_test_ext().execute_with(|| {
        initialize_trust();
        // (ALICE, CHARLIE), (ALICE, BOB)
        assert_ok!(ZdTrust::untrust(Origin::signed(ALICE), BOB));
        assert_eq!(ZdTrust::is_trust(&ALICE, &BOB), false);

        ZdReputation::set_step(&TIRStep::SEED);
        assert_ok!(ZdTrust::untrust(Origin::signed(ALICE), CHARLIE));
        assert_eq!(ZdTrust::is_trust(&ALICE, &CHARLIE), false);
        assert_eq!(ZdTrust::is_trust_old(&ALICE, &CHARLIE), true);

        let trust_event = Event::zd_trust(crate::Event::Untrusted(ALICE, BOB));
        assert!(System::events()
            .iter()
            .any(|record| record.event == trust_event));
    });
}

#[test]
fn trust_should_fail() {
    new_test_ext().execute_with(|| {
        initialize_trust();
        assert_noop!(
            ZdTrust::trust(Origin::signed(ALICE), BOB),
            Error::<Test>::RepeatTrust
        );
        assert_noop!(
            ZdTrust::trust(Origin::signed(ALICE), ALICE),
            Error::<Test>::UnableTrustYourself
        );
    });
}

#[test]
fn untrust_should_fail() {
    new_test_ext().execute_with(|| {
        initialize_trust();
        assert_noop!(
            ZdTrust::untrust(Origin::signed(ALICE), DAVE),
            Error::<Test>::NonExistent
        );
        assert_noop!(
            ZdTrust::untrust(Origin::signed(ALICE), ALICE),
            Error::<Test>::UnableUntrustYourself
        );
    });
}
