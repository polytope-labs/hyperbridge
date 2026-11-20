use super::{AbiType, EventSpec, FieldSpec, PalletSpec};

pub const KEY: &str = "explorer";

/**
 * The logs a block explorer needs: one per block, extrinsic and event, whatever the pallet.
 * No pallet is named with an empty string, so the mapper never maps an event to these.
 * `crate::explorer` builds them from the whole block instead.
 */
pub fn spec() -> PalletSpec {
	PalletSpec {
		key: KEY,
		metadata_name: "",
		events: vec![
			EventSpec::new(
				"Block",
				vec![
					FieldSpec::new("specVersion", "", AbiType::Uint(32)),
					FieldSpec::new("parentHash", "", AbiType::Bytes32),
					FieldSpec::new("stateRoot", "", AbiType::Bytes32),
					FieldSpec::new("extrinsicsRoot", "", AbiType::Bytes32),
					FieldSpec::new("extrinsicsCount", "", AbiType::Uint(32)),
					FieldSpec::new("eventsCount", "", AbiType::Uint(32)),
				],
			),
			EventSpec::new(
				"Extrinsic",
				vec![
					FieldSpec::new("index", "", AbiType::Uint(32)),
					FieldSpec::new("signed", "", AbiType::Bool),
					FieldSpec::new("success", "", AbiType::Bool),
					FieldSpec::new("signer", "", AbiType::Bytes),
					FieldSpec::new("section", "", AbiType::String),
					FieldSpec::new("method", "", AbiType::String),
					FieldSpec::new("args", "", AbiType::String),
					FieldSpec::new("eventsCount", "", AbiType::Uint(32)),
				],
			),
			event(),
		],
	}
}

/** One event of any pallet, with its arguments as JSON. The governance logs have it too. */
pub fn event() -> EventSpec {
	EventSpec::new(
		"Event",
		vec![
			FieldSpec::new("index", "", AbiType::Uint(32)),
			FieldSpec::new("isExtrinsic", "", AbiType::Bool),
			FieldSpec::new("extrinsicIndex", "", AbiType::Uint(32)),
			FieldSpec::new("section", "", AbiType::String),
			FieldSpec::new("method", "", AbiType::String),
			FieldSpec::new("args", "", AbiType::String),
		],
	)
}
