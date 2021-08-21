#![cfg(test)]

use crate as zd_trust;
use frame_support::sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};
use frame_support::{construct_runtime, parameter_types, traits::GenesisBuild};

pub use frame_system as system;
use sp_core::H256;
pub use sp_runtime::{Perbill, Permill};
pub use zd_reputation;

pub type AccountId = u64;
pub type BlockNumber = u64;

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const CHARLIE: AccountId = 3;
pub const DAVE: AccountId = 4;
pub const EVE: AccountId = 5;
pub const FERDIE: AccountId = 6;

pub const INIT_PERIOD: BlockNumber = 10;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.

construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Module, Call, Config, Storage, Event<T>},
        ZdReputation: zd_reputation::{Module, Call, Storage, Event<T>, Config<T>},
        ZdSeeds: zd_seeds::{Module, Call, Storage, Event<T>},
        ZdTrust: zd_trust::{Module, Call, Storage, Event<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

parameter_types! {
    pub const ConfirmationPeriod: BlockNumber = 120;
    pub const ChallengePerior: BlockNumber = 100;
}

impl zd_reputation::Config for Test {
    type Event = Event;
}

impl zd_seeds::Config for Test {
    type Event = Event;
    type Reputation = ZdReputation;
}

parameter_types! {
    pub const DampingFactor: Perbill = Perbill::from_percent(100);
}

impl zd_trust::Config for Test {
    type Event = Event;
    type DampingFactor = DampingFactor;
    type SeedsBase = ZdSeeds;
    type Reputation = ZdReputation;
}

impl system::Config for Test {
    type BaseCallFilter = ();
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type AccountData = ();
}

pub struct ExtBuilder {
    period: BlockNumber,
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self { period: INIT_PERIOD }
    }
}

impl ExtBuilder {
    pub fn build(self) -> sp_io::TestExternalities {
        let mut t = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        zd_reputation::GenesisConfig::<Test> {
            period: self.period,
        }
        .assimilate_storage(&mut t)
        .unwrap();

        t.into()
    }
}
