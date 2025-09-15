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

use crate::{
	AccountId, BalancesConfig, CollatorSelectionConfig, ParachainInfoConfig, PolkadotXcmConfig,
	RuntimeGenesisConfig, SessionConfig, SessionKeys, SudoConfig, EXISTENTIAL_DEPOSIT,
};

use alloc::{vec, vec::Vec};

use polkadot_sdk::{sp_core::Pair, staging_xcm as xcm, *};

use cumulus_primitives_core::ParaId;
use hex_literal::hex;
use json::Value;
use parachains_common::AuraId;
use sp_core::sr25519;
use sp_genesis_builder::PresetId;
use sp_keyring::Sr25519Keyring;

/// The default XCM version to set in genesis config.
const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;

/// Parachain id used for genesis config presets of parachain template.
pub const PARACHAIN_ID: u32 = 1000;

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we have just one key).
pub fn template_session_keys(keys: AuraId) -> SessionKeys {
	SessionKeys { aura: keys }
}

fn testnet_genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	endowed_accounts: Vec<AccountId>,
	root: AccountId,
	id: ParaId,
) -> Value {
	let genesis = RuntimeGenesisConfig {
		balances: BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, 1u128 << 60))
				.collect::<Vec<_>>(),
			..Default::default()
		},
		parachain_info: ParachainInfoConfig { parachain_id: id, ..Default::default() },
		collator_selection: CollatorSelectionConfig {
			invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect::<Vec<_>>(),
			candidacy_bond: EXISTENTIAL_DEPOSIT * 16,
			..Default::default()
		},
		session: SessionConfig {
			keys: invulnerables
				.into_iter()
				.map(|(acc, aura)| {
					(
						acc.clone(),                 // account id
						acc,                         // validator id
						template_session_keys(aura), // session keys
					)
				})
				.collect::<Vec<_>>(),
			..Default::default()
		},
		polkadot_xcm: PolkadotXcmConfig {
			safe_xcm_version: Some(SAFE_XCM_VERSION),
			..Default::default()
		},
		sudo: SudoConfig { key: Some(root) },
		..Default::default()
	};

	json::to_value(genesis).expect("Could not build genesis config.")
}

fn local_testnet_genesis() -> Value {
	testnet_genesis(
		// initial collators.
		vec![
			(Sr25519Keyring::Alice.to_account_id(), Sr25519Keyring::Alice.public().into()),
			(Sr25519Keyring::Bob.to_account_id(), Sr25519Keyring::Bob.public().into()),
		],
		vec![
			sr25519::Pair::from_string("Alice", None).unwrap().public().into(),
			sr25519::Pair::from_string("Bob", None).unwrap().public().into(),
			sr25519::Pair::from_string("Charlie", None).unwrap().public().into(),
			sr25519::Pair::from_string("Dave", None).unwrap().public().into(),
			sr25519::Pair::from_string("Eve", None).unwrap().public().into(),
			sr25519::Pair::from_string("Ferdie", None).unwrap().public().into(),
			sr25519::Pair::from_string("Alice//stash", None).unwrap().public().into(),
			sr25519::Pair::from_string("Bob//stash", None).unwrap().public().into(),
			sr25519::Pair::from_string("Charlie//stash", None).unwrap().public().into(),
			sr25519::Pair::from_string("Dave//stash", None).unwrap().public().into(),
			sr25519::Pair::from_string("Eve//stash", None).unwrap().public().into(),
			sr25519::Pair::from_string("Ferdie//stash", None).unwrap().public().into(),
		],
		Sr25519Keyring::Alice.to_account_id(),
		PARACHAIN_ID.into(),
	)
}

fn development_config_genesis() -> Value {
	testnet_genesis(
		// initial collators.
		vec![
			(Sr25519Keyring::Alice.to_account_id(), Sr25519Keyring::Alice.public().into()),
			(Sr25519Keyring::Bob.to_account_id(), Sr25519Keyring::Bob.public().into()),
		],
		vec![
			// Controller Accounts
			hex!["d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"].into(), /* Alice */
			hex!["8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48"].into(), // Bob
			hex!["90b5ab205c6974c9ea841be688864633dc9ca8a357843eeacf2314649965fe22"].into(), /* Charlie */
			hex!["306721211d5404bd9da88e0204360a1a9ab8b87c66c1bc2fcdd37f3c2222cc20"].into(), /* Dave */
			hex!["e659a7a1628cdd93febc04a4e0646ea20e9f5f0ce097d9a05290d4a9e054df4e"].into(), // Eve
			hex!["1cbd2d43530a44705ad088af313e18f80b53ef16b36177cd4b77b846f2a5f07c"].into(), /* Ferdie */
			// Stash Accounts (derived)
			hex!["ac5e01613b2046a6f3b7b84d436ff3c114995a9b3f360706346b3f74f7b57cd6"].into(), /* Alice//stash */
			hex!["a63b0a2c073a5a401f7b03a1653139363e8a4ad6875955a1532f7a14e9102c48"].into(), /* Bob//stash */
			hex!["d626c92d634d318892c575a7e11f1a54050226c7104a37a7605d33684a0d8a22"].into(), /* Charlie//stash */
			hex!["5e6c19a135f0f3532f7b475908a8e329202681532454f762a4a350f585aa1620"].into(), /* Dave//stash */
			hex!["887962b318721c54b333a5477c014798e26e4e3751096734796191b333c1034e"].into(), /* Eve//stash */
			hex!["38b33535d61e050f2b3eac6466333c87f4c549646487e07663473f324c45477c"].into(), /* Ferdie//stash */
		],
		Sr25519Keyring::Alice.to_account_id(),
		PARACHAIN_ID.into(),
	)
}

/// Provides the JSON representation of predefined genesis config for given `id`.
pub fn get_preset(id: &PresetId) -> Option<vec::Vec<u8>> {
	let patch = match id.as_str().try_into() {
		Ok(sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET) => local_testnet_genesis(),
		Ok(sp_genesis_builder::DEV_RUNTIME_PRESET) => development_config_genesis(),
		_ => return None,
	};
	Some(
		json::to_string(&patch)
			.expect("serialization to json is expected to work. qed.")
			.into_bytes(),
	)
}

/// List of supported presets.
pub fn preset_names() -> Vec<PresetId> {
	vec![
		PresetId::from(sp_genesis_builder::DEV_RUNTIME_PRESET),
		PresetId::from(sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET),
	]
}
