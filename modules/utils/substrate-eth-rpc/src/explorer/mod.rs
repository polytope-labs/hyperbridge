//! The `explorer` logs: the whole block as a block explorer shows it, built from the same
//! decode the pallet logs come from. A block whose extrinsics fail to decode still gets its
//! logs, with the calls left blank, so an explorer never stalls the other logs of a block.

mod args;

use alloy_dyn_abi::DynSolValue;
use alloy_primitives::U256;
use subxt_core::{blocks::Extrinsics, config::SubstrateConfig, Metadata};
use tracing::warn;

use crate::{
	abi::encode_values,
	mapping::{explorer, MappedLog},
	substrate::RawBlock,
	types::{pallet_address, DecodedEvent, Phase},
};
use args::{camel_case, TypeKey};

/** A decoded call, or blank when the extrinsic couldn't be decoded. */
#[derive(Default)]
struct Call {
	signed: bool,
	signer: Vec<u8>,
	section: String,
	method: String,
	args: serde_json::Value,
}

/** The block, extrinsic and event logs of a block, each with the phase it belongs to. */
pub fn logs(
	metadata: &Metadata,
	raw: &RawBlock,
	spec_version: u32,
	events: &[DecodedEvent],
) -> Vec<(MappedLog, Phase)> {
	let spec = explorer::spec();
	let topic = |name: &str| {
		spec.events
			.iter()
			.find(|e| e.name == name)
			.map(|e| e.topic0())
			.unwrap_or_default()
	};
	let address = pallet_address(explorer::KEY);
	let log = |name, values| MappedLog {
		address,
		topics: vec![topic(name)],
		data: encode_values(values),
	};

	let calls = calls(metadata, raw);
	let in_extrinsic = |index: usize| {
		events.iter().filter(move |e| e.phase == Phase::ApplyExtrinsic(index as u32))
	};
	let failed = |index: usize| {
		in_extrinsic(index).any(|e| e.pallet == "System" && e.variant == "ExtrinsicFailed")
	};

	let block = log(
		"Block",
		vec![
			uint(spec_version),
			DynSolValue::FixedBytes(raw.parent_hash, 32),
			DynSolValue::FixedBytes(raw.state_root, 32),
			DynSolValue::FixedBytes(raw.extrinsics_root, 32),
			uint(raw.extrinsics.len() as u32),
			uint(events.len() as u32),
		],
	);
	let extrinsics = calls.into_iter().enumerate().map(|(index, call)| {
		let values = vec![
			uint(index as u32),
			DynSolValue::Bool(call.signed),
			DynSolValue::Bool(!failed(index)),
			DynSolValue::Bytes(call.signer),
			DynSolValue::String(call.section),
			DynSolValue::String(call.method),
			DynSolValue::String(call.args.to_string()),
			uint(in_extrinsic(index).count() as u32),
		];
		(log("Extrinsic", values), Phase::ApplyExtrinsic(index as u32))
	});
	let events = events.iter().map(|event| {
		let extrinsic = match event.phase {
			Phase::ApplyExtrinsic(index) => Some(index),
			_ => None,
		};
		let values = vec![
			uint(event.event_index),
			DynSolValue::Bool(extrinsic.is_some()),
			uint(extrinsic.unwrap_or_default()),
			DynSolValue::String(camel_case(&event.pallet)),
			DynSolValue::String(event.variant.clone()),
			DynSolValue::String(event_args(metadata, event).to_string()),
		];
		(log("Event", values), event.phase.clone())
	});

	std::iter::once((block, Phase::Initialization))
		.chain(extrinsics)
		.chain(events)
		.collect()
}

fn calls(metadata: &Metadata, raw: &RawBlock) -> Vec<Call> {
	let blank = || raw.extrinsics.iter().map(|_| Call::default()).collect();
	let extrinsics = match Extrinsics::<SubstrateConfig>::decode_from(
		raw.extrinsics.clone(),
		metadata.clone(),
	) {
		Ok(extrinsics) => extrinsics,
		Err(error) => {
			warn!(block = raw.number, %error, "explorer: extrinsics did not decode");
			return blank();
		},
	};
	extrinsics
		.iter()
		.map(|extrinsic| {
			let decoded = extrinsic.extrinsic_metadata().and_then(|meta| {
				let values = extrinsic.field_values()?;
				Ok((meta.pallet.name().to_string(), meta.variant, values))
			});
			let (section, method, args) = match decoded {
				Ok((pallet, variant, values)) => (
					camel_case(&pallet),
					camel_case(&variant.name),
					args::args(metadata, &variant.fields, &values, TypeKey::Call),
				),
				Err(error) => {
					warn!(block = raw.number, index = extrinsic.index(), %error, "explorer: call did not decode");
					Default::default()
				},
			};
			Call {
				signed: extrinsic.is_signed(),
				signer: extrinsic.address_bytes().map(account).unwrap_or_default(),
				section,
				method,
				args,
			}
		})
		.collect()
}

fn event_args(metadata: &Metadata, event: &DecodedEvent) -> serde_json::Value {
	metadata
		.pallet_by_name(&event.pallet)
		.and_then(|pallet| pallet.event_variants())
		.and_then(|variants| variants.iter().find(|v| v.name == event.variant))
		.map(|variant| args::args(metadata, &variant.fields, &event.fields, TypeKey::Event))
		.unwrap_or(serde_json::Value::Null)
}

/** The account behind a `MultiAddress::Id`, or the address bytes as they are for any other. */
fn account(address: &[u8]) -> Vec<u8> {
	match address {
		[0, id @ ..] if id.len() == 32 => id.to_vec(),
		other => other.to_vec(),
	}
}

fn uint(n: u32) -> DynSolValue {
	DynSolValue::Uint(U256::from(n), 32)
}

#[cfg(test)]
mod tests {
	use alloy_dyn_abi::{DynSolType, DynSolValue};
	use alloy_primitives::B256;
	use serde_json::Value as Json;

	use super::*;
	use crate::decode::{
		decode_events, decode_metadata,
		testdata::{Fixture, GARGANTUA, NEXUS},
	};

	/** A decoded `Extrinsic` or `Event` log, keyed as the explorer indexer reads it. */
	fn decode(log: &MappedLog) -> (String, Vec<DynSolValue>) {
		let spec = explorer::spec();
		let event = spec.events.iter().find(|e| e.topic0() == log.topics[0]).unwrap();
		let types = DynSolType::Tuple(event.fields.iter().map(|f| f.ty.dyn_type()).collect());
		let DynSolValue::Tuple(values) = types.abi_decode_params(&log.data).unwrap() else {
			unreachable!()
		};
		(event.name.to_string(), values)
	}

	fn block_logs(fixture: &Fixture) -> (Vec<(MappedLog, Phase)>, usize) {
		let metadata = decode_metadata(fixture.metadata).unwrap();
		let events = decode_events(&metadata, fixture.events).unwrap();
		let raw = RawBlock {
			hash: B256::repeat_byte(1),
			number: fixture.block_number(),
			parent_hash: B256::repeat_byte(2),
			state_root: B256::repeat_byte(3),
			extrinsics_root: B256::repeat_byte(4),
			extrinsics: fixture.extrinsics(),
		};
		(logs(&metadata, &raw, 1, &events), events.len())
	}

	fn text(value: &DynSolValue) -> &str {
		value.as_str().unwrap()
	}

	fn number(value: &DynSolValue) -> u64 {
		value.as_uint().unwrap().0.to::<u64>()
	}

	#[test]
	fn every_extrinsic_and_event_gets_a_log_after_the_block_log() {
		for fixture in [NEXUS, GARGANTUA] {
			let (logs, events) = block_logs(&fixture);
			let extrinsics = fixture.extrinsics().len();
			assert_eq!(logs.len(), 1 + extrinsics + events);

			let (name, block) = decode(&logs[0].0);
			assert_eq!(name, "Block");
			assert_eq!(number(&block[4]), extrinsics as u64);
			assert_eq!(number(&block[5]), events as u64);

			let extrinsic_events: u64 =
				logs[1..=extrinsics].iter().map(|(log, _)| number(&decode(log).1[7])).sum();
			let applied = logs[1 + extrinsics..]
				.iter()
				.filter(|(_, phase)| matches!(phase, Phase::ApplyExtrinsic(_)))
				.count();
			assert_eq!(extrinsic_events, applied as u64);
		}
	}

	#[test]
	fn calls_and_events_carry_their_arguments() {
		let (logs, _) = block_logs(&NEXUS);
		let (_, timestamp) = decode(&logs[1].0);
		assert_eq!((text(&timestamp[4]), text(&timestamp[5])), ("timestamp", "set"));
		assert_eq!(timestamp[2], DynSolValue::Bool(true));
		let args: Json = serde_json::from_str(text(&timestamp[6])).unwrap();
		assert_eq!(args[0]["name"], "now");
		assert_eq!(args[0]["type"], "Moment");
		assert!(args[0]["value"].is_u64());

		let (_, ismp) = decode(&logs[3].0);
		let args: Json = serde_json::from_str(text(&ismp[6])).unwrap();
		assert!(args[0]["value"].is_array(), "a Vec of one message is still a list");

		let events: Vec<_> = logs
			.iter()
			.map(|(log, _)| decode(log))
			.filter(|(name, _)| name == "Event")
			.map(|(_, values)| values)
			.collect();
		let updated = events.iter().find(|v| text(&v[4]) == "StateMachineUpdated").unwrap();
		let args: Json = serde_json::from_str(text(&updated[5])).unwrap();
		assert_eq!(args[0]["value"]["stateId"]["evm"], 56);
		assert_eq!(args[1]["typeName"], "u64");

		let transfer = events.iter().find(|v| text(&v[3]) == "balances").unwrap();
		let args: Json = serde_json::from_str(text(&transfer[5])).unwrap();
		assert_eq!(args[0]["typeName"], "AccountId");
		assert_eq!(args[0]["value"].as_str().unwrap().len(), 66);
	}
}
