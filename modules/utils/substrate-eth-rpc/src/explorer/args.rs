//! Call and event arguments as JSON, in the shape the Statescan site renders. Values follow
//! polkadot.js `toJSON`, except that numbers too large for a JavaScript number are decimal
//! strings and accounts are hex, which the site encodes as addresses itself.

use scale_info::{form::PortableForm, Field, TypeDef, TypeDefPrimitive};
use scale_value::{Composite, Primitive, Value, ValueDef};
use serde_json::{json, Map, Value as Json};
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

/** `[{name, type | typeName, value}]` for each field of a call or event. */
pub fn args(
	metadata: &Metadata,
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
			let name = field.name.as_deref().map(camel_case).unwrap_or_else(|| type_name.clone());
			json!({ "name": name, key.as_str(): type_name, "value": to_json(metadata, value) })
		})
		.collect();
	Json::Array(args)
}

/** `{callIndex, section, method, args}` for a call, as nested calls such as a batch's are shown. */
pub fn call(metadata: &Metadata, pallet: &str, method: &str, values: &Composite<u32>) -> Json {
	let pallet = metadata.pallet_by_name(pallet);
	let variant = pallet.as_ref().and_then(|p| p.call_variant_by_name(method));
	let call_index = pallet
		.as_ref()
		.zip(variant)
		.map(|(p, v)| format!("0x{:02x}{:02x}", p.index(), v.index))
		.unwrap_or_default();
	let args = variant
		.map(|v| args(metadata, &v.fields, values, TypeKey::Call))
		.unwrap_or(Json::Null);
	json!({
		"callIndex": call_index,
		"section": pallet.map(|p| camel_case(p.name())).unwrap_or_default(),
		"method": camel_case(method),
		"args": args,
	})
}

fn to_json(metadata: &Metadata, value: &Value<u32>) -> Json {
	let ty = metadata.types().resolve(value.context);
	let name = ty.and_then(|ty| ty.path.segments.last()).map(String::as_str);
	match &value.value {
		ValueDef::Variant(pallet) if name == Some("RuntimeCall") => match unnamed(&pallet.values) {
			[Value { value: ValueDef::Variant(method), .. }] =>
				call(metadata, &pallet.name, &method.name, &method.values),
			_ => Json::Null,
		},
		ValueDef::Variant(variant) if name == Some("Option") => match unnamed(&variant.values) {
			[inner] => to_json(metadata, inner),
			_ => Json::Null,
		},
		ValueDef::Variant(variant) if variant.values.is_empty() =>
			Json::String(variant.name.clone()),
		ValueDef::Variant(variant) => {
			let inner = match &variant.values {
				Composite::Unnamed(values) if values.len() == 1 => to_json(metadata, &values[0]),
				fields => fields_json(metadata, fields),
			};
			json!({ camel_case(&variant.name): inner })
		},
		ValueDef::Composite(Composite::Unnamed(values)) => match ty.map(|ty| &ty.type_def) {
			Some(TypeDef::Sequence(seq)) if is_u8(metadata, seq.type_param.id) => hex(values),
			Some(TypeDef::Array(array)) if is_u8(metadata, array.type_param.id) => hex(values),
			Some(TypeDef::Composite(_)) if values.len() == 1 => to_json(metadata, &values[0]),
			_ => Json::Array(values.iter().map(|v| to_json(metadata, v)).collect()),
		},
		ValueDef::Composite(fields) => fields_json(metadata, fields),
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
fn fields_json(metadata: &Metadata, fields: &Composite<u32>) -> Json {
	match fields {
		Composite::Named(fields) => Json::Object(
			fields
				.iter()
				.map(|(name, value)| (camel_case(name), to_json(metadata, value)))
				.collect::<Map<_, _>>(),
		),
		Composite::Unnamed(values) =>
			Json::Array(values.iter().map(|v| to_json(metadata, v)).collect()),
	}
}

fn is_u8(metadata: &Metadata, id: u32) -> bool {
	metadata
		.types()
		.resolve(id)
		.is_some_and(|ty| matches!(ty.type_def, TypeDef::Primitive(TypeDefPrimitive::U8)))
}

fn hex(values: &[Value<u32>]) -> Json {
	let bytes: Vec<u8> = values
		.iter()
		.filter_map(|value| match value.value {
			ValueDef::Primitive(Primitive::U128(n)) => u8::try_from(n).ok(),
			_ => None,
		})
		.collect();
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
}
