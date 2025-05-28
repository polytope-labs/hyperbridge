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
