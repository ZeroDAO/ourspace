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

use crate as zd_refresh_reputation;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup}, testing::Header,
};
use frame_support::{construct_runtime, parameter_types, traits::GenesisBuild};
use frame_system as system;
use orml_currencies::BasicCurrencyAdapter;
use orml_traits::parameter_type_with_key;
use sp_core::H256;
use sp_runtime::Perbill;
use zd_primitives::Balance;

pub type Amount = i128;
pub type AccountId = u32;
pub type CurrencyId = u128;
pub type BlockNumber = u64;

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const CHARLIE: AccountId = 3;
pub const DAVE: AccountId = 4;
pub const EVE: AccountId = 5;
pub const FERDIE: AccountId = 6;
pub const CHALLENGER: AccountId = 7;
pub const PATHFINDER: AccountId = 8;
pub const SWEEPRT: AccountId = 9;

pub const SEED1: AccountId = 10;
pub const SEED2: AccountId = 11;
pub const SEED3: AccountId = 12;
pub const TARGET: AccountId = 13;

pub const SUB_CHALLENGER: AccountId = 14;

pub const SEED4: AccountId = 15;

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
	pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
		0
	};
}

parameter_types! {
    pub const BaceToken: CurrencyId = ZDAO;
    pub const ConfirmationPeriod: BlockNumber = 120;
    pub const ChallengePerior: BlockNumber = 100;
    pub const BlockHashCount: u32 = 250;
    pub const SS58Prefix: u8 = 42;

    pub const ShareRatio: Perbill = Perbill::from_percent(80);
    pub const FeeRation: Perbill = Perbill::from_percent(3);
    pub const SelfRation: Perbill = Perbill::from_percent(3);
    pub const MaxUpdateCount: u32 = 4;

    pub const DampingFactor: Perbill = Perbill::from_percent(100);
    pub const ExistentialDeposit: u128 = 500;
    pub const MaxLocks: u32 = 50;

    pub const ReceiverProtectionPeriod: BlockNumber = 100;
    pub const MaxTrustCount: u32 = 600;
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
    type MaxTrustCount = MaxTrustCount;
	type WeightInfo = ();
}

impl zd_reputation::Config for Test {
    type Event = Event;
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
    type WeightInfo = ();
}

parameter_types! {
    /// The reputation must be refreshed within this time period.
    pub const RefRepuTiomeOut: BlockNumber = 14_400;
    /// Amount needed for staking when refreshing reputation and seeds.
    pub const UpdateStakingAmount: Balance = 1_000_000_000;
	/// Response time period of challenge system.
	pub const ChallengeTimeout: BlockNumber = 100;
    /// Response time period of challenge system.
	pub const ChallengeStakingAmount: Balance = 100;
}

impl zd_challenges::Config for Test {
    type Event = Event;
    type CurrencyId = CurrencyId;
    type Reputation = ZdReputation;
    type ZdToken = ZdToken;
    type ChallengeStakingAmount = ChallengeStakingAmount;
    type ChallengeTimeout = ChallengeTimeout;
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

impl system::Config for Test {
    type BaseCallFilter = ();
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u32;
    type BlockNumber = BlockNumber;
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
        Currencies: orml_currencies::{Module, Call, Event<T>},
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
                (FERDIE, ZDAO, 1000_000_000_000_000u128),
                (PATHFINDER, ZDAO, 1000_000_000_000_000u128),
                (CHALLENGER, ZDAO, 1000_000_000_000_000u128),
                (SWEEPRT, ZDAO, 1000_000_000u128),
                (SUB_CHALLENGER, ZDAO, 1000_000_000_000_000u128),
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
