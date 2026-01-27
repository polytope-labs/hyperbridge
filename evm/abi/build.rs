use std::env;

fn main() -> anyhow::Result<()> {
	let base_dir = env::current_dir()?.parent().unwrap().display().to_string();

	// This no longer builds because of cancun
	#[cfg(feature = "build-abi")]
	{
		use ethers_contract_abigen::MultiAbigen;
		use ethers_solc::{remappings::Remapping, Project, ProjectPathsConfig, Solc, SolcConfig};
		use std::{fs, path::PathBuf};

		// Set up paths
		let root = PathBuf::from(&base_dir);
		let mut paths = ProjectPathsConfig::builder()
			.root(&root)
			.sources(root.join("src"))
			.lib(root.join("lib"))
			.build()?;

		// Parse remappings from remappings.txt
		let remappings_path = root.join("remappings.txt");
		if remappings_path.exists() {
			let remappings_content = fs::read_to_string(&remappings_path)?;
			for line in remappings_content.lines() {
				let line = line.trim();
				if line.is_empty() || line.starts_with('#') {
					continue;
				}
				if let Some((name, path)) = line.split_once('=') {
					let remapping = Remapping {
						context: None,
						name: name.to_string(),
						path: root.join(path).to_string_lossy().to_string(),
					};
					// Remove any existing remapping with the same name
					paths.remappings.retain(|r| r.name != remapping.name);
					paths.remappings.push(remapping);
				}
			}
		}

		// Configure solc with a specific version and optimizations
		let mut solc_config = SolcConfig::builder().build();
		solc_config.settings.optimizer.enabled = Some(true);
		solc_config.settings.optimizer.runs = Some(200);

		// Set a specific Solidity version to avoid multi-version compilation
		// This prevents artifacts like HeaderImpl.0.8.20.json and HeaderImpl.0.8.30.json
		let solc = Solc::find_svm_installed_version("0.8.30")
			.map_err(|e| anyhow::anyhow!("Failed to find or install Solidity 0.8.30: {}", e))?
			.ok_or_else(|| anyhow::anyhow!("Failed to find or install Solidity 0.8.30"))?;

		solc_config.settings.evm_version = Some(ethers_solc::EvmVersion::Shanghai);

		// Create the project with fixed version
		let project = Project::builder()
			.paths(paths)
			.solc_config(solc_config)
			.solc(solc)
			.set_auto_detect(false) // Disable auto-detection to avoid multi-version artifacts
			.build()?;

		// Compile the project
		println!("cargo:warning=Compiling Solidity contracts with version 0.8.30...");
		let compiled =
			project.compile().map_err(|e| anyhow::anyhow!("Compilation failed: {}", e))?;

		if compiled.has_compiler_errors() {
			eprintln!("Solidity compilation errors:");
			for error in compiled.output().errors.iter() {
				if error.severity.is_error() {
					eprintln!("  {}", error);
				}
			}
			anyhow::bail!("Solidity compilation failed with errors");
		}

		println!("cargo:warning=Compilation successful, generating Rust bindings...");

		// Generate Rust bindings for the specified contracts
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
			.map_err(|e| anyhow::anyhow!("MultiAbigen creation failed: {}", e))?
			.build()
			.map_err(|e| anyhow::anyhow!("Binding generation failed: {}", e))?
			.write_to_module(format!("{base_dir}/abi/src/generated"), false)
			.map_err(|e| anyhow::anyhow!("Failed to write bindings: {}", e))?;

		println!("cargo:warning=Rust bindings generated successfully");
	}

	println!("cargo:rerun-if-changed={base_dir}/src");
	println!("cargo:rerun-if-changed={base_dir}/foundry.toml");
	println!("cargo:rerun-if-changed={base_dir}/remappings.txt");

	Ok(())
}
