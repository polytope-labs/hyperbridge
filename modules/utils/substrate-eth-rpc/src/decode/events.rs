use subxt_core::{
	config::SubstrateConfig,
	events::{EventDetails, Events, Phase as SubxtPhase},
	Metadata,
};

use super::DecodeError;
use crate::types::{DecodedEvent, Hasher, Phase, H256};

#[derive(Debug, Clone)]
pub struct EventRecordDecoded {
	pub event: DecodedEvent,
	pub topics: Vec<H256>,
}

pub fn decode_event_records(
	metadata: &Metadata,
	events_bytes: &[u8],
) -> Result<Vec<EventRecordDecoded>, DecodeError> {
	let has_events_entry = metadata
		.pallet_by_name("System")
		.and_then(|system| system.storage())
		.and_then(|storage| storage.entry_by_name("Events"))
		.is_some();
	if !has_events_entry {
		return Err(DecodeError::MissingSystemEvents);
	}

	let events = Events::<SubstrateConfig>::decode_from(events_bytes.to_vec(), metadata.clone());
	let mut records = Vec::with_capacity(events.len() as usize);
	for (index, details) in events.iter().enumerate() {
		let index = index as u32;
		let details =
			details.map_err(|source| DecodeError::Event { index, source: Box::new(source) })?;
		records.push(decode_record(index, details)?);
	}
	if records.len() as u32 != events.len() {
		return Err(DecodeError::TruncatedEvents {
			expected: events.len(),
			decoded: records.len() as u32,
		});
	}
	Ok(records)
}

pub fn decode_events(
	metadata: &Metadata,
	events_bytes: &[u8],
) -> Result<Vec<DecodedEvent>, DecodeError> {
	Ok(decode_event_records(metadata, events_bytes)?
		.into_iter()
		.map(|record| record.event)
		.collect())
}

pub fn extrinsic_hashes(hasher: Hasher, extrinsics: &[Vec<u8>]) -> Vec<H256> {
	extrinsics.iter().map(|bytes| hasher.hash(bytes)).collect()
}

fn decode_record(
	index: u32,
	details: EventDetails<SubstrateConfig>,
) -> Result<EventRecordDecoded, DecodeError> {
	let pallet = details.pallet_name().to_string();
	let variant = details.variant_name().to_string();
	let fields = details.field_values().map_err(|source| DecodeError::EventFields {
		index,
		pallet: pallet.clone(),
		variant: variant.clone(),
		source: Box::new(source),
	})?;
	let phase = match details.phase() {
		SubxtPhase::ApplyExtrinsic(i) => Phase::ApplyExtrinsic(i),
		SubxtPhase::Finalization => Phase::Finalization,
		SubxtPhase::Initialization => Phase::Initialization,
	};
	let topics = details.topics().iter().map(|topic| H256::from(topic.0)).collect();
	Ok(EventRecordDecoded {
		event: DecodedEvent { pallet, variant, phase, event_index: index, fields },
		topics,
	})
}
