#![cfg(test)]

use super::*;
use crate::mock::*;
use frame_support::{assert_noop, assert_ok, dispatch};

fn initialize_trust() {
    assert_eq!(ZdTrust::do_trust(&1,&2), Ok(()));
}

fn new_test_ext() -> sp_io::TestExternalities {
    let mut ext = ExtBuilder::default().build();
    ext.execute_with(|| System::set_block_number(1));
    ext
}
