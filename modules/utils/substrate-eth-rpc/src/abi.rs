use alloy_dyn_abi::DynSolValue;
use alloy_primitives::{keccak256, Address, Bytes, B256, U256};
use ismp::host::StateMachine;
use scale_value::{Composite, Primitive, Value, ValueDef, Variant};

use crate::mapping::{AbiType, FieldSpec, MappingError};

pub fn encode_data<T>(fields: &[(&AbiType, &Value<T>)]) -> Result<Bytes, MappingError> {
	let values = fields
		.iter()
		.map(|(ty, v)| to_dyn_value(ty, v))
		.collect::<Result<Vec<_>, _>>()?;
	Ok(encode_values(values))
}

pub fn encode_values(values: Vec<DynSolValue>) -> Bytes {
	DynSolValue::Tuple(values).abi_encode_params().into()
}

pub fn encode_topic<T>(ty: &AbiType, v: &Value<T>) -> Result<B256, MappingError> {
	topic_word(&to_dyn_value(ty, v)?)
}

pub fn topic_word(value: &DynSolValue) -> Result<B256, MappingError> {
	match value {
		DynSolValue::Bytes(b) => Ok(keccak256(b)),
		DynSolValue::String(s) => Ok(keccak256(s.as_bytes())),
		DynSolValue::Tuple(_) | DynSolValue::Array(_) | DynSolValue::FixedArray(_) =>
			Ok(keccak256(value.abi_encode_params())),
		other => other.as_word().ok_or_else(|| MappingError::TypeMismatch {
			expected: "word-sized value".into(),
			got: format!("{other:?}"),
		}),
	}
}

pub fn lookup<'a, T>(fields: &'a Composite<T>, source: &str) -> Option<&'a Value<T>> {
	match fields {
		Composite::Named(values) => values.iter().find(|(n, _)| n == source).map(|(_, v)| v),
		Composite::Unnamed(values) => source.parse::<usize>().ok().and_then(|i| values.get(i)),
	}
}

pub fn to_dyn_value<T>(ty: &AbiType, v: &Value<T>) -> Result<DynSolValue, MappingError> {
	match ty {
		AbiType::Bytes32 => fixed_bytes(v, 32),
		AbiType::Bytes4 => fixed_bytes(v, 4),
		AbiType::Bytes => Ok(DynSolValue::Bytes(bytes(v)?)),
		AbiType::String => Ok(DynSolValue::String(string(v)?)),
		AbiType::Bool =>
			peel(v).as_bool().map(DynSolValue::Bool).ok_or_else(|| mismatch("bool", v)),
		AbiType::Uint(bits) => Ok(DynSolValue::Uint(uint(v, *bits)?, *bits)),
		AbiType::Address => {
			let raw = bytes(v)?;
			if raw.len() != 20 {
				return Err(mismatch("20 bytes", v));
			}
			Ok(DynSolValue::Address(Address::from_slice(&raw)))
		},
		AbiType::Tuple(fields) => tuple(fields, v),
		AbiType::StateMachine => Ok(DynSolValue::String(state_machine(v)?.to_string())),
		AbiType::Option(inner) => option(inner, v),
		AbiType::Enum(variants) => enum_discriminant(variants, v),
	}
}

pub fn state_machine<T>(v: &Value<T>) -> Result<StateMachine, MappingError> {
	let var = variant(v)?;
	let values = var.values.values().collect::<Vec<_>>();
	match (var.name.as_str(), values.as_slice()) {
		("Evm", [id]) => Ok(StateMachine::Evm(u32_of(id)?)),
		("Polkadot", [id]) => Ok(StateMachine::Polkadot(u32_of(id)?)),
		("Kusama", [id]) => Ok(StateMachine::Kusama(u32_of(id)?)),
		("Substrate", [id]) => Ok(StateMachine::Substrate(array4(id)?)),
		("Tendermint", [id]) => Ok(StateMachine::Tendermint(array4(id)?)),
		("Relay", _) => {
			let relay = lookup(&var.values, "relay")
				.ok_or_else(|| MappingError::MissingField("relay".into()))?;
			let para_id = lookup(&var.values, "para_id")
				.ok_or_else(|| MappingError::MissingField("para_id".into()))?;
			Ok(StateMachine::Relay { relay: array4(relay)?, para_id: u32_of(para_id)? })
		},
		_ => Err(mismatch("StateMachine variant", v)),
	}
}

fn default_value(ty: &AbiType) -> DynSolValue {
	match ty {
		AbiType::Bytes32 => DynSolValue::FixedBytes(B256::ZERO, 32),
		AbiType::Bytes4 => DynSolValue::FixedBytes(B256::ZERO, 4),
		AbiType::Bytes => DynSolValue::Bytes(Vec::new()),
		AbiType::String | AbiType::StateMachine => DynSolValue::String(String::new()),
		AbiType::Bool => DynSolValue::Bool(false),
		AbiType::Uint(bits) => DynSolValue::Uint(U256::ZERO, *bits),
		AbiType::Address => DynSolValue::Address(Address::ZERO),
		AbiType::Enum(_) => DynSolValue::Uint(U256::ZERO, 8),
		AbiType::Tuple(fields) =>
			DynSolValue::Tuple(fields.iter().map(|f| default_value(&f.ty)).collect()),
		AbiType::Option(inner) =>
			DynSolValue::Tuple(vec![DynSolValue::Bool(false), default_value(inner)]),
	}
}

fn fixed_bytes<T>(v: &Value<T>, len: usize) -> Result<DynSolValue, MappingError> {
	let raw = bytes(v)?;
	if raw.len() != len {
		return Err(mismatch(&format!("{len} bytes"), v));
	}
	let mut word = B256::ZERO;
	word[..len].copy_from_slice(&raw);
	Ok(DynSolValue::FixedBytes(word, len))
}

fn tuple<T>(fields: &[FieldSpec], v: &Value<T>) -> Result<DynSolValue, MappingError> {
	let ValueDef::Composite(composite) = &peel(v).value else {
		return Err(mismatch("composite", v));
	};
	let values = fields
		.iter()
		.map(|f| {
			let value = lookup(composite, f.source)
				.ok_or_else(|| MappingError::MissingField(f.source.into()))?;
			to_dyn_value(&f.ty, value).map_err(|source| MappingError::Field {
				event: String::new(),
				field: f.name.into(),
				source: Box::new(source),
			})
		})
		.collect::<Result<Vec<_>, _>>()?;
	Ok(DynSolValue::Tuple(values))
}

fn option<T>(inner: &AbiType, v: &Value<T>) -> Result<DynSolValue, MappingError> {
	let var = variant(v)?;
	let values = var.values.values().collect::<Vec<_>>();
	match (var.name.as_str(), values.as_slice()) {
		("None", []) =>
			Ok(DynSolValue::Tuple(vec![DynSolValue::Bool(false), default_value(inner)])),
		("Some", [value]) =>
			Ok(DynSolValue::Tuple(vec![DynSolValue::Bool(true), to_dyn_value(inner, value)?])),
		_ => Err(mismatch("Option variant", v)),
	}
}

fn enum_discriminant<T>(
	variants: &[(&str, u8)],
	v: &Value<T>,
) -> Result<DynSolValue, MappingError> {
	let var = variant(v)?;
	if !var.values.is_empty() {
		return Err(mismatch("fieldless variant", v));
	}
	variants
		.iter()
		.find(|(name, _)| *name == var.name)
		.map(|(_, disc)| DynSolValue::Uint(U256::from(*disc), 8))
		.ok_or_else(|| mismatch("known enum variant", v))
}

// Newtype wrappers such as `H256([u8; 32])`, `AccountId32([u8; 32])` and `BoundedVec(Vec<u8>)`
// decode as a single-element unnamed composite around the payload.
fn peel<T>(v: &Value<T>) -> &Value<T> {
	let mut current = v;
	loop {
		let ValueDef::Composite(Composite::Unnamed(values)) = &current.value else {
			return current;
		};
		match values.as_slice() {
			[inner] if matches!(inner.value, ValueDef::Composite(_)) => current = inner,
			_ => return current,
		}
	}
}

fn variant<T>(v: &Value<T>) -> Result<&Variant<T>, MappingError> {
	match &peel(v).value {
		ValueDef::Variant(var) => Ok(var),
		_ => Err(mismatch("variant", v)),
	}
}

fn bytes<T>(v: &Value<T>) -> Result<Vec<u8>, MappingError> {
	match &peel(v).value {
		ValueDef::Primitive(Primitive::String(s)) => Ok(s.as_bytes().to_vec()),
		ValueDef::Composite(composite) => composite
			.values()
			.map(|b| u128_of(b).and_then(|n| u8::try_from(n).map_err(|_| mismatch("byte", b))))
			.collect(),
		_ => Err(mismatch("bytes", v)),
	}
}

fn string<T>(v: &Value<T>) -> Result<String, MappingError> {
	if let Some(s) = peel(v).as_str() {
		return Ok(s.to_string());
	}
	String::from_utf8(bytes(v)?).map_err(|_| mismatch("utf-8 string", v))
}

fn array4<T>(v: &Value<T>) -> Result<[u8; 4], MappingError> {
	bytes(v)?.try_into().map_err(|_| mismatch("4 bytes", v))
}

fn u128_of<T>(v: &Value<T>) -> Result<u128, MappingError> {
	peel(v).as_u128().ok_or_else(|| mismatch("unsigned integer", v))
}

fn u32_of<T>(v: &Value<T>) -> Result<u32, MappingError> {
	u32::try_from(u128_of(v)?).map_err(|_| mismatch("u32", v))
}

fn uint<T>(v: &Value<T>, bits: usize) -> Result<U256, MappingError> {
	let peeled = peel(v);
	let value = match &peeled.value {
		ValueDef::Primitive(Primitive::U128(n)) => U256::from(*n),
		ValueDef::Primitive(Primitive::U256(le)) => U256::from_le_bytes(*le),
		ValueDef::Composite(composite) if composite.len() == 4 => {
			let mut limbs = [0u64; 4];
			for (limb, value) in limbs.iter_mut().zip(composite.values()) {
				*limb = u64::try_from(u128_of(value)?).map_err(|_| mismatch("u64 limb", value))?;
			}
			U256::from_limbs(limbs)
		},
		ValueDef::Composite(composite) if composite.len() == 32 => {
			let raw: [u8; 32] = bytes(peeled)?.try_into().expect("length checked");
			U256::from_le_bytes(raw)
		},
		_ => return Err(mismatch(&format!("uint{bits}"), v)),
	};
	if value.bit_len() > bits {
		return Err(MappingError::TypeMismatch {
			expected: format!("uint{bits}"),
			got: format!("{value} ({} bits)", value.bit_len()),
		});
	}
	Ok(value)
}

fn mismatch<T>(expected: &str, v: &Value<T>) -> MappingError {
	MappingError::TypeMismatch { expected: expected.into(), got: describe(v) }
}

fn describe<T>(v: &Value<T>) -> String {
	match &v.value {
		ValueDef::Composite(Composite::Named(values)) =>
			format!("named composite({})", values.len()),
		ValueDef::Composite(Composite::Unnamed(values)) =>
			format!("unnamed composite({})", values.len()),
		ValueDef::Variant(var) => format!("variant {}({})", var.name, var.values.len()),
		ValueDef::BitSequence(_) => "bit sequence".into(),
		ValueDef::Primitive(p) => format!("{p:?}"),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use alloy_primitives::b256;

	fn sm(name: &str, values: Vec<Value<()>>) -> Value<()> {
		Value::unnamed_variant(name, values)
	}

	#[test]
	fn formats_state_machines() {
		let cases = [
			(sm("Evm", vec![Value::u128(1)]), "EVM-1"),
			(sm("Polkadot", vec![Value::u128(3367)]), "POLKADOT-3367"),
			(sm("Kusama", vec![Value::u128(4009)]), "KUSAMA-4009"),
			(sm("Substrate", vec![Value::from_bytes(b"cere")]), "SUBSTRATE-cere"),
			(sm("Tendermint", vec![Value::from_bytes(b"osmo")]), "TNDRMINT-osmo"),
			(
				Value::named_variant(
					"Relay",
					[("relay", Value::from_bytes(b"pase")), ("para_id", Value::u128(1000))],
				),
				"RELAY-pase-1000",
			),
		];
		for (value, expected) in cases {
			assert_eq!(state_machine(&value).unwrap().to_string(), expected);
			assert_eq!(
				to_dyn_value(&AbiType::StateMachine, &value).unwrap(),
				DynSolValue::String(expected.into())
			);
		}
	}

	#[test]
	fn rejects_unknown_state_machine_variant() {
		let err = state_machine(&sm("Solana", vec![Value::u128(1)])).unwrap_err();
		assert!(matches!(err, MappingError::TypeMismatch { .. }), "{err}");
	}

	#[test]
	fn uint_accepts_every_u256_layout() {
		let expected: U256 = U256::from(0x1234_5678_9abc_def0_u128) << 64;
		let limbs = Value::unnamed_composite([Value::unnamed_composite([
			Value::u128(0),
			Value::u128(0x1234_5678_9abc_def0),
			Value::u128(0),
			Value::u128(0),
		])]);
		let le_bytes = Value::from_bytes(expected.to_le_bytes::<32>());
		let primitive = Value::primitive(Primitive::U256(expected.to_le_bytes::<32>()));
		for value in [limbs, le_bytes, primitive] {
			assert_eq!(uint(&value, 256).unwrap(), expected);
		}
		assert_eq!(uint(&Value::u128(7), 64).unwrap(), U256::from(7));
	}

	#[test]
	fn uint_rejects_out_of_range() {
		let err = uint(&Value::u128(256), 8).unwrap_err();
		assert!(matches!(err, MappingError::TypeMismatch { .. }), "{err}");
	}

	#[test]
	fn bytes32_unwraps_account_id_newtype() {
		let account = Value::unnamed_composite([Value::from_bytes([0xab; 32])]);
		assert_eq!(
			to_dyn_value(&AbiType::Bytes32, &account).unwrap(),
			DynSolValue::FixedBytes(B256::repeat_byte(0xab), 32)
		);
		let short = Value::unnamed_composite([Value::from_bytes([0xab; 31])]);
		assert!(to_dyn_value(&AbiType::Bytes32, &short).is_err());
	}

	#[test]
	fn bytes_unwraps_bounded_vec() {
		let bounded = Value::unnamed_composite([Value::from_bytes([1, 2, 3])]);
		assert_eq!(
			to_dyn_value(&AbiType::Bytes, &bounded).unwrap(),
			DynSolValue::Bytes(vec![1, 2, 3])
		);
		let single = Value::unnamed_composite([Value::from_bytes([9])]);
		assert_eq!(to_dyn_value(&AbiType::Bytes, &single).unwrap(), DynSolValue::Bytes(vec![9]));
		let empty = Value::unnamed_composite([Value::from_bytes([])]);
		assert_eq!(to_dyn_value(&AbiType::Bytes, &empty).unwrap(), DynSolValue::Bytes(vec![]));
	}

	#[test]
	fn option_encodes_as_flag_and_payload() {
		let ty = AbiType::Option(Box::new(AbiType::Uint(64)));
		assert_eq!(
			to_dyn_value(&ty, &Value::unnamed_variant("None", Vec::<Value<()>>::new())).unwrap(),
			DynSolValue::Tuple(vec![DynSolValue::Bool(false), DynSolValue::Uint(U256::ZERO, 64)])
		);
		assert_eq!(
			to_dyn_value(&ty, &Value::unnamed_variant("Some", [Value::u128(5)])).unwrap(),
			DynSolValue::Tuple(vec![DynSolValue::Bool(true), DynSolValue::Uint(U256::from(5), 64)])
		);
	}

	#[test]
	fn enum_maps_variant_name_to_discriminant() {
		let ty = AbiType::Enum(&[("A", 1), ("B", 7)]);
		assert_eq!(
			to_dyn_value(&ty, &Value::unnamed_variant("B", Vec::<Value<()>>::new())).unwrap(),
			DynSolValue::Uint(U256::from(7), 8)
		);
		assert!(to_dyn_value(&ty, &Value::unnamed_variant("C", Vec::<Value<()>>::new())).is_err());
	}

	#[test]
	fn topics_follow_solidity_indexed_rules() {
		assert_eq!(
			encode_topic(&AbiType::Bytes32, &Value::from_bytes([0x11; 32])).unwrap(),
			B256::repeat_byte(0x11)
		);
		let mut word = B256::ZERO;
		word[31] = 42;
		assert_eq!(encode_topic(&AbiType::Uint(64), &Value::u128(42)).unwrap(), word);
		assert_eq!(encode_topic(&AbiType::Bool, &Value::bool(true)).unwrap(), {
			let mut w = B256::ZERO;
			w[31] = 1;
			w
		});
		assert_eq!(
			encode_topic(&AbiType::String, &Value::string("")).unwrap(),
			b256!("c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470")
		);
	}

	#[test]
	fn encode_data_is_a_flat_parameter_list() {
		let data = encode_data(&[
			(&AbiType::Uint(64), &Value::u128(1)),
			(&AbiType::Bool, &Value::bool(true)),
		])
		.unwrap();
		assert_eq!(data.len(), 64);
		assert_eq!(data[31], 1);
		assert_eq!(data[63], 1);
		assert!(encode_data::<()>(&[]).unwrap().is_empty());
	}
}
