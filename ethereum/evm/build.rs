use ethers_contract_abigen::MultiAbigen;
use std::{env, fs};

fn main() -> anyhow::Result<()> {
	let base_dir = env::current_dir()?.display().to_string();

	let sources = vec![
		("L2OutputOracle", format!("{base_dir}/abis/L2OutputOracle.json")),
		("IRollup", format!("{base_dir}/abis/IRollupCore.json")),
		("OVM_gasPriceOracle", format!("{base_dir}/abis/OVM_gasPriceOracle.json")),
		("ArbGasInfo", format!("{base_dir}/abis/ArbGasInfo.json")),
	];

	MultiAbigen::new(sources)
		.unwrap()
		.build()
		.unwrap()
		.write_to_module(format!("{base_dir}/src/abi"), false)
		.unwrap();

	// remove the added mod.rs
	fs::remove_file(format!("{base_dir}/src/abi/mod.rs"))?;

	println!("cargo:rerun-if-changed={base_dir}/abis");

	Ok(())
}
