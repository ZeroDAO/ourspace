// Copyright 2021 ZeroDAO
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg(test)]

use super::*;
use crate::mock::*;
use frame_support::{assert_noop, assert_ok};
use zd_primitives::{SWEEPER_PERIOD, Progress, Pool};

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
    status: ChallengeStatus::Examine,
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
    remark: u32,
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
            remark: 0,
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
                            status: ChallengeStatus::Free,
                            ..DEFAULT_METADATA
                        };
                        <Metadatas<Test>>::insert(&APP_ID,&TARGET,&init_metadata);
                        // ChallengeTimeout
                        System::set_block_number($value.set_now);
                    }
                    assert_ok!(ZdChallenges::launch(
                        &APP_ID,
                        &TARGET,
                        &Metadata {
                            pool: Pool {
                                staking: $value.staking,
                                earnings: $value.earnings,
                            },
                            progress: Progress {
                                total: $value.total,
                                done: 0,
                            },
                            challenger: CHALLENGER,
                            pathfinder: PATHINFER,
                            score: $value.score,
                            remark: $value.remark,
                            ..Metadata::default()
                        }
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
                            remark: $value.remark,
                            last_update: $value.set_now,
                            ..DEFAULT_METADATA
                        }
                    );
                    assert_eq!(ZdToken::total_staking(), staking_amount);
                    assert_eq!(ZdToken::free_balance(&CHALLENGER), 1000_000_000_000_000u128 - staking_amount);
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
        remark: 21,
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
        assert!(ZdChallenges::launch(&APP_ID,&TARGET, &Metadata {
            pool: Pool {
                staking: 1000,
                earnings: 0,
            },
            progress: Progress {
                total: 100,
                done: 0,
            },
            challenger: DAVE,
            pathfinder: PATHINFER,
            score: 10,
            remark: 0,
            ..Metadata::default()
        }).is_err());
        assert_eq!(
            ZdChallenges::get_metadata(&APP_ID, &TARGET),
            Metadata::default()
        )
    });
}

macro_rules! new_challenge_no_allowed {
    ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                new_test_ext().execute_with(|| {
                    let (total,status,now) = $value;
                    let init_metadata = Metadata {
                        progress: Progress {
                            total: total,
                            done: 0,
                        },
                        last_update: 1,
                        status: status,
                        ..DEFAULT_METADATA
                    };
                    <Metadatas<Test>>::insert(&APP_ID,&TARGET,&init_metadata);
                    // ChallengeTimeout
                    System::set_block_number(now);

                    assert_noop!(ZdChallenges::launch(
                        &APP_ID,
                        &TARGET,
                        &Metadata {
                            pool: Pool {
                                staking: 10,
                                earnings: 10,
                            },
                            progress: Progress {
                                total: 20,
                                done: 0,
                            },
                            challenger: CHALLENGER,
                            pathfinder: PATHINFER,
                            score: 10,
                            remark: 0,
                            ..Metadata::default()
                        },
                    ),Error::<Test>::NoChallengeAllowed);
                });
            }
        )*
    }
}

new_challenge_no_allowed! {
    new_challenge_no_allowed_5: (0,ChallengeStatus::Evidence,5u64),
    new_challenge_no_allowed_6: (0,ChallengeStatus::Examine,21345),
    new_challenge_no_allowed_7: (0,ChallengeStatus::Reply,51),
    new_challenge_no_allowed_8: (0,ChallengeStatus::Arbitral,533314),
}

fn init_challenge(total: u32, done: u32, status: ChallengeStatus) {
    let init_metadata = Metadata {
        progress: Progress {
            total: total,
            done: done,
        },
        status: status,
        ..DEFAULT_METADATA
    };

    <Metadatas<Test>>::mutate(&APP_ID, &TARGET, |m| *m = init_metadata);
}

#[test]
fn next_should_work() {
    new_test_ext().execute_with(|| {
        init_challenge(300, 20, ChallengeStatus::Free);
        System::set_block_number(3);
        assert_ok!(ZdChallenges::next(
            &APP_ID,
            &CHALLENGER,
            &TARGET,
            &100,
            |score, remark, is_all_done| -> Result<(u64, u32), DispatchError> {
                assert_eq!(score, DEFAULT_METADATA.score);
                assert_eq!(remark, DEFAULT_METADATA.remark);
                assert_eq!(is_all_done, false);
                Ok((211, 322))
            }
        ));
        let metadata = ZdChallenges::get_metadata(&APP_ID, &TARGET);
        assert_eq!(metadata.progress.done, 100 + 20);
        assert_eq!(metadata.score, 211);
        assert_eq!(metadata.remark, 322);
    });
}

#[test]
fn next_should_fail() {
    new_test_ext().execute_with(|| {
        init_challenge(100, 20, ChallengeStatus::Free);
        System::set_block_number(3);
        assert_noop!(
            ZdChallenges::next(
                &APP_ID,
                &EVE,
                &TARGET,
                &80,
                |score, remark, _| -> Result<(u64, u32), DispatchError> { Ok((score, remark)) }
            ),
            Error::<Test>::NoPermission
        );
        assert_noop!(
            ZdChallenges::next(
                &APP_ID,
                &CHALLENGER,
                &TARGET,
                &300,
                |score, remark, _| -> Result<(u64, u32), DispatchError> { Ok((score, remark)) }
            ),
            Error::<Test>::TooMany
        );
        System::set_block_number(200);
        assert_noop!(
            ZdChallenges::next(
                &APP_ID,
                &CHALLENGER,
                &TARGET,
                &90,
                |score, remark, _| -> Result<(u64, u32), DispatchError> { Ok((score, remark)) }
            ),
            Error::<Test>::ProgressErr
        );
        let metadata = ZdChallenges::get_metadata(&APP_ID, &TARGET);
        assert_eq!(metadata.progress.total, 100);
        assert_eq!(metadata.progress.done, 20);
    });
}

#[test]
fn examine_should_work() {
    new_test_ext().execute_with(|| {
        init_challenge(100, 100, ChallengeStatus::Reply);
        assert_ok!(ZdChallenges::examine(&APP_ID, &CHALLENGER, &TARGET, 22));
        let metadata = ZdChallenges::get_metadata(&APP_ID, &TARGET);
        assert_eq!(metadata.remark, 22);
        assert_eq!(metadata.status, ChallengeStatus::Examine);
        assert_eq!(metadata.last_update, 1);
    });
}

#[test]
fn examine_should_fail() {
    new_test_ext().execute_with(|| {
        init_challenge(100, 100, ChallengeStatus::Reply);
        assert_noop!(
            ZdChallenges::examine(&APP_ID, &EVE, &TARGET, 22),
            Error::<Test>::NoPermission
        );
        let metadata = ZdChallenges::get_metadata(&APP_ID, &TARGET);
        assert_eq!(metadata.status, ChallengeStatus::Reply);
        assert_eq!(metadata.progress.total, 100);
        assert_eq!(metadata.progress.done, 100);
        init_challenge(100, 10, ChallengeStatus::Reply);
        assert_noop!(
            ZdChallenges::examine(&APP_ID, &CHALLENGER, &TARGET, 22),
            Error::<Test>::NoChallengeAllowed
        );
        let metadata = ZdChallenges::get_metadata(&APP_ID, &TARGET);
        assert_eq!(metadata.status, ChallengeStatus::Reply);
        assert_eq!(metadata.progress.total, 100);
        assert_eq!(metadata.progress.done, 10);
        init_challenge(100, 100, ChallengeStatus::Free);
        assert_noop!(
            ZdChallenges::examine(&APP_ID, &CHALLENGER, &TARGET, 22),
            Error::<Test>::NoChallengeAllowed
        );
        let metadata = ZdChallenges::get_metadata(&APP_ID, &TARGET);
        assert_eq!(metadata.status, ChallengeStatus::Free);
        assert_eq!(metadata.progress.total, 100);
        assert_eq!(metadata.progress.done, 100);
    });
}

#[test]
fn reply_should_work() {
    new_test_ext().execute_with(|| {
        init_challenge(100, 100, ChallengeStatus::Examine);
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
        assert_eq!(metadata.status, ChallengeStatus::Reply);
        assert_eq!(metadata.last_update, 1);
    });
}

#[test]
fn reply_should_fail() {
    new_test_ext().execute_with(|| {
        init_challenge(100, 100, ChallengeStatus::Examine);
        assert_noop!(
            ZdChallenges::reply(
                &APP_ID,
                &EVE,
                &TARGET,
                100,
                12,
                |is_all_done, _, _| -> Result<u64, DispatchError> {
                    assert_eq!(is_all_done, false);
                    Ok(60)
                }
            ),
            Error::<Test>::NoPermission
        );
        init_challenge(100, 100, ChallengeStatus::Free);
        assert_noop!(
            ZdChallenges::reply(
                &APP_ID,
                &PATHINFER,
                &TARGET,
                100,
                12,
                |is_all_done, _, _| -> Result<u64, DispatchError> {
                    assert_eq!(is_all_done, false);
                    Ok(60)
                }
            ),
            Error::<Test>::StatusErr
        );
        init_challenge(100, 100, ChallengeStatus::Examine);
        assert_noop!(
            ZdChallenges::reply(
                &APP_ID,
                &PATHINFER,
                &TARGET,
                100,
                120,
                |is_all_done, _, _| -> Result<u64, DispatchError> {
                    assert_eq!(is_all_done, false);
                    Ok(60)
                }
            ),
            Error::<Test>::ProgressErr
        );
    });
}

#[test]
fn evidence_should_work() {
    new_test_ext().execute_with(|| {
        init_challenge(100, 100, ChallengeStatus::Reply);
        assert_ok!(
            ZdChallenges::evidence(
                &APP_ID,
                &CHALLENGER,
                &TARGET,
                |_, _| -> Result<bool, DispatchError> { Ok(true) }
            ),
            None
        );
        let metadata = ZdChallenges::get_metadata(&APP_ID, &TARGET);
        assert_eq!(metadata.status, ChallengeStatus::Arbitral);
        init_challenge(100, 100, ChallengeStatus::Reply);
        assert_ok!(
            ZdChallenges::evidence(
                &APP_ID,
                &CHALLENGER,
                &TARGET,
                |_, _| -> Result<bool, DispatchError> { Ok(false) }
            ),
            Some(0)
        );
        let metadata = ZdChallenges::get_metadata(&APP_ID, &TARGET);
        assert_eq!(metadata.status, ChallengeStatus::Free);
        assert_eq!(metadata.pathfinder, CHALLENGER);
        assert_eq!(metadata.joint_benefits, false);
    });
}

#[test]
fn evidence_should_fail() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            ZdChallenges::evidence(
                &APP_ID,
                &CHALLENGER,
                &TARGET,
                |_, _| -> Result<bool, DispatchError> { Ok(true) }
            ),
            Error::<Test>::NonExistent
        );
        init_challenge(100, 10, ChallengeStatus::Reply);
        assert_noop!(
            ZdChallenges::evidence(
                &APP_ID,
                &EVE,
                &TARGET,
                |_, _| -> Result<bool, DispatchError> { Ok(true) }
            ),
            Error::<Test>::NoPermission
        );
        assert_noop!(
            ZdChallenges::evidence(
                &APP_ID,
                &CHALLENGER,
                &TARGET,
                |_, _| -> Result<bool, DispatchError> { Ok(true) }
            ),
            Error::<Test>::ProgressErr
        );
        init_challenge(100, 100, ChallengeStatus::Examine);
        assert_noop!(
            ZdChallenges::evidence(
                &APP_ID,
                &CHALLENGER,
                &TARGET,
                |_, _| -> Result<bool, DispatchError> { Ok(true) }
            ),
            Error::<Test>::StatusErr
        );
    });
}

#[test]
fn arbitral_should_work() {
    new_test_ext().execute_with(|| {
        init_challenge(100, 100, ChallengeStatus::Reply);
        assert_ok!(ZdChallenges::arbitral(
            &APP_ID,
            &CHALLENGER,
            &TARGET,
            |_,_| -> Result<(bool, bool, u64), DispatchError> {
                // joint_benefits, restart, score
                Ok((true, false, 18))
            }
        ));
        let metadata = ZdChallenges::get_metadata(&APP_ID, &TARGET);
        assert_eq!(metadata.joint_benefits, true);
        assert_eq!(metadata.score, 18);

        init_challenge(100, 100, ChallengeStatus::Reply);
        System::set_block_number(9);
        assert_ok!(ZdChallenges::arbitral(
            &APP_ID,
            &CHALLENGER,
            &TARGET,
            |_,_| -> Result<(bool, bool, u64), DispatchError> {
                // joint_benefits, restart, score
                Ok((true, false, 60))
            }
        ));
        let metadata = ZdChallenges::get_metadata(&APP_ID, &TARGET);
        assert_eq!(metadata.joint_benefits, true);
        assert_eq!(metadata.score, 60);
        assert_eq!(metadata.last_update, 9);

        System::set_block_number(100);

        assert_ok!(ZdChallenges::arbitral(
            &APP_ID,
            &FERDIE,
            &TARGET,
            |_,_| -> Result<(bool, bool, u64), DispatchError> {
                // joint_benefits, restart, score
                Ok((true, false, 60))
            }
        ));

        let metadata = ZdChallenges::get_metadata(&APP_ID, &TARGET);
        assert_eq!(metadata.joint_benefits, true);
        assert_eq!(metadata.score, 60);
        assert_eq!(metadata.last_update, 100);
        let staking_amount = ZdChallenges::challenge_staking_amount();
        assert_eq!(ZdToken::total_staking(), staking_amount);
        assert_eq!(
            ZdToken::free_balance(&FERDIE),
            1000_000_000_000_000u128 - staking_amount
        );
    });
}

#[test]
fn arbitral_should_fail() {
    new_test_ext().execute_with(|| {
        init_challenge(100, 100, ChallengeStatus::Examine);
        assert_noop!(
            ZdChallenges::arbitral(
                &APP_ID,
                &CHALLENGER,
                &TARGET,
                |_,_| -> Result<(bool, bool, u64), DispatchError> {
                    // joint_benefits, restart, score
                    Ok((true, false, 60))
                }
            ),
            Error::<Test>::StatusErr
        );
        init_challenge(100, 10, ChallengeStatus::Reply);
        assert_noop!(
            ZdChallenges::arbitral(
                &APP_ID,
                &CHALLENGER,
                &TARGET,
                |_,_| -> Result<(bool, bool, u64), DispatchError> {
                    // joint_benefits, restart, score
                    Ok((true, false, 60))
                }
            ),
            Error::<Test>::ProgressErr
        );
        init_challenge(100, 100, ChallengeStatus::Reply);
        System::set_block_number(3);
        assert_noop!(
            ZdChallenges::arbitral(
                &APP_ID,
                &FERDIE,
                &TARGET,
                |_,_| -> Result<(bool, bool, u64), DispatchError> {
                    // joint_benefits, restart, score
                    Ok((true, false, 60))
                }
            ),
            Error::<Test>::NoPermission
        );
        System::set_block_number(30);
        assert!(ZdChallenges::arbitral(
            &APP_ID,
            &EVE,
            &TARGET,
            |_,_| -> Result<(bool, bool, u64), DispatchError> {
                // joint_benefits, restart, score
                Ok((true, false, 60))
            }
        )
        .is_err());
    });
}

macro_rules! settle_should_work {
    ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                new_test_ext().execute_with(|| {
                    let (now,staking,joint_benefits,restart) = $value;
                    System::set_block_number(now);
                    let init_metadata = Metadata {
                        progress: Progress {
                            total: 100,
                            done: 100,
                        },
                        pool: Pool {
                            staking: staking,
                            earnings: 200,
                        },
                        ..DEFAULT_METADATA
                    };
                    <Metadatas<Test>>::insert(&APP_ID,&TARGET,&init_metadata);
                    // init staking pool
                    assert_ok!(ZdChallenges::staking(&FERDIE, 10000000));

                    let free_balance = ZdToken::free_balance(&CHALLENGER);

                    assert_ok!(ZdChallenges::settle(
                        &APP_ID,
                        &TARGET,
                        joint_benefits,
                        restart,
                        100,
                    ));

                    let metadata = ZdChallenges::get_metadata(&APP_ID, &TARGET);

                    match restart {
                        true => {
                            if joint_benefits {
                                assert_eq!(ZdToken::free_balance(&CHALLENGER), free_balance + (staking / 2));
                                assert_eq!(metadata.pool.staking, staking - (staking / 2));
                                assert_eq!(metadata.pathfinder, PATHINFER);
                            } else {
                                assert_eq!(metadata.pathfinder, CHALLENGER);
                            }
                            assert_eq!(metadata.status, ChallengeStatus::Free);
                        },
                        false => {
                            assert_eq!(metadata.joint_benefits, joint_benefits);
                            assert_eq!(metadata.score, 100);
                        },
                    }

                    assert_eq!(metadata.last_update, now);
                });
            }
        )*
    }
}

settle_should_work! {
    // now,staking,joint_benefits,restart
    settle_should_work_0: (10,100,true,true),
    settle_should_work_1: (20,100000,true,true),
    settle_should_work_2: (100,0,true,true),
    settle_should_work_3: (10,655555,true,true),
    settle_should_work_4: (10,655551,true,true),
    settle_should_work_5: (10,100,false,true),
    settle_should_work_6: (10,100,true,false),
}

macro_rules! harvest_should_work {
    ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                new_test_ext().execute_with(|| {
                    let (who, status, done, joint_benefits,staking,now) = $value;
                    // init staking pool
                    assert_ok!(ZdChallenges::staking(&FERDIE, 10000000));
                    let init_metadata = Metadata {
                        progress: Progress {
                            total: 100,
                            done: done,
                        },
                        pool: Pool {
                            staking: staking,
                            earnings: 200,
                        },
                        status: status,
                        joint_benefits: joint_benefits,
                        ..DEFAULT_METADATA
                    };
                    <Metadatas<Test>>::insert(&APP_ID, &TARGET, &init_metadata);

                    System::set_block_number(now);

                    let total_amount = staking + 200;
                    let (sweeper_fee, awards) = if who == CHALLENGER || who == PATHINFER {
                        (0,total_amount)
                    } else {
                        total_amount.with_fee()
                    };
                    let pathfinder_balance = ZdToken::free_balance(&PATHINFER);
                    let challenger_balance = ZdToken::free_balance(&CHALLENGER);
                    let sweeper_balance = ZdToken::free_balance(&who);

                    assert_ok!(ZdChallenges::harvest(&who, &APP_ID, &TARGET));

                    match status {
                        ChallengeStatus::Free => {
                            assert_eq!(ZdToken::free_balance(&PATHINFER), pathfinder_balance + awards);
                        },
                        ChallengeStatus::Examine => {
                            assert_eq!(ZdToken::free_balance(&CHALLENGER), challenger_balance + awards);
                        },
                        ChallengeStatus::Reply => {
                            match done == 100 {
                                true => {
                                    assert_eq!(ZdToken::free_balance(&PATHINFER), pathfinder_balance + awards);
                                },
                                false => {
                                    assert_eq!(ZdToken::free_balance(&CHALLENGER), challenger_balance + awards);
                                },
                            }
                        },
                        ChallengeStatus::Evidence => {
                            match done == 100 {
                                false => {
                                    assert_eq!(ZdToken::free_balance(&PATHINFER), pathfinder_balance + awards);
                                },
                                true => {
                                    assert_eq!(ZdToken::free_balance(&CHALLENGER), challenger_balance + awards);
                                },
                            }
                        },
                        ChallengeStatus::Arbitral => {
                            match joint_benefits {
                                true => {
                                    assert_eq!(ZdToken::free_balance(&PATHINFER), pathfinder_balance + (awards / 2));
                                    assert_eq!(ZdToken::free_balance(&CHALLENGER), challenger_balance + (awards - (awards / 2)));
                                },
                                false => {
                                    assert_eq!(ZdToken::free_balance(&PATHINFER), pathfinder_balance + awards);
                                },
                            }
                        },
                    }
                    if sweeper_fee > 0 {
                        assert_eq!(ZdToken::free_balance(&who), sweeper_balance + sweeper_fee);
                    }
                });
            }
        )*
    }
}

harvest_should_work! {
    // who, status, done, joint_benefits,staking
    harvest_should_work_0: (SWEEPER,ChallengeStatus::Free,10,false,200, SWEEPER_PERIOD + 2),
    harvest_should_work_1: (PATHINFER,ChallengeStatus::Free,10,false,200221,ChallengeTimeout::get() + 2),
    harvest_should_work_2: (CHALLENGER,ChallengeStatus::Free,10,false,20784,ChallengeTimeout::get() + 2),
    harvest_should_work_3: (CHALLENGER,ChallengeStatus::Examine,10,false,10,ChallengeTimeout::get() + 2),
    harvest_should_work_4: (SWEEPER,ChallengeStatus::Examine,10,false,241111,SWEEPER_PERIOD + 2),
    harvest_should_work_5: (SWEEPER,ChallengeStatus::Reply,100,false,2345564,SWEEPER_PERIOD + 2),
    harvest_should_work_6: (PATHINFER,ChallengeStatus::Reply,100,false,22,ChallengeTimeout::get() + 2),
    harvest_should_work_7: (CHALLENGER,ChallengeStatus::Reply,10,false,46453,ChallengeTimeout::get() + 2),
    harvest_should_work_8: (CHALLENGER,ChallengeStatus::Evidence,100,false,42334,ChallengeTimeout::get() + 2),
    harvest_should_work_9: (PATHINFER,ChallengeStatus::Evidence,10,false,478786,ChallengeTimeout::get() + 2),
    harvest_should_work_10: (SWEEPER,ChallengeStatus::Evidence,10,false,45333,SWEEPER_PERIOD + 2),
    harvest_should_work_11: (SWEEPER,ChallengeStatus::Arbitral,10,true,75333,SWEEPER_PERIOD + 2),
    harvest_should_work_12: (PATHINFER,ChallengeStatus::Arbitral,10,false,46454,ChallengeTimeout::get() + 2),
    harvest_should_work_13: (SWEEPER,ChallengeStatus::Free,10,false,0,SWEEPER_PERIOD + 2),
}
