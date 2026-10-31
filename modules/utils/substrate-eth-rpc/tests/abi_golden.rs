use std::{collections::BTreeSet, fs, path::PathBuf};

use alloy_json_abi::JsonAbi;
use substrate_eth_rpc::{cli::emit_abi::render_abi, mapping::all_pallets};

const REGEN: &str = "rerun `SKIP_WASM_BUILD=1 cargo run -p substrate-eth-rpc -- emit-abi --out modules/utils/substrate-eth-rpc/abis`";

fn abis_dir() -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("abis")
}

fn abi_path(key: &str) -> PathBuf {
	abis_dir().join(format!("{key}.abi.json"))
}

fn read(key: &str) -> String {
	let path = abi_path(key);
	fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}; {REGEN}", path.display()))
}

#[test]
fn committed_abis_match_mapping_table() {
	let pallets = all_pallets();
	for spec in &pallets {
		assert!(
			read(spec.key) == render_abi(spec),
			"{} does not match the mapping table; {REGEN}",
			abi_path(spec.key).display()
		);
	}
	let committed = fs::read_dir(abis_dir())
		.unwrap()
		.map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
		.collect::<BTreeSet<_>>();
	let expected = pallets.iter().map(|p| format!("{}.abi.json", p.key)).collect::<BTreeSet<_>>();
	assert_eq!(committed, expected, "abis/ has files that no pallet produces; delete them");
}

#[test]
fn committed_abis_parse_and_match_topics() {
	for spec in all_pallets() {
		let abi: JsonAbi =
			serde_json::from_str(&read(spec.key)).unwrap_or_else(|e| panic!("{}: {e}", spec.key));
		assert!(abi.functions.is_empty() && abi.errors.is_empty(), "{}", spec.key);
		assert_eq!(
			abi.events.values().map(Vec::len).sum::<usize>(),
			spec.events.len(),
			"{}",
			spec.key
		);
		for event in &spec.events {
			let parsed = abi
				.events
				.get(event.name)
				.and_then(|e| e.first())
				.unwrap_or_else(|| panic!("{}.{} missing", spec.key, event.name));
			assert_eq!(parsed.signature(), event.signature(), "{}.{}", spec.key, event.name);
			assert_eq!(parsed.selector(), event.topic0(), "{}.{}", spec.key, event.name);
			assert_eq!(parsed.inputs.len(), event.fields.len(), "{}.{}", spec.key, event.name);
			for (input, field) in parsed.inputs.iter().zip(&event.fields) {
				assert_eq!(input.name, field.name, "{}.{}", spec.key, event.name);
				assert_eq!(input.indexed, field.indexed, "{}.{}", spec.key, event.name);
			}
		}
	}
}
