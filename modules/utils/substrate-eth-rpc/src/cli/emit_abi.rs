use std::{
	fs,
	path::{Path, PathBuf},
};

use alloy_json_abi::{Event, EventParam, InternalType, Param};
use anyhow::Context;

use crate::mapping::{all_pallets, AbiType, EventSpec, FieldSpec, PalletSpec};

pub fn pallet_abi(spec: &PalletSpec) -> Vec<Event> {
	spec.events.iter().map(event).collect()
}

// Serialized straight from the alloy structs: a `serde_json::Value` round trip would order
// keys by whether `serde_json/preserve_order` is enabled anywhere in the build.
pub fn render_abi(spec: &PalletSpec) -> String {
	let mut json =
		serde_json::to_string_pretty(&pallet_abi(spec)).expect("abi events serialize to json");
	json.push('\n');
	json
}

pub fn write_abis(out: &Path) -> anyhow::Result<Vec<PathBuf>> {
	fs::create_dir_all(out).with_context(|| format!("creating {}", out.display()))?;
	all_pallets()
		.iter()
		.map(|spec| {
			let path = out.join(format!("{}.abi.json", spec.key));
			fs::write(&path, render_abi(spec))
				.with_context(|| format!("writing {}", path.display()))?;
			Ok(path)
		})
		.collect()
}

fn event(spec: &EventSpec) -> Event {
	Event {
		name: spec.name.into(),
		inputs: spec.fields.iter().map(event_param).collect(),
		anonymous: false,
	}
}

fn event_param(field: &FieldSpec) -> EventParam {
	let struct_name = pascal_case(field.name);
	let (ty, internal_type, components) = json_type(&field.ty, &struct_name);
	EventParam { name: field.name.into(), indexed: field.indexed, internal_type, ty, components }
}

fn param(name: &str, ty: &AbiType, struct_name: &str) -> Param {
	let (ty, internal_type, components) = json_type(ty, struct_name);
	Param { name: name.into(), internal_type, ty, components }
}

/**
 * Tuples are tagged `struct <Name>` like solc output, so ABI code generators can name them. The
 * name is the field path in PascalCase: `stateMachineHeight.id` is `StateMachineHeightId`.
 */
fn json_type(ty: &AbiType, struct_name: &str) -> (String, Option<InternalType>, Vec<Param>) {
	let tuple = |components| {
		let internal_type = InternalType::Struct { contract: None, ty: struct_name.into() };
		("tuple".to_string(), Some(internal_type), components)
	};
	match ty {
		AbiType::Tuple(fields) => tuple(
			fields
				.iter()
				.map(|f| param(f.name, &f.ty, &format!("{struct_name}{}", pascal_case(f.name))))
				.collect(),
		),
		AbiType::Option(inner) => tuple(vec![
			param("isSome", &AbiType::Bool, ""),
			param("value", inner, &format!("{struct_name}Value")),
		]),
		other => {
			let ty = other.sol_type();
			let internal_type = InternalType::parse(&ty);
			(ty, internal_type, Vec::new())
		},
	}
}

fn pascal_case(name: &str) -> String {
	let mut chars = name.chars();
	chars.next().map(|c| c.to_ascii_uppercase().to_string() + chars.as_str()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn render_abi_key_order_is_fixed() {
		let json = render_abi(&all_pallets()[0]);
		let event = json.find("\"type\": \"event\"").expect("event tag");
		let name = json.find("\"name\"").expect("name");
		let inputs = json.find("\"inputs\"").expect("inputs");
		let anonymous = json.find("\"anonymous\"").expect("anonymous");
		assert!(event < name && name < inputs && inputs < anonymous, "{json}");
		assert!(
			json.contains("\"indexed\": false,\n        \"internalType\": \"") ||
				json.contains("\"indexed\": true,\n        \"internalType\": \""),
			"{json}"
		);
	}

	/** Code generators declare one struct per name, so a name can't stand for two shapes. */
	#[test]
	fn struct_names_have_one_shape_per_pallet() {
		fn collect(params: &[Param], shapes: &mut Vec<(String, String)>) {
			for param in params {
				if let Some(InternalType::Struct { ty, .. }) = &param.internal_type {
					shapes.push((ty.clone(), serde_json::to_string(&param.components).unwrap()));
				}
				collect(&param.components, shapes);
			}
		}
		for spec in all_pallets() {
			let mut shapes = vec![];
			for event in pallet_abi(&spec) {
				let params: Vec<Param> = event
					.inputs
					.iter()
					.map(|input| Param {
						name: input.name.clone(),
						ty: input.ty.clone(),
						internal_type: input.internal_type.clone(),
						components: input.components.clone(),
					})
					.collect();
				collect(&params, &mut shapes);
			}
			let mut by_name = std::collections::BTreeMap::new();
			for (name, shape) in shapes {
				let first = by_name.entry(name.clone()).or_insert_with(|| shape.clone());
				assert_eq!(*first, shape, "{}: struct {name} has two shapes", spec.key);
			}
		}
	}
}
