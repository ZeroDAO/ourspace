#![cfg(test)]

use crate as zd_challenges;
use frame_support::sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};
use frame_support::{construct_runtime, parameter_types, traits::GenesisBuild};
use orml_currencies::BasicCurrencyAdapter;
use orml_traits::parameter_type_with_key;

pub use frame_system as system;
use sp_core::H256;
pub use sp_runtime::{Perbill, Permill};

pub type Amount = i128;
pub type AccountId = u64;
pub type Balance = u128;
pub type CurrencyId = u128;
pub type BlockNumber = u64;

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const CHARLIE: AccountId = 3;
pub const DAVE: AccountId = 4;
pub const EVE: AccountId = 5;
pub const FERDIE: AccountId = 6;
pub const SWEEPER: AccountId = 7;

pub const ZDAO: CurrencyId = 1;

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
        Balances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
        ZdReputation: zd_reputation::{Module, Call, Storage, Event<T>, Config<T>},
        ZdChallenges: zd_challenges::{Module, Call, Storage, Event<T>},
        ZdToken: zd_tokens::{Module, Call, Event<T>},

        Tokens: orml_tokens::{Module, Storage, Event<T>, Config<T>},
        Currencies: orml_currencies::{Module, Storage, Event<T>},
    }
);

parameter_types! {
    pub const SocialPoolAccountId: AccountId = 10000;
}

impl zd_tokens::Config for Test {
    type Event = Event;
    type CurrencyId = CurrencyId;
    type WeightInfo = ();
    type Currency = Currencies;
    type SocialPool = SocialPoolAccountId;
    type Amount = Amount;
    type BaceToken = BaceToken;
}

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

parameter_types! {
    pub const ChallengeTimeout: BlockNumber = 10;
    pub const ChallengeStakingAmount: Balance = 100;
}

impl zd_challenges::Config for Test {
    type Event = Event;
    type CurrencyId = CurrencyId;
    type ZdToken = ZdToken;
    type Reputation = ZdReputation;
    type ChallengeStakingAmount = ChallengeStakingAmount;
    type ChallengeTimeout = ChallengeTimeout;
}

parameter_types! {
    pub const ExistentialDeposit: u128 = 500;
    pub const MaxLocks: u32 = 50;
    pub const BaceToken: CurrencyId = ZDAO;
}

parameter_types! {
    pub const GetNativeCurrencyId: CurrencyId = 0;
}

impl orml_currencies::Config for Test {
    type Event = Event;
    type MultiCurrency = Tokens;
    type NativeCurrency = BasicCurrencyAdapter<Test, Balances, Amount, BlockNumber>;
    type GetNativeCurrencyId = GetNativeCurrencyId;
    type WeightInfo = ();
}

parameter_type_with_key! {
    pub ExistentialDeposits: |currency_id: CurrencyId| -> Balance {
        match currency_id {
			&ZDAO => 1,
			_ => 0,
		}
    };
}

impl orml_tokens::Config for Test {
    type Event = Event;
    type Balance = Balance;
    type Amount = Amount;
    type CurrencyId = CurrencyId;
    type WeightInfo = ();
    type ExistentialDeposits = ExistentialDeposits;
    type OnDust = ();
}

impl pallet_balances::Config for Test {
    type MaxLocks = MaxLocks;
    /// The type for recording an account's balance.
    type Balance = Balance;
    /// The ubiquitous event type.
    type Event = Event;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Test>;
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
    type AccountData = pallet_balances::AccountData<u128>;
}

pub struct ExtBuilder {
    period: BlockNumber,
    endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            period: INIT_PERIOD,
            endowed_accounts: vec![
                (ALICE, ZDAO, 1000_000_000_000_000u128),
                (BOB, ZDAO, 1000_000_000_000_000u128),
                (FERDIE, ZDAO, 1000_000_000_000_000u128),
                (SWEEPER, ZDAO, 1000_000_000_000_000u128),
            ],
        }
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

        orml_tokens::GenesisConfig::<Test> {
            endowed_accounts: self.endowed_accounts,
        }
        .assimilate_storage(&mut t)
        .unwrap();

        t.into()
    }
}
