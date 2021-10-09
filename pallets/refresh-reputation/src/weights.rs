//! Weights for zd_refresh_reputation
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 3.0.0
//! DATE: 2021-10-09, STEPS: [50, ], REPEAT: 20, LOW RANGE: [], HIGH RANGE: []
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 128

// Executed Command:
// ./target/release/zerodao-node
// benchmark
// --chain=dev
// --execution=wasm
// --wasm-execution=compiled
// --pallet=zd_refresh_reputation
// --extrinsic=*
// --steps=50
// --repeat=20
// --heap-pages=4096
// --output=./pallets/refresh-reputation/src/weights.rs
// --template=./scripts/pallet-weight-template.hbs


#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(clippy::unnecessary_cast)]

use frame_support::{
    traits::Get,
    weights::{constants::RocksDbWeight, Weight},
};
use sp_std::marker::PhantomData;

/// Weight functions needed for zd_refresh_reputation.
pub trait WeightInfo {
    fn start() -> Weight;
    fn refresh(a: u32, ) -> Weight;
    fn harvest_ref_all() -> Weight;
    fn harvest_ref_all_sweeper() -> Weight;
    fn challenge() -> Weight;
    fn challenge_update(a: u32, ) -> Weight;
    fn harvest_challenge() -> Weight;
    fn arbitral(a: u32, ) -> Weight;
}

/// Weights for zd_refresh_reputation using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn start() -> Weight {
        (840_100_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(27 as Weight))
            .saturating_add(T::DbWeight::get().writes(24 as Weight))
    }
    fn refresh(a: u32, ) -> Weight {
        (0 as Weight)
            .saturating_add((4_300_286_000 as Weight).saturating_mul(a as Weight))
            .saturating_add(T::DbWeight::get().reads(7 as Weight))
            .saturating_add(T::DbWeight::get().reads((604 as Weight).saturating_mul(a as Weight)))
            .saturating_add(T::DbWeight::get().writes(6 as Weight))
            .saturating_add(T::DbWeight::get().writes((602 as Weight).saturating_mul(a as Weight)))
    }
    fn harvest_ref_all() -> Weight {
        (623_700_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(5 as Weight))
            .saturating_add(T::DbWeight::get().writes(503 as Weight))
    }
    fn harvest_ref_all_sweeper() -> Weight {
        (692_300_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(6 as Weight))
            .saturating_add(T::DbWeight::get().writes(504 as Weight))
    }
    fn challenge() -> Weight {
        (152_700_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(11 as Weight))
            .saturating_add(T::DbWeight::get().writes(7 as Weight))
    }
    fn challenge_update(a: u32, ) -> Weight {
        (33_367_000 as Weight)
            .saturating_add((9_208_000 as Weight).saturating_mul(a as Weight))
            .saturating_add(T::DbWeight::get().reads(3 as Weight))
            .saturating_add(T::DbWeight::get().reads((1 as Weight).saturating_mul(a as Weight)))
            .saturating_add(T::DbWeight::get().writes(2 as Weight))
            .saturating_add(T::DbWeight::get().writes((1 as Weight).saturating_mul(a as Weight)))
    }
    fn harvest_challenge() -> Weight {
        (149_400_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(6 as Weight))
            .saturating_add(T::DbWeight::get().writes(5 as Weight))
    }
    fn arbitral(a: u32, ) -> Weight {
        (228_473_000 as Weight)
            .saturating_add((78_882_000 as Weight).saturating_mul(a as Weight))
            .saturating_add(T::DbWeight::get().reads(21 as Weight))
            .saturating_add(T::DbWeight::get().reads((4 as Weight).saturating_mul(a as Weight)))
            .saturating_add(T::DbWeight::get().writes(8 as Weight))
            .saturating_add(T::DbWeight::get().writes((1 as Weight).saturating_mul(a as Weight)))
    }
}

// For backwards compatibility and tests
impl WeightInfo for () {
    fn start() -> Weight {
        (840_100_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(27 as Weight))
            .saturating_add(RocksDbWeight::get().writes(24 as Weight))
    }
    fn refresh(a: u32, ) -> Weight {
        (0 as Weight)
            .saturating_add((4_300_286_000 as Weight).saturating_mul(a as Weight))
            .saturating_add(RocksDbWeight::get().reads(7 as Weight))
            .saturating_add(RocksDbWeight::get().reads((604 as Weight).saturating_mul(a as Weight)))
            .saturating_add(RocksDbWeight::get().writes(6 as Weight))
            .saturating_add(RocksDbWeight::get().writes((602 as Weight).saturating_mul(a as Weight)))
    }
    fn harvest_ref_all() -> Weight {
        (623_700_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(5 as Weight))
            .saturating_add(RocksDbWeight::get().writes(503 as Weight))
    }
    fn harvest_ref_all_sweeper() -> Weight {
        (692_300_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(6 as Weight))
            .saturating_add(RocksDbWeight::get().writes(504 as Weight))
    }
    fn challenge() -> Weight {
        (152_700_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(11 as Weight))
            .saturating_add(RocksDbWeight::get().writes(7 as Weight))
    }
    fn challenge_update(a: u32, ) -> Weight {
        (33_367_000 as Weight)
            .saturating_add((9_208_000 as Weight).saturating_mul(a as Weight))
            .saturating_add(RocksDbWeight::get().reads(3 as Weight))
            .saturating_add(RocksDbWeight::get().reads((1 as Weight).saturating_mul(a as Weight)))
            .saturating_add(RocksDbWeight::get().writes(2 as Weight))
            .saturating_add(RocksDbWeight::get().writes((1 as Weight).saturating_mul(a as Weight)))
    }
    fn harvest_challenge() -> Weight {
        (149_400_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(6 as Weight))
            .saturating_add(RocksDbWeight::get().writes(5 as Weight))
    }
    fn arbitral(a: u32, ) -> Weight {
        (228_473_000 as Weight)
            .saturating_add((78_882_000 as Weight).saturating_mul(a as Weight))
            .saturating_add(RocksDbWeight::get().reads(21 as Weight))
            .saturating_add(RocksDbWeight::get().reads((4 as Weight).saturating_mul(a as Weight)))
            .saturating_add(RocksDbWeight::get().writes(8 as Weight))
            .saturating_add(RocksDbWeight::get().writes((1 as Weight).saturating_mul(a as Weight)))
    }
}
