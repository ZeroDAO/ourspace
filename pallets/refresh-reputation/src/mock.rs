#![cfg(test)]

use crate as zd_refresh_reputation;
use frame_support::sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};
use frame_support::{construct_runtime, parameter_types, traits::GenesisBuild};
use frame_system as system;
use orml_currencies::BasicCurrencyAdapter;
use orml_traits::parameter_type_with_key;
use sp_core::H256;
use sp_runtime::{traits::Zero, Perbill};
use zd_primitives::{Balance, BlockNumber};

pub type Amount = i128;
pub type AccountId = u64;
pub type CurrencyId = u128;

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const CHARLIE: AccountId = 3;
pub const DAVE: AccountId = 4;
pub const EVE: AccountId = 5;
pub const FERDIE: AccountId = 6;

pub const ZDAO: CurrencyId = 1;

pub const INIT_PERIOD: BlockNumber = 10;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.

impl pallet_balances::Config for Test {
    type MaxLocks = MaxLocks;
    type Balance = Balance;
    type Event = Event;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
}

impl orml_currencies::Config for Test {
    type Event = Event;
    type MultiCurrency = Tokens;
    type NativeCurrency = BasicCurrencyAdapter<Test, Balances, Amount, BlockNumber>;
    type GetNativeCurrencyId = BaceToken;
    type WeightInfo = ();
}

parameter_type_with_key! {
    pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
        Zero::zero()
    };
}

parameter_types! {
    pub const BaceToken: CurrencyId = ZDAO;
    pub const ConfirmationPeriod: BlockNumber = 120;
    pub const ChallengePerior: BlockNumber = 100;
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;

    pub const ShareRatio: Perbill = Perbill::from_percent(80);
    pub const FeeRation: Perbill = Perbill::from_percent(3);
    pub const SelfRation: Perbill = Perbill::from_percent(3);
    pub const MaxUpdateCount: u32 = 4;
    pub const UpdateStakingAmount: Balance = 1_000;

    pub const DampingFactor: Perbill = Perbill::from_percent(80);
    pub const ExistentialDeposit: u128 = 500;
    pub const MaxLocks: u32 = 50;

    pub const ReceiverProtectionPeriod: BlockNumber = 100;
    pub const RefRepuTiomeOut: BlockNumber = 100;
}

impl zd_seeds::Config for Test {
    type Event = Event;
    type Reputation = ZdReputation;
}

impl zd_trust::Config for Test {
    type Event = Event;
    type DampingFactor = DampingFactor;
    type SeedsBase = ZdSeeds;
    type Reputation = ZdReputation;
}

impl zd_reputation::Config for Test {
    type Event = Event;
    type ChallengePerior = ChallengePerior;
    type ConfirmationPeriod = ConfirmationPeriod;
}

impl zd_refresh_reputation::Config for Test {
    type Event = Event;
    type MultiBaseToken = ZdToken;
    type MaxUpdateCount = MaxUpdateCount;
    type UpdateStakingAmount = UpdateStakingAmount;
    type ConfirmationPeriod = ConfirmationPeriod;
    type Reputation = ZdReputation;
    type TrustBase = ZdTrust;
    type ChallengeBase = Challenges;
    type SeedsBase = ZdSeeds;
    type RefRepuTiomeOut = RefRepuTiomeOut;
}

impl zd_challenges::Config for Test {
    type Event = Event;
    type CurrencyId = CurrencyId;
    type BaceToken = BaceToken;
    type Currency = Tokens;
    type Reputation = ZdReputation;
    type ReceiverProtectionPeriod = ReceiverProtectionPeriod;
    type UpdateStakingAmount = UpdateStakingAmount;
    type ChallengePerior = ChallengePerior;
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

impl zd_tokens::Config for Test {
    type Event = Event;
    type CurrencyId = CurrencyId;
    type WeightInfo = ();
    type Currency = Currencies;
    type Balance = Balance;
    type Amount = Amount;
}

impl system::Config for Test {
    type BaseCallFilter = ();
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = BlockNumber;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
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

construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Module, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Module, Call, Storage, Event<T>},
        ZdReputation: zd_reputation::{Module, Call, Storage, Event<T>},
        ZdRefreshReputation: zd_refresh_reputation::{Module, Call, Storage, Event<T>},
        ZdSeeds: zd_seeds::{Module, Call, Storage, Event<T>},
        Currencies: orml_currencies::{Module, Event<T>},
        ZdTrust: zd_trust::{Module, Call, Event<T>},
        Tokens: orml_tokens::{Module, Storage, Event<T>, Config<T>},
        Challenges: zd_challenges::{Module, Storage, Event<T>},
        ZdToken: zd_tokens::{Module, Call, Event<T>},
    }
);

pub struct ExtBuilder {
    endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
    period: BlockNumber,
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            endowed_accounts: vec![
                (ALICE, ZDAO, 1000_000_000_000_000u128),
                (BOB, ZDAO, 1000_000_000_000_000u128),
            ],
            period: INIT_PERIOD,
        }
    }
}

impl ExtBuilder {
    pub fn build(self) -> sp_io::TestExternalities {
        let mut t = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        orml_tokens::GenesisConfig::<Test> {
            endowed_accounts: self.endowed_accounts,
        }
        .assimilate_storage(&mut t)
        .unwrap();

        zd_reputation::GenesisConfig::<Test> {
            period: self.period,
        }
        .assimilate_storage(&mut t)
        .unwrap();

        t.into()
    }
}
