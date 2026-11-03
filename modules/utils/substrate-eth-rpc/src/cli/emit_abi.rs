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
	let (ty, components) = json_type(&field.ty);
	EventParam {
		name: field.name.into(),
		indexed: field.indexed,
		internal_type: InternalType::parse(&ty),
		ty,
		components,
	}
}

fn param(name: &str, ty: &AbiType) -> Param {
	let (ty, components) = json_type(ty);
	Param { name: name.into(), internal_type: InternalType::parse(&ty), ty, components }
}

fn json_type(ty: &AbiType) -> (String, Vec<Param>) {
	match ty {
		AbiType::Tuple(fields) =>
			("tuple".into(), fields.iter().map(|f| param(f.name, &f.ty)).collect()),
		AbiType::Option(inner) =>
			("tuple".into(), vec![param("isSome", &AbiType::Bool), param("value", inner)]),
		other => (other.sol_type(), Vec::new()),
	}
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
}
