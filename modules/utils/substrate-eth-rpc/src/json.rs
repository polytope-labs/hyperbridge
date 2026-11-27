//! Calls, events and storage values as JSON. Values follow polkadot.js `toJSON`, except that
//! numbers too large for a JavaScript number are decimal strings. Accounts are hex for the
//! block explorer, whose site encodes them as addresses itself, and SS58 for governance.

use codec::{Compact, Decode};
use scale_info::{form::PortableForm, Field, PortableRegistry, TypeDef, TypeDefPrimitive};
use scale_value::{Composite, Primitive, Value, ValueDef};
use serde_json::{json, Map, Value as Json};
use sp_crypto_hashing::blake2_512;
use subxt_core::Metadata;

const MAX_SAFE_INTEGER: u128 = (1 << 53) - 1;

/**
 * How deep composites, variants, sequences, arrays and tuples may nest in bytes decoded from
 * outside the chain. Each level adds at most two JSON levels, so the JSON stays well under
 * serde_json's limit of 128, and a call can nest at most 24 calls (two levels each), fewer
 * when the innermost call's arguments nest further.
 */
pub const MAX_DEPTH: usize = 48;

/** Which key holds a field's type name: `type` for call arguments, `typeName` for events. */
#[derive(Clone, Copy)]
pub enum TypeKey {
	Call,
	Event,
}

impl TypeKey {
	fn as_str(self) -> &'static str {
		match self {
			TypeKey::Call => "type",
			TypeKey::Event => "typeName",
		}
	}
}

/** Encodes values with one runtime's types. */
pub struct Encoder<'a> {
	pub metadata: &'a Metadata,
	/** The SS58 prefix accounts are encoded with, or `None` to leave them as hex. */
	pub ss58: Option<u16>,
}

impl<'a> Encoder<'a> {
	/** An encoder that leaves accounts as hex. */
	pub fn hex(metadata: &'a Metadata) -> Self {
		Self { metadata, ss58: None }
	}

	/** An encoder that writes accounts as SS58 with the chain's own prefix. */
	pub fn ss58(metadata: &'a Metadata) -> Self {
		Self { metadata, ss58: Some(ss58_prefix(metadata)) }
	}

	/** `[{name, type | typeName, value}]` for each field of a call or event. */
	pub fn args(
		&self,
		fields: &[Field<PortableForm>],
		values: &Composite<u32>,
		key: TypeKey,
	) -> Json {
		let values: Vec<&Value<u32>> = match values {
			Composite::Named(values) => values.iter().map(|(_, value)| value).collect(),
			Composite::Unnamed(values) => values.iter().collect(),
		};
		let args = fields
			.iter()
			.zip(values)
			.map(|(field, value)| {
				let type_name = field.type_name.as_deref().map(sanitize).unwrap_or_default();
				let name =
					field.name.as_deref().map(camel_case).unwrap_or_else(|| type_name.clone());
				json!({ "name": name, key.as_str(): type_name, "value": self.value(value) })
			})
			.collect();
		Json::Array(args)
	}

	/** `{callIndex, section, method, args}` for a call, as nested calls such as a batch's are
	 * shown. */
	pub fn call(&self, pallet: &str, method: &str, values: &Composite<u32>) -> Json {
		let pallet = self.metadata.pallet_by_name(pallet);
		let variant = pallet.as_ref().and_then(|p| p.call_variant_by_name(method));
		let call_index = pallet
			.as_ref()
			.zip(variant)
			.map(|(p, v)| format!("0x{:02x}{:02x}", p.index(), v.index))
			.unwrap_or_default();
		let args = variant
			.map(|v| self.args(&v.fields, values, TypeKey::Call))
			.unwrap_or(Json::Null);
		json!({
			"callIndex": call_index,
			"section": pallet.map(|p| camel_case(p.name())).unwrap_or_default(),
			"method": camel_case(method),
			"args": args,
		})
	}

	/**
	 * SCALE bytes of type `ty` as JSON, or `None` when they aren't exactly one value of it. The
	 * bytes are checked before anything is allocated for them: they must nest no deeper than
	 * [`MAX_DEPTH`], and their sequences together may declare no more items than there are
	 * bytes.
	 */
	pub fn decode(&self, ty: u32, bytes: &[u8]) -> Option<Json> {
		self.checked(ty, bytes).map(|value| self.value(&value))
	}

	/**
	 * The call in `call`, itself or one nested in it, whose encoding `matches`, as JSON. A
	 * batch's bytes hold calls that failed or never ran as well, so a call is found by its
	 * hash rather than its place.
	 */
	pub fn nested_call(&self, call: &[u8], matches: impl Fn(&[u8]) -> bool) -> Option<Json> {
		let ty = self.metadata.outer_enums().call_enum_ty();
		let root = self.checked(ty, call)?;
		self.find_call(&root, ty, &matches).map(|value| self.value(value))
	}

	/** The first call in `value`, depth first, whose encoding `matches`. `checked` bounds how
	 * deep this goes. */
	fn find_call<'v>(
		&self,
		value: &'v Value<u32>,
		ty: u32,
		matches: &impl Fn(&[u8]) -> bool,
	) -> Option<&'v Value<u32>> {
		if value.context == ty {
			let mut bytes = Vec::new();
			let encoded =
				scale_value::scale::encode_as_type(value, ty, self.metadata.types(), &mut bytes);
			if encoded.is_ok() && matches(&bytes) {
				return Some(value);
			}
		}
		let mut children = match &value.value {
			ValueDef::Composite(fields) => Some(fields.values()),
			ValueDef::Variant(variant) => Some(variant.values.values()),
			_ => None,
		}?;
		children.find_map(|child| self.find_call(child, ty, matches))
	}

	/** `bytes` decoded as exactly one value of type `ty`, after the checks `decode` names. */
	fn checked(&self, ty: u32, bytes: &[u8]) -> Option<Value<u32>> {
		let types = self.metadata.types();
		let mut check = Check { types, input: bytes, items: bytes.len() };
		check.value(ty, 0)?;
		if !check.input.is_empty() {
			return None;
		}
		let mut input = bytes;
		let value = scale_value::scale::decode_as_type(&mut input, ty, types).ok()?;
		input.is_empty().then_some(value)
	}

	pub fn value(&self, value: &Value<u32>) -> Json {
		let ty = self.metadata.types().resolve(value.context);
		let name = ty.and_then(|ty| ty.path.segments.last()).map(String::as_str);
		match &value.value {
			ValueDef::Variant(pallet) if name == Some("RuntimeCall") =>
				match unnamed(&pallet.values) {
					[Value { value: ValueDef::Variant(method), .. }] =>
						self.call(&pallet.name, &method.name, &method.values),
					_ => Json::Null,
				},
			ValueDef::Variant(variant) if name == Some("Option") =>
				match unnamed(&variant.values) {
					[inner] => self.value(inner),
					_ => Json::Null,
				},
			ValueDef::Variant(variant) if variant.values.is_empty() =>
				Json::String(variant.name.clone()),
			ValueDef::Variant(variant) => {
				let inner = match &variant.values {
					Composite::Unnamed(values) if values.len() == 1 => self.value(&values[0]),
					fields => self.fields(fields),
				};
				json!({ camel_case(&variant.name): inner })
			},
			ValueDef::Composite(Composite::Unnamed(values)) => match ty.map(|ty| &ty.type_def) {
				Some(TypeDef::Composite(_)) if name == Some("AccountId32") =>
					match (self.ss58, account(values)) {
						(Some(prefix), account) if account.len() == 32 =>
							Json::String(ss58(&account, prefix)),
						(_, bytes) => hex(&bytes),
					},
				Some(TypeDef::Sequence(seq)) if self.is_u8(seq.type_param.id) =>
					hex(&bytes(values)),
				Some(TypeDef::Array(array)) if self.is_u8(array.type_param.id) =>
					hex(&bytes(values)),
				Some(TypeDef::Composite(_)) if values.len() == 1 => self.value(&values[0]),
				Some(TypeDef::Tuple(_)) if values.is_empty() => Json::Null,
				_ => Json::Array(values.iter().map(|v| self.value(v)).collect()),
			},
			ValueDef::Composite(fields) => self.fields(fields),
			ValueDef::Primitive(primitive) => match primitive {
				Primitive::Bool(b) => Json::Bool(*b),
				Primitive::Char(c) => Json::String(c.to_string()),
				Primitive::String(s) => Json::String(s.clone()),
				Primitive::U128(n) if *n <= MAX_SAFE_INTEGER => json!(*n as u64),
				Primitive::U128(n) => Json::String(n.to_string()),
				Primitive::I128(n) if n.unsigned_abs() <= MAX_SAFE_INTEGER => json!(*n as i64),
				Primitive::I128(n) => Json::String(n.to_string()),
				Primitive::U256(bytes) | Primitive::I256(bytes) =>
					Json::String(format!("0x{}", alloy_primitives::hex::encode(bytes))),
			},
			ValueDef::BitSequence(bits) =>
				Json::String(bits.iter().map(|bit| if bit { '1' } else { '0' }).collect()),
		}
	}

	/** Named fields as an object with camelCase keys, unnamed ones as an array. */
	fn fields(&self, fields: &Composite<u32>) -> Json {
		match fields {
			Composite::Named(fields) => Json::Object(
				fields
					.iter()
					.map(|(name, value)| (camel_case(name), self.value(value)))
					.collect::<Map<_, _>>(),
			),
			Composite::Unnamed(values) =>
				Json::Array(values.iter().map(|v| self.value(v)).collect()),
		}
	}

	fn is_u8(&self, id: u32) -> bool {
		self.metadata
			.types()
			.resolve(id)
			.is_some_and(|ty| matches!(ty.type_def, TypeDef::Primitive(TypeDefPrimitive::U8)))
	}
}

/**
 * Walks SCALE bytes as a type without allocating. scale_value reserves room for a sequence's
 * declared length before decoding an item and recurses without a limit, so untrusted bytes
 * pass this walk first. Each item that takes up input starts at a byte no other item starts
 * at, so a well formed value never declares more sequence items than it has bytes.
 */
struct Check<'a> {
	types: &'a PortableRegistry,
	input: &'a [u8],
	/** How many more sequence items the input may declare. */
	items: usize,
}

impl<'a> Check<'a> {
	fn value(&mut self, ty: u32, depth: usize) -> Option<()> {
		let types = self.types;
		let ty = types.resolve(ty)?;
		let nested = (depth < MAX_DEPTH).then_some(depth + 1);
		match &ty.type_def {
			TypeDef::Composite(composite) => {
				let depth = nested?;
				composite.fields.iter().try_for_each(|field| self.value(field.ty.id, depth))
			},
			TypeDef::Variant(variants) => {
				let depth = nested?;
				let index = *self.take(1)?.first()?;
				let variant = variants.variants.iter().find(|variant| variant.index == index)?;
				variant.fields.iter().try_for_each(|field| self.value(field.ty.id, depth))
			},
			TypeDef::Sequence(sequence) => {
				let depth = nested?;
				let len = usize::try_from(Compact::<u64>::decode(&mut self.input).ok()?.0).ok()?;
				self.items = self.items.checked_sub(len)?;
				self.items(sequence.type_param.id, len, depth)
			},
			TypeDef::Array(array) => self.items(array.type_param.id, array.len as usize, nested?),
			TypeDef::Tuple(tuple) => {
				let depth = nested?;
				tuple.fields.iter().try_for_each(|field| self.value(field.id, depth))
			},
			TypeDef::Primitive(TypeDefPrimitive::Str) => {
				let len = Compact::<u32>::decode(&mut self.input).ok()?.0;
				self.take(len as usize).map(drop)
			},
			TypeDef::Primitive(primitive) => self.take(primitive_size(primitive)?).map(drop),
			TypeDef::Compact(_) => Compact::<u128>::decode(&mut self.input).ok().map(drop),
			TypeDef::BitSequence(bits) => {
				let TypeDef::Primitive(store) = &types.resolve(bits.bit_store_type.id)?.type_def
				else {
					return None;
				};
				let store = primitive_size(store)?;
				let len = Compact::<u32>::decode(&mut self.input).ok()?.0 as usize;
				self.take(len.div_ceil(store * 8) * store).map(drop)
			},
		}
	}

	/** `len` items of type `item`, taking a byte string in one step. */
	fn items(&mut self, item: u32, len: usize, depth: usize) -> Option<()> {
		let types = self.types;
		match types.resolve(item).map(|ty| &ty.type_def) {
			Some(TypeDef::Primitive(TypeDefPrimitive::U8)) => self.take(len).map(drop),
			_ => (0..len).try_for_each(|_| self.value(item, depth)),
		}
	}

	fn take(&mut self, len: usize) -> Option<&'a [u8]> {
		let (taken, rest) = self.input.split_at_checked(len)?;
		self.input = rest;
		Some(taken)
	}
}

/** The encoded size of a fixed width primitive. */
fn primitive_size(primitive: &TypeDefPrimitive) -> Option<usize> {
	match primitive {
		TypeDefPrimitive::Bool | TypeDefPrimitive::U8 | TypeDefPrimitive::I8 => Some(1),
		TypeDefPrimitive::U16 | TypeDefPrimitive::I16 => Some(2),
		TypeDefPrimitive::U32 | TypeDefPrimitive::I32 | TypeDefPrimitive::Char => Some(4),
		TypeDefPrimitive::U64 | TypeDefPrimitive::I64 => Some(8),
		TypeDefPrimitive::U128 | TypeDefPrimitive::I128 => Some(16),
		TypeDefPrimitive::U256 | TypeDefPrimitive::I256 => Some(32),
		TypeDefPrimitive::Str => None,
	}
}

/** `System::SS58Prefix`, or the generic Substrate prefix 42 for a runtime without one. */
fn ss58_prefix(metadata: &Metadata) -> u16 {
	metadata
		.pallet_by_name("System")
		.and_then(|pallet| pallet.constant_by_name("SS58Prefix"))
		.and_then(|constant| u16::decode(&mut constant.value()).ok())
		.unwrap_or(42)
}

/** An account's SS58 address, as polkadot.js encodes it. */
pub fn ss58(account: &[u8], prefix: u16) -> String {
	let mut payload = match prefix {
		0..=63 => vec![prefix as u8],
		_ => vec![
			((prefix & 0b1111_1100) as u8 >> 2) | 0b0100_0000,
			(prefix >> 8) as u8 | ((prefix & 0b11) as u8) << 6,
		],
	};
	payload.extend_from_slice(account);
	let checksum = blake2_512(&[b"SS58PRE", &payload[..]].concat());
	payload.extend_from_slice(&checksum[..2]);
	bs58::encode(payload).into_string()
}

fn bytes(values: &[Value<u32>]) -> Vec<u8> {
	values
		.iter()
		.filter_map(|value| match value.value {
			ValueDef::Primitive(Primitive::U128(n)) => u8::try_from(n).ok(),
			_ => None,
		})
		.collect()
}

/** The bytes of an `AccountId32`, a newtype around its 32 byte array. */
fn account(values: &[Value<u32>]) -> Vec<u8> {
	match values {
		[Value { value: ValueDef::Composite(Composite::Unnamed(inner)), .. }] => bytes(inner),
		values => bytes(values),
	}
}

fn hex(bytes: &[u8]) -> Json {
	Json::String(format!("0x{}", alloy_primitives::hex::encode(bytes)))
}

fn unnamed(values: &Composite<u32>) -> &[Value<u32>] {
	match values {
		Composite::Unnamed(values) => values,
		Composite::Named(_) => &[],
	}
}

/**
 * The type name polkadot.js gives a field, which the site matches on: `T::Balance` is
 * `Balance`, `AccountIdLookupOf<T>` is `AccountIdLookupOf` and a boxed runtime call is `Call`.
 */
fn sanitize(type_name: &str) -> String {
	let mut name = type_name.to_string();
	while let Some(start) = name.find("<T as ") {
		let Some(end) = name[start..].find(">::") else { break };
		name.replace_range(start..start + end + 3, "");
	}
	let name = name.replace("T::", "").replace("<T, I>", "").replace("<T>", "");
	match name.strip_prefix("Box<").and_then(|inner| inner.strip_suffix('>')) {
		Some("RuntimeCall") => "Call".into(),
		Some(inner) => inner.into(),
		None => name,
	}
}

/** `transfer_keep_alive` as `transferKeepAlive`, and `ParachainSystem` as `parachainSystem`. */
pub fn camel_case(name: &str) -> String {
	let mut out = String::with_capacity(name.len());
	let mut upper = false;
	for (i, c) in name.chars().enumerate() {
		match c {
			'_' => upper = true,
			c if i == 0 => out.extend(c.to_lowercase()),
			c if upper => {
				out.extend(c.to_uppercase());
				upper = false;
			},
			c => out.push(c),
		}
	}
	out
}

#[cfg(test)]
mod tests {
	use codec::Encode;

	use super::*;
	use crate::{
		decode::{decode_metadata, testdata::NEXUS},
		types::Hasher,
	};

	fn call_index(metadata: &Metadata, pallet: &str, call: &str) -> [u8; 2] {
		let pallet = metadata.pallet_by_name(pallet).unwrap();
		[pallet.index(), pallet.call_variant_by_name(call).unwrap().index]
	}

	/** `System::remark(remark)` wrapped in `depth` calls of `Utility::as_derivative`. */
	fn nested_remark(metadata: &Metadata, depth: usize, remark: &[u8]) -> Vec<u8> {
		let derivative = [&call_index(metadata, "Utility", "as_derivative")[..], &[0, 0]].concat();
		let remark = [&call_index(metadata, "System", "remark")[..], &remark.encode()].concat();
		[derivative.repeat(depth), remark].concat()
	}

	#[test]
	fn declared_lengths_beyond_the_input_are_rejected_before_allocating() {
		let metadata = decode_metadata(NEXUS.metadata).unwrap();
		let encoder = Encoder::ss58(&metadata);
		let ty = metadata.outer_enums().call_enum_ty();
		let remark = call_index(&metadata, "System", "remark");

		for len in [1u64 << 34, u64::MAX] {
			let call = [&remark[..], &codec::Compact(len).encode()].concat();
			assert_eq!(encoder.decode(ty, &call), None);
		}
		let call = nested_remark(&metadata, 0, b"hello");
		assert_eq!(encoder.decode(ty, &call).unwrap()["args"][0]["value"], "0x68656c6c6f");
	}

	#[test]
	fn nesting_is_bounded_and_stays_parseable() {
		let metadata = decode_metadata(NEXUS.metadata).unwrap();
		let encoder = Encoder::ss58(&metadata);
		let ty = metadata.outer_enums().call_enum_ty();

		let deepest = (0..)
			.take_while(|depth| {
				encoder.decode(ty, &nested_remark(&metadata, *depth, &[])).is_some()
			})
			.last()
			.unwrap();
		assert!(deepest >= 20, "only {deepest} nested calls decode");
		let text = encoder.decode(ty, &nested_remark(&metadata, deepest, &[])).unwrap().to_string();
		assert!(serde_json::from_str::<Json>(&text).is_ok());

		// Deep enough to overflow the stack if the walk or the decode recursed without a bound.
		assert_eq!(encoder.decode(ty, &nested_remark(&metadata, 100_000, &[])), None);
	}

	#[test]
	fn nested_calls_are_found_by_their_hash() {
		let metadata = decode_metadata(NEXUS.metadata).unwrap();
		let encoder = Encoder::ss58(&metadata);
		let remark =
			|text: &[u8]| [&call_index(&metadata, "System", "remark")[..], &text.encode()].concat();
		let (first, second) = (remark(b"a"), remark(b"b"));
		let batch = [
			&call_index(&metadata, "Utility", "batch")[..],
			&codec::Compact(2u32).encode(),
			&first,
			&second,
		]
		.concat();
		let hash_of = |call: &[u8]| Hasher::Keccak.hash(call);

		let found = encoder.nested_call(&batch, |bytes| hash_of(bytes) == hash_of(&second));
		assert_eq!(found.unwrap()["args"][0]["value"], "0x62");
		let found = encoder.nested_call(&batch, |bytes| hash_of(bytes) == hash_of(&batch));
		assert_eq!(found.unwrap()["method"], "batch");
		assert_eq!(encoder.nested_call(&batch, |bytes| hash_of(bytes) == hash_of(b"")), None);
	}

	#[test]
	fn trailing_bytes_are_rejected() {
		let metadata = decode_metadata(NEXUS.metadata).unwrap();
		let encoder = Encoder::ss58(&metadata);
		let ty = metadata.outer_enums().call_enum_ty();

		let call = nested_remark(&metadata, 1, b"hi");
		assert!(encoder.decode(ty, &call).is_some());
		assert_eq!(encoder.decode(ty, &[&call[..], &[0]].concat()), None);
	}

	#[test]
	fn names_follow_polkadot_js() {
		assert_eq!(camel_case("transfer_keep_alive"), "transferKeepAlive");
		assert_eq!(camel_case("ParachainSystem"), "parachainSystem");
		assert_eq!(camel_case("set"), "set");
		assert_eq!(sanitize("T::Balance"), "Balance");
		assert_eq!(sanitize("Compact<T::Balance>"), "Compact<Balance>");
		assert_eq!(sanitize("AccountIdLookupOf<T>"), "AccountIdLookupOf");
		assert_eq!(sanitize("Box<<T as Config>::RuntimeCall>"), "Call");
		assert_eq!(sanitize("Vec<<T as Config>::RuntimeCall>"), "Vec<RuntimeCall>");
		assert_eq!(sanitize("<T as pallet_ismp::Config>::Balance"), "Balance");
	}

	#[test]
	fn accounts_encode_as_polkadot_js_does() {
		// Alice, as polkadot.js shows her on Polkadot (0), Substrate (42) and Kusama (2)
		let alice = alloy_primitives::hex!(
			"d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"
		);
		assert_eq!(ss58(&alice, 0), "15oF4uVJwmo4TdGW7VfQxNLavjCXviqxT9S1MgbjMNHr6Sp5");
		assert_eq!(ss58(&alice, 42), "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY");
		assert_eq!(ss58(&alice, 2), "HNZata7iMYWmk5RvZRTiAsSDhV8366zq2YGb3tLH5Upf74F");
	}
}
