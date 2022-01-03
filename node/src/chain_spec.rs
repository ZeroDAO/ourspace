use sc_service::{ChainType, Properties};
use sp_core::{crypto::UncheckedInto, sr25519, Pair, Public};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{IdentifyAccount, Verify};

use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_runtime::Perbill;
use zerodao::ContractsConfig;
use zerodao::{
    opaque::SessionKeys, AccountId, BabeConfig, Balance, BalancesConfig, BlockNumber,
    CouncilConfig, CurrencyId, GenesisConfig, GrandpaConfig, ImOnlineConfig, SessionConfig,
    Signature, StakerStatus, StakingConfig, SudoConfig, SystemConfig, TechnicalCommitteeConfig,
    TokensConfig, ZdReputationConfig, WASM_BINARY,
};

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

type AccountPublic = <Signature as Verify>::Signer;

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

fn session_keys(
    babe: BabeId,
    grandpa: GrandpaId,
    im_online: ImOnlineId,
    authority_discovery: AuthorityDiscoveryId,
) -> SessionKeys {
    SessionKeys {
        babe,
        grandpa,
        im_online,
        authority_discovery,
    }
}

/// Generate an Babe authority key.
pub fn authority_keys_from_seed(
    s: &str,
) -> (
    AccountId,
    AccountId,
    BabeId,
    GrandpaId,
    ImOnlineId,
    AuthorityDiscoveryId,
) {
    (
        get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", s)),
        get_account_id_from_seed::<sr25519::Public>(s),
        get_from_seed::<BabeId>(s),
        get_from_seed::<GrandpaId>(s),
        get_from_seed::<ImOnlineId>(s),
        get_from_seed::<AuthorityDiscoveryId>(s),
    )
}

pub fn development_config() -> Result<ChainSpec, String> {
    let wasm_binary =
        WASM_BINARY.ok_or_else(|| "Development wasm binary not available".to_string())?;

    Ok(ChainSpec::from_genesis(
        // Name
        "ZeroDAO Network",
        // ID
        "ZeroDAO",
        ChainType::Development,
        move || {
            testnet_genesis(
                wasm_binary,
                // Initial PoA authorities
                vec![authority_keys_from_seed("Alice")],
                100,
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                // Pre-funded accounts
                vec![
                    (
                        get_account_id_from_seed::<sr25519::Public>("Alice"),
                        10000000u128.pow(12),
                    ),
                    (
                        get_account_id_from_seed::<sr25519::Public>("Bob"),
                        10000000u128.pow(12),
                    ),
                    (
                        get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
                        10000u128.pow(12),
                    ),
                    (
                        get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
                        10000u128.pow(12),
                    ),
                ],
                true,
            )
        },
        // Bootnodes
        vec![],
        // Telemetry
        None,
        // Protocol ID
        None,
        // Properties
        Some(zd_properties()),
        // Extensions
        None,
    ))
}

pub fn local_testnet_config() -> Result<ChainSpec, String> {
    let wasm_binary =
        WASM_BINARY.ok_or_else(|| "Development wasm binary not available".to_string())?;

    Ok(ChainSpec::from_genesis(
        // Name
        "ZeroDAO Dev",
        // ID
        "zeroDAO_dev",
        ChainType::Local,
        move || {
            testnet_genesis(
                wasm_binary,
                // Initial PoA authorities
                vec![
                    authority_keys_from_seed("Alice"),
                    authority_keys_from_seed("Bob"),
                ],
                100,
                // Sudo account
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                // Pre-funded accounts
                vec![
                    (
                        get_account_id_from_seed::<sr25519::Public>("Alice"),
                        50000000u128.pow(12),
                    ),
                    (
                        get_account_id_from_seed::<sr25519::Public>("Bob"),
                        50000000u128.pow(12),
                    ),
                    (
                        get_account_id_from_seed::<sr25519::Public>("Charlie"),
                        50000000u128.pow(12),
                    ),
                    (
                        get_account_id_from_seed::<sr25519::Public>("Dave"),
                        50000000u128.pow(12),
                    ),
                    (
                        get_account_id_from_seed::<sr25519::Public>("Eve"),
                        50000000u128.pow(12),
                    ),
                    (
                        get_account_id_from_seed::<sr25519::Public>("Ferdie"),
                        50000000u128.pow(12),
                    ),
                    (
                        get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
                        50000000u128.pow(12),
                    ),
                    (
                        get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
                        50000000u128.pow(12),
                    ),
                    (
                        get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
                        50000000u128.pow(12),
                    ),
                    (
                        get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
                        50000000u128.pow(12),
                    ),
                    (
                        get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
                        50000000u128.pow(12),
                    ),
                    (
                        get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
                        50000000u128.pow(12),
                    ),
                ],
                true,
            )
        },
        // Bootnodes
        vec![],
        // Telemetry
        None,
        // Protocol ID
        None,
        // Properties
        Some(zd_properties()),
        // Extensions
        None,
    ))
}

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
    wasm_binary: &[u8],
    initial_authorities: Vec<(
        AccountId,
        AccountId,
        BabeId,
        GrandpaId,
        ImOnlineId,
        AuthorityDiscoveryId,
    )>,
    period: BlockNumber,
    root_key: AccountId,
    endowed_accounts: Vec<(AccountId, u128)>,
    enable_println: bool,
) -> GenesisConfig {
    const STASH: Balance = 20_000;
    GenesisConfig {
        pallet_sudo: Some(SudoConfig {
            // Assign network admin rights.
            key: root_key,
        }),
        frame_system: Some(SystemConfig {
            // Add Wasm runtime to storage.
            code: wasm_binary.to_vec(),
            changes_trie_config: Default::default(),
        }),
        pallet_balances: Some(BalancesConfig {
            balances: endowed_accounts.clone(),
        }),
        pallet_grandpa: Some(GrandpaConfig {
            authorities: vec![],
        }),
        // Staking related configs
        pallet_babe: Some(BabeConfig {
            authorities: vec![],
        }),
        pallet_im_online: Some(ImOnlineConfig { keys: vec![] }),
        //pallet_treasury: Some(Default::default()),
        pallet_session: Some(SessionConfig {
            keys: initial_authorities
                .iter()
                .map(|x| {
                    (
                        x.0.clone(),
                        x.0.clone(),
                        session_keys(x.2.clone(), x.3.clone(), x.4.clone(), x.5.clone()),
                    )
                })
                .collect::<Vec<_>>(),
        }),
        pallet_staking: Some(StakingConfig {
            validator_count: initial_authorities.len() as u32 * 2,
            minimum_validator_count: initial_authorities.len() as u32,
            stakers: initial_authorities
                .iter()
                .map(|x| (x.0.clone(), x.1.clone(), STASH, StakerStatus::Validator))
                .collect(),
            invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
            slash_reward_fraction: Perbill::from_percent(10),
            ..Default::default()
        }),
        pallet_collective_Instance1: Some(CouncilConfig {
            members: vec![],
            phantom: Default::default(),
        }),
        pallet_collective_Instance2: Some(TechnicalCommitteeConfig {
            members: vec![],
            phantom: Default::default(),
        }),
        pallet_membership_Instance1: Some(Default::default()),
        pallet_elections_phragmen: Some(Default::default()),
        pallet_treasury: Some(Default::default()),
        pallet_contracts: Some(ContractsConfig {
            current_schedule: pallet_contracts::Schedule {
                enable_println,
                ..Default::default()
            },
        }),
        zd_reputation: Some(ZdReputationConfig { period }),
        orml_tokens: Some(TokensConfig {
            endowed_accounts: endowed_accounts
                .iter()
                .cloned()
                .map(|k| (k.0.clone(), CurrencyId::SOCI, k.1))
                .collect(),
        }),
    }
}

pub fn zd_properties() -> Properties {
    let mut properties = Properties::new();

    properties.insert("tokenSymbol".into(), "ZOO".into());
    properties.insert("tokenDecimals".into(), 12.into());

    properties
}
