use std::{
	collections::HashMap,
	sync::{Arc, Mutex},
};

use codec::Decode;
use subxt_core::Metadata;

use super::DecodeError;

const METADATA_MAGIC: &[u8] = b"meta";

pub fn decode_metadata(raw: &[u8]) -> Result<Metadata, DecodeError> {
	if raw.starts_with(METADATA_MAGIC) {
		return Metadata::decode(&mut &raw[..]).map_err(DecodeError::Metadata);
	}
	let opaque = Option::<Vec<u8>>::decode(&mut &raw[..])
		.map_err(DecodeError::Metadata)?
		.ok_or(DecodeError::EmptyMetadata)?;
	Metadata::decode(&mut &opaque[..]).map_err(DecodeError::Metadata)
}

pub fn spec_version(metadata: &Metadata) -> Option<u32> {
	let constant = metadata.pallet_by_name("System")?.constant_by_name("Version")?;
	let value =
		scale_value::scale::decode_as_type(&mut constant.value(), constant.ty(), metadata.types())
			.ok()?;
	let scale_value::ValueDef::Composite(scale_value::Composite::Named(fields)) = value.value
	else {
		return None;
	};
	let (_, spec_version) = fields.into_iter().find(|(name, _)| name == "spec_version")?;
	spec_version.as_u128().and_then(|v| u32::try_from(v).ok())
}

#[derive(Default)]
pub struct MetadataCache {
	inner: Mutex<HashMap<u32, Arc<Metadata>>>,
}

impl MetadataCache {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn get(&self, spec_version: u32) -> Option<Arc<Metadata>> {
		self.inner.lock().expect("metadata cache poisoned").get(&spec_version).cloned()
	}

	pub fn insert(&self, spec_version: u32, raw: &[u8]) -> Result<Arc<Metadata>, DecodeError> {
		let metadata = Arc::new(decode_metadata(raw)?);
		let mut inner = self.inner.lock().expect("metadata cache poisoned");
		Ok(inner.entry(spec_version).or_insert(metadata).clone())
	}
}
