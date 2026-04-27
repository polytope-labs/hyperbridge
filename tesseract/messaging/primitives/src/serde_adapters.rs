// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

//! Shared serde adapters used by per-chain config types.

/// Serde adapter for `Option<StateMachine>` that round-trips through the
/// stringly form (e.g. `"POLKADOT-3367"` or `"EVM-1"`) produced by
/// [`StateMachine`]'s `Display` / `FromStr` impls. `serde_hex_utils` only
/// ships an adapter for the non-optional variant, so this fills that gap
/// and keeps both [`tesseract_evm::EvmConfig`] and
/// [`tesseract_substrate::SubstrateConfig`] reading the same TOML shape.
pub mod option_state_machine {
	use ismp::host::StateMachine;
	use serde::{Deserialize, Deserializer, Serialize, Serializer};

	pub fn serialize<S: Serializer>(
		value: &Option<StateMachine>,
		serializer: S,
	) -> Result<S::Ok, S::Error> {
		value.as_ref().map(|sm| sm.to_string()).serialize(serializer)
	}

	pub fn deserialize<'de, D: Deserializer<'de>>(
		deserializer: D,
	) -> Result<Option<StateMachine>, D::Error> {
		let raw: Option<String> = Option::deserialize(deserializer)?;
		raw.map(|s| s.parse::<StateMachine>().map_err(serde::de::Error::custom))
			.transpose()
	}
}
