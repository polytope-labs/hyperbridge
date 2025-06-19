use ethers_contract_abigen::MultiAbigen;
use std::{env, fs};

fn main() -> anyhow::Result<()> {
	let base_dir = env::current_dir()?.display().to_string();

	let sources = vec![
		("L2OutputOracle", format!("{base_dir}/abis/L2OutputOracle.json")),
		("DisputeGameFactory", format!("{base_dir}/abis/DisputeGameFactory.json")),
		("FaultDisputeGame", format!("{base_dir}/abis/FaultDisputeGame.json")),
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
