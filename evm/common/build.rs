use ethers_contract_abigen::MultiAbigen;
use std::{env, fs};

fn main() -> anyhow::Result<()> {
	let base_dir = env::current_dir()?.display().to_string();

	let sources = vec![
		("OVM_gasPriceOracle", format!("{base_dir}/abis/OVM_gasPriceOracle.json")),
		("ArbGasInfo", format!("{base_dir}/abis/ArbGasInfo.json")),
		("Erc20", format!("{base_dir}/abis/ERC20.json")),
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
