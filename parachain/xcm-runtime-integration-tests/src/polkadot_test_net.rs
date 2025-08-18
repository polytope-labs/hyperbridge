#![cfg(test)]
use polkadot_sdk::*;
use frame_support::{
    assert_ok,
    sp_runtime::{
        traits::{AccountIdConversion, Block as BlockT, Dispatchable},
        BuildStorage, FixedU128, Permill,
    },
    traits::{GetCallMetadata, OnInitialize},
};
pub use gargantua_runtime::{AccountId, Treasury, VestingPalletId};

use cumulus_primitives_core::ParaId;
use cumulus_test_relay_sproof_builder::RelayStateSproofBuilder;
use frame_support::traits::OnRuntimeUpgrade;
pub use frame_system::RawOrigin;
use gargantua_runtime::{RuntimeEvent, RuntimeOrigin};
use polkadot_runtime_parachains::configuration::HostConfiguration;
use sp_consensus_beefy::ecdsa_crypto::AuthorityId as BeefyId;
use sp_core::storage::Storage;
use sp_core::H160;
pub use xcm_emulator::Network;
use xcm_emulator::{decl_test_networks, decl_test_parachains, decl_test_relay_chains};


pub type BlockNumber = u32;
pub type Moment = u64;
pub type AssetId = u32;
pub type Balance = u128;

pub const ALICE: [u8; 32] = [0u8; 32];
pub const BOB: [u8; 32] = [1u8; 32];
pub const CHARLIE: [u8; 32] = [2u8; 32];
pub const DAVE: [u8; 32] = [3u8; 32];
pub const UNKNOWN: [u8; 32] = [4u8; 32];


pub const UNITS: Balance = 1_000_000_000_000;

pub const ASSET_HUB_PARA_ID: u32 = 1_000;
pub const HYPERBRIDGE_PARA_ID: u32 = 2_000;

pub const ALICE_INITIAL_NATIVE_BALANCE: Balance = 1_000 * UNITS;
pub const ALICE_INITIAL_DAI_BALANCE: Balance = 2_000 * UNITS;
pub const ALICE_INITIAL_LRNA_BALANCE: Balance = 200 * UNITS;
pub const ALICE_INITIAL_DOT_BALANCE: Balance = 2_000 * UNITS;
pub const BOB_INITIAL_NATIVE_BALANCE: Balance = 1_000 * UNITS;
pub const BOB_INITIAL_LRNA_BALANCE: Balance = 1_000 * UNITS;
pub const BOB_INITIAL_DAI_BALANCE: Balance = 1_000_000_000 * UNITS;
pub const CHARLIE_INITIAL_NATIVE_BALANCE: Balance = 1_000 * UNITS;
pub const CHARLIE_INITIAL_LRNA_BALANCE: Balance = 1_000 * UNITS;

/*pub fn parachain_reserve_account() -> AccountId {
    polkadot_parachain::primitives::Sibling::from(ACALA_PARA_ID).into_account_truncating()
}*/

pub const BRIDGE: AssetId = 0;

pub const DAI: AssetId = 2;
pub const DOT: AssetId = 3;
pub const INSUFFICIENT_ASSET: AssetId = 500;

pub const NOW: Moment = 1689844300000; // unix time in milliseconds

pub type Rococo = RococoRelayChain<TestNet>;
pub type Hyperbridge = HyperbridgeParachain<TestNet>;
pub type AssetHub = AssetHubParachain<TestNet>;

decl_test_networks! {
	pub struct TestNet {
		relay_chain = RococoRelayChain,
		parachains = vec![
			HyperbridgeParachain,
			AssetHubParachain,
		],
		bridge = ()
	},
}

decl_test_relay_chains! {
	#[api_version(11)]
	pub struct RococoRelayChain {
		genesis = rococo::genesis(),
		on_init = {
			rococo_runtime::System::set_block_number(1);
		},
		runtime = rococo_runtime,
		core = {
			SovereignAccountOf: rococo_runtime::xcm_config::LocationConverter,
		},
		pallets = {
			XcmPallet: rococo_runtime::XcmPallet,
			Balances: rococo_runtime::Balances,
			Hrmp: rococo_runtime::Hrmp,
		}
	}
}

decl_test_parachains! {
	pub struct HyperbridgeParachain {
		genesis = gargantua::genesis(),
		on_init = {
			gargantua_runtime::System::set_block_number(1);
			gargantua_runtime::Timestamp::set_timestamp(NOW);
			gargantua_runtime::AuraExt::on_initialize(1);
		},
		runtime = gargantua_runtime,
		core = {
			XcmpMessageHandler: gargantua_runtime::XcmpQueue,
			LocationToAccountId: gargantua_runtime::xcm::LocationToAccountId,
			ParachainInfo: gargantua_runtime::ParachainInfo,
			MessageOrigin: cumulus_primitives_core::AggregateMessageOrigin,
		},
		pallets = {
			PolkadotXcm: gargantua_runtime::PolkadotXcm,
			Balances: gargantua_runtime::Balances,
		}
	},

	pub struct AssetHubParachain {
		genesis = para::genesis(ASSET_HUB_PARA_ID),
		on_init = {
			gargantua_runtime::System::set_block_number(1);
			gargantua_runtime::AuraExt::on_initialize(1);
		},
		runtime = gargantua_runtime,
		core = {
			XcmpMessageHandler: gargantua_runtime::XcmpQueue,
			LocationToAccountId: gargantua_runtime::xcm::LocationToAccountId,
			ParachainInfo: gargantua_runtime::ParachainInfo,
			MessageOrigin: cumulus_primitives_core::AggregateMessageOrigin,
		},
		pallets = {
			PolkadotXcm: gargantua_runtime::PolkadotXcm,
			Balances: gargantua_runtime::Balances,
		}
	},
}

pub mod rococo {
    use super::*;

    fn get_host_configuration() -> HostConfiguration<BlockNumber> {
        HostConfiguration {
            minimum_validation_upgrade_delay: 5,
            validation_upgrade_cooldown: 5u32,
            validation_upgrade_delay: 5,
            code_retention_period: 1200,
            max_code_size: MAX_CODE_SIZE,
            max_pov_size: MAX_POV_SIZE,
            max_head_data_size: 32 * 1024,
            max_upward_queue_count: 8,
            max_upward_queue_size: 1024 * 1024,
            max_downward_message_size: 1024,
            max_upward_message_size: 50 * 1024,
            max_upward_message_num_per_candidate: 5,
            hrmp_sender_deposit: 0,
            hrmp_recipient_deposit: 0,
            hrmp_channel_max_capacity: 8,
            hrmp_channel_max_total_size: 8 * 1024,
            hrmp_max_parachain_inbound_channels: 4,
            hrmp_channel_max_message_size: 1024 * 1024,
            hrmp_max_parachain_outbound_channels: 4,
            hrmp_max_message_num_per_candidate: 5,
            dispute_period: 6,
            no_show_slots: 2,
            n_delay_tranches: 25,
            needed_approvals: 2,
            relay_vrf_modulo_samples: 2,
            zeroth_delay_tranche_width: 0,
            ..Default::default()
        }
    }

    use sp_core::{sr25519, Pair, Public};

    use polkadot_primitives::{AssignmentId, ValidatorId};
    use sc_consensus_grandpa::AuthorityId as GrandpaId;
    use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
    use sp_consensus_babe::AuthorityId as BabeId;

    /// Helper function to generate a crypto pair from seed
    fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
        TPublic::Pair::from_string(&format!("//{}", seed), None)
            .expect("static values are valid; qed")
            .public()
    }

    #[allow(clippy::type_complexity)]
    pub fn initial_authorities() -> Vec<(
        AccountId,
        AccountId,
        BabeId,
        GrandpaId,
        ValidatorId,
        AssignmentId,
        AuthorityDiscoveryId,
        BeefyId,
    )> {
        let no_beefy = get_authority_keys_from_seed_no_beefy("Alice");
        let with_beefy = (
            no_beefy.0,
            no_beefy.1,
            no_beefy.2,
            no_beefy.3,
            no_beefy.4,
            no_beefy.5,
            no_beefy.6,
            get_from_seed::<BeefyId>("Alice"),
        );
        vec![with_beefy]
    }

    fn session_keys(
        babe: BabeId,
        grandpa: GrandpaId,
        para_validator: ValidatorId,
        para_assignment: AssignmentId,
        authority_discovery: AuthorityDiscoveryId,
        beefy: BeefyId,
    ) -> rococo_runtime::SessionKeys {
        rococo_runtime::SessionKeys {
            babe,
            grandpa,
            para_validator,
            para_assignment,
            authority_discovery,
            beefy,
        }
    }

    pub fn get_authority_keys_from_seed_no_beefy(
        seed: &str,
    ) -> (
        AccountId,
        AccountId,
        BabeId,
        GrandpaId,
        ValidatorId,
        AssignmentId,
        AuthorityDiscoveryId,
    ) {
        (
            get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
            get_account_id_from_seed::<sr25519::Public>(seed),
            get_from_seed::<BabeId>(seed),
            get_from_seed::<GrandpaId>(seed),
            get_from_seed::<ValidatorId>(seed),
            get_from_seed::<AssignmentId>(seed),
            get_from_seed::<AuthorityDiscoveryId>(seed),
        )
    }

    pub fn genesis() -> Storage {
        let genesis_config = rococo_runtime::RuntimeGenesisConfig {
            balances: rococo_runtime::BalancesConfig {
                balances: vec![
                    (AccountId::from(ALICE), 2_002 * UNITS),
                    (ParaId::from(HYPERBRIDGE_PARA_ID).into_account_truncating(), 10 * UNITS),
                ],
            },
            session: rococo_runtime::SessionConfig {
                keys: initial_authorities()
                    .iter()
                    .map(|x| {
                        (
                            x.0.clone(),
                            x.0.clone(),
                            session_keys(
                                x.2.clone(),
                                x.3.clone(),
                                x.4.clone(),
                                x.5.clone(),
                                x.6.clone(),
                                x.7.clone(),
                            ),
                        )
                    })
                    .collect::<Vec<_>>(),
                non_authority_keys: Default::default(),
            },
            configuration: rococo_runtime::ConfigurationConfig {
                config: get_host_configuration(),
            },
            xcm_pallet: rococo_runtime::XcmPalletConfig {
                safe_xcm_version: Some(3),
                ..Default::default()
            },
            babe: rococo_runtime::BabeConfig {
                authorities: Default::default(),
                epoch_config: rococo_runtime::BABE_GENESIS_EPOCH_CONFIG,
                ..Default::default()
            },
            ..Default::default()
        };

        genesis_config.build_storage().unwrap()
    }
}

use sp_core::{sr25519, Pair, Public};
use sp_runtime::{
    traits::{IdentifyAccount, Verify},
    MultiSignature,
};
type AccountPublic = <MultiSignature as Verify>::Signer;

/// Helper function to generate a crypto pair from seed
fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// Helper function to generate an account ID from seed.
fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

pub use sp_consensus_aura::sr25519::AuthorityId as AuraId;
pub mod collators {
    use super::*;

    pub fn invulnerables() -> Vec<(AccountId, AuraId)> {
        vec![
            (
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                get_from_seed::<AuraId>("Alice"),
            ),
            (
                get_account_id_from_seed::<sr25519::Public>("Bob"),
                get_from_seed::<AuraId>("Bob"),
            ),
        ]
    }
}

pub mod hyperbridge {
    use super::*;

    pub fn genesis() -> Storage {
        let stable_amount = 50_000 * UNITS * 1_000_000;
        let native_amount = 936_329_588_000_000_000;
        let dot_amount = 87_719_298_250_000_u128;
        let eth_amount = 63_750_000_000_000_000_000u128;
        let btc_amount = 1_000_000_000u128;

        let existential_deposit = NativeExistentialDeposit::get();

        let genesis_config = gargantua_runtime::RuntimeGenesisConfig {
            balances: gargantua_runtime::BalancesConfig {
                balances: vec![
                    (AccountId::from(ALICE), ALICE_INITIAL_NATIVE_BALANCE),
                    (AccountId::from(BOB), BOB_INITIAL_NATIVE_BALANCE),
                    (AccountId::from(CHARLIE), CHARLIE_INITIAL_NATIVE_BALANCE),
                    (AccountId::from(DAVE), 1_000 * UNITS),
                ],
            },
            collator_selection: gargantua_runtime::CollatorSelectionConfig {
                invulnerables: collators::invulnerables().iter().cloned().map(|(acc, _)| acc).collect(),
                candidacy_bond: 2 * UNITS,
                ..Default::default()
            },
            session: gargantua_runtime::SessionConfig {
                keys: collators::invulnerables()
                    .into_iter()
                    .map(|(acc, aura)| {
                        (
                            acc.clone(),                                   // account id
                            acc,                                           // validator id
                            gargantua_runtime::opaque::SessionKeys { aura }, // session keys
                        )
                    })
                    .collect(),
                non_authority_keys: Default::default(),
            },
            parachain_info: gargantua_runtime::ParachainInfoConfig {
                parachain_id: HYPERBRIDGE_PARA_ID.into(),
                ..Default::default()
            },
            polkadot_xcm: gargantua_runtime::PolkadotXcmConfig {
                safe_xcm_version: Some(3),
                ..Default::default()
            },
            ..Default::default()
        };
        genesis_config.build_storage().unwrap()
    }
}

pub mod para {
    use super::*;

    pub fn genesis(para_id: u32) -> Storage {
        let genesis_config = gargantua_runtime::RuntimeGenesisConfig {
            balances: gargantua_runtime::BalancesConfig {
                balances: vec![(AccountId::from(ALICE), ALICE_INITIAL_NATIVE_BALANCE)],
            },
            collator_selection: gargantua_runtime::CollatorSelectionConfig {
                invulnerables: collators::invulnerables().iter().cloned().map(|(acc, _)| acc).collect(),
                candidacy_bond: UNITS * 16,
                ..Default::default()
            },
            session: gargantua_runtime::SessionConfig {
                keys: collators::invulnerables()
                    .into_iter()
                    .map(|(acc, aura)| {
                        (
                            acc.clone(),                                   // account id
                            acc,                                           // validator id
                            gargantua_runtime::opaque::SessionKeys { aura }, // session keys
                        )
                    })
                    .collect(),
                non_authority_keys: Default::default(),
            },
            parachain_info: gargantua_runtime::ParachainInfoConfig {
                parachain_id: para_id.into(),
                ..Default::default()
            },
            polkadot_xcm: gargantua_runtime::PolkadotXcmConfig {
                safe_xcm_version: Some(3),
                ..Default::default()
            },
            duster: gargantua_runtime::DusterConfig {
                account_blacklist: vec![Treasury::account_id()],
                reward_account: Some(Treasury::account_id()),
                dust_account: Some(Treasury::account_id()),
            },
            ..Default::default()
        };

        genesis_config.build_storage().unwrap()
    }
}



pub fn set_relaychain_block_number(number: BlockNumber) {
    use gargantua_runtime::ParachainSystem;
    use sp_core::{Encode, Get};
    use xcm_emulator::HeaderT;

    // We need to set block number this way as well because tarpaulin code coverage tool does not like the way
    // how we set the block number with `cumulus-test-relay-sproof-builder` package
    rococo_run_to_block(number);

    ParachainSystem::on_initialize(number);

    let mut sproof_builder = RelayStateSproofBuilder::default();

    let parent_head_data = {
        let header = cumulus_primitives_core::relay_chain::Header::new(
            number,
            sp_core::H256::from_low_u64_be(0),
            sp_core::H256::from_low_u64_be(0),
            Default::default(),
            Default::default(),
        );
        cumulus_primitives_core::relay_chain::HeadData(header.encode())
    };

    sproof_builder.para_id = gargantua_runtime::ParachainInfo::get();
    sproof_builder.included_para_head = Some(parent_head_data.clone());

    let (relay_storage_root, proof) = sproof_builder.into_state_root_and_proof();

    assert_ok!(ParachainSystem::set_validation_data(
		RuntimeOrigin::none(),
		cumulus_primitives_parachain_inherent::ParachainInherentData {
			validation_data: cumulus_primitives_core::PersistedValidationData {
				parent_head: Default::default(),
				relay_parent_number: number,
				relay_parent_storage_root: relay_storage_root,
				max_pov_size: Default::default(),
			},
			relay_chain_state: proof,
			downward_messages: Default::default(),
			horizontal_messages: Default::default(),
		}
	));
}

pub fn hyperbridge_run_to_next_block() {
    use frame_support::traits::OnFinalize;

    let b = gargantua_runtime::System::block_number();
    gargantua_runtime::System::on_finalize(b);


    gargantua_runtime::System::set_block_number(b + 1);
    gargantua_runtime::System::on_initialize(b + 1);


    gargantua_runtime::System::set_block_number(b + 1);
}

pub fn hyperbridge_run_to_block(to: BlockNumber) {
    let b = gargantua_runtime::System::block_number();
    assert!(b <= to, "the current block number {:?} is higher than expected.", b);

    while gargantua_runtime::System::block_number() < to {
        hyperbridge_run_to_next_block();
    }
}

pub fn hyperbridge_finalize_block() {
    use frame_support::traits::OnFinalize;

    let b = gargantua_runtime::System::block_number();

    gargantua_runtime::System::on_finalize(b);
}

pub fn rococo_run_to_block(to: BlockNumber) {
    use frame_support::traits::OnFinalize;

    while gargantua_runtime::System::block_number() < to {
        let b = gargantua_runtime::System::block_number();

        gargantua_runtime::System::on_finalize(b);


        gargantua_runtime::System::on_initialize(b + 1);

        gargantua_runtime::System::set_block_number(b + 1);
    }
}

use xcm_emulator::pallet_message_queue;

pub fn assert_xcm_message_processing_failed() {
    assert!(gargantua_runtime::System::events().iter().any(|r| matches!(
		r.event,
		gargantua_runtime::RuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: false, .. })
	)));
}

pub fn assert_xcm_message_processing_passed() {
    assert!(gargantua_runtime::System::events().iter().any(|r| matches!(
		r.event,
		gargantua_runtime::RuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true, .. })
	)));
}

