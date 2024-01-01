// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::runtime_api::opaque::{AccountId, AuraId, Signature};
use cumulus_primitives_core::ParaId;
use gargantua_runtime::EXISTENTIAL_DEPOSIT;
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_core::{sr25519, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec<T> = sc_service::GenericChainSpec<T, Extensions>;

/// The default XCM version to set in genesis config.
const SAFE_XCM_VERSION: u32 = staging_xcm::prelude::XCM_VERSION;

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// The extensions for the [`ChainSpec`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ChainSpecGroup, ChainSpecExtension)]
#[serde(deny_unknown_fields)]
pub struct Extensions {
    /// The relay chain of the Parachain.
    pub relay_chain: String,
    /// The id of the Parachain.
    pub para_id: u32,
}

impl Extensions {
    /// Try to get the extension from the given `ChainSpec`.
    pub fn try_get(chain_spec: &dyn sc_service::ChainSpec) -> Option<&Self> {
        sc_chain_spec::get_extension(chain_spec.extensions())
    }
}

type AccountPublic = <Signature as Verify>::Signer;

/// Generate collator keys from seed.
///
/// This function's return type must always match the session keys of the chain in tuple format.
pub fn get_collator_keys_from_seed(seed: &str) -> AuraId {
    get_from_seed::<AuraId>(seed)
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

pub fn gargantua_development_config(id: u32) -> ChainSpec<gargantua_runtime::RuntimeGenesisConfig> {
    // Give your base currency a unit name and decimal places
    let mut properties = sc_chain_spec::Properties::new();
    properties.insert("tokenSymbol".into(), "DEV".into());
    properties.insert("tokenDecimals".into(), 12.into());
    properties.insert("ss58Format".into(), 42.into());

    ChainSpec::from_genesis(
        // Name
        "Hyperbridge-dev",
        // ID
        "gargantua",
        ChainType::Development,
        move || {
            gargantua_testnet_genesis(
                // initial collators.
                vec![
                    (
                        get_account_id_from_seed::<sr25519::Public>("Alice"),
                        get_collator_keys_from_seed("Alice"),
                    ),
                    (
                        get_account_id_from_seed::<sr25519::Public>("Bob"),
                        get_collator_keys_from_seed("Bob"),
                    ),
                ],
                vec![
                    get_account_id_from_seed::<sr25519::Public>("Alice"),
                    get_account_id_from_seed::<sr25519::Public>("Bob"),
                    get_account_id_from_seed::<sr25519::Public>("Charlie"),
                    get_account_id_from_seed::<sr25519::Public>("Dave"),
                    get_account_id_from_seed::<sr25519::Public>("Eve"),
                    get_account_id_from_seed::<sr25519::Public>("Ferdie"),
                    get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
                ],
                id.into(),
                get_account_id_from_seed::<sr25519::Public>("Alice"),
            )
        },
        Vec::new(),
        None,
        None,
        None,
        None,
        Extensions {
            relay_chain: "rococo-local".into(), // You MUST set this to the correct network!
            para_id: id,
        },
    )
}

pub fn messier_development_config(id: u32) -> ChainSpec<messier_runtime::RuntimeGenesisConfig> {
    // Give your base currency a unit name and decimal places
    let mut properties = sc_chain_spec::Properties::new();
    properties.insert("tokenSymbol".into(), "DEV".into());
    properties.insert("tokenDecimals".into(), 12.into());
    properties.insert("ss58Format".into(), 42.into());

    ChainSpec::from_genesis(
        // Name
        "Hyperbridge-dev",
        // ID
        "messier",
        ChainType::Development,
        move || {
            messier_testnet_genesis(
                // initial collators.
                vec![
                    (
                        get_account_id_from_seed::<sr25519::Public>("Alice"),
                        get_collator_keys_from_seed("Alice"),
                    ),
                    (
                        get_account_id_from_seed::<sr25519::Public>("Bob"),
                        get_collator_keys_from_seed("Bob"),
                    ),
                ],
                vec![
                    get_account_id_from_seed::<sr25519::Public>("Alice"),
                    get_account_id_from_seed::<sr25519::Public>("Bob"),
                    get_account_id_from_seed::<sr25519::Public>("Charlie"),
                    get_account_id_from_seed::<sr25519::Public>("Dave"),
                    get_account_id_from_seed::<sr25519::Public>("Eve"),
                    get_account_id_from_seed::<sr25519::Public>("Ferdie"),
                    get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
                ],
                id.into(),
                get_account_id_from_seed::<sr25519::Public>("Alice"),
            )
        },
        Vec::new(),
        None,
        None,
        None,
        None,
        Extensions {
            relay_chain: "rococo-local".into(), // You MUST set this to the correct network!
            para_id: id,
        },
    )
}

fn messier_testnet_genesis(
    invulnerables: Vec<(AccountId, AuraId)>,
    endowed_accounts: Vec<AccountId>,
    id: ParaId,
    sudo: AccountId,
) -> messier_runtime::RuntimeGenesisConfig {
    messier_runtime::RuntimeGenesisConfig {
        system: messier_runtime::SystemConfig {
            code: messier_runtime::WASM_BINARY
                .expect("WASM binary was not build, please build it!")
                .to_vec(),
            ..Default::default()
        },
        balances: messier_runtime::BalancesConfig {
            balances: endowed_accounts.iter().cloned().map(|k| (k, 1 << 60)).collect(),
        },
        parachain_info: messier_runtime::ParachainInfoConfig {
            parachain_id: id,
            ..Default::default()
        },
        collator_selection: messier_runtime::CollatorSelectionConfig {
            invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
            candidacy_bond: EXISTENTIAL_DEPOSIT * 16,
            ..Default::default()
        },
        session: messier_runtime::SessionConfig {
            keys: invulnerables
                .into_iter()
                .map(|(acc, aura)| {
                    (
                        acc.clone(),                           // account id
                        acc,                                   // validator id
                        messier_runtime::SessionKeys { aura }, // session keys
                    )
                })
                .collect(),
        },
        // no need to pass anything to aura, in fact it will panic if we do. Session will take care
        // of this.
        aura: Default::default(),
        aura_ext: Default::default(),
        parachain_system: Default::default(),
        // ismp_parachain: messier_runtime::IsmpParachainConfig { parachains: vec![sibling] },
        sudo: messier_runtime::SudoConfig { key: Some(sudo) },
        polkadot_xcm: messier_runtime::PolkadotXcmConfig {
            safe_xcm_version: Some(SAFE_XCM_VERSION),
            ..Default::default()
        },
    }
}
fn gargantua_testnet_genesis(
    invulnerables: Vec<(AccountId, AuraId)>,
    endowed_accounts: Vec<AccountId>,
    id: ParaId,
    sudo: AccountId,
) -> gargantua_runtime::RuntimeGenesisConfig {
    gargantua_runtime::RuntimeGenesisConfig {
        system: gargantua_runtime::SystemConfig {
            code: gargantua_runtime::WASM_BINARY
                .expect("WASM binary was not build, please build it!")
                .to_vec(),
            ..Default::default()
        },
        balances: gargantua_runtime::BalancesConfig {
            balances: endowed_accounts.iter().cloned().map(|k| (k, 1 << 60)).collect(),
        },
        parachain_info: gargantua_runtime::ParachainInfoConfig {
            parachain_id: id,
            ..Default::default()
        },
        collator_selection: gargantua_runtime::CollatorSelectionConfig {
            invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
            candidacy_bond: EXISTENTIAL_DEPOSIT * 16,
            ..Default::default()
        },
        session: gargantua_runtime::SessionConfig {
            keys: invulnerables
                .into_iter()
                .map(|(acc, aura)| {
                    (
                        acc.clone(),                             // account id
                        acc,                                     // validator id
                        gargantua_runtime::SessionKeys { aura }, // session keys
                    )
                })
                .collect(),
        },
        // no need to pass anything to aura, in fact it will panic if we do. Session will take care
        // of this.
        aura: Default::default(),
        aura_ext: Default::default(),
        parachain_system: Default::default(),
        // ismp_parachain: gargantua_runtime::IsmpParachainConfig { parachains: vec![sibling] },
        sudo: gargantua_runtime::SudoConfig { key: Some(sudo) },
        polkadot_xcm: gargantua_runtime::PolkadotXcmConfig {
            safe_xcm_version: Some(SAFE_XCM_VERSION),
            ..Default::default()
        },
    }
}

// fn testnet_genesis_json(
//     invulnerables: Vec<(AccountId, AuraId)>,
//     endowed_accounts: Vec<AccountId>,
//     root: AccountId,
//     id: ParaId,
// ) -> serde_json::Value {
//     serde_json::json!({
//         "balances": {
//             "balances": endowed_accounts.iter().cloned().map(|k| (k, 1u64 <<
// 60)).collect::<Vec<_>>(),         },
//         "parachainInfo": {
//             "parachainId": id,
//         },
//         "collatorSelection": {
//             "invulnerables": invulnerables.iter().cloned().map(|(acc, _)|
// acc).collect::<Vec<_>>(),             "candidacyBond": EXISTENTIAL_DEPOSIT * 16,
//         },
//         "session": {
//             "keys": invulnerables
//                 .into_iter()
//                 .map(|(acc, aura)| {
//                     (
//                         acc.clone(),                 // account id
//                         acc,                         // validator id
//                         session_keys(aura), // session keys
//                     )
//                 })
//             .collect::<Vec<_>>(),
//         },
//         "polkadotXcm": {
//             "safeXcmVersion": Some(SAFE_XCM_VERSION),
//         },
//         "sudo": { "key": Some(root) }
//     })
// }
