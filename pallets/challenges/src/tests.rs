#![cfg(test)]

use super::*;
use crate::mock::{Event, *};
use frame_support::{assert_noop, assert_ok};

const APP_ID: AppId = *b"test    ";
const CHALLENGER: AccountId = ALICE;
const PATHINFER: AccountId = BOB;
const TARGET: AccountId = CHARLIE;

const DEFAULT_METADATA: Metadata<AccountId, BlockNumber> = Metadata {
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
    pathfinder: PATHINFER,
    status: Status::EXAMINE,
    challenger: CHALLENGER,
};

fn new_test_ext() -> sp_io::TestExternalities {
    let mut ext = ExtBuilder::default().build();
    ext.execute_with(|| System::set_block_number(1));
    ext
}

pub struct NewChallengeData {
    earnings: u128,
    staking: u128,
    total: u32,
    score: u64,
    init: bool,
    set_now: BlockNumber,
}

impl Default for NewChallengeData {
    fn default() -> Self {
        Self {
            earnings: 0,
            staking: 0,
            total: 100,
            score: 0,
            init: false,
            set_now: 1,
        }
    }
}

macro_rules! new_challenge_should_work {
    ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                new_test_ext().execute_with(|| {
                    let staking_amount = ZdChallenges::challenge_staking_amount();
                    let mut init_staking: u128 = 0;
                    let mut init_earnings: u128 = 0;
                    if $value.init {
                        init_staking = 2000;
                        init_earnings = 8000;
                        let init_metadata = Metadata {
                            pool: Pool {
                                staking: init_staking,
                                earnings: init_earnings,
                            },
                            progress: Progress {
                                total: $value.total,
                                done: 0,
                            },
                            last_update: 1,
                            status: Status::FREE,
                            ..DEFAULT_METADATA
                        };
                        <Metadatas<Test>>::insert(&APP_ID,&TARGET,&init_metadata);
                        // ChallengeTimeout
                        System::set_block_number($value.set_now);
                    }
                    assert_ok!(ZdChallenges::new(
                        &APP_ID,
                        &CHALLENGER,
                        &PATHINFER,
                        $value.earnings,
                        $value.staking,
                        &TARGET,
                        $value.total,
                        $value.score
                    ));
                    assert_eq!(
                        ZdChallenges::get_metadata(&APP_ID, &TARGET),
                        Metadata {
                            pool: Pool {
                                staking: init_staking + $value.staking + staking_amount,
                                earnings: init_earnings + $value.earnings,
                            },
                            progress: Progress {
                                total: $value.total,
                                done: 0,
                            },
                            score: $value.score,
                            last_update: $value.set_now,
                            ..DEFAULT_METADATA
                        }
                    );
                    assert_eq!(Currencies::total_staking(ZDAO), staking_amount);
                    assert_eq!(Currencies::free_balance(ZDAO, &CHALLENGER), 1000_000_000_000_000u128 - staking_amount);
                });
            }
        )*
    }
}

new_challenge_should_work! {
    new_challenge_should_work_0: NewChallengeData {
        earnings: 20000,
        staking: 1000,
        score: 100,
        ..NewChallengeData::default()
    },
    new_challenge_should_work_1: NewChallengeData {
        earnings: 0,
        staking: 1000000,
        score: 100,
        ..NewChallengeData::default()
    },
    new_challenge_should_work_2: NewChallengeData {
        total: 0,
        score: 100,
        ..NewChallengeData::default()
    },
    new_challenge_should_work_3: NewChallengeData {
        earnings: 20000,
        staking: 10000000000u128,
        total: 0,
        ..NewChallengeData::default()
    },

    new_challenge_should_work_4: NewChallengeData {
        earnings: 20000,
        staking: 1000,
        score: 100,
        init: true,
        set_now: 100,
        ..NewChallengeData::default()
    },
    new_challenge_should_work_5: NewChallengeData {
        earnings: 0,
        staking: 1000000,
        score: 100,
        init: true,
        set_now: 20,
        ..NewChallengeData::default()
    },
    new_challenge_should_work_6: NewChallengeData {
        total: 0,
        score: 100,
        init: true,
        set_now: 2,
        ..NewChallengeData::default()
    },
    new_challenge_should_work_7: NewChallengeData {
        earnings: 20000,
        staking: 10000000000u128,
        total: 0,
        init: true,
        set_now: 2,
        ..NewChallengeData::default()
    },
}

#[test]
fn new_challenge_staking_fail() {
    new_test_ext().execute_with(|| {
        assert!(
            ZdChallenges::new(&APP_ID, &DAVE, &PATHINFER, 0, 1000, &TARGET, 100, 0).is_err()
        );
        assert_eq!(ZdChallenges::get_metadata(&APP_ID, &TARGET),Metadata::default())
    });
}

macro_rules! new_challenge_no_allowed {
    ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                new_test_ext().execute_with(|| {
                    let init_metadata = Metadata {
                        progress: Progress {
                            total: $value.0,
                            done: 0,
                        },
                        last_update: 1,
                        status: $value.1,
                        ..DEFAULT_METADATA
                    };
                    <Metadatas<Test>>::insert(&APP_ID,&TARGET,&init_metadata);
                    // ChallengeTimeout
                    System::set_block_number($value.2);

                    assert_noop!(ZdChallenges::new(
                        &APP_ID,
                        &CHALLENGER,
                        &PATHINFER,
                        10,
                        10,
                        &TARGET,
                        20,
                        10
                    ),Error::<Test>::NoChallengeAllowed);
                });
            }
        )*
    }
}

new_challenge_no_allowed! {
    new_challenge_no_allowed_0: (10,Status::FREE,5),
    new_challenge_no_allowed_1: (10,Status::FREE,2),
    new_challenge_no_allowed_2: (10,Status::FREE,0),
    new_challenge_no_allowed_3: (0,Status::FREE,20),
    new_challenge_no_allowed_4: (0,Status::FREE,1000),
    new_challenge_no_allowed_5: (0,Status::EVIDENCE,5),
    new_challenge_no_allowed_6: (0,Status::EXAMINE,5),
    new_challenge_no_allowed_7: (0,Status::REPLY,5),
    new_challenge_no_allowed_8: (0,Status::ARBITRATION,5),
}

fn init_challenge(total: u32,done: u32,status: Status) {
    let init_metadata = Metadata {
        progress: Progress {
            total: total,
            done: done,
        },
        status: status,
        ..DEFAULT_METADATA
    };

    <Metadatas<Test>>::mutate(&APP_ID,&TARGET,|m|*m = init_metadata);
}

#[test]
fn next_should_work() {
    new_test_ext().execute_with(|| {
        init_challenge(300,20, Status::FREE);
        System::set_block_number(3);
        assert_ok!(ZdChallenges::next(&APP_ID,&CHALLENGER,&TARGET,&100,|score,remark,is_all_done|-> Result<(u64, u32), DispatchError> {
            assert_eq!(score,DEFAULT_METADATA.score);
            assert_eq!(remark,DEFAULT_METADATA.remark);
            assert_eq!(is_all_done,false);
            Ok((211, 322))
        }));
        let metadata = ZdChallenges::get_metadata(&APP_ID, &TARGET);
        assert_eq!(metadata.progress.done, 100 + 20);
        assert_eq!(metadata.score, 211);
        assert_eq!(metadata.remark, 322);
    });
}

#[test]
fn next_should_fail() {
    new_test_ext().execute_with(|| {
        init_challenge(100,20, Status::FREE);
        System::set_block_number(3);
        assert_noop!(ZdChallenges::next(&APP_ID,&EVE,&TARGET,&80,|score,remark,_|-> Result<(u64, u32), DispatchError> {
            Ok((score, remark))
        }),Error::<Test>::NoPermission);
        assert_noop!(ZdChallenges::next(&APP_ID,&CHALLENGER,&TARGET,&300,|score,remark,_|-> Result<(u64, u32), DispatchError> {
            Ok((score, remark))
        }),Error::<Test>::TooMany);
        System::set_block_number(200);
        assert_noop!(ZdChallenges::next(&APP_ID,&CHALLENGER,&TARGET,&90,|score,remark,_|-> Result<(u64, u32), DispatchError> {
            Ok((score, remark))
        }),Error::<Test>::ProgressErr);
        let metadata = ZdChallenges::get_metadata(&APP_ID, &TARGET);
        assert_eq!(metadata.progress.total, 100);
        assert_eq!(metadata.progress.done, 20);
    });
}

#[test]
fn examine_should_work() {
    new_test_ext().execute_with(|| {
        init_challenge(100,100, Status::REPLY);
        assert_ok!(ZdChallenges::examine(
            &APP_ID,
            CHALLENGER,
            &TARGET,
            22
        ));
        let metadata = ZdChallenges::get_metadata(&APP_ID, &TARGET);
        assert_eq!(metadata.remark, 22);
        assert_eq!(metadata.status, Status::EXAMINE);
        assert_eq!(metadata.last_update, 1);
    });
}

#[test]
fn examine_should_fail() {
    new_test_ext().execute_with(|| {
        init_challenge(100,100, Status::REPLY);
        assert_noop!(ZdChallenges::examine(
            &APP_ID,
            EVE,
            &TARGET,
            22
        ),Error::<Test>::NoPermission);
        let metadata = ZdChallenges::get_metadata(&APP_ID, &TARGET);
        assert_eq!(metadata.status, Status::REPLY);
        assert_eq!(metadata.progress.total, 100);
        assert_eq!(metadata.progress.done, 100);
        init_challenge(100,10, Status::REPLY);
        assert_noop!(ZdChallenges::examine(
            &APP_ID,
            CHALLENGER,
            &TARGET,
            22
        ),Error::<Test>::NoChallengeAllowed);
        let metadata = ZdChallenges::get_metadata(&APP_ID, &TARGET);
        assert_eq!(metadata.status, Status::REPLY);
        assert_eq!(metadata.progress.total, 100);
        assert_eq!(metadata.progress.done, 10);
        init_challenge(100,100, Status::FREE);
        assert_noop!(ZdChallenges::examine(
            &APP_ID,
            CHALLENGER,
            &TARGET,
            22
        ),Error::<Test>::NoChallengeAllowed);
        let metadata = ZdChallenges::get_metadata(&APP_ID, &TARGET);
        assert_eq!(metadata.status, Status::FREE);
        assert_eq!(metadata.progress.total, 100);
        assert_eq!(metadata.progress.done, 100);
    });
}

#[test]
fn reply_should_work() {
    new_test_ext().execute_with(|| {
        init_challenge(100,100, Status::EXAMINE);
        assert_ok!(ZdChallenges::reply(
            &APP_ID,
            &PATHINFER,
            &TARGET,
            100,
            12,
            |is_all_done, _, _| -> Result<u64, DispatchError> {
                assert_eq!(is_all_done, false);
                Ok(60)
            }
        ));
        let metadata = ZdChallenges::get_metadata(&APP_ID, &TARGET);
        assert_eq!(metadata.score, 60);
        assert_eq!(metadata.status, Status::REPLY);
        assert_eq!(metadata.last_update, 1);
    });
}

