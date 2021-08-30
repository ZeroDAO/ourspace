#![cfg(test)]

use super::*;
use crate::mock::{Event, *};
use frame_support::assert_ok;

fn new_test_ext() -> sp_io::TestExternalities {
    let mut ext = ExtBuilder::default().build();
    ext.execute_with(|| System::set_block_number(1));
    ext
}

