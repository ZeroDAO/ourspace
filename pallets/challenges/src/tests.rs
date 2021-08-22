#![cfg(test)]

use super::*;
use crate::mock::{Event, *};
use frame_support::{assert_noop, assert_ok};

const APP_ID: AppId = *b"test    ";
const DefaultMetadata: Metadata<AccountId,BlockNumber> = Metadata {
    pool: Pool {
        staking: 0,
        earnings: 0,
    },
    joint_benefits: false,
    progress: Progress {
        total: 100,
        done: 0,
    },
    last_update: 1,
    remark: 0,
    score: 0,
    pathfinder: BOB,
    status: Status::EXAMINE,
    challenger: ALICE,
};

fn new_test_ext() -> sp_io::TestExternalities {
    let mut ext = ExtBuilder::default().build();
    ext.execute_with(|| System::set_block_number(1));
    ext
}

#[test]
fn new_challenge_should_work() {
    new_test_ext().execute_with(|| {

        assert_ok!(ZdChallenges::new(
            &APP_ID, &ALICE, &BOB, 200_200, 100_100, &CHARLIE, 100, 0
        ));

        assert_eq!(
            ZdChallenges::get_metadata(&APP_ID, &CHARLIE),
            Metadata {
                pool: Pool {
                    staking: 100_100,
                    earnings: 200_200,
                },
                ..DefaultMetadata
            }
        );

        assert_eq!(Currencies::total_staking(ZDAO), 100_100u128);
        assert_eq!(Currencies::free_balance(ZDAO, &ALICE), 1000_000_000_000_000u128 - 100_100u128);
    });
}
