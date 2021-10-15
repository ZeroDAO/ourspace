//! Weights for zd_refresh_seeds
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 3.0.0
//! DATE: 2021-10-15, STEPS: [50, ], REPEAT: 20, LOW RANGE: [], HIGH RANGE: []
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 128

// Executed Command:
// ./target/release/zerodao-node
// benchmark
// --chain=dev
// --execution=wasm
// --wasm-execution=compiled
// --pallet=zd_refresh_seeds
// --extrinsic=*
// --steps=50
// --repeat=20
// --heap-pages=4096
// --output=./pallets/refresh-seeds/src/weights.rs
// --template=./scripts/pallet-weight-template.hbs


#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(clippy::unnecessary_cast)]

use frame_support::{
    traits::Get,
    weights::{constants::RocksDbWeight, Weight},
};
use sp_std::marker::PhantomData;

/// Weight functions needed for zd_refresh_seeds.
pub trait WeightInfo {
    fn start() -> Weight;
    fn add() -> Weight;
    fn challenge() -> Weight;
    fn examine() -> Weight;
    fn reply_hash(a: u32, ) -> Weight;
    fn reply_hash_next(a: u32, ) -> Weight;
    fn reply_path(a: u32, ) -> Weight;
    fn reply_path_next(a: u32, ) -> Weight;
    fn reply_num(a: u32, ) -> Weight;
    fn evidence_of_shorter() -> Weight;
    fn number_too_low(a: u32, ) -> Weight;
    fn missed_in_hashs() -> Weight;
    fn missed_in_paths() -> Weight;
    fn invalid_evidence() -> Weight;
    fn harvest_challenge() -> Weight;
    fn harvest_seed() -> Weight;
}

/// Weights for zd_refresh_seeds using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn start() -> Weight {
        (25_400_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(1 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn add() -> Weight {
        (110_400_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(7 as Weight))
            .saturating_add(T::DbWeight::get().writes(7 as Weight))
    }
    fn challenge() -> Weight {
        (117_300_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(8 as Weight))
            .saturating_add(T::DbWeight::get().writes(8 as Weight))
    }
    fn examine() -> Weight {
        (110_200_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(5 as Weight))
            .saturating_add(T::DbWeight::get().writes(3 as Weight))
    }
    fn reply_hash(a: u32, ) -> Weight {
        (165_184_000 as Weight)
            .saturating_add((311_000 as Weight).saturating_mul(a as Weight))
            .saturating_add(T::DbWeight::get().reads(3 as Weight))
            .saturating_add(T::DbWeight::get().writes(3 as Weight))
    }
    fn reply_hash_next(a: u32, ) -> Weight {
        (144_403_000 as Weight)
            .saturating_add((204_000 as Weight).saturating_mul(a as Weight))
            .saturating_add(T::DbWeight::get().reads(4 as Weight))
            .saturating_add(T::DbWeight::get().writes(3 as Weight))
    }
    fn reply_path(a: u32, ) -> Weight {
        (0 as Weight)
            .saturating_add((81_709_000 as Weight).saturating_mul(a as Weight))
            .saturating_add(T::DbWeight::get().reads(8 as Weight))
            .saturating_add(T::DbWeight::get().reads((2 as Weight).saturating_mul(a as Weight)))
            .saturating_add(T::DbWeight::get().writes(3 as Weight))
    }
    fn reply_path_next(a: u32, ) -> Weight {
        (0 as Weight)
            .saturating_add((82_553_000 as Weight).saturating_mul(a as Weight))
            .saturating_add(T::DbWeight::get().reads(9 as Weight))
            .saturating_add(T::DbWeight::get().reads((2 as Weight).saturating_mul(a as Weight)))
            .saturating_add(T::DbWeight::get().writes(4 as Weight))
    }
    fn reply_num(a: u32, ) -> Weight {
        (0 as Weight)
            .saturating_add((34_567_000 as Weight).saturating_mul(a as Weight))
            .saturating_add(T::DbWeight::get().reads(7 as Weight))
            .saturating_add(T::DbWeight::get().reads((2 as Weight).saturating_mul(a as Weight)))
            .saturating_add(T::DbWeight::get().writes(2 as Weight))
    }
    fn evidence_of_shorter() -> Weight {
        (113_000_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(10 as Weight))
            .saturating_add(T::DbWeight::get().writes(7 as Weight))
    }
    fn number_too_low(a: u32, ) -> Weight {
        (0 as Weight)
            .saturating_add((34_532_000 as Weight).saturating_mul(a as Weight))
            .saturating_add(T::DbWeight::get().reads(10 as Weight))
            .saturating_add(T::DbWeight::get().reads((2 as Weight).saturating_mul(a as Weight)))
            .saturating_add(T::DbWeight::get().writes(7 as Weight))
    }
    fn missed_in_hashs() -> Weight {
        (116_500_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(9 as Weight))
            .saturating_add(T::DbWeight::get().writes(3 as Weight))
    }
    fn missed_in_paths() -> Weight {
        (262_900_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(13 as Weight))
            .saturating_add(T::DbWeight::get().writes(7 as Weight))
    }
    fn invalid_evidence() -> Weight {
        (58_100_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(6 as Weight))
            .saturating_add(T::DbWeight::get().writes(2 as Weight))
    }
    fn harvest_challenge() -> Weight {
        (152_500_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(7 as Weight))
            .saturating_add(T::DbWeight::get().writes(9 as Weight))
    }
    fn harvest_seed() -> Weight {
        (243_600_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(13 as Weight))
            .saturating_add(T::DbWeight::get().writes(10 as Weight))
    }
}

// For backwards compatibility and tests
impl WeightInfo for () {
    fn start() -> Weight {
        (25_400_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(1 as Weight))
            .saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
    fn add() -> Weight {
        (110_400_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(7 as Weight))
            .saturating_add(RocksDbWeight::get().writes(7 as Weight))
    }
    fn challenge() -> Weight {
        (117_300_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(8 as Weight))
            .saturating_add(RocksDbWeight::get().writes(8 as Weight))
    }
    fn examine() -> Weight {
        (110_200_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(5 as Weight))
            .saturating_add(RocksDbWeight::get().writes(3 as Weight))
    }
    fn reply_hash(a: u32, ) -> Weight {
        (165_184_000 as Weight)
            .saturating_add((311_000 as Weight).saturating_mul(a as Weight))
            .saturating_add(RocksDbWeight::get().reads(3 as Weight))
            .saturating_add(RocksDbWeight::get().writes(3 as Weight))
    }
    fn reply_hash_next(a: u32, ) -> Weight {
        (144_403_000 as Weight)
            .saturating_add((204_000 as Weight).saturating_mul(a as Weight))
            .saturating_add(RocksDbWeight::get().reads(4 as Weight))
            .saturating_add(RocksDbWeight::get().writes(3 as Weight))
    }
    fn reply_path(a: u32, ) -> Weight {
        (0 as Weight)
            .saturating_add((81_709_000 as Weight).saturating_mul(a as Weight))
            .saturating_add(RocksDbWeight::get().reads(8 as Weight))
            .saturating_add(RocksDbWeight::get().reads((2 as Weight).saturating_mul(a as Weight)))
            .saturating_add(RocksDbWeight::get().writes(3 as Weight))
    }
    fn reply_path_next(a: u32, ) -> Weight {
        (0 as Weight)
            .saturating_add((82_553_000 as Weight).saturating_mul(a as Weight))
            .saturating_add(RocksDbWeight::get().reads(9 as Weight))
            .saturating_add(RocksDbWeight::get().reads((2 as Weight).saturating_mul(a as Weight)))
            .saturating_add(RocksDbWeight::get().writes(4 as Weight))
    }
    fn reply_num(a: u32, ) -> Weight {
        (0 as Weight)
            .saturating_add((34_567_000 as Weight).saturating_mul(a as Weight))
            .saturating_add(RocksDbWeight::get().reads(7 as Weight))
            .saturating_add(RocksDbWeight::get().reads((2 as Weight).saturating_mul(a as Weight)))
            .saturating_add(RocksDbWeight::get().writes(2 as Weight))
    }
    fn evidence_of_shorter() -> Weight {
        (113_000_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(10 as Weight))
            .saturating_add(RocksDbWeight::get().writes(7 as Weight))
    }
    fn number_too_low(a: u32, ) -> Weight {
        (0 as Weight)
            .saturating_add((34_532_000 as Weight).saturating_mul(a as Weight))
            .saturating_add(RocksDbWeight::get().reads(10 as Weight))
            .saturating_add(RocksDbWeight::get().reads((2 as Weight).saturating_mul(a as Weight)))
            .saturating_add(RocksDbWeight::get().writes(7 as Weight))
    }
    fn missed_in_hashs() -> Weight {
        (116_500_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(9 as Weight))
            .saturating_add(RocksDbWeight::get().writes(3 as Weight))
    }
    fn missed_in_paths() -> Weight {
        (262_900_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(13 as Weight))
            .saturating_add(RocksDbWeight::get().writes(7 as Weight))
    }
    fn invalid_evidence() -> Weight {
        (58_100_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(6 as Weight))
            .saturating_add(RocksDbWeight::get().writes(2 as Weight))
    }
    fn harvest_challenge() -> Weight {
        (152_500_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(7 as Weight))
            .saturating_add(RocksDbWeight::get().writes(9 as Weight))
    }
    fn harvest_seed() -> Weight {
        (243_600_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(13 as Weight))
            .saturating_add(RocksDbWeight::get().writes(10 as Weight))
    }
}
