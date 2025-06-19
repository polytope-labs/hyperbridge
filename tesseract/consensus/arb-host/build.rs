use ethers_contract_abigen::MultiAbigen;
use std::{env, fs};

fn main() -> anyhow::Result<()> {
	let base_dir = env::current_dir()?.display().to_string();

	let sources = vec![
		("IRollup", format!("{base_dir}/abis/IRollupCore.json")),
		("IRollupBold", format!("{base_dir}/abis/IRollupCoreBold.json")),
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
