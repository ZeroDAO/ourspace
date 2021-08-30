#![cfg(test)]

use super::*;
use crate::mock::{Event, *};
use frame_support::assert_ok;

fn new_test_ext() -> sp_io::TestExternalities {
    let mut ext = ExtBuilder::default().build();
    ext.execute_with(|| System::set_block_number(1));
    ext
}

fn init_graph() {
    /* 
    //       B
    //   ↗  ↓  ↘  
    // A     E <-  D
    //   ↘     ↗
    //       C
    */
    let paths = vec![[A, B], [A, C], [B, D], [B, E], [D, E], [C, D]];
    for path in paths {
        assert_ok!(ZdTrust::trust(Origin::signed(path[0]), path[1]));
    }

    // 
}

#[test]
fn start_should_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(ZdRefreshSeeds::start(
            Origin::signed(CHALLENGER),
        ));
    });
}
