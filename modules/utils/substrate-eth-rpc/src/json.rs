//! Calls, events and storage values as JSON. Values follow polkadot.js `toJSON`, except that
//! numbers too large for a JavaScript number are decimal strings. Accounts are hex for the
//! block explorer, whose site encodes them as addresses itself, and SS58 for governance.

use scale_info::{form::PortableForm, Field, TypeDef, TypeDefPrimitive};
use scale_value::{Composite, Primitive, Value, ValueDef};
use serde_json::{json, Map, Value as Json};
use sp_crypto_hashing::blake2_512;
use subxt_core::Metadata;

const MAX_SAFE_INTEGER: u128 = (1 << 53) - 1;

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

	/** SCALE bytes of type `ty` as JSON, or `None` when they don't decode as it. */
	pub fn decode(&self, ty: u32, bytes: &[u8]) -> Option<Json> {
		let value =
			scale_value::scale::decode_as_type(&mut &bytes[..], ty, self.metadata.types()).ok()?;
		Some(self.value(&value))
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

/** `System::SS58Prefix`, or the generic Substrate prefix 42 for a runtime without one. */
fn ss58_prefix(metadata: &Metadata) -> u16 {
	metadata
		.pallet_by_name("System")
		.and_then(|pallet| pallet.constant_by_name("SS58Prefix"))
		.and_then(|constant| <u16 as codec::Decode>::decode(&mut constant.value()).ok())
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
	use super::*;

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
