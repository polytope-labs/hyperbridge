mod events;
mod metadata;

pub use events::{decode_event_records, decode_events, extrinsic_hashes, EventRecordDecoded};
pub use metadata::{decode_metadata, spec_version, MetadataCache};

#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
	#[error("failed to decode runtime metadata: {0}")]
	Metadata(#[source] codec::Error),
	#[error("runtime metadata payload is `None`")]
	EmptyMetadata,
	#[error("metadata has no System.Events storage entry")]
	MissingSystemEvents,
	#[error("failed to decode event record at index {index}: {source}")]
	Event {
		index: u32,
		#[source]
		source: Box<subxt_core::Error>,
	},
	#[error("failed to decode fields of {pallet}.{variant} at index {index}: {source}")]
	EventFields {
		index: u32,
		pallet: String,
		variant: String,
		#[source]
		source: Box<subxt_core::Error>,
	},
	#[error("events payload declares {expected} records but only {decoded} could be decoded")]
	TruncatedEvents { expected: u32, decoded: u32 },
}

#[cfg(test)]
pub(crate) mod testdata {
	use alloy_primitives::hex;

	const NEXUS_METADATA: &[u8] =
		include_bytes!("../../fixtures/hyperbridge-nexus/metadata/8700.scale");
	const NEXUS_BLOCK: &str = include_str!("../../fixtures/hyperbridge-nexus/blocks/12483541.json");
	const NEXUS_EVENTS: [u8; system_events_len(NEXUS_BLOCK)] = system_events(NEXUS_BLOCK);

	const BIFROST_METADATA: &[u8] =
		include_bytes!("../../fixtures/bifrost-mainnet/metadata/25000.scale");
	const BIFROST_BLOCK: &str = include_str!("../../fixtures/bifrost-mainnet/blocks/12861850.json");
	const BIFROST_EVENTS: [u8; system_events_len(BIFROST_BLOCK)] = system_events(BIFROST_BLOCK);

	const SYSTEM_EVENTS_KEY: &[u8] =
		b"\"0x26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7\"";

	pub(crate) struct Fixture {
		pub(crate) metadata: &'static [u8],
		pub(crate) events: &'static [u8],
		block: &'static str,
	}

	pub(crate) const NEXUS: Fixture =
		Fixture { metadata: NEXUS_METADATA, events: &NEXUS_EVENTS, block: NEXUS_BLOCK };

	pub(crate) const BIFROST: Fixture =
		Fixture { metadata: BIFROST_METADATA, events: &BIFROST_EVENTS, block: BIFROST_BLOCK };

	impl Fixture {
		fn block(&self) -> serde_json::Value {
			serde_json::from_str(self.block).unwrap()
		}

		pub(crate) fn extrinsics(&self) -> Vec<Vec<u8>> {
			let block = self.block();
			let encoded = block["extrinsics"].as_array().unwrap();
			encoded.iter().map(|e| hex::decode(e.as_str().unwrap()).unwrap()).collect()
		}

		pub(crate) fn block_number(&self) -> u64 {
			self.block()["number"].as_u64().unwrap()
		}
	}

	// `blocks` and `server` tests bind the events to `const` items, so the hex value stored
	// in the fixture JSON has to be decoded at compile time.
	const fn system_events_hex(block: &str) -> (usize, usize) {
		let bytes = block.as_bytes();
		let mut at = 0;
		'search: while at + SYSTEM_EVENTS_KEY.len() <= bytes.len() {
			let mut i = 0;
			while i < SYSTEM_EVENTS_KEY.len() {
				if bytes[at + i] != SYSTEM_EVENTS_KEY[i] {
					at += 1;
					continue 'search;
				}
				i += 1;
			}
			break;
		}
		assert!(at + SYSTEM_EVENTS_KEY.len() <= bytes.len(), "fixture has no System.Events value");
		at += SYSTEM_EVENTS_KEY.len();
		while bytes[at] != b'"' {
			at += 1;
		}
		let start = at + 3;
		let mut end = start;
		while bytes[end] != b'"' {
			end += 1;
		}
		(start, end)
	}

	const fn system_events_len(block: &str) -> usize {
		let (start, end) = system_events_hex(block);
		(end - start) / 2
	}

	const fn system_events<const N: usize>(block: &str) -> [u8; N] {
		let (start, _) = system_events_hex(block);
		let bytes = block.as_bytes();
		let mut out = [0u8; N];
		let mut i = 0;
		while i < N {
			out[i] = (nibble(bytes[start + 2 * i]) << 4) | nibble(bytes[start + 2 * i + 1]);
			i += 1;
		}
		out
	}

	const fn nibble(c: u8) -> u8 {
		match c {
			b'0'..=b'9' => c - b'0',
			b'a'..=b'f' => c - b'a' + 10,
			b'A'..=b'F' => c - b'A' + 10,
			_ => panic!("invalid hex digit in fixture"),
		}
	}
}

#[cfg(test)]
mod tests {
	use std::sync::Arc;

	use alloy_primitives::b256;
	use codec::Encode;
	use scale_value::Composite;

	use super::{
		testdata::{BIFROST, NEXUS},
		*,
	};
	use crate::types::{Hasher, Phase};

	#[test]
	fn fixture_metadata_is_unwrapped_v15() {
		assert_eq!(&NEXUS.metadata[..5], b"meta\x0f");
		assert_eq!(&BIFROST.metadata[..5], b"meta\x0f");
	}

	#[test]
	fn metadata_decodes_and_reports_spec_version() {
		assert_eq!(spec_version(&decode_metadata(NEXUS.metadata).unwrap()), Some(8700));
		assert_eq!(spec_version(&decode_metadata(BIFROST.metadata).unwrap()), Some(25000));
	}

	#[test]
	fn metadata_decodes_option_wrapped_payload() {
		let wrapped = Some(NEXUS.metadata.to_vec()).encode();
		assert_eq!(spec_version(&decode_metadata(&wrapped).unwrap()), Some(8700));
		assert!(matches!(
			decode_metadata(&None::<Vec<u8>>.encode()),
			Err(DecodeError::EmptyMetadata)
		));
		assert!(matches!(decode_metadata(b"meta\x0f\x00"), Err(DecodeError::Metadata(_))));
	}

	#[test]
	fn nexus_events_decode_with_block_metadata() {
		let metadata = decode_metadata(NEXUS.metadata).unwrap();
		let events = decode_events(&metadata, NEXUS.events).unwrap();
		assert_eq!(NEXUS.block_number(), 12483541);
		assert_eq!(events.len(), 15);
		assert!(events.iter().enumerate().all(|(i, e)| e.event_index == i as u32));

		assert_eq!(
			(events[0].pallet.as_str(), events[0].variant.as_str()),
			("Treasury", "UpdatedInactive")
		);
		assert_eq!(events[0].phase, Phase::Initialization);

		assert_eq!(
			(events[3].pallet.as_str(), events[3].variant.as_str()),
			("System", "ExtrinsicSuccess")
		);
		assert_eq!(events[3].phase, Phase::ApplyExtrinsic(0));

		let update = &events[8];
		assert_eq!(
			(update.pallet.as_str(), update.variant.as_str()),
			("Ismp", "StateMachineUpdated")
		);
		assert_eq!(update.phase, Phase::ApplyExtrinsic(2));
		let Composite::Named(fields) = &update.fields else { panic!("expected named fields") };
		let names: Vec<&str> = fields.iter().map(|(name, _)| name.as_str()).collect();
		assert_eq!(names, ["state_machine_id", "latest_height"]);
		assert_eq!(fields[1].1.as_u128(), Some(129861335));

		assert_eq!(events[14].phase, Phase::ApplyExtrinsic(3));
		assert_eq!(
			(events[14].pallet.as_str(), events[14].variant.as_str()),
			("System", "ExtrinsicSuccess")
		);
	}

	#[test]
	fn bifrost_events_decode_with_block_metadata() {
		let metadata = decode_metadata(BIFROST.metadata).unwrap();
		let records = decode_event_records(&metadata, BIFROST.events).unwrap();
		assert_eq!(BIFROST.block_number(), 12861850);
		assert_eq!(records.len(), 6);
		assert!(records.iter().all(|r| r.topics.is_empty()));

		let first = &records[0].event;
		assert_eq!(
			(first.pallet.as_str(), first.variant.as_str()),
			("BuyBack", "UsdtSetSwapOutMinSuccess")
		);
		assert_eq!(first.phase, Phase::Initialization);

		let success = &records[1].event;
		assert_eq!(
			(success.pallet.as_str(), success.variant.as_str()),
			("System", "ExtrinsicSuccess")
		);
		assert_eq!(success.phase, Phase::ApplyExtrinsic(0));
		let Composite::Named(fields) = &success.fields else { panic!("expected named fields") };
		assert_eq!(fields[0].0, "dispatch_info");

		let request = &records[5].event;
		assert_eq!((request.pallet.as_str(), request.variant.as_str()), ("Ismp", "Request"));
		assert_eq!(request.phase, Phase::Finalization);
		let Composite::Named(fields) = &request.fields else { panic!("expected named fields") };
		let names: Vec<&str> = fields.iter().map(|(name, _)| name.as_str()).collect();
		assert_eq!(names, ["dest_chain", "source_chain", "request_nonce", "commitment"]);
		assert_eq!(fields[2].1.as_u128(), Some(67755));
	}

	#[test]
	fn corrupted_events_fail() {
		let metadata = decode_metadata(NEXUS.metadata).unwrap();
		let truncated = &NEXUS.events[..NEXUS.events.len() / 2];
		assert!(matches!(
			decode_events(&metadata, truncated),
			Err(DecodeError::Event { .. } | DecodeError::TruncatedEvents { .. })
		));

		let mut garbage = NEXUS.events.to_vec();
		garbage[2] = 0xff;
		assert!(decode_events(&metadata, &garbage).is_err());

		let mut inflated = NEXUS.events.to_vec();
		inflated[0] = 0xfc;
		assert!(matches!(
			decode_events(&metadata, &inflated),
			Err(DecodeError::TruncatedEvents { expected: 63, decoded: 15 })
		));
	}

	#[test]
	fn events_with_foreign_metadata_fail() {
		let metadata = decode_metadata(BIFROST.metadata).unwrap();
		assert!(decode_events(&metadata, NEXUS.events).is_err());
	}

	#[test]
	fn nexus_extrinsic_hashes_use_keccak() {
		let extrinsics = NEXUS.extrinsics();
		let hashes = extrinsic_hashes(Hasher::Keccak, &extrinsics);
		assert_eq!(hashes.len(), 4);
		assert_eq!(
			hashes[0],
			b256!("6b9fcfabf66209ddefa4e4d8323ee8ed7eba88766d92f0505a6af621f9f662b8")
		);
		assert_eq!(
			hashes[2],
			b256!("7a4bb3c123f36954f3517d87ea185ff065c2c30d7bf9c29a58810324d89ee3cf")
		);
		assert_eq!(
			hashes[3],
			b256!("26dd7378df3b8cc3888334113edf43b2586738731c6af47663e1d60e760bd853")
		);
	}

	#[test]
	fn bifrost_extrinsic_hashes_use_blake2() {
		let extrinsics = BIFROST.extrinsics();
		let hashes = extrinsic_hashes(Hasher::Blake2, &extrinsics);
		assert_eq!(hashes.len(), 2);
		assert_eq!(
			hashes[0],
			b256!("6c34db0885d148d9edc5a6451f423328295de475c4fec1017521c41cc28dcd58")
		);
		assert_eq!(
			hashes[1],
			b256!("a786d63465b2df01a47e0b62a91145d860b4173f821898433dd1510c3ce929b0")
		);
		assert_ne!(extrinsic_hashes(Hasher::Keccak, &extrinsics)[0], hashes[0]);
	}

	#[test]
	fn metadata_cache_returns_shared_arc() {
		let cache = MetadataCache::new();
		assert!(cache.get(8700).is_none());
		let inserted = cache.insert(8700, NEXUS.metadata).unwrap();
		let first = cache.get(8700).unwrap();
		let second = cache.get(8700).unwrap();
		assert!(Arc::ptr_eq(&inserted, &first));
		assert!(Arc::ptr_eq(&first, &second));
		let reinserted = cache.insert(8700, BIFROST.metadata).unwrap();
		assert!(Arc::ptr_eq(&reinserted, &first));
		assert!(cache.insert(1, b"not metadata").is_err());
		assert!(cache.get(1).is_none());
	}
}
