//! Weights for zd_tokens
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 3.0.0
//! DATE: 2021-10-28, STEPS: [50, ], REPEAT: 20, LOW RANGE: [], HIGH RANGE: []
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 128

// Executed Command:
// ./target/release/zerodao-node
// benchmark
// --chain=dev
// --execution=wasm
// --wasm-execution=compiled
// --pallet=zd_tokens
// --extrinsic=*
// --steps=50
// --repeat=20
// --heap-pages=4096
// --output=./pallets/tokens/src/weights.rs
// --template=./scripts/pallet-weight-template.hbs


#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(clippy::unnecessary_cast)]

use frame_support::{
    traits::Get,
    weights::{constants::RocksDbWeight, Weight},
};
use sp_std::marker::PhantomData;

/// Weight functions needed for zd_tokens.
pub trait WeightInfo {
    fn transfer_social() -> Weight;
    fn claim() -> Weight;
}

/// Weights for zd_tokens using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn transfer_social() -> Weight {
        (100_400_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(4 as Weight))
            .saturating_add(T::DbWeight::get().writes(4 as Weight))
    }
    fn claim() -> Weight {
        (87_400_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(3 as Weight))
            .saturating_add(T::DbWeight::get().writes(3 as Weight))
    }
}

// For backwards compatibility and tests
impl WeightInfo for () {
    fn transfer_social() -> Weight {
        (100_400_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(4 as Weight))
            .saturating_add(RocksDbWeight::get().writes(4 as Weight))
    }
    fn claim() -> Weight {
        (87_400_000 as Weight)
            .saturating_add(RocksDbWeight::get().reads(3 as Weight))
            .saturating_add(RocksDbWeight::get().writes(3 as Weight))
    }
}
