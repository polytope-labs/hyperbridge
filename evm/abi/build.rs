use std::env;

fn main() -> anyhow::Result<()> {
	let base_dir = env::current_dir()?.parent().unwrap().display().to_string();

	#[cfg(feature = "build-abi")]
	{
		use ethers_contract_abigen::MultiAbigen;
		use forge_testsuite::Runner;
		use std::path::PathBuf;
		// first compile the project.

		let _ = Runner::new(PathBuf::from(&base_dir));

		let sources = vec![
			("EvmHost", format!("{base_dir}/out/EvmHost.sol/EvmHost.json")),
			("Handler", format!("{base_dir}/out/HandlerV1.sol/HandlerV1.json")),
			("Beefy", format!("{base_dir}/out/BeefyV1.sol/BeefyV1.json")),
			("SP1Beefy", format!("{base_dir}/out/SP1Beefy.sol/SP1Beefy.json")),
			("PingModule", format!("{base_dir}/out/PingModule.sol/PingModule.json")),
			("HostManager", format!("{base_dir}/out/HostManager.sol/HostManager.json")),
			("ERC20", format!("{base_dir}/out/ERC20.sol/ERC20.json")),
		];

		MultiAbigen::new(sources)
			.unwrap()
			.build()
			.unwrap()
			.write_to_module(format!("{base_dir}/abi/src/generated"), false)
			.unwrap();
	}

	println!("cargo:rerun-if-changed={base_dir}/out");

	Ok(())
}
