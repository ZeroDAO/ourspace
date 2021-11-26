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

//! Weights for zd_refresh_seeds
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 3.0.0
//! DATE: 2021-11-26, STEPS: [50, ], REPEAT: 20, LOW RANGE: [], HIGH RANGE: []
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


#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(clippy::unnecessary_cast)]
#[no_coverage]

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
        (28_400_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(1 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn add() -> Weight {
        (119_100_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(7 as Weight))
            .saturating_add(T::DbWeight::get().writes(7 as Weight))
    }
    fn challenge() -> Weight {
        (132_300_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(8 as Weight))
            .saturating_add(T::DbWeight::get().writes(8 as Weight))
    }
    fn examine() -> Weight {
        (90_400_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(5 as Weight))
            .saturating_add(T::DbWeight::get().writes(3 as Weight))
    }
    fn reply_hash(a: u32, ) -> Weight {
        (118_097_000 as Weight)
            .saturating_add((155_000 as Weight).saturating_mul(a as Weight))
            .saturating_add(T::DbWeight::get().reads(3 as Weight))
            .saturating_add(T::DbWeight::get().writes(3 as Weight))
    }
    fn reply_hash_next(a: u32, ) -> Weight {
        (106_768_000 as Weight)
            .saturating_add((88_000 as Weight).saturating_mul(a as Weight))
            .saturating_add(T::DbWeight::get().reads(4 as Weight))
            .saturating_add(T::DbWeight::get().writes(3 as Weight))
    }
    fn reply_path(a: u32, ) -> Weight {
        (0 as Weight)
            .saturating_add((86_072_000 as Weight).saturating_mul(a as Weight))
            .saturating_add(T::DbWeight::get().reads(8 as Weight))
            .saturating_add(T::DbWeight::get().reads((2 as Weight).saturating_mul(a as Weight)))
            .saturating_add(T::DbWeight::get().writes(3 as Weight))
    }
    fn reply_path_next(a: u32, ) -> Weight {
        (0 as Weight)
            .saturating_add((79_266_000 as Weight).saturating_mul(a as Weight))
            .saturating_add(T::DbWeight::get().reads(9 as Weight))
            .saturating_add(T::DbWeight::get().reads((2 as Weight).saturating_mul(a as Weight)))
            .saturating_add(T::DbWeight::get().writes(4 as Weight))
    }
    fn reply_num(a: u32, ) -> Weight {
        (0 as Weight)
            .saturating_add((38_006_000 as Weight).saturating_mul(a as Weight))
            .saturating_add(T::DbWeight::get().reads(7 as Weight))
            .saturating_add(T::DbWeight::get().reads((2 as Weight).saturating_mul(a as Weight)))
            .saturating_add(T::DbWeight::get().writes(2 as Weight))
    }
    fn evidence_of_shorter() -> Weight {
        (116_200_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(10 as Weight))
            .saturating_add(T::DbWeight::get().writes(7 as Weight))
    }
    fn number_too_low(a: u32, ) -> Weight {
        (0 as Weight)
            .saturating_add((39_172_000 as Weight).saturating_mul(a as Weight))
            .saturating_add(T::DbWeight::get().reads(10 as Weight))
            .saturating_add(T::DbWeight::get().reads((2 as Weight).saturating_mul(a as Weight)))
            .saturating_add(T::DbWeight::get().writes(7 as Weight))
    }
    fn missed_in_hashs() -> Weight {
        (111_700_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(9 as Weight))
            .saturating_add(T::DbWeight::get().writes(3 as Weight))
    }
    fn missed_in_paths() -> Weight {
        (288_400_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(13 as Weight))
            .saturating_add(T::DbWeight::get().writes(7 as Weight))
    }
    fn invalid_evidence() -> Weight {
        (128_000_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(6 as Weight))
            .saturating_add(T::DbWeight::get().writes(2 as Weight))
    }
    fn harvest_challenge() -> Weight {
        (335_400_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(7 as Weight))
            .saturating_add(T::DbWeight::get().writes(9 as Weight))
    }
    fn harvest_seed() -> Weight {
        (299_800_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(12 as Weight))
            .saturating_add(T::DbWeight::get().writes(9 as Weight))
    }
}

// For backwards compatibility and tests
impl WeightInfo for () {
    fn start() -> Weight {
        (28_400_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(1 as Weight))
            .saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
    fn add() -> Weight {
        (119_100_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(7 as Weight))
            .saturating_add(RocksDbWeight::get().writes(7 as Weight))
    }
    fn challenge() -> Weight {
        (132_300_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(8 as Weight))
            .saturating_add(RocksDbWeight::get().writes(8 as Weight))
    }
    fn examine() -> Weight {
        (90_400_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(5 as Weight))
            .saturating_add(RocksDbWeight::get().writes(3 as Weight))
    }
    fn reply_hash(a: u32, ) -> Weight {
        (118_097_000 as Weight)
            .saturating_add((155_000 as Weight).saturating_mul(a as Weight))
            .saturating_add(RocksDbWeight::get().reads(3 as Weight))
            .saturating_add(RocksDbWeight::get().writes(3 as Weight))
    }
    fn reply_hash_next(a: u32, ) -> Weight {
        (106_768_000 as Weight)
            .saturating_add((88_000 as Weight).saturating_mul(a as Weight))
            .saturating_add(RocksDbWeight::get().reads(4 as Weight))
            .saturating_add(RocksDbWeight::get().writes(3 as Weight))
    }
    fn reply_path(a: u32, ) -> Weight {
        (0 as Weight)
            .saturating_add((86_072_000 as Weight).saturating_mul(a as Weight))
            .saturating_add(RocksDbWeight::get().reads(8 as Weight))
            .saturating_add(RocksDbWeight::get().reads((2 as Weight).saturating_mul(a as Weight)))
            .saturating_add(RocksDbWeight::get().writes(3 as Weight))
    }
    fn reply_path_next(a: u32, ) -> Weight {
        (0 as Weight)
            .saturating_add((79_266_000 as Weight).saturating_mul(a as Weight))
            .saturating_add(RocksDbWeight::get().reads(9 as Weight))
            .saturating_add(RocksDbWeight::get().reads((2 as Weight).saturating_mul(a as Weight)))
            .saturating_add(RocksDbWeight::get().writes(4 as Weight))
    }
    fn reply_num(a: u32, ) -> Weight {
        (0 as Weight)
            .saturating_add((38_006_000 as Weight).saturating_mul(a as Weight))
            .saturating_add(RocksDbWeight::get().reads(7 as Weight))
            .saturating_add(RocksDbWeight::get().reads((2 as Weight).saturating_mul(a as Weight)))
            .saturating_add(RocksDbWeight::get().writes(2 as Weight))
    }
    fn evidence_of_shorter() -> Weight {
        (116_200_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(10 as Weight))
            .saturating_add(RocksDbWeight::get().writes(7 as Weight))
    }
    fn number_too_low(a: u32, ) -> Weight {
        (0 as Weight)
            .saturating_add((39_172_000 as Weight).saturating_mul(a as Weight))
            .saturating_add(RocksDbWeight::get().reads(10 as Weight))
            .saturating_add(RocksDbWeight::get().reads((2 as Weight).saturating_mul(a as Weight)))
            .saturating_add(RocksDbWeight::get().writes(7 as Weight))
    }
    fn missed_in_hashs() -> Weight {
        (111_700_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(9 as Weight))
            .saturating_add(RocksDbWeight::get().writes(3 as Weight))
    }
    fn missed_in_paths() -> Weight {
        (288_400_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(13 as Weight))
            .saturating_add(RocksDbWeight::get().writes(7 as Weight))
    }
    fn invalid_evidence() -> Weight {
        (128_000_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(6 as Weight))
            .saturating_add(RocksDbWeight::get().writes(2 as Weight))
    }
    fn harvest_challenge() -> Weight {
        (335_400_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(7 as Weight))
            .saturating_add(RocksDbWeight::get().writes(9 as Weight))
    }
    fn harvest_seed() -> Weight {
        (299_800_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(12 as Weight))
            .saturating_add(RocksDbWeight::get().writes(9 as Weight))
    }
}
