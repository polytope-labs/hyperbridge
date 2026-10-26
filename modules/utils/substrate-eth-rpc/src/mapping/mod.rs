mod balances;
mod bandwidth;
mod collator_manager;
mod consensus_incentives;
mod intents_coprocessor;
mod ismp;
mod relayer;
pub mod types;

use std::{
	collections::{BTreeSet, HashMap, HashSet},
	sync::{
		atomic::{AtomicU64, Ordering},
		Mutex,
	},
};

use alloy_primitives::{Address, Bytes, B256};
use scale_value::{Composite, ValueDef};
use tracing::warn;

pub use types::{state_machine_id, AbiType, EventSpec, FieldSpec, MappingError, PalletSpec};

use crate::{
	abi::{encode_values, lookup, to_dyn_value, topic_word},
	types::{pallet_address, DecodedEvent},
};

pub fn all_pallets() -> Vec<PalletSpec> {
	vec![
		ismp::spec(),
		consensus_incentives::spec(),
		balances::spec(),
		relayer::spec(),
		collator_manager::spec(),
		bandwidth::spec(),
		intents_coprocessor::spec(),
	]
}

pub fn pallets_for(keys: &[String]) -> Result<Vec<PalletSpec>, MappingError> {
	let all = all_pallets();
	keys.iter()
		.map(|key| {
			all.iter()
				.find(|p| p.key == key)
				.cloned()
				.ok_or_else(|| MappingError::UnknownPallet(key.clone()))
		})
		.collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedLog {
	pub address: Address,
	pub topics: Vec<B256>,
	pub data: Bytes,
}

struct EventEntry {
	spec: EventSpec,
	topic0: B256,
	sources: BTreeSet<String>,
}

struct PalletEntry {
	address: Address,
	events: Vec<EventEntry>,
}

pub struct Mapper {
	specs: Vec<PalletSpec>,
	pallets: HashMap<&'static str, PalletEntry>,
	skipped: AtomicU64,
	warned: Mutex<HashSet<String>>,
}

impl Mapper {
	pub fn new(specs: Vec<PalletSpec>) -> Self {
		let pallets = specs
			.iter()
			.map(|pallet| {
				let events = pallet
					.events
					.iter()
					.map(|spec| EventEntry {
						topic0: spec.topic0(),
						sources: spec.fields.iter().map(|f| f.source.to_string()).collect(),
						spec: spec.clone(),
					})
					.collect();
				(pallet.metadata_name, PalletEntry { address: pallet_address(pallet.key), events })
			})
			.collect();
		Self { specs, pallets, skipped: AtomicU64::new(0), warned: Mutex::new(HashSet::new()) }
	}

	pub fn specs(&self) -> &[PalletSpec] {
		&self.specs
	}

	pub fn skipped_count(&self) -> u64 {
		self.skipped.load(Ordering::Relaxed)
	}

	pub fn map_event(&self, ev: &DecodedEvent) -> Result<Option<MappedLog>, MappingError> {
		let Some(pallet) = self.pallets.get(ev.pallet.as_str()) else {
			return Ok(None);
		};
		let fields = flatten(&ev.fields);
		let present = present_fields(fields);
		let Some(entry) =
			pallet.events.iter().find(|e| e.spec.name == ev.variant && e.sources == present)
		else {
			self.skip(ev, &present);
			return Ok(None);
		};

		let mut topics = vec![entry.topic0];
		let mut data = Vec::new();
		for field in &entry.spec.fields {
			let value = lookup(fields, field.source)
				.ok_or_else(|| MappingError::MissingField(field.source.into()))?;
			let converted = to_dyn_value(&field.ty, value).and_then(|v| {
				if field.indexed {
					topic_word(&v).map(|t| topics.push(t))
				} else {
					data.push(v);
					Ok(())
				}
			});
			converted.map_err(|source| MappingError::Field {
				event: format!("{}::{}", ev.pallet, ev.variant),
				field: field.name.into(),
				source: Box::new(source),
			})?;
		}
		Ok(Some(MappedLog { address: pallet.address, topics, data: encode_values(data) }))
	}

	fn skip(&self, ev: &DecodedEvent, present: &BTreeSet<String>) {
		self.skipped.fetch_add(1, Ordering::Relaxed);
		let key = format!(
			"{}::{}({})",
			ev.pallet,
			ev.variant,
			present.iter().cloned().collect::<Vec<_>>().join(",")
		);
		let mut warned = self.warned.lock().unwrap_or_else(|e| e.into_inner());
		if warned.insert(key.clone()) {
			warn!(event = %key, "no log mapping for event shape, skipping");
		}
	}
}

fn flatten(fields: &Composite<u32>) -> &Composite<u32> {
	if let Composite::Unnamed(values) = fields {
		if let [single] = values.as_slice() {
			if let ValueDef::Composite(inner @ Composite::Named(_)) = &single.value {
				return inner;
			}
		}
	}
	fields
}

fn present_fields(fields: &Composite<u32>) -> BTreeSet<String> {
	match fields {
		Composite::Named(values) => values.iter().map(|(n, _)| n.clone()).collect(),
		Composite::Unnamed(values) => (0..values.len()).map(|i| i.to_string()).collect(),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::types::Phase;
	use alloy_dyn_abi::{DynSolType, DynSolValue};
	use alloy_primitives::{hex, U256};
	use scale_value::Value;

	fn mapper() -> Mapper {
		Mapper::new(all_pallets())
	}

	fn spec_for(key: &str, event: &str) -> EventSpec {
		all_pallets()
			.into_iter()
			.find(|p| p.key == key)
			.unwrap()
			.events
			.into_iter()
			.find(|e| e.name == event)
			.unwrap()
	}

	fn event(pallet: &str, variant: &str, fields: Composite<()>) -> DecodedEvent {
		DecodedEvent {
			pallet: pallet.into(),
			variant: variant.into(),
			phase: Phase::ApplyExtrinsic(1),
			event_index: 0,
			fields: fields.map_context(|_| 0u32),
		}
	}

	fn named(fields: Vec<(&str, Value<()>)>) -> Composite<()> {
		Composite::named(fields)
	}

	fn h256(byte: u8) -> Value<()> {
		Value::unnamed_composite([Value::from_bytes([byte; 32])])
	}

	fn bounded(bytes: &[u8]) -> Value<()> {
		Value::unnamed_composite([Value::from_bytes(bytes)])
	}

	fn evm(id: u32) -> Value<()> {
		Value::unnamed_variant("Evm", [Value::u128(id as u128)])
	}

	fn polkadot(id: u32) -> Value<()> {
		Value::unnamed_variant("Polkadot", [Value::u128(id as u128)])
	}

	fn substrate(id: &[u8; 4]) -> Value<()> {
		Value::unnamed_variant("Substrate", [Value::from_bytes(id)])
	}

	fn state_machine_id_value(state_id: Value<()>, consensus: &[u8; 4]) -> Value<()> {
		Value::named_composite([
			("state_id", state_id),
			("consensus_state_id", Value::from_bytes(consensus)),
		])
	}

	fn u256_limbs(value: u128) -> Value<()> {
		Value::unnamed_composite([Value::unnamed_composite([
			Value::u128(value & u64::MAX as u128),
			Value::u128(value >> 64),
			Value::u128(0),
			Value::u128(0),
		])])
	}

	fn tier(name: &str) -> Value<()> {
		Value::unnamed_variant(name, Vec::<Value<()>>::new())
	}

	fn uint(v: u128, bits: usize) -> DynSolValue {
		DynSolValue::Uint(U256::from(v), bits)
	}

	fn word(byte: u8) -> DynSolValue {
		DynSolValue::FixedBytes(B256::repeat_byte(byte), 32)
	}

	fn bytes4(b: &[u8; 4]) -> DynSolValue {
		let mut w = B256::ZERO;
		w[..4].copy_from_slice(b);
		DynSolValue::FixedBytes(w, 4)
	}

	fn s(v: &str) -> DynSolValue {
		DynSolValue::String(v.into())
	}

	fn decode(spec: &EventSpec, data: &[u8]) -> Vec<DynSolValue> {
		let ty = DynSolType::Tuple(
			spec.fields.iter().filter(|f| !f.indexed).map(|f| f.ty.dyn_type()).collect(),
		);
		match ty.abi_decode_params(data).unwrap() {
			DynSolValue::Tuple(values) => values,
			other => vec![other],
		}
	}

	fn check(
		key: &str,
		ev: DecodedEvent,
		expected_topics: Vec<B256>,
		expected_data: Vec<DynSolValue>,
	) -> MappedLog {
		let spec = spec_for(key, &ev.variant);
		let log = mapper().map_event(&ev).unwrap().expect("event should map");
		assert_eq!(log.address, pallet_address(key));
		let mut topics = vec![spec.topic0()];
		topics.extend(expected_topics);
		assert_eq!(log.topics, topics, "{}", ev.variant);
		assert_eq!(decode(&spec, &log.data), expected_data, "{}", ev.variant);
		log
	}

	#[test]
	fn pinned_signatures_and_topics() {
		let expected: &[(&str, &str, &str)] = &[
			(
				"ismp",
				"StateMachineUpdated((string,bytes4),uint64)",
				"cefeb1676a1a4225d963e08bda949f2873e16c20054b8afa8a1d628b2b725c0f",
			),
			(
				"ismp",
				"Request(string,string,uint64,bytes32)",
				"f1658cbabc2cd0fb6a2afc70b7cacd964a736b4647d31e463bd9ec03ad0a7b1f",
			),
			(
				"ismp",
				"Response(string,string,uint64,bytes32,bytes32)",
				"9a2c39d6a6ede51cb2b0e4abc13261e7389e649a6740b252f1920df1b84eb19b",
			),
			(
				"ismp",
				"PostRequestHandled(bytes32,bytes)",
				"8022b6cfef54247771c9f7de47d54e287f9924c81a0656eb184ea12bc5d033ab",
			),
			(
				"ismp",
				"GetRequestHandled(bytes32,bytes)",
				"8904ce197960ce838b9d49ee33ce24c75f010e0e3e83aa1bfaf74f89a52bade7",
			),
			(
				"ismp",
				"PostRequestTimeoutHandled(bytes32,string,string)",
				"c0c04fbf98bb9494d465a0b524b6467cccc05bfef8d18272545a0251fec1b13d",
			),
			(
				"ismp",
				"GetRequestTimeoutHandled(bytes32,string,string)",
				"1a69fccd6b8fc5781ec19e3ab9f96ef6ebdd84fdc09b1edec98e58950b3ca9c7",
			),
			(
				"consensusIncentives",
				"RelayerRewarded(bytes32,uint128,((string,bytes4),uint64))",
				"d71c124ab1542710e2e5fb83f763c55fc790e9861913f1ea940c716501d35763",
			),
			(
				"balances",
				"Transfer(bytes32,bytes32,uint128)",
				"f1e64c742f11e9d021f557d03df972f0fafc2745c6c961a489e71fa4dc8fd66f",
			),
			(
				"relayer",
				"AccumulateFees(bytes,string,uint256)",
				"340eba5ac80a5971e972132ad15bd2baa18d16b1a33f86bc384c673022cab0e1",
			),
			(
				"relayer",
				"Withdraw(bytes,bytes,string,uint256)",
				"011a3e27a0bf8449c8fe43293045be39990450f08e237446d712dba727877a17",
			),
			(
				"collatorManager",
				"CollatorRewarded(bytes32,uint128)",
				"a05855b592037e9d8f74757ff14b7409e44f106ff2a2f6038ad18fdaa79f971a",
			),
			(
				"bandwidth",
				"BandwidthCredited(string,bytes,string,uint8,uint128,uint64)",
				"9771698180262897038c9221a5d7aecd9e0769231e813b5a1dee060491c9fc4b",
			),
			(
				"bandwidth",
				"BandwidthConsumed(string,bytes,uint128,uint128)",
				"1d0261831778f55e8c473489ccf74446f193721568bb176f5b6591c7f16693e3",
			),
			(
				"bandwidth",
				"SubscriptionEvicted(string,bytes,uint8,uint128)",
				"881726332b0660ed82ab24a052e6f04e4495d14d6d6ffe2f164e22dd0560a741",
			),
			(
				"bandwidth",
				"ForceCredited(string,bytes,uint8,uint128,uint64)",
				"2d0e3cfe0b30604face8a5f41abd16cabc954829f74386cc59c7f504b949c2ae",
			),
			(
				"bandwidth",
				"TierSet(uint8,(bool,(uint128,uint64)))",
				"8cbf01d7c997bd1e5c31b6dfec09586b406f0e1c22e17b51fc5863a7924ed2dd",
			),
			(
				"intentsCoprocessor",
				"BidPlaced(bytes32,bytes32,bytes32,uint128)",
				"e748081ee84c233cccf40e896838190e251a6365294df61dfe135c4de730042a",
			),
		];
		let actual = all_pallets()
			.into_iter()
			.flat_map(|p| {
				p.events
					.into_iter()
					.map(move |e| (p.key, e.signature(), hex::encode(e.topic0())))
			})
			.collect::<Vec<_>>();
		assert_eq!(actual.len(), expected.len());
		for ((key, sig, topic), (ekey, esig, etopic)) in actual.iter().zip(expected) {
			assert_eq!((*key, sig.as_str(), topic.as_str()), (*ekey, *esig, *etopic));
		}
	}

	#[test]
	fn pallet_keys_and_metadata_names() {
		let pallets = all_pallets();
		let keys = pallets.iter().map(|p| p.key).collect::<Vec<_>>();
		assert_eq!(
			keys,
			[
				"ismp",
				"consensusIncentives",
				"balances",
				"relayer",
				"collatorManager",
				"bandwidth",
				"intentsCoprocessor"
			]
		);
		let names = pallets.iter().map(|p| p.metadata_name).collect::<Vec<_>>();
		assert_eq!(
			names,
			[
				"Ismp",
				"ConsensusIncentives",
				"Balances",
				"Relayer",
				"CollatorManager",
				"Bandwidth",
				"IntentsCoprocessor"
			]
		);
	}

	#[test]
	fn pallets_for_filters_and_rejects_unknown() {
		let keys = ["bandwidth".to_string(), "ismp".to_string()];
		let specs = pallets_for(&keys).unwrap();
		assert_eq!(specs.iter().map(|p| p.key).collect::<Vec<_>>(), ["bandwidth", "ismp"]);
		let err = pallets_for(&["nope".to_string()]).unwrap_err();
		assert!(matches!(err, MappingError::UnknownPallet(ref k) if k == "nope"), "{err}");
	}

	#[test]
	fn ismp_state_machine_updated() {
		check(
			"ismp",
			event(
				"Ismp",
				"StateMachineUpdated",
				named(vec![
					("state_machine_id", state_machine_id_value(evm(1), b"ETH0")),
					("latest_height", Value::u128(12345)),
				]),
			),
			vec![],
			vec![DynSolValue::Tuple(vec![s("EVM-1"), bytes4(b"ETH0")]), uint(12345, 64)],
		);
	}

	#[test]
	fn ismp_request() {
		let log = check(
			"ismp",
			event(
				"Ismp",
				"Request",
				named(vec![
					("dest_chain", evm(1)),
					("source_chain", polkadot(3367)),
					("request_nonce", Value::u128(7)),
					("commitment", h256(0xaa)),
				]),
			),
			vec![B256::repeat_byte(0xaa)],
			vec![s("EVM-1"), s("POLKADOT-3367"), uint(7, 64)],
		);
		let words = [
			"0000000000000000000000000000000000000000000000000000000000000060",
			"00000000000000000000000000000000000000000000000000000000000000a0",
			"0000000000000000000000000000000000000000000000000000000000000007",
			"0000000000000000000000000000000000000000000000000000000000000005",
			"45564d2d31000000000000000000000000000000000000000000000000000000",
			"000000000000000000000000000000000000000000000000000000000000000d",
			"504f4c4b41444f542d3333363700000000000000000000000000000000000000",
		];
		assert_eq!(hex::encode(&log.data), words.concat());
	}

	#[test]
	fn ismp_response() {
		check(
			"ismp",
			event(
				"Ismp",
				"Response",
				named(vec![
					("dest_chain", substrate(b"cere")),
					("source_chain", evm(56)),
					("request_nonce", Value::u128(1)),
					("commitment", h256(0x01)),
					("req_commitment", h256(0x02)),
				]),
			),
			vec![B256::repeat_byte(0x01), B256::repeat_byte(0x02)],
			vec![s("SUBSTRATE-cere"), s("EVM-56"), uint(1, 64)],
		);
	}

	#[test]
	fn ismp_handled_events_flatten_single_struct_tuple_variant() {
		for variant in ["PostRequestHandled", "GetRequestHandled"] {
			let inner = Value::named_composite([
				("commitment", h256(0x33)),
				("relayer", Value::from_bytes([0xde, 0xad])),
			]);
			check(
				"ismp",
				event("Ismp", variant, Composite::unnamed([inner])),
				vec![B256::repeat_byte(0x33)],
				vec![DynSolValue::Bytes(vec![0xde, 0xad])],
			);
		}
	}

	#[test]
	fn ismp_timeout_handled_events() {
		for variant in ["PostRequestTimeoutHandled", "GetRequestTimeoutHandled"] {
			let inner = Value::named_composite([
				("commitment", h256(0x44)),
				("source", evm(1)),
				("dest", polkadot(3367)),
			]);
			check(
				"ismp",
				event("Ismp", variant, Composite::unnamed([inner])),
				vec![B256::repeat_byte(0x44)],
				vec![s("EVM-1"), s("POLKADOT-3367")],
			);
		}
	}

	#[test]
	fn consensus_incentives_relayer_rewarded() {
		check(
			"consensusIncentives",
			event(
				"ConsensusIncentives",
				"RelayerRewarded",
				named(vec![
					("relayer", h256(0x55)),
					("amount", Value::u128(1_000)),
					(
						"state_machine_height",
						Value::named_composite([
							("id", state_machine_id_value(polkadot(2030), b"PAS0")),
							("height", Value::u128(99)),
						]),
					),
				]),
			),
			vec![B256::repeat_byte(0x55)],
			vec![
				uint(1_000, 128),
				DynSolValue::Tuple(vec![
					DynSolValue::Tuple(vec![s("POLKADOT-2030"), bytes4(b"PAS0")]),
					uint(99, 64),
				]),
			],
		);
	}

	#[test]
	fn balances_transfer() {
		check(
			"balances",
			event(
				"Balances",
				"Transfer",
				named(vec![("from", h256(0x01)), ("to", h256(0x02)), ("amount", Value::u128(5))]),
			),
			vec![B256::repeat_byte(0x01), B256::repeat_byte(0x02)],
			vec![uint(5, 128)],
		);
	}

	#[test]
	fn relayer_accumulate_fees_and_withdraw() {
		let amount = 10u128.pow(20);
		check(
			"relayer",
			event(
				"Relayer",
				"AccumulateFees",
				named(vec![
					("address", bounded(&[0x11; 20])),
					("state_machine", evm(42161)),
					("amount", u256_limbs(amount)),
				]),
			),
			vec![],
			vec![DynSolValue::Bytes(vec![0x11; 20]), s("EVM-42161"), uint(amount, 256)],
		);
		check(
			"relayer",
			event(
				"Relayer",
				"Withdraw",
				named(vec![
					("address", bounded(&[0x11; 20])),
					("beneficiary_address", bounded(&[0x22; 20])),
					("state_machine", evm(8453)),
					("amount", u256_limbs(amount)),
				]),
			),
			vec![],
			vec![
				DynSolValue::Bytes(vec![0x11; 20]),
				DynSolValue::Bytes(vec![0x22; 20]),
				s("EVM-8453"),
				uint(amount, 256),
			],
		);
	}

	#[test]
	fn collator_manager_collator_rewarded() {
		check(
			"collatorManager",
			event(
				"CollatorManager",
				"CollatorRewarded",
				named(vec![("collator", h256(0x66)), ("amount", Value::u128(77))]),
			),
			vec![B256::repeat_byte(0x66)],
			vec![uint(77, 128)],
		);
	}

	#[test]
	fn bandwidth_events() {
		let app = bounded(&[0xab; 20]);
		check(
			"bandwidth",
			event(
				"Bandwidth",
				"BandwidthCredited",
				named(vec![
					("app_chain", evm(1)),
					("app", app.clone()),
					("paid_from", evm(10)),
					("tier", tier("TierTwo")),
					("bytes", Value::u128(1 << 30)),
					("expires_at", Value::u128(1_800_000_000)),
				]),
			),
			vec![],
			vec![
				s("EVM-1"),
				DynSolValue::Bytes(vec![0xab; 20]),
				s("EVM-10"),
				uint(2, 8),
				uint(1 << 30, 128),
				uint(1_800_000_000, 64),
			],
		);
		check(
			"bandwidth",
			event(
				"Bandwidth",
				"BandwidthConsumed",
				named(vec![
					("source", evm(1)),
					("app", app.clone()),
					("bytes", Value::u128(512)),
					("remaining", Value::u128(1024)),
				]),
			),
			vec![],
			vec![s("EVM-1"), DynSolValue::Bytes(vec![0xab; 20]), uint(512, 128), uint(1024, 128)],
		);
		check(
			"bandwidth",
			event(
				"Bandwidth",
				"SubscriptionEvicted",
				named(vec![
					("app_chain", evm(1)),
					("app", app.clone()),
					("tier", tier("TierFour")),
					("lost_bytes", Value::u128(3)),
				]),
			),
			vec![],
			vec![s("EVM-1"), DynSolValue::Bytes(vec![0xab; 20]), uint(4, 8), uint(3, 128)],
		);
		check(
			"bandwidth",
			event(
				"Bandwidth",
				"ForceCredited",
				named(vec![
					("app_chain", evm(1)),
					("app", app),
					("tier", tier("TierOne")),
					("bytes", Value::u128(9)),
					("expires_at", Value::u128(10)),
				]),
			),
			vec![],
			vec![
				s("EVM-1"),
				DynSolValue::Bytes(vec![0xab; 20]),
				uint(1, 8),
				uint(9, 128),
				uint(10, 64),
			],
		);
	}

	#[test]
	fn bandwidth_tier_set_option() {
		let config = Value::named_composite([
			("bytes", Value::u128(100)),
			("duration_secs", Value::u128(2_592_000)),
		]);
		check(
			"bandwidth",
			event(
				"Bandwidth",
				"TierSet",
				named(vec![
					("tier", tier("TierThree")),
					("config", Value::unnamed_variant("Some", [config])),
				]),
			),
			vec![],
			vec![
				uint(3, 8),
				DynSolValue::Tuple(vec![
					DynSolValue::Bool(true),
					DynSolValue::Tuple(vec![uint(100, 128), uint(2_592_000, 64)]),
				]),
			],
		);
		check(
			"bandwidth",
			event(
				"Bandwidth",
				"TierSet",
				named(vec![
					("tier", tier("TierThree")),
					("config", Value::unnamed_variant("None", Vec::<Value<()>>::new())),
				]),
			),
			vec![],
			vec![
				uint(3, 8),
				DynSolValue::Tuple(vec![
					DynSolValue::Bool(false),
					DynSolValue::Tuple(vec![uint(0, 128), uint(0, 64)]),
				]),
			],
		);
	}

	#[test]
	fn intents_coprocessor_bid_placed() {
		check(
			"intentsCoprocessor",
			event(
				"IntentsCoprocessor",
				"BidPlaced",
				named(vec![
					("filler", h256(0x0f)),
					("commitment", h256(0x0c)),
					("bid", h256(0x0b)),
					("deposit", Value::u128(1_000)),
				]),
			),
			vec![B256::repeat_byte(0x0f), B256::repeat_byte(0x0c)],
			vec![word(0x0b), uint(1_000, 128)],
		);
	}

	#[test]
	fn legacy_bid_placed_shape_is_skipped() {
		let m = mapper();
		let legacy = event(
			"IntentsCoprocessor",
			"BidPlaced",
			named(vec![
				("filler", h256(0x0f)),
				("commitment", h256(0x0c)),
				("deposit", Value::u128(1_000)),
			]),
		);
		assert_eq!(m.map_event(&legacy).unwrap(), None);
		assert_eq!(m.skipped_count(), 1);
		assert_eq!(m.map_event(&legacy).unwrap(), None);
		assert_eq!(m.skipped_count(), 2);
	}

	#[test]
	fn unlisted_variant_of_mapped_pallet_is_skipped() {
		let m = mapper();
		let ev = event("Ismp", "Errors", named(vec![("errors", Value::unnamed_composite([]))]));
		assert_eq!(m.map_event(&ev).unwrap(), None);
		assert_eq!(m.skipped_count(), 1);
	}

	#[test]
	fn unmapped_pallet_is_silent() {
		let m = mapper();
		let ev = event("System", "ExtrinsicSuccess", named(vec![]));
		assert_eq!(m.map_event(&ev).unwrap(), None);
		assert_eq!(m.skipped_count(), 0);
	}

	#[test]
	fn configured_subset_ignores_other_pallets() {
		let m = Mapper::new(pallets_for(&["ismp".to_string()]).unwrap());
		let ev = event(
			"Balances",
			"Transfer",
			named(vec![("from", h256(0x01)), ("to", h256(0x02)), ("amount", Value::u128(5))]),
		);
		assert_eq!(m.map_event(&ev).unwrap(), None);
		assert_eq!(m.skipped_count(), 0);
		assert_eq!(m.specs().len(), 1);
	}

	#[test]
	fn field_type_errors_name_the_event_and_field() {
		let ev = event(
			"Balances",
			"Transfer",
			named(vec![("from", h256(0x01)), ("to", Value::u128(2)), ("amount", Value::u128(5))]),
		);
		let err = mapper().map_event(&ev).unwrap_err();
		match err {
			MappingError::Field { event, field, .. } => {
				assert_eq!(event, "Balances::Transfer");
				assert_eq!(field, "to");
			},
			other => panic!("unexpected error {other}"),
		}
	}
}
