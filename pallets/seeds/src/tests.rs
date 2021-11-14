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
use crate::mock::{Event, *};
use frame_support::{assert_noop, assert_ok, dispatch};

fn initialize_seeds(seeds: Vec<<Test as system::Config>::AccountId>) {
    for seed in seeds.iter() {
        <ZdSeeds as SeedsBase<_>>::add_seed(seed);
    }
}

fn new_test_ext() -> sp_io::TestExternalities {
    let mut ext = ExtBuilder::default().build();
    ext.execute_with(|| System::set_block_number(1));
    ext
}

#[test]
fn get_seed_count_should_work() {
    new_test_ext().execute_with(|| {
        assert_eq!(<ZdSeeds as SeedsBase<_>>::get_seed_count(), 0);
        initialize_seeds(vec![
            1u64,
            2u64,
            3u64,
            4u64,
            5u64
        ]);
        assert_eq!(<ZdSeeds as SeedsBase<_>>::get_seed_count(), 5);
    });
}

#[test]
fn is_seed_should_work() {
    new_test_ext().execute_with(|| {
        initialize_seeds(vec![
            1u64,
        ]);
        assert_eq!(<ZdSeeds as SeedsBase<_>>::is_seed(&1u64), true);
        assert_eq!(<ZdSeeds as SeedsBase<_>>::is_seed(&2u64), false);
    });
}

#[test]
fn remove_all_should_work() {
    new_test_ext().execute_with(|| {
        initialize_seeds(vec![
            1u64,
            2u64,
            3u64,
            4u64,
            5u64,
            6u64,
        ]);
        assert_eq!(<ZdSeeds as SeedsBase<_>>::get_seed_count(), 6);
        <ZdSeeds as SeedsBase<_>>::remove_all();
        assert_eq!(<ZdSeeds as SeedsBase<_>>::get_seed_count(), 0);
        let seeds_event = Event::zd_seeds(crate::Event::SeedAdded(1));
        assert!(System::events().iter().any(|record| record.event == seeds_event));
    });
}

#[test]
fn new_seed_test() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            ZdSeeds::new_seed(Origin::signed(ALICE), ALICE),
            dispatch::DispatchError::BadOrigin
        );
        assert_ok!(ZdSeeds::new_seed(Origin::root(), ALICE));
        assert_eq!(ZdSeeds::is_seed(&ALICE), true);
        assert_noop!(
            ZdSeeds::new_seed(Origin::root(), ALICE),
            Error::<Test>::AlreadySeedUser
        );
        ZdReputation::set_step(&TIRStep::Reputation);
        assert_noop!(
            ZdSeeds::new_seed(Origin::root(), BOB),
            Error::<Test>::StatusErr
        );
    });
}

#[test]
fn remove_seed_test() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            ZdSeeds::remove_seed(Origin::signed(ALICE), ALICE),
            dispatch::DispatchError::BadOrigin
        );
        assert_noop!(
            ZdSeeds::remove_seed(Origin::root(), ALICE),
            Error::<Test>::NotSeedUser
        );
        assert_ok!(ZdSeeds::new_seed(Origin::root(), ALICE));
        assert_ok!(ZdSeeds::new_seed(Origin::root(), BOB));
        assert_ok!(ZdSeeds::remove_seed(Origin::root(), ALICE));

        let seeds_event = Event::zd_seeds(crate::Event::SeedRemoved(ALICE));
        assert!(System::events().iter().any(|record| record.event == seeds_event));

        assert_eq!(ZdSeeds::is_seed(&ALICE), false);
        assert_eq!(ZdSeeds::get_seed_count(), 1);

        ZdReputation::set_step(&TIRStep::Reputation);
        assert_noop!(
            ZdSeeds::remove_seed(Origin::root(), BOB),
            Error::<Test>::StatusErr
        );
    });
}
